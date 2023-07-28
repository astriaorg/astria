//! The driver is the top-level coordinator that runs and manages all the components
//! necessary for this reader.

use std::sync::Arc;

use astria_sequencer_relayer::types::SequencerBlockData;
use color_eyre::eyre::{
    eyre,
    Result,
    WrapErr as _,
};
use futures::StreamExt;
use sync_wrapper::SyncWrapper;
use tokio::{
    select,
    sync::{
        mpsc::{
            self,
            UnboundedReceiver,
            UnboundedSender,
        },
        Mutex,
    },
};
use tracing::{
    debug,
    info,
    warn,
};

use crate::{
    alert::AlertSender,
    block_verifier::BlockVerifier,
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

    /// The gossip network must be wrapped in a `SyncWrapper` for now, as the transport
    /// within the gossip network is not `Send`.
    /// See https://github.com/astriaorg/astria/issues/111 for more details.
    network: SyncWrapper<GossipNetwork>,

    block_verifier: Arc<BlockVerifier>,

    is_shutdown: Mutex<bool>,
}

impl Driver {
    pub async fn new(
        conf: Config,
        alert_tx: AlertSender,
    ) -> Result<(Self, executor::JoinHandle, Option<reader::JoinHandle>)> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (executor_join_handle, executor_tx) = executor::spawn(&conf, alert_tx.clone())
            .await
            .wrap_err("failed to construct Executor")?;

        let block_verifier = Arc::new(
            BlockVerifier::new(&conf.tendermint_url)
                .wrap_err("failed to construct BlockVerifier")?,
        );

        let (reader_join_handle, reader_tx) = if conf.disable_finalization {
            (None, None)
        } else {
            let (reader_join_handle, reader_tx) =
                reader::spawn(&conf, executor_tx.clone(), block_verifier.clone())
                    .await
                    .wrap_err("failed to construct data availability Reader")?;
            (Some(reader_join_handle), Some(reader_tx))
        };

        Ok((
            Self {
                cmd_tx: cmd_tx.clone(),
                cmd_rx,
                reader_tx,
                executor_tx,
                network: SyncWrapper::new(
                    GossipNetwork::new(conf.bootnodes, conf.libp2p_private_key, conf.libp2p_port)
                        .wrap_err("failed to construct gossip network")?,
                ),
                block_verifier,
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
                res = self.network.get_mut().0.next() => {
                    if let Some(res) = res {
                        match res {
                            Ok(event) => {
                                if let Err(e) = self.handle_network_event(event).await {
                                    debug!(error = ?e, "failed to handle network event");
                                }
                            }
                            Err(err) => {
                                warn!(error = ?err, "encountered error while polling p2p network");
                            }
                        }
                    }
                },
                cmd = self.cmd_rx.recv() => {
                    if let Some(cmd) = cmd {
                        self.handle_driver_command(cmd).await.wrap_err("failed to handle driver command")?;
                    } else {
                        info!("Driver command channel closed.");
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle_network_event(&self, event: NetworkEvent) -> Result<()> {
        match event {
            NetworkEvent::NewListenAddr(addr) => {
                info!("listening on {}", addr);
            }
            NetworkEvent::GossipsubMessage(msg) => {
                debug!("received gossip message: {:?}", msg);
                let block = SequencerBlockData::from_bytes(&msg.data)
                    .wrap_err("failed to deserialize SequencerBlockData received from network")?;

                // validate block received from gossip network
                self.block_verifier
                    .validate_sequencer_block(&block)
                    .await
                    .wrap_err("invalid block received from gossip network")?;

                self.executor_tx
                    .send(ExecutorCommand::BlockReceivedFromGossipNetwork {
                        block: Box::new(block),
                    })
                    .wrap_err("failed to send SequencerBlockData from network to executor")?;
            }
            _ => debug!("received network event: {:?}", event),
        }

        Ok(())
    }

    async fn handle_driver_command(&mut self, cmd: DriverCommand) -> Result<()> {
        match cmd {
            DriverCommand::Shutdown => {
                self.shutdown().await?;
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
    async fn shutdown(&mut self) -> Result<()> {
        let mut is_shutdown = self.is_shutdown.lock().await;
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
