//! This module defines the alerts the driver consumer may receive from the driver.
//!
//! Communication of such alerts is performed via unbounded [tokio mpsc
//! channels](tokio::sync::mpsc). Thus, the application in which the driver is integrated may be
//! driven by these alerts.

use color_eyre::eyre::Error;
use tokio::sync::mpsc::UnboundedSender;

/// The channel used by the driver to send out alerts.
pub(crate) type AlertSender = UnboundedSender<Alert>;

/// The alerts that the driver may send the driver user.
#[derive(Debug)]
pub enum Alert {
    /// Send when a block has been received from the gossip network.
    BlockReceivedFromGossipNetwork {
        /// The height of the block received
        block_height: u64,
    },

    /// Send when a block has been received from the data layer.
    BlockReceivedFromDataAvailability {
        /// The height of the block received
        block_height: u64,
    },

    /// An error from somewhere inside the driver.
    DriverError(Error),
}
