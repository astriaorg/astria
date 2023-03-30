use color_eyre::eyre::Result;
use sequencer_relayer::sequencer_block::SequencerBlock;
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task,
};

use crate::execution_client::ExecutionRpcClient;

use crate::config::Config;
use crate::driver;

pub(crate) type JoinHandle = task::JoinHandle<Result<()>>;

/// The channel for sending commands to the executor task.
pub(crate) type Sender = UnboundedSender<ExecutorCommand>;
/// The channel the executor task uses to listen for commands.
type Receiver = UnboundedReceiver<ExecutorCommand>;

/// spawns a executor task and returns a tuple with the task's join handle
/// and the channel for sending commands to this executor
pub(crate) async fn spawn(
    conf: &Config,
    driver_tx: driver::Sender,
) -> Result<(JoinHandle, Sender)> {
    log::info!("Spawning executor task.");
    let (mut executor, executor_tx) = Executor::new(conf, driver_tx).await?;
    let join_handle = task::spawn(async move { executor.run().await });
    log::info!("Spawned executor task.");
    Ok((join_handle, executor_tx))
}

#[derive(Debug)]
pub(crate) enum ExecutorCommand {
    /// Command for when a block is received
    BlockReceived {
        block: Box<SequencerBlock>,
    },

    Shutdown,
}

#[allow(dead_code)] // TODO - remove after developing
struct Executor {
    /// Channel on which executor commands are received.
    cmd_rx: Receiver,
    /// Channel on which the executor sends commands to the driver.
    driver_tx: driver::Sender,
    /// The execution rpc client that we use to send messages to the execution service
    execution_rpc_client: ExecutionRpcClient,
}

impl Executor {
    /// Creates a new Executor instance and returns a command sender and an alert receiver.
    async fn new(conf: &Config, driver_tx: driver::Sender) -> Result<(Self, Sender)> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

        // TODO - error handling
        let execution_rpc_client = ExecutionRpcClient::new(&conf.rpc_address)
            .await
            .expect("uh oh");

        Ok((
            Self {
                cmd_rx,
                driver_tx,
                execution_rpc_client,
            },
            cmd_tx,
        ))
    }

    async fn run(&mut self) -> Result<()> {
        log::info!("Starting executor event loop.");

        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                ExecutorCommand::BlockReceived { block } => {
                    log::info!("ExecutorCommand::BlockReceived {:#?}", block);
                    self.execute_block(*block).await?;
                }
                ExecutorCommand::Shutdown => {
                    log::info!("Shutting down executor event loop.");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Uses RPC to send block to execution service
    async fn execute_block(&mut self, _block: SequencerBlock) -> Result<()> {
        // TODO - handle error properly
        let fake_header: Vec<u8> = vec![0, 1, 255];
        let fake_tx: Vec<Vec<u8>> = vec![vec![0, 1, 255], vec![1, 2, 3], vec![1, 0, 1, 1]];
        self.execution_rpc_client
            .call_do_block(fake_header, fake_tx)
            .await
            .expect("uh oh do block");

        Ok(())
    }
}
