//! The driver is the top-level coordinator that runs and manages all the components
//! necessary for this reader.

use std::fmt;

use astria_sequencer_types::SequencerBlockData;
use color_eyre::eyre::{
    eyre,
    Result,
    WrapErr as _,
};
use sequencer_client::{
    tendermint,
    NewBlockStreamError,
    WebSocketClient,
};
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
    task::JoinHandle,
};
use tracing::{
    info,
    instrument,
    span,
    warn,
    Instrument,
    Level,
};

use crate::{
    block_verifier::BlockVerifier,
    config::Config,
    executor,
    executor::ExecutorCommand,
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

#[derive(Debug)]
pub struct Driver {
    pub cmd_tx: Sender,

    /// The channel on which other components in the driver sends the driver messages.
    cmd_rx: Receiver,

    /// The channel used to send messages to the reader task.
    reader_tx: Option<reader::Sender>,

    /// The channel used to send messages to the executor task.
    executor_tx: executor::Sender,

    /// A client that subscribes to new sequencer blocks from cometbft.
    sequencer_client: SequencerClient,

    is_shutdown: Mutex<bool>,
}

struct SequencerClient {
    client: WebSocketClient,
    _driver: JoinHandle<Result<(), tendermint::Error>>,
}

impl SequencerClient {
    async fn new(url: &str) -> Result<Self, tendermint::Error> {
        let (client, driver) = WebSocketClient::new(url).await?;
        Ok(Self {
            client,
            _driver: tokio::spawn(async move { driver.run().await }),
        })
    }
}

impl fmt::Debug for SequencerClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SequencerClient")
            .field("client", &self.client)
            .finish_non_exhaustive()
    }
}

impl Driver {
    #[instrument(name = "driver", skip_all)]
    pub async fn new(
        conf: Config,
    ) -> Result<(Self, executor::JoinHandle, Option<reader::JoinHandle>)> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let executor_span = span!(Level::ERROR, "executor::spawn");
        let (executor_join_handle, executor_tx) = executor::spawn(&conf)
            .instrument(executor_span)
            .await
            .wrap_err("failed to construct Executor")?;

        let block_verifier = BlockVerifier::new(&conf.tendermint_url)
            .wrap_err("failed to construct block verifier")?;

        let (reader_join_handle, reader_tx) = if conf.disable_finalization {
            (None, None)
        } else {
            let reader_span = span!(Level::ERROR, "reader::spawn");
            let (reader_join_handle, reader_tx) =
                reader::spawn(&conf, executor_tx.clone(), block_verifier)
                    .instrument(reader_span)
                    .await
                    .wrap_err("failed to construct data availability Reader")?;
            (Some(reader_join_handle), Some(reader_tx))
        };

        let sequencer_client = SequencerClient::new(&conf.sequencer_url)
            .await
            .wrap_err("failed constructing a cometbft websocket client to read off sequencer")?;

        Ok((
            Self {
                cmd_tx: cmd_tx.clone(),
                cmd_rx,
                reader_tx,
                executor_tx,
                sequencer_client,
                is_shutdown: Mutex::new(false),
            },
            executor_join_handle,
            reader_join_handle,
        ))
    }

    /// Runs the Driver event loop.
    #[instrument(name = "driver", skip_all)]
    pub async fn run(&mut self) -> Result<()> {
        use futures::StreamExt as _;
        use sequencer_client::SequencerSubscriptionClientExt as _;
        info!("Starting driver event loop.");
        let mut new_blocks = self
            .sequencer_client
            .client
            .subscribe_new_block_data()
            .await
            .wrap_err("failed subscribing to sequencer to receive new blocks")?;
        // FIXME(https://github.com/astriaorg/astria/issues/381): the event handlers
        // here block the select loop because they `await` their return.
        loop {
            select! {
                new_block = new_blocks.next() => {
                    if let Some(block) = new_block {
                        self.handle_new_block(block).await
                    } else {
                        warn!("sequencer new-block subscription closed unexpectedly; shutting down driver");
                        break;
                    }
                }
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

    async fn handle_new_block(&self, block: Result<SequencerBlockData, NewBlockStreamError>) {
        let block = match block {
            Err(err) => {
                warn!(err.msg = %err, err.cause = ?err, "encountered an error while receiving a new block from sequencer");
                return;
            }
            Ok(new_block) => new_block,
        };

        if let Err(err) = self
            .executor_tx
            .send(ExecutorCommand::BlockReceivedFromSequencer {
                block: Box::new(block),
            })
        {
            warn!(err.msg = %err, err.cause = ?err, "failed sending new block received from sequencer to executor");
        }
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
