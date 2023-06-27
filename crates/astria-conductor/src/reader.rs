use std::sync::Arc;

use astria_sequencer_relayer::{
    data_availability::{
        CelestiaClient,
        CelestiaClientBuilder,
        SequencerNamespaceData,
        SignedNamespaceData,
    },
    sequencer_block::SequencerBlock,
};
use color_eyre::eyre::{
    eyre,
    Result,
    WrapErr as _,
};
use ed25519_consensus::VerificationKey;
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
    error,
    info,
    warn,
};

use crate::{
    block_verifier::BlockVerifier,
    config::Config,
    executor,
};

pub(crate) type JoinHandle = task::JoinHandle<Result<()>>;

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
) -> Result<(JoinHandle, Sender)> {
    info!("Spawning reader task.");
    let (mut reader, reader_tx) = Reader::new(&conf.celestia_node_url, executor_tx, block_verifier)
        .await
        .wrap_err("failed to create Reader")?;
    let join_handle = task::spawn(async move { reader.run().await });
    info!("Spawned reader task.");
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
}

impl Reader {
    /// Creates a new Reader instance and returns a command sender and an alert receiver.
    pub async fn new(
        celestia_node_url: &str,
        executor_tx: executor::Sender,
        block_verifier: Arc<BlockVerifier>,
    ) -> Result<(Self, Sender)> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let celestia_client = CelestiaClientBuilder::new(celestia_node_url.to_owned())
            .build()
            .wrap_err("failed creating celestia client")?;

        // TODO: we should probably pass in the height we want to start at from some genesis/config
        // file
        let curr_block_height = celestia_client.get_latest_height().await?;
        Ok((
            Self {
                cmd_rx,
                executor_tx,
                celestia_client,
                curr_block_height,
                block_verifier,
            },
            cmd_tx,
        ))
    }

    async fn run(&mut self) -> Result<()> {
        info!("Starting reader event loop.");

        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                ReaderCommand::GetNewBlocks => {
                    let blocks = self
                        .get_new_blocks()
                        .await
                        .map_err(|e| eyre!("failed to get new block: {}", e))?;
                    for block in blocks {
                        self.process_block(block)
                            .await
                            .map_err(|e| eyre!("failed to process block: {}", e))?;
                    }
                }
                ReaderCommand::Shutdown => {
                    info!("Shutting down reader event loop.");
                    break;
                }
            }
        }

        Ok(())
    }

    /// get_new_blocks fetches any new sequencer blocks from Celestia.
    pub async fn get_new_blocks(&mut self) -> Result<Vec<SequencerBlock>> {
        debug!("ReaderCommand::GetNewBlocks");
        let mut blocks = vec![];

        // get the latest celestia block height
        let prev_height = self.curr_block_height;
        self.curr_block_height = self.celestia_client.get_latest_height().await?;
        info!(
            "checking celestia blocks {} to {}",
            prev_height, self.curr_block_height
        );

        // check for any new sequencer blocks written from the previous to current block height
        for height in prev_height..self.curr_block_height {
            let res = self
                .celestia_client
                .get_sequencer_namespace_data(height, None)
                .await;

            match res {
                Ok(datas) => {
                    // continue as celestia block doesn't have a sequencer block
                    if datas.is_empty() {
                        continue;
                    };

                    for data in datas {
                        // validate data
                        self.block_verifier
                            .validate_signed_namespace_data(&data)
                            .await
                            .wrap_err("failed to validate signed namepsace data")?;

                        let block = match self.get_sequencer_block_from_namespace_data(&data).await
                        {
                            Ok(block) => block,
                            Err(e) => {
                                // this means someone submitted an invalid block to celestia;
                                // we can ignore it
                                warn!(error = ?e, "failed to get sequencer block from namespace data");
                                continue;
                            }
                        };

                        if let Err(e) = self.block_verifier.validate_sequencer_block(&block).await {
                            // this means someone submitted an invalid block to celestia;
                            // we can ignore it
                            warn!(error = ?e, "sequencer block failed validation");
                            continue;
                        }

                        blocks.push(block);
                    }
                }
                Err(e) => {
                    // just log the error for now.
                    // any blocks that weren't fetched will be handled in the next cycle
                    error!("{}", e.to_string());
                }
            }
        }

        // sort blocks by height
        // TODO: there isn't a guarantee that the blocks aren't severely out of order,
        // and we need to ensure that there are no gaps between the block heights before processing.
        blocks.sort_by(|a, b| a.header.height.cmp(&b.header.height));
        Ok(blocks)
    }

    /// get the full SequencerBlock from the base SignedNamespaceData
    async fn get_sequencer_block_from_namespace_data(
        &self,
        data: &SignedNamespaceData<SequencerNamespaceData>,
    ) -> Result<SequencerBlock> {
        // the reason the public key type needs to be converted is due to serialization
        // constraints, probably fix this later
        let verification_key = VerificationKey::try_from(&*data.public_key.0)?;

        // pass the public key to `get_sequencer_block` which does the signature validation for us
        let block = self
            .celestia_client
            .get_sequencer_block(&data.data, Some(verification_key))
            .await
            .map_err(|e| eyre!("failed to get rollup data: {}", e))?;

        Ok(block)
    }

    /// Processes an individual block
    async fn process_block(&self, block: SequencerBlock) -> Result<()> {
        self.executor_tx.send(
            executor::ExecutorCommand::BlockReceivedFromDataAvailability {
                block: Box::new(block),
            },
        )?;

        Ok(())
    }
}
