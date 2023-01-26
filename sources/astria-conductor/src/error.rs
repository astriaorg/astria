//! This module includes the errors that could occur in the driver. They are
//! reported via the [alert system](crate::alert), as most operations via the
//! driver happen asynchronously.

use std::fmt;
use std::result::Result as StdResult;

use thiserror;
pub use tokio::{io::Error as IoError, sync::mpsc::error::SendError, task::JoinError};

/// A special result type for rvrs
pub type Result<T, E = Error> = StdResult<T, E>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The channel on which some component in engine was listening or sending
    /// died.
    Channel,

    /// Holds global IO related errors.
    Io(IoError),
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Channel => write!(fmt, "channel error"),
            Error::Io(e) => e.fmt(fmt),
        }
    }
}

// impl std::error::Error for Error {
//     fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
//         match self {
//             Error::Io(e) => Some(e),
//             _ => None,
//         }
//     }
// }

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Self::Io(e)
    }
}

impl<T> From<SendError<T>> for Error {
    fn from(_: SendError<T>) -> Self {
        Self::Channel
    }
}

impl From<JoinError> for Error {
    fn from(_: JoinError) -> Self {
        Self::Channel
    }
}
