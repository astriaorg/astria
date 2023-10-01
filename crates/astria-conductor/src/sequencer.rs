//! The driver is the top-level coordinator that runs and manages all the components
//! necessary for this reader.

use astria_sequencer_types::SequencerBlockData;
use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use sequencer_client::NewBlockStreamError;
use tokio::{
    select,
    sync::oneshot,
};
use tracing::{
    info,
    instrument,
    warn,
};

use crate::{
    executor,
    executor::ExecutorCommand,
};

pub(crate) struct Reader {
    /// The channel used to send messages to the executor task.
    executor_tx: executor::Sender,

    pool: deadpool::managed::Pool<crate::client_provider::ClientProvider>,

    shutdown: oneshot::Receiver<()>,
}

impl Reader {
    #[instrument(name = "driver", skip_all)]
    pub(crate) async fn new(
        pool: deadpool::managed::Pool<crate::client_provider::ClientProvider>,
        shutdown: oneshot::Receiver<()>,
        executor_tx: executor::Sender,
    ) -> eyre::Result<Self> {
        Ok(Self {
            executor_tx,
            pool,
            shutdown,
        })
    }

    /// Run the sequencer reader event loop
    #[instrument(skip_all)]
    pub(crate) async fn run_until_stopped(mut self) -> eyre::Result<()> {
        use futures::StreamExt as _;
        use sequencer_client::SequencerSubscriptionClientExt as _;

        info!("Starting driver event loop.");
        let mut new_blocks = self
            .pool
            .get()
            .await
            .wrap_err("failed getting an initial sequencer client from the pool")?
            .subscribe_new_block_data()
            .await
            .wrap_err("failed subscribing to sequencer to receive new blocks")?;
        loop {
            select! {
                shutdown = &mut self.shutdown => {
                    match shutdown {
                        Err(e) => warn!(error.message = %e, "shutdown channel return with error; shutting down"),
                        Ok(()) => info!("received shutdown signal; shutting down"),
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
            }
        }
        Ok(())
    }

    fn handle_new_block(&self, block: eyre::Result<SequencerBlockData, NewBlockStreamError>) {
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
