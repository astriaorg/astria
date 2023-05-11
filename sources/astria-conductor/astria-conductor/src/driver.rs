//! The driver is the top-level coordinator that runs and manages all the components
//! necessary for this reader/validator.

use std::sync::Mutex;

use color_eyre::eyre::{
    eyre,
    Result,
};
use futures::StreamExt;
use log::{
    debug,
    info,
};
use sequencer_relayer::sequencer_block::SequencerBlock;
use tokio::{
    select,
    sync::mpsc::{
        self,
        UnboundedReceiver,
        UnboundedSender,
    },
};

use crate::{
    alert::AlertSender,
    config::Config,
    executor,
    executor::ExecutorCommand,
    network::{
        Event as NetworkEvent,
        GossipNetwork,
    },
    reader::{
        self,
        ReaderCommand,
    },
};

/// The channel through which the user can send commands to the driver.
pub(crate) type Sender = UnboundedSender<DriverCommand>;
/// The channel on which the driver listens for commands from the user.
pub(crate) type Receiver = UnboundedReceiver<DriverCommand>;

/// The type of commands that the driver can receive.
#[derive(Debug)]
pub enum DriverCommand {
    /// Get new blocks
    GetNewBlocks,
    /// Gracefully shuts down the driver and its components.
    Shutdown,
}

pub struct Driver {
    pub cmd_tx: Sender,

    /// The channel on which other components in the driver sends the driver messages.
    cmd_rx: Receiver,

    /// The channel used to send messages to the reader task.
    reader_tx: Option<reader::Sender>,

    /// The channel used to send messages to the executor task.
    executor_tx: executor::Sender,

    network: GossipNetwork,

    is_shutdown: Mutex<bool>,
}

impl Driver {
    pub async fn new(
        conf: Config,
        alert_tx: AlertSender,
    ) -> Result<(Self, executor::JoinHandle, Option<reader::JoinHandle>)> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (executor_join_handle, executor_tx) = executor::spawn(&conf, alert_tx.clone()).await?;

        let (reader_join_handle, reader_tx) = if conf.disable_finalization {
            (None, None)
        } else {
            let (reader_join_handle, reader_tx) = reader::spawn(&conf, executor_tx.clone()).await?;
            (Some(reader_join_handle), Some(reader_tx))
        };

        Ok((
            Self {
                cmd_tx: cmd_tx.clone(),
                cmd_rx,
                reader_tx,
                executor_tx,
                network: GossipNetwork::new(conf.bootnodes)?,
                is_shutdown: Mutex::new(false),
            },
            executor_join_handle,
            reader_join_handle,
        ))
    }

    /// Runs the Driver event loop.
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting driver event loop.");
        loop {
            select! {
                res = self.network.0.next() => {
                    if let Some(res) = res {
                        self.handle_network_event(res)?;
                    }
                },
                cmd = self.cmd_rx.recv() => {
                    if let Some(cmd) = cmd {
                        self.handle_driver_command(cmd)?;
                    } else {
                        info!("Driver command channel closed.");
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_network_event(&self, event: NetworkEvent) -> Result<()> {
        match event {
            NetworkEvent::NewListenAddr(addr) => {
                info!("listening on {}", addr);
            }
            NetworkEvent::Message(msg) => {
                debug!("received gossip message: {:?}", msg);
                let block = SequencerBlock::from_bytes(&msg.data)?;
                self.executor_tx
                    .send(ExecutorCommand::BlockReceivedFromGossipNetwork {
                        block: Box::new(block),
                    })?;
            }
            _ => debug!("received network event: {:?}", event),
        }

        Ok(())
    }

    fn handle_driver_command(&mut self, cmd: DriverCommand) -> Result<()> {
        match cmd {
            DriverCommand::Shutdown => {
                self.shutdown()?;
            }

            DriverCommand::GetNewBlocks => {
                let Some(reader_tx) = &self.reader_tx else {
                    return Ok(());
                };

                reader_tx
                    .send(ReaderCommand::GetNewBlocks)
                    .map_err(|e| eyre!("reader rx channel closed: {}", e))?;
            }
        }

        Ok(())
    }

    /// Sends shutdown commands to the other actors.
    fn shutdown(&mut self) -> Result<()> {
        let mut is_shutdown = self.is_shutdown.lock().unwrap();
        if *is_shutdown {
            return Ok(());
        }
        *is_shutdown = true;

        info!("Shutting down driver.");
        self.executor_tx.send(ExecutorCommand::Shutdown)?;

        let Some(reader_tx) = &self.reader_tx else {
            return Ok(());
        };
        reader_tx.send(ReaderCommand::Shutdown)?;

        Ok(())
    }
}
