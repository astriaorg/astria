use color_eyre::eyre::{eyre, Result};
use log::{error, info};
use sequencer_relayer::{da::CelestiaClient, sequencer_block::SequencerBlock};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task,
};

use crate::config::Config;
use crate::executor;

pub(crate) type JoinHandle = task::JoinHandle<Result<()>>;

/// The channel for sending commands to the reader task.
pub(crate) type Sender = UnboundedSender<ReaderCommand>;
/// The channel the reader task uses to listen for commands.
type Receiver = UnboundedReceiver<ReaderCommand>;

/// spawns a reader task and returns a tuple with the task's join handle
/// and the channel for sending commands to this reader
pub(crate) async fn spawn(
    conf: &Config,
    executor_tx: executor::Sender,
) -> Result<(JoinHandle, Sender)> {
    info!("Spawning reader task.");
    let (mut reader, reader_tx) = Reader::new(&conf.celestia_node_url, executor_tx).await?;
    let join_handle = task::spawn(async move { reader.run().await });
    info!("Spawned reader task.");
    Ok((join_handle, reader_tx))
}

#[derive(Debug)]
pub(crate) enum ReaderCommand {
    /// Get new blocks
    GetNewBlocks,

    Shutdown,
}

struct Reader {
    /// Channel on which reader commands are received.
    cmd_rx: Receiver,

    /// The channel used to send messages to the executor task.
    executor_tx: executor::Sender,

    /// The client used to communicate with Celestia.
    celestia_client: CelestiaClient,

    /// the last block height fetched from Celestia
    curr_block_height: u64,
}

impl Reader {
    /// Creates a new Reader instance and returns a command sender and an alert receiver.
    async fn new(celestia_node_url: &str, executor_tx: executor::Sender) -> Result<(Self, Sender)> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let celestia_client = CelestiaClient::new(celestia_node_url.to_owned())?;

        // TODO: we should probably pass in the height we want to start at from some genesis/config file
        let curr_block_height = celestia_client.get_latest_height().await?;
        Ok((
            Self {
                cmd_rx,
                executor_tx,
                celestia_client,
                curr_block_height,
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
    async fn get_new_blocks(&mut self) -> Result<Vec<SequencerBlock>> {
        info!("ReaderCommand::GetNewBlocks");
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
            let res = self.get_block(height).await;

            match res {
                Ok(block) => {
                    // continue as celestia block doesn't have a sequencer block
                    let Some(block) = block else {
                        continue;
                    };

                    // sequencer block's height
                    let height = block.header.height.parse::<u64>()?;
                    info!("got sequencer block with height: {:?}", height);
                    blocks.push(block);
                }
                Err(e) => {
                    // just log the error for now.
                    // any blocks that weren't fetched will be handled in the next cycle
                    error!("{}", e.to_string());
                }
            }
        }

        Ok(blocks)
    }

    /// Gets an individual block for a given Celestia height
    async fn get_block(&self, height: u64) -> Result<Option<SequencerBlock>> {
        let res = self.celestia_client.get_blocks(height, None).await?;
        // TODO: we need to verify the block using the expected proposer's key (by passing in their pubkey above)
        // and ensure there's only one block signed by them
        Ok(res.into_iter().next())
    }

    /// Processes an individual block
    async fn process_block(&self, block: SequencerBlock) -> Result<()> {
        self.executor_tx
            .send(executor::ExecutorCommand::BlockReceived {
                block: Box::new(block),
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const DEFAULT_CELESTIA_ENDPOINT: &str = "http://localhost:26659";

    #[tokio::test]
    async fn test_reader_get_new_blocks() {
        let (executor_tx, _) = mpsc::unbounded_channel();

        let (mut reader, _reader_tx) = Reader::new(DEFAULT_CELESTIA_ENDPOINT, executor_tx)
            .await
            .unwrap();

        let blocks = reader.get_new_blocks().await.unwrap();
        assert!(blocks.len() > 0);
    }
}
