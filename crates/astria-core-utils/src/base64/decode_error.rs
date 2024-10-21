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
pub struct DecodeError(base64::DecodeError);

impl Display for DecodeError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self.0 {
            base64::DecodeError::InvalidByte(index, byte) => {
                write!(formatter, "invalid byte {byte} at index {index}")
            }
            base64::DecodeError::InvalidLength(len) => {
                write!(formatter, "invalid input length {len}")
            }
            base64::DecodeError::InvalidLastSymbol(index, byte) => {
                write!(formatter, "invalid final byte {byte} at index {index}")
            }
            base64::DecodeError::InvalidPadding => {
                write!(formatter, "padding absent or invalid")
            }
        }
    }
}

impl error::Error for DecodeError {}

impl DecodeError {
    pub(super) fn new(source: base64::DecodeError) -> Self {
        Self(source)
    }
}
