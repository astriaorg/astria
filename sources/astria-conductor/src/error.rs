//! This module includes the errors that could occur in the driver. They are
//! reported via the [alert system](crate::alert), as most operations via the
//! driver happen asynchronously.

use std::fmt;
use std::result::Result as StdResult;

pub use rs_cnc::error::Error as CelestiaClientError;
pub use tokio::{io::Error as IoError, sync::mpsc::error::SendError, task::JoinError};

/// A special result type for astria-conductor
pub type Result<T, E = Error> = StdResult<T, E>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The channel on which some component in engine was listening or sending died.
    Channel(String),

    /// An error with the Celestia client
    CelestiaClient(String),

    /// Holds global IO related errors.
    Io(IoError),
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::CelestiaClient(e) => write!(fmt, "celestia client error: {}", e),
            Error::Channel(e) => write!(fmt, "channel error: {}", e),
            Error::Io(e) => e.fmt(fmt),
        }
    }
}

// NOTE - you must implement the From<T> trait for Error for all the different
//   types of errors we expect to see in the application.

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Self::Io(e)
    }
}

impl<T> From<SendError<T>> for Error {
    fn from(e: SendError<T>) -> Self {
        Self::Channel(e.to_string())
    }
}

impl From<JoinError> for Error {
    fn from(e: JoinError) -> Self {
        Self::Channel(e.to_string())
    }
}

impl From<CelestiaClientError> for Error {
    fn from(e: CelestiaClientError) -> Self {
        Self::CelestiaClient(e.to_string())
    }
}
