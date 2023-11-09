use std::time::Duration;

use celestia_client::{
    // celestia_rpc::client,
    // celestia_rpc::client,
    // celestia_tendermint::block,
    celestia_types::{
        nmt::Namespace,
        Height,
    },
    jsonrpsee::http_client::HttpClient,
    CelestiaClientExt as _,
    SequencerNamespaceData,
    SEQUENCER_NAMESPACE,
};
use color_eyre::eyre::{
    self,
    // Error,
    WrapErr as _,
};
use futures::{
    future::FusedFuture,
    FutureExt,
};
// use futures::stream::FuturesOrdered;
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
    block_verifier::{
        self,
        BlockVerifier,
    },
    executor,
    types::SequencerBlockSubset,
};

mod sync;
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

    /// Namespace ID
    namespace: Namespace,

    get_latest_height: Option<JoinHandle<eyre::Result<Height>>>,

    /// A map of in-flight queries to celestia for new sequencer blobs at a given height
    fetch_sequencer_blobs_at_height: JoinMap<Height, eyre::Result<Vec<SequencerNamespaceData>>>,

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
    sync_done: oneshot::Sender<()>,
}

impl Reader {
    /// Creates a new Reader instance and returns a command sender.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn new(
        initial_da_height: u32,
        firm_commit_height: u32,
        celestia_node_url: &str,
        celestia_bearer_token: &str,
        celestia_poll_interval: Duration,
        executor_tx: executor::Sender,
        block_verifier: BlockVerifier,
        namespace: Namespace,
        shutdown: oneshot::Receiver<()>,
        sync_done: oneshot::Sender<()>,
    ) -> eyre::Result<Self> {
        use celestia_client::celestia_rpc::HeaderClient;

        let celestia_client = celestia_client::celestia_rpc::client::new_http(
            celestia_node_url,
            Some(celestia_bearer_token),
        )
        .wrap_err("failed constructing celestia http client")?;

        // TODO: we should probably pass in the height we want to start at from some genesis/config
        // file
        let current_block_height = celestia_client
            .header_network_head()
            .await
            .wrap_err("failed to get network head from celestia to extract latest head")?
            .header
            .height;

        info!(da_height = %current_block_height, "creating Reader");

        Ok(Self {
            executor_tx,
            celestia_client,
            celestia_poll_interval,
            current_block_height,
            get_latest_height: None,
            fetch_sequencer_blobs_at_height: JoinMap::new(),
            verify_sequencer_blobs_and_assemble_rollups: JoinMap::new(),
            block_verifier,
            namespace,
            initial_da_block_height: initial_da_height,
            firm_commit_height,
            shutdown,
            sync_done,
        })
    }

    #[instrument(skip(self))]
    pub(crate) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        info!("Starting reader event loop.");

        // TODO ghi(https://github.com/astriaorg/astria/issues/470): add sync
        // functionality to data availability reader
        let Self {
            executor_tx,
            celestia_client,
            celestia_poll_interval,
            current_block_height: _,
            mut get_latest_height,
            mut fetch_sequencer_blobs_at_height,
            mut verify_sequencer_blobs_and_assemble_rollups,
            block_verifier,
            namespace: _,
            initial_da_block_height,
            firm_commit_height,
            mut shutdown,
            sync_done,
        } = self;
        // let celestia_client = celestia_client.clone();
        // let initial_da_block_height = initial_da_block_height;
        // let firm_commit_height = firm_commit_height;
        let sync_start_height = find_da_sync_start_height(
            celestia_client.clone(),
            initial_da_block_height,
            firm_commit_height,
        )
        .await;
        // let sync_done = sync_done;
        // self.sync_done = sync_done;

        let latest_height = find_latest_height(celestia_client.clone()).await?.value() as u32;

        // let mut sync = self.sync_blocks(sync_start_height, latest_height);
        let mut sync = sync::run(
            sync_start_height,
            latest_height,
            self.namespace,
            executor_tx.clone(),
            celestia_client,
            block_verifier.clone(),
        )
        .boxed()
        .fuse();
        // self.fetch_sequencer_blobs_up_to_latest_height(Ok(Ok(height)));

        // TODO: add da sync done to asycn run the sync
        // TODO: add sync done check to the event loop below

        let mut sync_done = Some(sync_done);
        let mut interval = tokio::time::interval(celestia_poll_interval);
        loop {
            let executor_tx = executor_tx.clone();

            select!(
                shutdown_res = &mut shutdown => {
                    match shutdown_res {
                        Ok(()) => info!("received shutdown command; exiting"),
                        Err(e) => {
                            let error = &e as &(dyn std::error::Error + 'static);
                            warn!(error, "shutdown receiver dropped; exiting");
                        }
                    }
                    break;
                }

                _ = interval.tick() => self.get_latest_height(),

                res = &mut sync, if !sync.is_terminated() => {
                    if let Err(e) = res {
                        let error: &(dyn std::error::Error + 'static) = e.as_ref();
                        warn!(error, "sync failed; continuing with normal operation");
                    } else {
                        info!("sync finished successfully");
                    }
                    // First sync at startup: notify conductor that sync is done.
                    // Every resync after: don't.
                    if let Some(sync_done) = sync_done.take() {
                        let _ = sync_done.send(());
                    }
                }

                res = async { get_latest_height.as_mut().unwrap().await }, if get_latest_height.is_some() => {
                    self.get_latest_height = None;
                    self.fetch_sequencer_blobs_up_to_latest_height(res);
                }

                Some((height, res)) = fetch_sequencer_blobs_at_height.join_next(), if !fetch_sequencer_blobs_at_height.is_empty() => {
                    self.process_sequencer_datas(height, res);
                }

                Some((height, res)) = verify_sequencer_blobs_and_assemble_rollups.join_next(), if !verify_sequencer_blobs_and_assemble_rollups.is_empty() => {
                    let span = tracing::info_span!("send_sequencer_subsets", %height);
                    span.in_scope(|| send_sequencer_subsets(executor_tx.clone(), res))
                        .wrap_err("failed sending sequencer subsets to executor")?;
                }
            )
        }
        Ok(())
    }

    fn get_latest_height(&mut self) {
        use celestia_client::celestia_rpc::HeaderClient;
        let client = self.celestia_client.clone();
        self.get_latest_height = Some(tokio::spawn(async move {
            Ok(client.header_network_head().await?.header.height)
        }))
    }

    // async fn find_latest_height(&mut self) -> eyre::Result<Height> {
    //     use celestia_client::celestia_rpc::HeaderClient;
    //     let client = self.celestia_client.clone();
    //     Ok(client.header_network_head().await?.header.height)
    // }

    /// Starts fetching sequencer blobs for each height between `self.current_height`
    /// and `latest_height` returned by celestia, populating `fetch_sequencer_blobs_at_height`.
    ///
    /// Note that this method evaluates the return value of the `fetch_latest_height` task. If it
    /// failed no heights are fetched.
    fn fetch_sequencer_blobs_up_to_latest_height(
        &mut self,
        latest_height_res: Result<eyre::Result<Height>, JoinError>,
    ) {
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
            height.end = %self.current_block_height,
            "spawning tasks to fetch sequencer blocks for different celestia heights",
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
                self.fetch_sequencer_blobs_at_height
                    .spawn(height, async move {
                        client
                            .get_sequencer_data(height, SEQUENCER_NAMESPACE)
                            .await
                            .wrap_err("failed to fetch sequencer data from celestia")
                            .map(|rsp| rsp.datas)
                    });
            }
        }
    }

    #[instrument(skip(self, sequencer_data_res))]
    fn process_sequencer_datas(
        &mut self,
        height: Height,
        sequencer_data_res: Result<eyre::Result<Vec<SequencerNamespaceData>>, JoinError>,
    ) {
        let sequencer_data = match sequencer_data_res {
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
                self.namespace,
            )
            .in_current_span(),
        );
    }
}

