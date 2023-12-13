use std::time::Duration;

use celestia_client::{
    celestia_rpc::HeaderClient as _,
    celestia_types::{
        nmt::Namespace,
        Height,
    },
    jsonrpsee::http_client::HttpClient,
    CelestiaClientExt as _,
    CelestiaSequencerBlob,
};
use color_eyre::eyre::{
    self,
    bail,
    WrapErr as _,
};
use futures::{
    future::FusedFuture,
    FutureExt,
};
use proto::native::sequencer::v1alpha1::RollupId;
use sequencer_client::SequencerBlock;
use tendermint::{
    block::Header,
    Hash,
};
use tokio::{
    select,
    sync::{
        mpsc,
        oneshot,
    },
    task::{
        self,
        JoinError,
        JoinHandle,
        JoinSet,
    },
};
use tokio_util::task::JoinMap;
use tracing::{
    debug,
    error,
    info,
    instrument,
    warn,
    Instrument,
};

use crate::{
    // block_verifier::BlockVerifier,
    executor,
};
mod block_verifier;
mod sync;
use block_verifier::BlockVerifier;

/// `SequencerBlockSubset` is a subset of a `SequencerBlock` that contains
/// information required for transaction data verification, and the transactions
/// for one specific rollup.
#[derive(Clone, Debug)]
pub(crate) struct SequencerBlockSubset {
    pub(crate) block_hash: Hash,
    pub(crate) header: Header,
    pub(crate) transactions: Vec<Vec<u8>>,
}

impl SequencerBlockSubset {
    pub(crate) fn from_sequencer_block(block: SequencerBlock, chain_id: RollupId) -> Self {
        let mut block = block.into_unchecked();
        let header = block.header;
        let block_hash = header.hash();
        let transactions = block
            .rollup_transactions
            .remove(&chain_id)
            .unwrap_or_default();
        Self {
            block_hash,
            header,
            transactions,
        }
    }
}

pub(crate) struct Reader {
    /// The channel used to send messages to the executor task.
    executor_tx: executor::Sender,

    /// The client used to communicate with Celestia.
    celestia_client: HttpClient,

    /// The between the reader waits until it queries celestia for new blocks
    celestia_poll_interval: Duration,

    /// the last block height fetched from Celestia
    current_block_height: Height,

    block_verifier: BlockVerifier,

    /// Sequencer Namespace ID
    sequencer_namespace: Namespace,
    /// Rollup Namespace ID
    rollup_namespace: Namespace,

    get_latest_height: Option<JoinHandle<eyre::Result<Height>>>,

    /// A map of in-flight queries to celestia for new sequencer blobs at a given height
    fetch_sequencer_blobs_at_height: JoinMap<Height, eyre::Result<Vec<CelestiaSequencerBlob>>>,

    /// A map of futures verifying that sequencer blobs read off celestia stem from sequencer
    /// before collecting their constituent rollup blobs. One task per celestia height.
    verify_sequencer_blobs_and_assemble_rollups:
        JoinMap<Height, eyre::Result<Vec<SequencerBlockSubset>>>,

    /// Initial DA block height
    initial_da_block_height: u32,

    /// Firm commit height from the rollup
    firm_commit_height: u32,

    shutdown: oneshot::Receiver<()>,

    /// The sync-done channel to notify `Conductor` that `Reader` has finished syncing.
    sync_done: Option<oneshot::Sender<()>>,
}

pub(crate) struct CelestiaReaderConfig {
    pub(crate) node_url: String,
    pub(crate) bearer_token: Option<String>,
    pub(crate) poll_interval: Duration,
}

