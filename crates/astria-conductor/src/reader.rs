use std::sync::Arc;

use astria_sequencer_relayer::data_availability::CelestiaClient;
use astria_sequencer_types::Namespace;
use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use tokio::{
    sync::mpsc::{
        self,
        UnboundedReceiver,
        UnboundedSender,
    },
    task,
};
use tracing::{
    debug,
    info,
    instrument,
    warn,
};

use crate::{
    block_verifier::BlockVerifier,
    config::Config,
    executor,
    types::SequencerBlockSubset,
};

pub(crate) type JoinHandle = task::JoinHandle<eyre::Result<()>>;

/// The channel for sending commands to the reader task.
pub type Sender = UnboundedSender<ReaderCommand>;
/// The channel the reader task uses to listen for commands.
type Receiver = UnboundedReceiver<ReaderCommand>;

/// spawns a reader task and returns a tuple with the task's join handle
/// and the channel for sending commands to this reader
pub(crate) async fn spawn(
    conf: &Config,
    executor_tx: executor::Sender,
    block_verifier: Arc<BlockVerifier>,
) -> eyre::Result<(JoinHandle, Sender)> {
    info!(actor_name = "reader", "Spawning reader task.");
    let (mut reader, reader_tx) = Reader::new(
        &conf.celestia_node_url,
        &conf.celestia_bearer_token,
        executor_tx,
        block_verifier,
        Namespace::from_slice(conf.chain_id.as_bytes()),
    )
    .await
    .wrap_err("failed to create Reader")?;
    let join_handle = task::spawn(async move { reader.run().await });
    info!(actor_name = "reader", "Spawned reader task.");
    Ok((join_handle, reader_tx))
}

#[derive(Debug)]
pub enum ReaderCommand {
    /// Get new blocks
    GetNewBlocks,

    Shutdown,
}

pub struct Reader {
    /// Channel on which reader commands are received.
    cmd_rx: Receiver,

    /// The channel used to send messages to the executor task.
    executor_tx: executor::Sender,

    /// The client used to communicate with Celestia.
    celestia_client: CelestiaClient,

    /// the last block height fetched from Celestia
    curr_block_height: u64,

    block_verifier: Arc<BlockVerifier>,

    /// Namespace ID
    namespace: Namespace,
}

impl Reader {
    /// Creates a new Reader instance and returns a command sender.
    pub async fn new(
        celestia_node_url: &str,
        celestia_bearer_token: &str,
        executor_tx: executor::Sender,
        block_verifier: Arc<BlockVerifier>,
        namespace: Namespace,
    ) -> eyre::Result<(Self, Sender)> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let celestia_client = CelestiaClient::builder()
            .endpoint(celestia_node_url)
            .bearer_token(celestia_bearer_token)
            .build()
            .wrap_err("failed creating celestia client")?;

        // TODO: we should probably pass in the height we want to start at from some genesis/config
        // file
        let curr_block_height = celestia_client.get_latest_height().await?;
        info!(
            actor_name = "reader",
            da_height = curr_block_height,
            "creating Reader"
        );

