//! This module includes the errors that could occur in the driver. They are
//! reported via the [alert system](crate::alert), as most operations via the
//! driver happen asynchronously.

use std::fmt;

pub use tokio::{io::Error as IoError, sync::mpsc::error::SendError, task::JoinError};

pub type Result<T, E = RvRsError> = std::result::Result<T, E>;

#[derive(Debug)]
#[non_exhaustive]
pub enum RvRsError {
    /// The channel on which some component in engine was listening or sending
    /// died.
    Channel,

    /// Holds global IO related errors.
    Io(IoError),
}

impl fmt::Display for RvRsError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use RvRsError::*;
        match self {
            Channel => write!(fmt, "channel error"),
            Io(e) => e.fmt(fmt),
        }
    }
}

impl std::error::Error for RvRsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use RvRsError::*;
        match self {
            Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<IoError> for RvRsError {
    fn from(e: IoError) -> Self {
        Self::Io(e)
    }
}

impl<T> From<SendError<T>> for RvRsError {
    fn from(_: SendError<T>) -> Self {
        Self::Channel
    }
}

impl From<JoinError> for RvRsError {
    fn from(_: JoinError) -> Self {
        Self::Channel
    }
}