impl Reader {
    /// Creates a new Reader instance and returns a command sender.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn new(
        initial_da_height: u32,
        firm_commit_height: u32,
        celestia_config: CelestiaReaderConfig,
        executor_tx: executor::Sender,
        sequencer_client_pool: deadpool::managed::Pool<crate::client_provider::ClientProvider>,
        sequencer_namespace: Namespace,
        rollup_namespace: Namespace,
        shutdown: oneshot::Receiver<()>,
        sync_done: oneshot::Sender<()>,
    ) -> eyre::Result<Self> {
        use celestia_client::celestia_rpc::HeaderClient;

        info!("creating da reader");

        let block_verifier = BlockVerifier::new(sequencer_client_pool);

        let celestia_client::celestia_rpc::Client::Http(celestia_client) =
            celestia_client::celestia_rpc::Client::new(
                &celestia_config.node_url,
                celestia_config.bearer_token.as_deref(),
            )
            .await
            .wrap_err("failed constructing celestia http client")?
        else {
            bail!("expected a celestia HTTP client but got a websocket client");
        };

        // TODO: we should probably pass in the height we want to start at from some genesis/config
        // file
        let current_block_height = celestia_client
            .header_network_head()
            .await
            .wrap_err("failed to get network head from celestia to extract latest head")?
            .header
            .height;

        info!(da_block_height = %current_block_height, "creating Reader");

        Ok(Self {
            executor_tx,
            celestia_client,
            celestia_poll_interval: celestia_config.poll_interval,
            current_block_height,
            get_latest_height: None,
            fetch_sequencer_blobs_at_height: JoinMap::new(),
            verify_sequencer_blobs_and_assemble_rollups: JoinMap::new(),
            block_verifier,
            sequencer_namespace,
            rollup_namespace,
            initial_da_block_height: initial_da_height,
            firm_commit_height,
            shutdown,
            sync_done: Some(sync_done),
        })
    }

    #[instrument(skip(self))]
    pub(crate) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        // Setup a dummy future that is always ready to be polled to be checked
        // if we don't need to sync from DA.
        #[allow(clippy::unused_async)]
        async fn ready() -> eyre::Result<()> {
            Ok(())
        }
        let mut sync = ready().boxed().fuse();

        info!("starting reader event loop.");

        // If the firm commit height is greater than 1, that means we have seen
        // sequencer blocks from DA before and we need to sync to the latest.
        // If the firm commit height is 0, that means we are starting the
        // conductor on a fresh chain and we don't need to sync.
        // TODO: make this check more sophisticated
        // if current block height / 5 >  1 -> run the sync
        // if self.firm_commit_height > 1 {
        let current_block_height = u32::try_from(self.current_block_height.value())
            .expect("casting from u64 to u32 failed");
        info!(current_block_height = %current_block_height, initial_da_block_height = %self.initial_da_block_height);
        if self.initial_da_block_height > current_block_height {
            bail!(
                "initial da block height is greater than the current block height; this should \
                 never happen"
            );
        }
        if (current_block_height - self.initial_da_block_height) >= 1 {
            info!("initializing da sync");
            let sync_start_height = find_da_sync_start_height(
                self.celestia_client.clone(),
                self.initial_da_block_height,
                self.firm_commit_height,
                self.sequencer_namespace,
            )
            .await;

            let latest_height = u32::try_from(
                find_latest_height(self.celestia_client.clone())
                    .await
                    .wrap_err("failed to get latest height from celestia")?
                    .value(),
            )
            .expect("casting from u64 to u32 failed");

            info!(start_sync_height = %sync_start_height, end_sync_height = %latest_height, "sync range");

            sync = sync::run(
                sync_start_height,
                latest_height,
                self.sequencer_namespace,
                self.executor_tx.clone(),
                self.celestia_client.clone(),
                self.block_verifier.clone(),
            )
            .boxed()
            .fuse();
        } else {
            info!(
                "current da block height is the same as the initial da block height; skipping da \
                 sync"
            );
        }

        // panic!("stopping here for testing");
        // TODO: add da sync done to asycn run the sync
        // TODO: add sync done check to the event loop below

        let mut interval = tokio::time::interval(self.celestia_poll_interval);
        loop {
            let executor_tx = self.executor_tx.clone();

            select!(
                shutdown_res = &mut self.shutdown => {
                    match shutdown_res {
                        Ok(()) => info!("received shutdown command; exiting"),
                        Err(e) => {
                            let error = &e as &(dyn std::error::Error + 'static);
                            warn!(error, "shutdown receiver dropped; exiting");
                        }
                    }
                    break;
                }

                _ = interval.tick() => {
                    let client = self.celestia_client.clone();
                    self.get_latest_height = Some(tokio::spawn(async move {
                        Ok(client.header_network_head().await?.header.height)
                    }));
                }

                res = &mut sync, if !sync.is_terminated() => {
                    if let Err(e) = res {
                        let error: &(dyn std::error::Error + 'static) = e.as_ref();
                        warn!(error, "sync failed; continuing with normal operation");
                    } else {
                        info!("sync finished successfully");
                    }
                    // First sync at startup: notify conductor that sync is done.
                    // Every resync after: don't.
                    if let Some(sync_done) = self.sync_done.take() {
                        let _ = sync_done.send(());
                    }
                }

                res = async { self.get_latest_height.as_mut().unwrap().await }, if self.get_latest_height.is_some() => {
                    self.get_latest_height = None;
                    self.fetch_sequencer_blobs_up_to_latest_height(res);
                }

                Some((height, res)) = self.fetch_sequencer_blobs_at_height.join_next(), if !self.fetch_sequencer_blobs_at_height.is_empty() => {
                    self.process_sequencer_blobs(height, res);
                }

                Some((height, res)) = self.verify_sequencer_blobs_and_assemble_rollups.join_next(), if !self.verify_sequencer_blobs_and_assemble_rollups.is_empty() => {
                    let span = tracing::info_span!("send_sequencer_subsets", %height);
                    span.in_scope(|| send_sequencer_subsets(executor_tx.clone(), res))
                        .wrap_err("failed sending sequencer subsets to executor")?;
                }
            );
        }
        Ok(())
    }

    /// Starts fetching sequencer blobs for each height between `self.current_height`
    /// and `latest_height` returned by celestia, populating `fetch_sequencer_blobs_at_height`.
    ///
    /// Note that this method evaluates the return value of the `fetch_latest_height` task. If it
    /// failed no heights are fetched.
    fn fetch_sequencer_blobs_up_to_latest_height(
        &mut self,
        latest_height_res: Result<eyre::Result<Height>, JoinError>,
    ) {
        info!("fetching DA blobs up to latest height");
        let latest_height = match latest_height_res {
            Err(e) => {
                let error = &e as &(dyn std::error::Error + 'static);
                warn!(error, "task querying celestia for latest height failed");
                return;
            }

            Ok(Err(e)) => {
                let error: &(dyn std::error::Error + 'static) = e.as_ref();
                warn!(
                    error,
                    "task querying celestia for latest height returned with an error"
                );
                return;
            }

            Ok(Ok(height)) => height,
        };

        if latest_height <= self.current_block_height {
            info!(
                height.celestia = %latest_height,
                height.previous = %self.current_block_height,
                "no new celestia height; not spawning tasks to fetch sequencer blocks"
            );
            return;
        }
        let first_new_height = self.current_block_height.increment();
        info!(
            height.start = %first_new_height,
            height.end = %latest_height,
            "spawning tasks to fetch sequencer blocks for different celestia heights",
        );
        assert!(
            first_new_height.value() <= latest_height.value(),
            "start of range is greater than end of range"
        );

        for height in first_new_height.value()..=latest_height.value() {
            let height = height.try_into().expect(
                "should be able to convert the u64 back to Height because it was obtained from \
                 Height::value",
            );
            let client = self.celestia_client.clone();
            if self.fetch_sequencer_blobs_at_height.contains_key(&height) {
                warn!(
                    %height,
                    "getting sequencer data from celestia already in flight, not spawning"
                );
            } else {
                let sequencer_namespace = self.sequencer_namespace;
                self.fetch_sequencer_blobs_at_height
                    .spawn(height, async move {
                        client
                            .get_sequencer_blobs(height, sequencer_namespace)
                            .await
                            .wrap_err("failed to fetch sequencer data from celestia")
                            .map(|rsp| rsp.sequencer_blobs)
                    });
            }
        }
    }

    #[instrument(skip_all, fields(height))]
    fn process_sequencer_blobs(
        &mut self,
        height: Height,
        sequencer_blob_res: Result<eyre::Result<Vec<CelestiaSequencerBlob>>, JoinError>,
    ) {
        let sequencer_data = match sequencer_blob_res {
            Err(e) => {
                let error = &e as &(dyn std::error::Error + 'static);
                warn!(error, "task querying celestia for sequencer data failed");
                return;
            }

            Ok(Err(e)) => {
                let error: &(dyn std::error::Error + 'static) = e.as_ref();
                warn!(
                    error,
                    "task querying celestia for sequencer data returned with an error"
                );
                return;
            }

            Ok(Ok(sequencer_data)) => sequencer_data,
        };

        // Set the current block height to the maximum height seen. Having reached this
        // handler means that we have successfully received valid (but unverified) sequencer
        // data at celestia `height`. If the next steps fail that is fine: re-requesting
        // the data will not change the verification failure.
        // If there are other tasks querying celestia for lower heights are still in
        // flight they are unaffected and will still be processed here.
        self.current_block_height = std::cmp::max(self.current_block_height, height);
        if self
            .verify_sequencer_blobs_and_assemble_rollups
            .contains_key(&height)
        {
            error!(
                "sequencer data is already being processed; no two sequencer data responses \
                 should have been received; this is a bug"
            );
            return;
        }
        self.verify_sequencer_blobs_and_assemble_rollups.spawn(
            height,
            verify_sequencer_blobs_and_assemble_rollups(
                height,
                sequencer_data,
                self.celestia_client.clone(),
                self.block_verifier.clone(),
                self.rollup_namespace,
            )
            .in_current_span(),
        );
    }
}

async fn find_latest_height(celestia_client: HttpClient) -> eyre::Result<Height> {
    use celestia_client::celestia_rpc::HeaderClient;
    Ok(celestia_client.header_network_head().await?.header.height)
}

#[instrument(skip_all, fields(height))]
fn send_sequencer_subsets(
    executor_tx: executor::Sender,
    sequencer_subsets_res: Result<eyre::Result<Vec<SequencerBlockSubset>>, JoinError>,
) -> eyre::Result<()> {
    let subsets = match sequencer_subsets_res {
        Err(e) => {
            let error = &e as &(dyn std::error::Error + 'static);
            warn!(error, "task processing sequencer data failed");
            return Ok(());
        }
        Ok(Err(e)) => {
            let error: &(dyn std::error::Error + 'static) = e.as_ref();
            warn!(
                error,
                "task processing sequencer data returned with an error"
            );
            return Ok(());
        }
        Ok(Ok(subsets)) => subsets,
    };
    executor_tx
        .send(executor::ExecutorCommand::FromCelestia(subsets))
        .wrap_err("failed sending processed sequencer subsets: executor channel is closed")
}

// TODO: add a pub da_find_start_block_height fn to use when starting Conductor
async fn find_da_sync_start_height(
    client: HttpClient,
    initial_da_height: u32,
    firm_commit_height: u32,
    sequencer_namespace: Namespace,
    // block_verifier: BlockVerifier,
) -> u32 {
    info!("finding da sync start height");

    // Celestia block time is approximately 12 seconds, the sequencer has an approximately 2
    // second block time. This means that ideally there are 6 sequencer blocks
    // in every Celestia block, but that may not always be the case. In
    // practice, the number of sequencer blocks per Celestia block change
    // between 4 and 5. We set the initial guess height assuming the minimum 4 sequencer blocks per
    // Celestia block to make sure that we are below the DA block that contained the most
    // recently executed firm commit on the rollup, then search forward to find the exact
    // Celestia block that contained it.
    // This search is required because there is no way to poll Celestia for
    // which block contianed a given sequencer block.
    info!(initial_da_height = %initial_da_height);
    info!(firm_commit_height = %firm_commit_height);

    // TODO: this is a problem because of the 3 empty blocks issue
    if firm_commit_height == 0 {
        return initial_da_height;
    }

    let mut guess_blob_height = Height::from(initial_da_height + (firm_commit_height / 4));
    info!(guess_block_height = %guess_blob_height, "guessing da block height before simple search");
    guess_blob_height = sync::find_first_da_block_with_sequencer_data(
        guess_blob_height,
        client.clone(),
        sequencer_namespace,
    )
    .await;
    info!(guess_block_height = %guess_blob_height, "guess da block height after simple search");

    // TODO: starting at the guess block height, search forward until we find
    // the block that contains the firm commit
    loop {
        let height = guess_blob_height.value();

        let sequencer_blobs = client
            .get_sequencer_blobs(height, sequencer_namespace)
            .await;

        match sequencer_blobs {
            Ok(blobs) => {
                if blobs.sequencer_blobs.is_empty() {
                    warn!(height = %height, "no sequencer data found in celestia block; skipping");
                    guess_blob_height = guess_blob_height.increment();
                    continue;
                } else if blobs.sequencer_blobs.into_iter().any(|b| {
                    let h: u32 =
                        u32::try_from(b.height().value()).expect("casting from u64 to u32 failed");
                    h == firm_commit_height
                }) {
                    return u32::try_from(guess_blob_height.value())
                        .expect("casting from u64 to u32 failed");
                }
            }
            Err(error) => {
                debug!(height = %height, error = %error, "no sequencer blobs found in da block at height; skipping");
                guess_blob_height = guess_blob_height.increment();
                continue;
            }
        }
    }

    // u32::try_from(guess_blob_height.value()).expect("casting from u64 to u32 failed")
    // FIXME: ^ eventually converte to smart search (binary search for example)
    // leaving this here for now as a first pass
    // FIXME: add an issue to make the search for the block we want more sophisticated

    // let mut num_blobs_searched = 1;
    // loop {
    //     // FIXME: add an issue to make error after certian number of guesses more
    //     // sophisticated
    //     assert!(
    //         (num_blobs_searched <= 1000),
    //         "searched more than 1000 blocks for the DA block containing the firm commit without \
    //          finding it, halting"
    //     );
    //     // NOTE: we are getting stuck here...
    //     let possible_blobs = sync::get_new_sequencer_block_data_with_retry(
    //         guess_blob_height,
    //         client.clone(),
    //         sequencer_namespace,
    //     )
    //     .await;

    //     let mut possible_blobs = match possible_blobs {
    //         Ok(blobs) => blobs,
    //         Err(e) => {
    //             guess_blob_height = guess_blob_height.increment();
    //             num_blobs_searched += 1;
    //             let error: &(dyn std::error::Error + 'static) = e.as_ref();
    //             warn!(da_guess_block_height = %guess_blob_height, error, "no sequencer data found
    // in celestia block; skipping");             continue;
    //         }
    //     };

    //     possible_blobs.sort_by(|a, b| a.header().height.cmp(&b.header().height));

    //     let blob = possible_blobs.iter().find(|seq_block| {
    //         u32::try_from(seq_block.header().height.value())
    //             .expect("casting from u64 to u32 failed")
    //             == firm_commit_height
    //     });

    //     if blob.is_none() {
    //         // check if we need to decrement or increment the guess block height
    //         let last_blob = possible_blobs.last();
    //         let last_blob_height = if let Some(block) = last_blob {
    //             u32::try_from(block.header().height.value())
    //                 .expect("casting from u64 to u32 failed")
    //         } else {
    //             guess_blob_height = guess_blob_height.increment();
    //             num_blobs_searched += 1;
    //             warn!(da_guess_block_height = %guess_blob_height, "no sequencer data found in
    // celestia block; skipping");             continue;
    //         };

    //         match last_blob_height.cmp(&firm_commit_height) {
    //             // match firm_commit_height.cmp(&last_blob_height) {
    //             std::cmp::Ordering::Less => {
    //                 debug!(
    //                     "{}",
    //                     format!(
    //                         "searching for firm commit seq block at height {} in da block at \
    //                          height {}, highest block seen: {}, incrementing da guess height",
    //                         firm_commit_height, guess_blob_height, last_blob_height
    //                     )
    //                 );
    //                 guess_blob_height = guess_blob_height.increment();
    //                 num_blobs_searched += 1;
    //             }
    //             std::cmp::Ordering::Greater => {
    //                 debug!(
    //                     "{}",
    //                     format!(
    //                         "searching for firm commit seq block at height {} in da block at \
    //                          height {}, highest block seen: {}, decrementing da guess height",
    //                         firm_commit_height, guess_blob_height, last_blob_height
    //                     )
    //                 );
    //                 let mut value = u32::try_from(guess_blob_height.value())
    //                     .expect("casting from u64 to u32 failed");
    //                 value -= 1;
    //                 guess_blob_height = Height::from(value);
    //                 num_blobs_searched += 1;
    //             }
    //             std::cmp::Ordering::Equal => {
    //                 // Ideally this case should never be hit. If this does
    //                 // happen the chain should halt.
    //                 error!(
    //                     %guess_blob_height,
    //                     %firm_commit_height,
    //                     "DA block is equal to the firm commit height"
    //                 );
    //                 panic!("sequencer block does not exists in da but was already executed");
    //             }
    //         }
    //     } else {
    //         info!(
    //             block.da = %guess_blob_height,
    //             block.firm = %firm_commit_height,
    //             "found firm commit in DA block"
    //         );
    //         return u32::try_from(guess_blob_height.value())
    //             .expect("casting from u64 to u32 failed");
    //     }
    // }
}
/// Verifies that each sequencer blob is genuinely derived from a sequencer block.
/// If it is, fetches its constituent rollup blobs from celestia and assembles
/// into a collection.
async fn verify_sequencer_blobs_and_assemble_rollups(
    height: Height,
    sequencer_blobs: Vec<CelestiaSequencerBlob>,
    client: HttpClient,
    block_verifier: BlockVerifier,
    rollup_namespace: Namespace,
) -> eyre::Result<Vec<SequencerBlockSubset>> {
    // spawn the verification tasks
    let mut verification_tasks = verify_all_blobs(sequencer_blobs, &block_verifier);

    let (assembly_tx, assembly_rx) = mpsc::channel(256);
    let block_assembler = task::spawn(assemble_blocks(assembly_rx));

    let mut fetch_and_verify_rollups = JoinSet::new();
    while let Some((block_hash, verification_result)) = verification_tasks.join_next().await {
        match verification_result {
            Err(e) => {
                let error = &e as &(dyn std::error::Error + 'static);
                warn!(block_hash = %DisplayBlockHash(block_hash), error, "task verifying sequencer data retrieved from celestia failed; dropping block");
            }
            Ok(Err(e)) => {
                let error: &(dyn std::error::Error + 'static) = e.as_ref();
                warn!(
                    block_hash = %DisplayBlockHash(block_hash),
                    error,
                    "task verifying sequencer data retrieved from celestia returned with an \
                     error; dropping block"
                );
            }
            Ok(Ok(data)) => {
                fetch_and_verify_rollups.spawn(
                    fetch_rollup_blob_and_forward_to_assembly(
                        client.clone(),
                        height,
                        data,
                        rollup_namespace,
                        assembly_tx.clone(),
                    )
                    .in_current_span(),
                );
            }
        }
    }

    // ensure that the last sender goes out of scope so that block assembler's exit condition fires
    std::mem::drop(assembly_tx);

    block_assembler
        .await
        .wrap_err("failed assembling sequencer block subsets")
}

/// Fetches a rollup blob for given height and namespace, and fowards it to the assembler.
///
/// If more than one rollup blob is received and pass verification, they are all dropped.
/// It is assumed that sequencer-relayer submits at most one rollup blob to celestia per
/// celestia height.
#[instrument(
    skip_all,
    fields(
        height,
        block_hash = %DisplayBlockHash(blob.block_hash()),
    )
)]
async fn fetch_rollup_blob_and_forward_to_assembly(
    client: HttpClient,
    height: Height,
    blob: CelestiaSequencerBlob,
    rollup_namespace: Namespace,
    block_tx: mpsc::Sender<SequencerBlockSubset>,
) {
    let mut rollups = match client
        .get_rollup_blobs_matching_sequencer_blob(height, rollup_namespace, &blob)
        .await
    {
        Err(e) => {
            let error = &e as &(dyn std::error::Error + 'static);
            warn!(error, "failed to get rollup data from celestia");
            return;
        }
        Ok(rollups) => rollups,
    };

    match rollups.len() {
        0 | 1 => {
            info!(
                n_rollups = rollups.len(),
                "forwarding rollup blobs to assembler"
            );
            let subset = SequencerBlockSubset {
                block_hash: blob.header().hash(),
                header: blob.header().clone(),
                transactions: rollups.pop().map_or(vec![], |rollup_blob| {
                    rollup_blob.into_unchecked().transactions
                }),
            };
            if block_tx.send(subset).await.is_err() {
                warn!("failed sending validated rollup data to block assembler; receiver dropped");
            }
        }
        n_rollups => warn!(
            n_rollups,
            "received more than one rollup blob for the given namespace, height, and sequencer \
             blob, which should not happen; dropping all blobs",
        ),
    }
}

async fn assemble_blocks(
    mut assembly_rx: mpsc::Receiver<SequencerBlockSubset>,
) -> Vec<SequencerBlockSubset> {
    let mut blocks = Vec::new();
    while let Some(subset) = assembly_rx.recv().await {
        blocks.push(subset);
    }
    blocks.sort_unstable_by(|a, b| a.header.height.cmp(&b.header.height));
    blocks
}

fn verify_all_blobs(
    blobs: Vec<CelestiaSequencerBlob>,
    block_verifier: &BlockVerifier,
) -> JoinMap<[u8; 32], eyre::Result<CelestiaSequencerBlob>> {
    let mut verification_tasks = JoinMap::new();
    for blob in blobs {
        let blob_hash = blob.block_hash();
        if verification_tasks.contains_key(&blob_hash) {
            warn!(
                block_hash = %DisplayBlockHash(blob_hash),
                "more than one sequencer data with the same block hash retrieved from celestia; \
                 only keeping the first"
            );
        } else {
            let verifier = block_verifier.clone();
            verification_tasks.spawn(
                blob_hash,
                async move {
                    verifier
                        .validate_sequencer_blob(&blob)
                        .await
                        .wrap_err("failed validating blob")?;
                    Ok(blob)
                }
                .in_current_span(),
            );
        }
    }
    verification_tasks
}

// async fn get_sequencer_data_from_da(
//     height: Height,
//     celestia_client: HttpClient,
//     sequencer_namespace: Namespace,
//     block_verifier: BlockVerifier,
// ) -> eyre::Result<Vec<CelestiaSequencerBlob>> {
//     // this first call should be the only tryhard call
//     let res = celestia_client
//         .get_sequencer_blobs(height, sequencer_namespace)
//         .await
//         .wrap_err("failed to fetch sequencer data from celestia")
//         // .map(|rsp| rsp.datas);
//         .map(|rsp| rsp.sequencer_blobs);

//     // need to make a verification block stream in addition to the block stream
//     // for running the verify_sequencer_blobs_and_assemble_rollups function
//     let seq_block_data = match res {
//         Ok(datas) => {
//             verify_sequencer_blobs_and_assemble_rollups(
//                 height,
//                 datas,
//                 celestia_client,
//                 block_verifier.clone(),
//                 sequencer_namespace,
//             )
//             .await
//         }
//         Err(e) => {
//             let error: &(dyn std::error::Error + 'static) = e.as_ref();
//             warn!(
//                 da_block_height = %height.value(),
//                 error,
//                 "task querying celestia for sequencer data returned with an error"
//             );
//             Err(e)
//         }
//     };
//     seq_block_data
// }

struct DisplayBlockHash([u8; 32]);

impl std::fmt::Display for DisplayBlockHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            f.write_fmt(format_args!("{byte:02x}"))?;
        }
        Ok(())
    }
}
