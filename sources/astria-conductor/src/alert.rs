//! This module defines the alerts the driver consumer may receive from the driver.
//!
//! Communication of such alerts is performed via unbounded [tokio mpsc channels](tokio::sync::mpsc).
//! Thus, the application in which the driver is integrated may be driven by these alerts.

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::error::Error;

/// The channel used by the driver to send out alerts.
pub(crate) type AlertSender = UnboundedSender<Alert>;
/// The channel on which alerts from the driver can be received.
/// See [`Alert`] for the type of messages that can be received.
pub type AlertReceiver = UnboundedReceiver<Alert>;

/// The alerts that the driver may send the driver user.
#[derive(Debug)]
#[non_exhaustive]
pub enum Alert {
    /// Send when a block has been received from the data layer.
    BlockReceived {
        /// The height of the block received
        block_height: u64,
    },

    /// An error from somewhere inside the driver.
    DriverError(Error),
}
