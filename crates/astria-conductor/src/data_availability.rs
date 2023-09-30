use std::time::Duration;

use astria_sequencer_relayer::data_availability::{
    CelestiaClient,
    SequencerNamespaceData,
    SignedNamespaceData,
};
use astria_sequencer_types::Namespace;
use color_eyre::eyre::{
    self,
    WrapErr as _,
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

use crate::{
    block_verifier::{
        self,
        BlockVerifier,
    },
    // config::Config,
    executor,
    types::SequencerBlockSubset,
};

#[derive(Debug)]
pub(crate) struct Reader {
    /// The channel used to send messages to the executor task.
    executor_tx: executor::Sender,

    /// The client used to communicate with Celestia.
    celestia_client: CelestiaClient,

    /// The between the reader waits until it queries celestia for new blocks
    celestia_poll_interval: Duration,

    /// the last block height fetched from Celestia
    current_block_height: u64,

    block_verifier: BlockVerifier,

    /// Namespace ID
    namespace: Namespace,

    get_latest_height: Option<JoinHandle<eyre::Result<u64>>>,

    /// A map of in-flight queries to celestia for new sequencer blocks
    get_sequencer_datas:
        JoinMap<u64, eyre::Result<Vec<SignedNamespaceData<SequencerNamespaceData>>>>,

    process_sequencer_datas: JoinMap<u64, eyre::Result<Vec<SequencerBlockSubset>>>,

    shutdown: oneshot::Receiver<()>,
}

impl Reader {
    /// Creates a new Reader instance and returns a command sender.
    pub(crate) async fn new(
        celestia_node_url: &str,
        celestia_bearer_token: &str,
        celestia_poll_interval: Duration,
        executor_tx: executor::Sender,
        block_verifier: BlockVerifier,
        namespace: Namespace,
        shutdown: oneshot::Receiver<()>,
    ) -> eyre::Result<Self> {
        let celestia_client = CelestiaClient::builder()
            .endpoint(celestia_node_url)
            .bearer_token(celestia_bearer_token)
            .build()
            .wrap_err("failed creating celestia client")?;

        // TODO: we should probably pass in the height we want to start at from some genesis/config
        // file
        let current_block_height = celestia_client.get_latest_height().await?;
        info!(da_height = current_block_height, "creating Reader");

        Ok(Self {
            executor_tx,
            celestia_client,
            celestia_poll_interval,
            current_block_height,
            get_latest_height: None,
            get_sequencer_datas: JoinMap::new(),
            process_sequencer_datas: JoinMap::new(),
            block_verifier,
            namespace,
            shutdown,
        })
    }

    #[instrument(skip(self))]
    pub(crate) async fn run(mut self) -> eyre::Result<()> {
        info!("Starting reader event loop.");

        let mut interval = tokio::time::interval(self.celestia_poll_interval);
        loop {
            select!(
                shutdown_res = &mut self.shutdown => {
                    match shutdown_res {
                        Ok(()) => info!("received shutdown command; exiting"),
                        Err(e) => warn!(error.message = %e, error.cause = ?e, "shutdown receiver dropped; exiting"),
                    }
                    break;
                }

                _ = interval.tick() => self.get_latest_height(),

                res = async { self.get_latest_height.as_mut().unwrap().await }, if self.get_latest_height.is_some() => {
                    self.get_latest_height = None;
                    self.get_sequencer_datas(res);
                }

                Some((height, res)) = self.get_sequencer_datas.join_next(), if !self.get_sequencer_datas.is_empty() => {
                    self.process_sequencer_datas(height, res);
                }

                Some((height, res)) = self.process_sequencer_datas.join_next(), if !self.process_sequencer_datas.is_empty() => {
                    let span = tracing::info_span!("send_sequencer_subsets", height);
                    span.in_scope(|| self.send_sequencer_subsets(res))
                        .wrap_err("failed sending sequencer subsets to executor")?;
                }
            )
        }
        Ok(())
    }

    fn get_latest_height(&mut self) {
        let client = self.celestia_client.inner().clone();
        self.get_latest_height = Some(tokio::spawn(async move {
            Ok(client.header_network_head().await?.height())
        }))
    }

    fn get_sequencer_datas(&mut self, latest_height_res: Result<eyre::Result<u64>, JoinError>) {
        let latest_height = match latest_height_res {
            Err(e) => {
                warn!(error.message = %e, error.cause = ?e, "task querying celestia for latest height failed");
                return;
            }

            Ok(Err(e)) => {
                warn!(error.message = %e, error.cause = ?e, "task querying celestia for latest height returned with an error");
                return;
            }

            Ok(Ok(height)) => height,
        };

        if latest_height <= self.current_block_height {
            info!(
                height.celestia = latest_height,
                height.previous = self.current_block_height,
                "no new celestia height; not spawning tasks to fetch sequencer blocks"
            );
            return;
        }
        let first_new_height = self.current_block_height + 1;
        info!(
            height.start = first_new_height,
            height.end = self.current_block_height,
            "spawning tasks to fetch sequencer blocks for different celestia heights",
        );
        for height in first_new_height..=self.current_block_height {
            let client = self.celestia_client.clone();
            if self.get_sequencer_datas.contains_key(&height) {
                warn!(
                    height,
                    "getting sequencer data from celestia already in flight, not spawning"
                );
            } else {
                self.get_sequencer_datas.spawn(height, async move {
                    client.get_sequencer_namespace_data(height).await
                });
            }
        }
    }

    #[instrument(skip(self, sequencer_data_res))]
    fn process_sequencer_datas(
        &mut self,
        height: u64,
        sequencer_data_res: Result<
            eyre::Result<Vec<SignedNamespaceData<SequencerNamespaceData>>>,
            JoinError,
        >,
    ) {
        let sequencer_data = match sequencer_data_res {
            Err(e) => {
                warn!(height, error.message = %e, error.cause = ?e, "task querying celestia for sequencer data failed");
                return;
            }

            Ok(Err(e)) => {
                warn!(error.message = %e, error.cause = ?e, "task querying celestia for sequencer data returned with an error");
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
        if self.process_sequencer_datas.contains_key(&height) {
            error!(
                "sequencer data is already being processed; no two sequencer data responses \
                 should have been received; this is a bug"
            );
            return;
        }
        self.process_sequencer_datas.spawn(
            height,
            process_sequencer_data(
                height,
                sequencer_data,
                self.celestia_client.clone(),
                self.block_verifier.clone(),
                self.namespace,
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
                warn!(error.message = %e, error.cause = ?e, "task processing sequencer data failed");
                return Ok(());
            }
            Ok(Err(e)) => {
                warn!(error.message = %e, error.cause = ?e, "task processing sequencer data returned with an error");
                return Ok(());
            }
            Ok(Ok(subsets)) => subsets,
        };
        self.executor_tx
            .send(executor::ExecutorCommand::FromCelestia(subsets))
            .wrap_err("failed sending processed sequencer subsets: executor channel is closed")
    }
}

async fn process_sequencer_data(
    height: u64,
    datas: Vec<SignedNamespaceData<SequencerNamespaceData>>,
    client: CelestiaClient,
    block_verifier: BlockVerifier,
    namespace: Namespace,
) -> eyre::Result<Vec<SequencerBlockSubset>> {
    // spawn the verification tasks
    let mut verification_tasks = verify_all_datas(datas, block_verifier);

    let (assembly_tx, assembly_rx) = mpsc::channel(256);
    let block_assembler = task::spawn(assemble_blocks(assembly_rx));

    let mut get_rollups = JoinSet::new();
    while let Some((block_hash, verification_result)) = verification_tasks.join_next().await {
        match verification_result {
            Err(e) => {
                warn!(%block_hash, error.message = %e, error.cause = ?e, "task verifying sequencer data retrieved from celestia failed; dropping block")
            }
            Ok(Err(e)) => {
                warn!(%block_hash, error.message = %e, error.cause = ?e, "task verifying sequencer data retrieved from celestia returned with an error; dropping block")
            }
            Ok(Ok(data)) => {
                get_rollups.spawn(
                    get_rollup(client.clone(), height, data, namespace, assembly_tx.clone())
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

async fn get_rollup(
    client: CelestiaClient,
    height: u64,
    data: SignedNamespaceData<SequencerNamespaceData>,
    namespace: Namespace,
    block_tx: mpsc::Sender<SequencerBlockSubset>,
) {
    let rollup_data = match client.get_rollup_data(height, &data, namespace).await {
        Err(e) => {
            warn!(error.message = %e, error.cause = ?e, "failed to get rollup data from celestia");
            return;
        }
        Ok(None) => {
            info!("could not find rollup data");
            return;
        }
        Ok(Some(rollup_data)) => rollup_data,
    };

    if block_verifier::validate_rollup_data(&rollup_data, data.data.action_tree_root).is_err() {
        warn!(
            "could not validate rollup data given the action tree root of the sequencer data; \
             dropping the block"
        )
    } else {
        let subset = SequencerBlockSubset {
            block_hash: data.data.block_hash,
            header: data.data.header,
            rollup_transactions: rollup_data.rollup_txs,
        };
        if block_tx.send(subset).await.is_err() {
            warn!("failed sending validated rollup data to block assembler; receiver dropped");
        }
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
    datas: Vec<SignedNamespaceData<SequencerNamespaceData>>,
    block_verifier: BlockVerifier,
) -> JoinMap<tendermint::Hash, eyre::Result<SignedNamespaceData<SequencerNamespaceData>>> {
    let mut verification_tasks = JoinMap::new();
    for data in datas {
        let block_hash = data.data.block_hash;
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
                    verifier
                        .validate_signed_namespace_data(&data)
                        .await
                        .map(|()| data)
                }
                .in_current_span(),
            );
        }
    }
    verification_tasks
}