        Ok((
            Self {
                cmd_rx,
                executor_tx,
                celestia_client,
                curr_block_height,
                block_verifier,
                namespace,
            },
            cmd_tx,
        ))
    }

    async fn run(&mut self) -> eyre::Result<()> {
        info!(actor_name = "reader", "Starting reader event loop.");

        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                ReaderCommand::GetNewBlocks => {
                    let blocks = match self.get_new_blocks().await {
                        Ok(blocks) => blocks,
                        Err(e) => {
                            warn!(
                                actor_name = "reader",
                                error.msg = %e,
                                error.cause = ?e,
                                "failed to get new blocks"
                            );
                            continue;
                        }
                    };
                    if let Some(blocks) = blocks {
                        for block in blocks {
                            if let Err(e) = self.process_block(block).await {
                                warn!(
                                    actor_name = "reader",
                                    error.msg = %e,
                                    error.cause = ?e,
                                    "failed to process block"
                                );
                            }
                        }
                    }
                }
                ReaderCommand::Shutdown => {
                    info!(actor_name = "reader", "Shutting down reader event loop.");
                    break;
                }
            }
        }

        Ok(())
    }

    /// get_new_blocks fetches any new sequencer blocks from Celestia.
    #[instrument(name = "Reader::get_new_blocks", skip_all)]
    pub async fn get_new_blocks(&mut self) -> eyre::Result<Option<Vec<SequencerBlockSubset>>> {
        // get the latest celestia block height
        let first_new_height = self.curr_block_height + 1;
        let curr_block_height = self
            .celestia_client
            .get_latest_height()
            .await
            .wrap_err("failed getting latest height from celestia")?;
        if curr_block_height <= self.curr_block_height {
            info!(
                actor_name = "reader",
                height.celestia = curr_block_height,
                height.previous = self.curr_block_height,
                "no new celestia height"
            );
            return Ok(None);
        }

        info!(
            actor_name = "reader",
            height.start = first_new_height,
            height.end = curr_block_height,
            "checking celestia blocks for range of heights",
        );
        let mut blocks = vec![];
        // check for any new sequencer blocks written from the previous to current block height
        'check_heights: for height in first_new_height..=curr_block_height {
            info!(
                actor_name = "reader",
                height, "querying data availability layer for sequencer namespace data"
            );
            let sequencer_namespaced_datas = match self
                .celestia_client
                .get_sequencer_namespace_data(height)
                .await
            {
                Ok(datas) => datas,
                Err(e) => {
                    warn!(
                        actor_name = "reader",
                        error.msg = %e,
                        error.cause = ?e,
                        height,
                        "failed getting sequencer namespace data from data availability layer");
                    continue 'check_heights;
                }
            };

            // update the stored current block height after every successful call to the data
            // availability layer
            // FIXME: is that correct? We have to figure out how to
            // retry heights that fail (and under which conditions)
            self.curr_block_height = height;
            'get_sequencer_blocks: for data in sequencer_namespaced_datas {
                if let Err(e) = self
                    .block_verifier
                    .validate_signed_namespace_data(&data)
                    .await
                {
                    // FIXME: provide more information here to identify the particular block?
                    warn!(
                        actor_name = "reader",
                        error.msg = %e,
                        error.cause = ?e,
                        "failed to validate signed namespace data; skipping"
                    );
                    continue 'get_sequencer_blocks;
                }

                let rollup_data = match self
                    .celestia_client
                    .get_rollup_data(height, &data, self.namespace)
                    .await
                    .wrap_err("failed to get rollup data")
                {
                    Ok(Some(rollup_data)) => rollup_data,
                    Ok(None) => {
                        debug!(
                            actor_name = "reader",
                            height, "reader was unable to find rollup data for the given height"
                        );
                        continue;
                    }
                    Err(e) => {
                        // this means someone submitted an invalid block to celestia;
                        // we can ignore it
                        warn!(
                            actor_name = "reader",
                            error.msg = %e,
                            error.cause = ?e,
                            "failed to get sequencer block from namespace data");
                        continue 'get_sequencer_blocks;
                    }
                };
                if let Err(e) = self
                    .block_verifier
                    .validate_rollup_data(
                        data.data.block_hash,
                        &data.data.header,
                        &data.data.last_commit,
                        &rollup_data,
                    )
                    .await
                {
                    // this means someone submitted an invalid block to celestia;
                    // we can ignore it
                    warn!(
                        actor_name = "reader",
                        error.msg = %e,
                        error.cause = ?e,
                        "failed to validate sequencer block"
                    );
                    continue 'get_sequencer_blocks;
                }
                blocks.push(SequencerBlockSubset {
                    block_hash: data.data.block_hash,
                    header: data.data.header,
                    rollup_transactions: rollup_data.rollup_txs,
                });
            }
        }

        // sort blocks by height
        // TODO: there isn't a guarantee that the blocks aren't severely out of order,
        // and we need to ensure that there are no gaps between the block heights before processing.
        blocks.sort_by(|a, b| a.header.height.cmp(&b.header.height));
        Ok(Some(blocks))
    }

    /// Processes an individual block
    async fn process_block(&self, block: SequencerBlockSubset) -> eyre::Result<()> {
        info!(
            actor_name = "reader",
            height = block.header.height.value(),
            "sequencer block received from DA layer"
        );
        self.executor_tx.send(
            executor::ExecutorCommand::BlockReceivedFromDataAvailability {
                block: Box::new(block),
            },
        )?;

        Ok(())
    }
}
