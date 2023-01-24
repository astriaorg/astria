//! The driver is the top-level coordinator that runs and manages all the components
//! necessary for this reader/validator.

use std::{thread, time};

use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::{
    error::*,
};

/// The channel through which the user can send commands to the driver.
pub(crate) type Sender = UnboundedSender<DriverCommand>;
/// The channel on which the driver listens for commands from the user.
pub(crate) type Receiver = UnboundedReceiver<DriverCommand>;

/// The type of commands that the driver can receive.
pub(crate) enum DriverCommand {
    /// Contains info for getting newest blocks.
    GetNewBlocks {
        last_block_height: u64,
    },

    /// Gracefully shuts down the driver.
    Shutdown,
}

pub(crate) struct Driver {
    /// The port on which other components in the driver sends the driver messages.
    cmd_rx: Receiver,
}

impl Driver {
    pub fn new() -> Result<(Self, Sender)> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

        Ok((
            Self {
                cmd_rx,
            },
            cmd_tx,
        ))
    }

    pub async fn run(&mut self) -> Result<()> {
        log::info!("Starting driver.");
        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                DriverCommand::GetNewBlocks { last_block_height } => {
                    self.get_new_blocks(last_block_height).await?;
                }
                DriverCommand::Shutdown => {
                    self.shutdown().await?;
                    break;
                }
            }
        }
        Ok(())
    }

    async fn get_new_blocks(&mut self, last_block_height: u64) -> Result<()> {
        log::info!("get_new_blocks({})", last_block_height);
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        log::info!("Shutting down driver.");
        Ok(())
    }
}