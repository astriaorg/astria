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
    GetNewBlocks,

    Shutdown,
}

#[allow(dead_code)] // TODO - remove after developing
struct Reader {
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
                cmd_rx,
                driver_tx,
                celestia_node_client,
                namespace_id: conf.namespace_id.to_owned(),
                last_block_height: 0,
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

        // FIXME - what to do?
        //   * we could add blocks to a VecDeque as they come in.
        //     * we

        let res = self
            .celestia_node_client
            // NOTE - requesting w/ height of 0 gives us the last block
            .namespaced_data(&self.namespace_id, 0)
            .await;

        match res {
            Ok(namespaced_data) => {
                println!("{:#?}", namespaced_data);

                if let Some(height) = namespaced_data.height {
                    println!("pushing {}", height);
                    self.blocks_queue.push(namespaced_data, height);

                    // TODO - clean this up and get bocks in parallel
                    // get blocks between current height and last height received
                    for h in (self.last_block_height + 1)..(height) {
                        let res = self
                            .celestia_node_client
                            .namespaced_data(&self.namespace_id, h)
                            .await;

                        match res {
                            Ok(namespaced_data) => {
                                if let Some(height) = namespaced_data.height {
                                    println!("pushing {}", height);
                                    self.blocks_queue.push(namespaced_data, height);
                                }
                            }
                            Err(e) => {
                                // FIXME - how do we want to handle an error here?
                                //  log it and the other blocks will be handled next time?
                                log::error!("{}", e.to_string());
                            }
                        }

                    }
                    self.process_blocks_queue().await?;
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

    async fn process_blocks_queue(&mut self) -> Result<()> {
        for (item, _) in self.blocks_queue.clone().into_sorted_iter() {
            // TODO - send a message to executor
            println!("processing block {:#?}", item);
        }
        self.blocks_queue.clear();

        Ok(())
    }
}
