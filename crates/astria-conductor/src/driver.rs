//! The driver is the top-level coordinator that runs and manages all the components
//! necessary for this reader.

use astria_sequencer_types::{
    Namespace,
    SequencerBlockData,
};
use color_eyre::eyre::{
    self,
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
    sync::oneshot,
    task::JoinHandle,
};
use tracing::{
    info,
    instrument,
    warn,
    Instrument,
};

use crate::{
    block_verifier::BlockVerifier,
    config::Config,
    executor,
    executor::ExecutorCommand,
    reader::Reader,
};

#[derive(Debug)]
pub(crate) struct Driver {
    /// The channel used to send messages to the executor task.
    executor_tx: executor::Sender,

    /// A client that subscribes to new sequencer blocks from cometbft.
    sequencer_client: WebSocketClient,

    sequencer_driver: JoinHandle<Result<(), tendermint::Error>>,

    shutdown: oneshot::Receiver<()>,

    reader_task: Option<JoinHandle<eyre::Result<()>>>,
    reader_shutdown: Option<oneshot::Sender<()>>,
}

impl Driver {
    #[instrument(name = "driver", skip_all)]
    pub(crate) async fn new(
        conf: Config,
        shutdown: oneshot::Receiver<()>,
        executor_tx: executor::Sender,
    ) -> Result<Self> {
        let (sequencer_client, sequencer_driver) = {
            let (client, driver) = WebSocketClient::new(&*conf.sequencer_url).await.wrap_err(
                "failed constructing a cometbft websocket client to read off sequencer",
            )?;
            let driver_handle = tokio::spawn(async move { driver.run().await });
            (client, driver_handle)
        };

        let block_verifier = BlockVerifier::new(sequencer_client.clone());

        let mut reader_task = None;
        let mut reader_shutdown = None;

        if !conf.disable_finalization {
            let (shutdown_tx, shutdown_rx) = oneshot::channel();
            let reader = Reader::new(
                &conf.celestia_node_url,
                &conf.celestia_bearer_token,
                std::time::Duration::from_secs(3),
                executor_tx.clone(),
                block_verifier,
                Namespace::from_slice(conf.chain_id.as_bytes()),
                shutdown_rx,
            )
            .await
            .wrap_err("failed constructing data availability reader")?;

            reader_shutdown = Some(shutdown_tx);
            reader_task = Some(tokio::spawn(reader.run().in_current_span()));
        };

        Ok(Self {
            executor_tx,
            shutdown,
            sequencer_client,
            sequencer_driver,
            reader_task,
            reader_shutdown,
        })
    }

    /// Runs the Driver event loop.
    #[instrument(name = "driver", skip_all)]
    pub(crate) async fn run_until_stopped(mut self) -> Result<()> {
        use futures::StreamExt as _;
        use sequencer_client::SequencerSubscriptionClientExt as _;

        info!("Starting driver event loop.");
        let mut new_blocks = self
            .sequencer_client
            .subscribe_new_block_data()
            .await
            .wrap_err("failed subscribing to sequencer to receive new blocks")?;
        // FIXME(https://github.com/astriaorg/astria/issues/381): the event handlers
        // here block the select loop because they `await` their return.
        loop {
            select! {
                shutdown = &mut self.shutdown => {
                    match shutdown {
                        Err(e) => warn!(error.message = %e, "shutdown channel return with error; shutting down"),
                        Ok(()) => info!("received shutdown signal; shutting down"),
                    }
                    break;
                }

                ret = async { self.reader_task.as_mut().unwrap().await }, if self.reader_task.is_some() => {
                    match ret {
                        Ok(Ok(())) => warn!("reader task exited unexpectedly; shutting down"),
                        Ok(Err(e)) => warn!(err.message = %e, err.cause = ?e, "reader task exited with error; shutting down"),
                        Err(e) => warn!(err.cause = ?e, "reader task failed; shutting down"),
                    }
                    break;
                }

                new_block = new_blocks.next() => {
                    if let Some(block) = new_block {
                        self.handle_new_block(block)
                    } else {
                        warn!("sequencer new-block subscription closed unexpectedly; shutting down driver");
                        break;
                    }
                }
                driver_res = &mut self.sequencer_driver => {
                    match driver_res {
                        Ok(Ok(())) => warn!("sequencer client websocket driver exited unexpectedly"),
                        Ok(Err(e)) => warn!(err.message = %e, err.cause = ?e, "sequencer client websocket driver exited with error"),
                        Err(e) => warn!(err.cause = ?e, "sequencer client driver task failed"),
                    }
                    break;
                }
            }
        }
        if let Some(reader_shutdown) = self.reader_shutdown {
            let _ = reader_shutdown.send(());
        };
        Ok(())
    }

    fn handle_new_block(&self, block: Result<SequencerBlockData, NewBlockStreamError>) {
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
}
