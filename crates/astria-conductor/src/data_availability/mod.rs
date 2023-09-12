use std::time::Duration;

use celestia_client::{
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
use proto::native::sequencer::v1alpha1::ChainId;
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
    error,
    info,
    instrument,
    warn,
    Instrument,
};

use crate::executor;

mod block_verifier;
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
    pub(crate) fn from_sequencer_block(block: SequencerBlock, chain_id: ChainId) -> Self {
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

    shutdown: oneshot::Receiver<()>,
}

pub(crate) struct CelestiaReaderConfig {
    pub(crate) node_url: String,
    pub(crate) bearer_token: Option<String>,
    pub(crate) poll_interval: Duration,
}

impl Reader {
    /// Creates a new Reader instance and returns a command sender.
    pub(crate) async fn new(
        celestia_config: CelestiaReaderConfig,
        executor_tx: executor::Sender,
        sequencer_client_pool: deadpool::managed::Pool<crate::client_provider::ClientProvider>,
        sequencer_namespace: Namespace,
        rollup_namespace: Namespace,
        shutdown: oneshot::Receiver<()>,
    ) -> eyre::Result<Self> {
        use celestia_client::celestia_rpc::HeaderClient;

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

        info!(da_height = %current_block_height, "creating Reader");

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
            shutdown,
        })
    }

    #[instrument(skip(self))]
    pub(crate) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        info!("Starting reader event loop.");

        // TODO ghi(https://github.com/astriaorg/astria/issues/470): add sync functionality to data availability reader

        let mut interval = tokio::time::interval(self.celestia_poll_interval);
        loop {
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

                _ = interval.tick() => self.get_latest_height(),

                res = async { self.get_latest_height.as_mut().unwrap().await }, if self.get_latest_height.is_some() => {
                    self.get_latest_height = None;
                    self.fetch_sequencer_blobs_up_to_latest_height(res);
                }

                Some((height, res)) = self.fetch_sequencer_blobs_at_height.join_next(), if !self.fetch_sequencer_blobs_at_height.is_empty() => {
                    self.process_sequencer_blobs(height, res);
                }

                Some((height, res)) = self.verify_sequencer_blobs_and_assemble_rollups.join_next(), if !self.verify_sequencer_blobs_and_assemble_rollups.is_empty() => {
                    let span = tracing::info_span!("send_sequencer_subsets", %height);
                    span.in_scope(|| self.send_sequencer_subsets(res))
                        .wrap_err("failed sending sequencer subsets to executor")?;
                }
            );
        }
        Ok(())
    }

    fn get_latest_height(&mut self) {
        use celestia_client::celestia_rpc::HeaderClient;
        let client = self.celestia_client.clone();
        self.get_latest_height = Some(tokio::spawn(async move {
            Ok(client.header_network_head().await?.header.height)
        }));
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

    #[instrument(skip_all, fields(height))]
    fn send_sequencer_subsets(
        &self,
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
        self.executor_tx
            .send(executor::ExecutorCommand::FromCelestia(subsets))
            .wrap_err("failed sending processed sequencer subsets: executor channel is closed")
    }
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

struct DisplayBlockHash([u8; 32]);

impl std::fmt::Display for DisplayBlockHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            f.write_fmt(format_args!("{byte:02x}"))?;
        }
        Ok(())
    }
}
