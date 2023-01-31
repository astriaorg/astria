use priority_queue::DoublePriorityQueue;
use rs_cnc::{CelestiaNodeClient, NamespacedDataResponse};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task,
};

use crate::{driver, error::*};
use crate::conf::Conf;

pub(crate) type JoinHandle = task::JoinHandle<Result<()>>;

/// The channel for sending commands to the reader task.
pub(crate) type Sender = UnboundedSender<ReaderCommand>;
/// The channel the reader task uses to listen for commands.
type Receiver = UnboundedReceiver<ReaderCommand>;

/// spawns a reader task and returns a tuple with the task's join handle
/// and the channel for sending commands to this reader
pub(crate) fn spawn(conf: &Conf, driver_tx: driver::Sender) -> Result<(JoinHandle, Sender)> {
    log::info!("Spawning reader task.");
    let (mut reader, reader_tx) = Reader::new(conf, driver_tx)?;
    let join_handle = task::spawn(async move { reader.run().await });
    log::info!("Spawned reader task.");
    Ok((join_handle, reader_tx))
}

#[derive(Debug)]
pub(crate) enum ReaderCommand {
    /// Get new blocks
    GetNewBlocks,

    /// Get a single block
    GetBlock {
        height: u64,
    },

    /// Process the blocks queue
    ProcessBlocksQueue,

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

    /// The client used to communicate with Celestia.
    celestia_node_client: CelestiaNodeClient,

    /// Namespace ID
    namespace_id: String,

    /// Keep track of the last block height fetched
    last_block_height: u64,

    /// A priority queue to hold blocks until we process them in order
    blocks_queue: DoublePriorityQueue<NamespacedDataResponse, u64>,
}

impl Reader {
    /// Creates a new Reader instance and returns a command sender and an alert receiver.
    fn new(conf: &Conf, driver_tx: driver::Sender) -> Result<(Self, Sender)> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let celestia_node_client = CelestiaNodeClient::new(conf.celestia_node_url.to_owned())?;
        Ok((
            Self {
                cmd_tx: cmd_tx.clone(),
                cmd_rx,
                driver_tx,
                celestia_node_client,
                namespace_id: conf.namespace_id.to_owned(),
                last_block_height: 0,
                // NOTE - we are using a DoublePriorityQueue because its `into_sorted_iter` order
                //  is small to large, which we want when processing blocks in order by their height.
                blocks_queue: DoublePriorityQueue::new(),
            },
            cmd_tx,
        ))
    }

    async fn run(&mut self) -> Result<()> {
        log::info!("Starting reader event loop.");

        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                ReaderCommand::GetNewBlocks => {
                    self.get_new_blocks().await?;
                }
                ReaderCommand::GetBlock { height } => {
                    self.get_block(height).await?;
                }
                ReaderCommand::ProcessBlocksQueue => {
                    self.process_blocks_queue().await?;
                }
                ReaderCommand::Shutdown => {
                    log::info!("Shutting down reader event loop.");
                    break;
                }
            }
        }

        Ok(())
    }

    /// This function is responsible for fetching all the latest blocks
    async fn get_new_blocks(&mut self) -> Result<()> {
        log::info!("ReaderCommand::GetNewBlocks");

        // get most recent block
        let res = self
            .celestia_node_client
            // NOTE - requesting w/ height of 0 gives us the last block
            .namespaced_data(&self.namespace_id, 0)
            .await;

        match res {
            Ok(namespaced_data) => {
                if let Some(height) = namespaced_data.height {
                    // push the most recent block to queue
                    self.blocks_queue.push(namespaced_data, height);
                    // get blocks between current height and last height received and push to queue
                    for h in (self.last_block_height + 1)..(height) {
                        self.cmd_tx.send(ReaderCommand::GetBlock { height: h })?;
                    }
                    // FIXME - is there a race condition possible here if a request from the previous
                    //  hasn't completed yet? i should probably try to parallelize but block on the requests above
                    self.cmd_tx.send(ReaderCommand::ProcessBlocksQueue)?;
                    self.last_block_height = height;
                }
            }
            Err(e) => {
                // just log the error for now.
                // any blocks that weren't fetched will be handled in the next cycle
                log::error!("{}", e.to_string());
            }
        }

        Ok(())
    }

    /// Gets an individual block and pushes it to the blocks queue
    async fn get_block(&mut self, height: u64) -> Result<()> {
        let res = self
            .celestia_node_client
            .namespaced_data(&self.namespace_id, height)
            .await;

        match res {
            Ok(namespaced_data) => {
                if let Some(height) = namespaced_data.height {
                    self.blocks_queue.push(namespaced_data, height);
                }
            }
            Err(e) => {
                // FIXME - how do we want to handle an error here?
                //  log it and the other blocks will be handled next time?
                log::error!("{}", e.to_string());
            }
        }
        Ok(())
    }

    /// Processes the blocks in the queue in order and sends them to the Executor.
    async fn process_blocks_queue(&mut self) -> Result<()> {
        for (item, _) in self.blocks_queue.clone().into_sorted_iter() {
            // TODO - send a message to executor
            log::info!("Processing block {:#?}", item);
        }
        self.blocks_queue.clear();

        Ok(())
    }
}
