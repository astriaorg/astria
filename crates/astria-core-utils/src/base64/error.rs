use std::{
    error,
    fmt::{
        self,
        Display,
        Formatter,
    },
};

/// Errors that can occur while decoding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// An invalid byte was found in the input.
    InvalidByte {
        /// The invalid byte.
        byte: u8,
        /// The index of the byte in the input data.
        index: usize,
    },
    /// The last non-padding input symbol's encoded 6 bits have nonzero bits that will be discarded.
    /// This is indicative of corrupted or truncated Base64.
    InvalidFinalByte {
        /// The invalid byte.
        byte: u8,
        /// The index of the byte in the input data.
        index: usize,
    },
    /// The length of the input, as measured in valid base64 symbols, is invalid.
    Length {
        /// The length in bytes.
        len: usize,
    },
    /// The padding was absent or incorrect.
    Padding,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match *self {
            Self::InvalidByte {
                byte,
                index,
            } => {
                write!(formatter, "invalid byte {byte} at index {index}")
            }
            Self::InvalidFinalByte {
                byte,
                index,
            } => {
                write!(formatter, "invalid final byte {byte} at index {index}")
            }
            Self::Length {
                len,
            } => write!(formatter, "invalid input length {len}"),
            Self::Padding => write!(formatter, "padding absent or invalid"),
        }
    }
}

impl error::Error for Error {}

impl From<base64::DecodeError> for Error {
    fn from(error: base64::DecodeError) -> Self {
        match error {
            base64::DecodeError::InvalidByte(index, byte) => Self::InvalidByte {
                byte,
                index,
            },
            base64::DecodeError::InvalidLastSymbol(index, byte) => Self::InvalidFinalByte {
                byte,
                index,
            },
            base64::DecodeError::InvalidLength(len) => Self::Length {
                len,
            },
            base64::DecodeError::InvalidPadding => Self::Padding,
        }
    }
}
