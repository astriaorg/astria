use color_eyre::eyre::Result;
use sequencer_relayer::{
    da::CelestiaClient,
    sequencer_block::{Namespace, SequencerBlock},
};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task,
};

use crate::config::Config;
use crate::{driver, executor};

pub(crate) type JoinHandle = task::JoinHandle<Result<()>>;

/// The channel for sending commands to the reader task.
pub(crate) type Sender = UnboundedSender<ReaderCommand>;
/// The channel the reader task uses to listen for commands.
type Receiver = UnboundedReceiver<ReaderCommand>;

/// spawns a reader task and returns a tuple with the task's join handle
/// and the channel for sending commands to this reader
pub(crate) fn spawn(
    conf: &Config,
    driver_tx: driver::Sender,
    executor_tx: executor::Sender,
) -> Result<(JoinHandle, Sender)> {
    log::info!("Spawning reader task.");
    let (mut reader, reader_tx) = Reader::new(
        &conf.celestia_node_url,
        Namespace::from_string(&conf.chain_id)?,
        driver_tx,
        executor_tx,
    )?;
    let join_handle = task::spawn(async move { reader.run().await });
    log::info!("Spawned reader task.");
    Ok((join_handle, reader_tx))
}

#[derive(Debug)]
#[allow(dead_code)] // TODO - remove after developing
pub(crate) enum ReaderCommand {
    /// Get new blocks
    GetNewBlocks,

    Shutdown,
}

#[allow(dead_code)] // TODO - remove after developing
struct Reader {
    /// Channel on which reader commands are sent.
    cmd_tx: Sender,
    /// Channel on which reader commands are received.
    cmd_rx: Receiver,
    /// Channel on which the reader sends commands to the driver.
    driver_tx: driver::Sender,

    /// The channel used to send messages to the executor task.
    executor_tx: executor::Sender,

    /// The client used to communicate with Celestia.
    celestia_client: CelestiaClient,

    /// Namespace ID
    namespace: Namespace,

    /// Keep track of the last block height fetched from Celestia
    last_block_height: u64,
}

impl Reader {
    /// Creates a new Reader instance and returns a command sender and an alert receiver.
    fn new(
        celestia_node_url: &str,
        namespace: Namespace,
        driver_tx: driver::Sender,
        executor_tx: executor::Sender,
    ) -> Result<(Self, Sender)> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let celestia_client = CelestiaClient::new(celestia_node_url.to_owned())?;
        Ok((
            Self {
                cmd_tx: cmd_tx.clone(),
                cmd_rx,
                driver_tx,
                executor_tx,
                celestia_client,
                namespace,
                last_block_height: 1,
            },
            cmd_tx,
        ))
    }

    async fn run(&mut self) -> Result<()> {
        log::info!("Starting reader event loop.");

        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                ReaderCommand::GetNewBlocks => {
                    let blocks = self.get_new_blocks().await?;
                    for block in blocks {
                        self.process_block(block).await?;
                    }
                }
                ReaderCommand::Shutdown => {
                    log::info!("Shutting down reader event loop.");
                    break;
                }
            }
        }

        Ok(())
    }

    /// get_new_blocks fetches any new sequencer blocks from Celestia.
    async fn get_new_blocks(&mut self) -> Result<Vec<SequencerBlock>> {
        log::info!("ReaderCommand::GetNewBlocks");
        let mut blocks = vec![];

        // get the latest celestia block height
        let latest_height = self.celestia_client.get_latest_height().await?;

        // check for any new sequencer blocks written from the previous to current block height
        for height in self.last_block_height..latest_height {
            let res = self.get_block(height).await;

            match res {
                Ok(block) => {
                    println!("block: {:?}", block);

                    // continue as celestia block doesn't have a sequencer block
                    let Some(block) = block else {
                        continue;
                    };

                    // sequencer block's height
                    let height = block.header.height.parse::<u64>()?;
                    println!("sequencer block height: {:?}", height);
                    blocks.push(block);
                }
                Err(e) => {
                    // just log the error for now.
                    // any blocks that weren't fetched will be handled in the next cycle
                    log::error!("{}", e.to_string());
                }
            }
        }

        self.last_block_height = latest_height;
        Ok(blocks)
    }

    /// Gets an individual block for a given Celestia height
    async fn get_block(&mut self, height: u64) -> Result<Option<SequencerBlock>> {
        let res = self.celestia_client.get_blocks(height, None).await?;
        // TODO: we need to verify the block using the expected proposer's key (by passing in their pubkey above)
        // and ensure there's only one block signed by them
        Ok(res.into_iter().next())
    }

    /// Processes an individual block
    async fn process_block(&mut self, block: SequencerBlock) -> Result<()> {
        self.last_block_height = block.header.height.parse::<u64>()?;
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
    use sequencer_relayer::sequencer_block::DEFAULT_NAMESPACE;

    const DEFAULT_CELESTIA_ENDPOINT: &str = "http://localhost:26659";

    #[tokio::test]
    async fn test_reader_get_new_blocks() {
        let (driver_tx, _) = mpsc::unbounded_channel();
        let (executor_tx, _) = mpsc::unbounded_channel();

        let (mut reader, _reader_tx) = Reader::new(
            DEFAULT_CELESTIA_ENDPOINT,
            DEFAULT_NAMESPACE.clone(),
            driver_tx,
            executor_tx,
        )
        .unwrap();

        let blocks = reader.get_new_blocks().await.unwrap();
        println!("blocks: {:?}", blocks);
        assert!(blocks.len() > 0);
    }
}
