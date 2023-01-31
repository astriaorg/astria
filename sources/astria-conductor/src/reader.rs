use rs_cnc::CelestiaNodeClient;
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task,
};

use crate::conf::Conf;
use crate::{driver, error::*};

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

    /// Keep track of the last block height fetched so we know how to get the next one
    last_block_height: Option<u64>,
}

impl Reader {
    /// Creates a new Reader instance and returns a command sender and an alert receiver.
    fn new(conf: &Conf, driver_tx: driver::Sender) -> Result<(Self, Sender)> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let celestia_node_client = CelestiaNodeClient::new(conf.celestia_node_url.to_owned())?;
        // initial last_block_height of 0
        let last_block_height = Some(0);
        Ok((
            Self {
                cmd_rx,
                driver_tx,
                celestia_node_client,
                namespace_id: conf.namespace_id.to_owned(),
                last_block_height,
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

    async fn get_new_blocks(&mut self) -> Result<()> {
        log::info!("ReaderCommand::GetNewBlocks");
        let height = 0;
        let res = self
            .celestia_node_client
            .namespaced_data(&self.namespace_id, height)
            .await;

        match res {
            Ok(namespaced_data) => {
                // TODO - increase last_block_height
                println!("{:#?}", namespaced_data);
            }
            Err(e) => {
                // FIXME - how do we want to handle an error here?
                println!("UH OH! {}", e.to_string());
            }
        }

        Ok(())
    }
}