// fn get_latest_height(
//     celestia_client: HttpClient,
//     get_latest_height: Option<JoinHandle<eyre::Result<Height>>>,
// ) { use celestia_client::celestia_rpc::HeaderClient; let client = celestia_client.clone();
//   get_latest_height = Some(tokio::spawn(async move {
//   Ok(client.header_network_head().await?.header.height) }))
// }
fn get_latest_height(
    celestia_client: HttpClient,
    get_latest_height: Option<JoinHandle<eyre::Result<Height>>>,
) {
    use celestia_client::celestia_rpc::HeaderClient;
    self.get_latest_height = Some(tokio::spawn(async move {
        Ok(celestia_client.header_network_head().await?.header.height)
    }))
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
) -> u32 {
    // Celestia block time is approximately 12 seconds, the sequencer has a 2
    // second block time. This means that ideally there are 6 sequencer blocks
    // in every Celestia block, but that may not always be the case. We set the
    // initial guess height assuming 5 sequencer blocks per Celestia block to
    // make sure that we are below the DA block that contained the most recently
    // executed firm commit on the rollup, then search forward to find the exact
    // Celestia block that contained it.
    // This search is required because there is no way to poll Celestia for
    // which block contianed a given sequencer block.
    let mut guess_block_height = Height::from(initial_da_height + (firm_commit_height / 5));
    // FIXME: ^ I'm sure there is a better way to estimate this value but
    // leaving this here for now as a first pass

    // FIXME: add an issue to make the search for the block we want more sophisticated
    // FIXME: add an issue to make error after certian number of guesses more sophisticated
    let mut num_blocks_searched = 1;
    loop {
        if num_blocks_searched > 1000 {
            panic!(
                "searched more than 1000 blocks for the DA block containing the firm commit \
                 without finding it, halting"
            );
        }

        let mut possible_blocks = client
            .get_sequencer_data(guess_block_height, SEQUENCER_NAMESPACE)
            .await
            .wrap_err("failed to fetch sequencer data from celestia")
            .map(|rsp| rsp.datas)
            .expect("failed to convert sequencer data response to sequencer data");

        possible_blocks.sort_by(|a, b| a.header.height.cmp(&b.header.height));

        let block = possible_blocks
            .iter()
            .find(|seq_block| seq_block.header.height.value() as u32 == firm_commit_height);
        match block {
            Some(_block) => {
                info!(
                    block.da = %guess_block_height,
                    block.firm = %firm_commit_height,
                    "found firm commit in DA block"
                );
                return guess_block_height.value() as u32;
            }
            None => {
                // check if we need to decrement or increment the guess block height
                let last_block_height = possible_blocks
                    .last()
                    .expect("vec of sequencer blocks from celestia should not be empty")
                    .header
                    .height
                    .value() as u32;
                match last_block_height.cmp(&firm_commit_height) {
                    std::cmp::Ordering::Less => {
                        // debug!(block.da = %guess_block_height, block.firm = %firm_commit_height,
                        // block.highest_seen = %last_block_height, "comparing heights");
                        debug!(
                            "{}",
                            format!(
                                "searching for firm commit seq block at height {} in da block at \
                                 height {}, highest block seen: {}, incrementing da guess height",
                                firm_commit_height, guess_block_height, last_block_height
                            )
                        );
                        guess_block_height = guess_block_height.increment();
                        num_blocks_searched += 1;
                    }
                    std::cmp::Ordering::Greater => {
                        debug!(
                            "{}",
                            format!(
                                "searching for firm commit seq block at height {} in da block at \
                                 height {}, highest block seen: {}, decrementing da guess height",
                                firm_commit_height, guess_block_height, last_block_height
                            )
                        );
                        let mut value = guess_block_height.value() as u32;
                        value -= 1;
                        guess_block_height = Height::from(value);
                        num_blocks_searched += 1;
                    }
                    std::cmp::Ordering::Equal => {
                        // Ideally this case should never be hit. If this does
                        // happen the chain should halt.
                        error!(
                            %guess_block_height,
                            %firm_commit_height,
                            "DA block is equal to the firm commit height"
                        );
                        panic!("sequencer block does not exists in da but was already executed");
                    }
                }
            }
        }
    }
}
/// Verifies that each sequencer blob is genuinely derived from a sequencer block.
/// If it is, fetches its constituent rollup blobs from celestia and assembles
/// into a collection.
async fn verify_sequencer_blobs_and_assemble_rollups(
    height: Height,
    sequencer_blobs: Vec<SequencerNamespaceData>,
    client: HttpClient,
    block_verifier: BlockVerifier,
    namespace: Namespace,
) -> eyre::Result<Vec<SequencerBlockSubset>> {
    // spawn the verification tasks
    let mut verification_tasks = verify_all_datas(sequencer_blobs, block_verifier);

    let (assembly_tx, assembly_rx) = mpsc::channel(256);
    let block_assembler = task::spawn(assemble_blocks(assembly_rx));

    let mut fetch_and_verify_rollups = JoinSet::new();
    while let Some((block_hash, verification_result)) = verification_tasks.join_next().await {
        match verification_result {
            Err(e) => {
                let error = &e as &(dyn std::error::Error + 'static);
                warn!(%block_hash, error, "task verifying sequencer data retrieved from celestia failed; dropping block")
            }
            Ok(Err(e)) => {
                let error: &(dyn std::error::Error + 'static) = e.as_ref();
                warn!(%block_hash, error, "task verifying sequencer data retrieved from celestia returned with an error; dropping block")
            }
            Ok(Ok(data)) => {
                fetch_and_verify_rollups.spawn(
                    fetch_verify_rollup_blob_and_forward_to_assembly(
                        client.clone(),
                        height,
                        data,
                        namespace,
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
#[instrument(skip_all, fields(height, block_hash = %data.block_hash))]
async fn fetch_verify_rollup_blob_and_forward_to_assembly(
    client: HttpClient,
    height: Height,
    data: SequencerNamespaceData,
    namespace: Namespace,
    block_tx: mpsc::Sender<SequencerBlockSubset>,
) {
    let mut rollups = match client
        .get_rollup_data_matching_sequencer_data(height, namespace, &data)
        .await
    {
        Err(e) => {
            let error = &e as &(dyn std::error::Error + 'static);
            warn!(error, "failed to get rollup data from celestia");
            return;
        }
        Ok(rollups) => rollups,
    };

    let num_rollups_received = rollups.len();
    info!(
        rollups.received = num_rollups_received,
        "received rollups; verifying"
    );
    rollups.retain(|rollup| {
        block_verifier::validate_rollup_data(rollup, data.action_tree_root).is_ok()
    });
    match rollups.len() {
        0 | 1 => {
            info!("rollup data found; forwarding to block assembler");
            let subset = SequencerBlockSubset {
                block_hash: data.block_hash,
                header: data.header.clone(),
                rollup_transactions: rollups.pop().map_or(vec![], |txs| txs.rollup_txs),
            };
            if block_tx.send(subset).await.is_err() {
                warn!("failed sending validated rollup data to block assembler; receiver dropped");
            }
        }
        num_rollups_verified => warn!(
            rollups.received = num_rollups_received,
            rollups.verified = num_rollups_verified,
            "more than one rollup remained after verification, which should not happen; dropping \
             all",
        ),
    }
}

async fn assemble_blocks(
    mut assembly_rx: mpsc::Receiver<SequencerBlockSubset>,
) -> Vec<SequencerBlockSubset> {
    let mut blocks = Vec::new();
    while let Some(subset) = assembly_rx.recv().await {
        blocks.push(subset)
    }
    blocks.sort_unstable_by(|a, b| a.header.height.cmp(&b.header.height));
    blocks
}

fn verify_all_datas(
    datas: Vec<SequencerNamespaceData>,
    block_verifier: BlockVerifier,
) -> JoinMap<tendermint::Hash, eyre::Result<SequencerNamespaceData>> {
    let mut verification_tasks = JoinMap::new();
    for data in datas {
        let block_hash = data.block_hash;
        if verification_tasks.contains_key(&block_hash) {
            warn!(%block_hash,
                "more than one sequencer data with the same block hash retrieved from celestia; \
                 only keeping the first"
            );
        } else {
            let verifier = block_verifier.clone();
            verification_tasks.spawn(
                block_hash,
                async move {
                    verifier.validate_sequencer_namespace_data(&data).await?;
                    Ok(data)
                }
                .in_current_span(),
            );
        }
    }
    verification_tasks
}

// TODO: add a sync function
// needs to be here because the normal da::Reader loop still needs to process da blocks
