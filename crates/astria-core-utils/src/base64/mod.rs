//! Base64 encoding and decoding using a URL-safe alphabet.

mod error;
#[cfg(feature = "serde")]
pub mod serde;

use std::fmt::{
    self,
    Debug,
    Display,
    Formatter,
};

use base64::{
    engine::general_purpose::URL_SAFE,
    Engine,
};
pub use error::Error;

/// A helper struct for base64-encoding bytes into a format string without heap allocation.
///
/// A new instance can be constructed via [`display`].
#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize),
    serde(bound(serialize = "T: AsRef<[u8]>"))
)]
pub struct DisplayFmt<T>(
    #[cfg_attr(feature = "serde", serde(serialize_with = "serde::serialize"))] T,
);

/// Returns a new `DisplayFmt`.
#[must_use]
pub fn display<T: AsRef<[u8]>>(bytes: T) -> DisplayFmt<T> {
    DisplayFmt(bytes)
}

impl<T: AsRef<[u8]>> DisplayFmt<T> {
    fn format(&self, formatter: &mut Formatter) -> Result<(), fmt::Error> {
        base64::display::Base64Display::new(self.0.as_ref(), &URL_SAFE).fmt(formatter)
    }
}

impl<T: AsRef<[u8]>> Display for DisplayFmt<T> {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), fmt::Error> {
        self.format(formatter)
    }
}

impl<T: AsRef<[u8]>> Debug for DisplayFmt<T> {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), fmt::Error> {
        self.format(formatter)
    }
}

/// Encodes `input` to base64.
pub fn encode<T: AsRef<[u8]>>(input: T) -> String {
    URL_SAFE.encode(input)
}

/// Decodes `input` from base64.
///
/// # Errors
///
/// Returns an error if decoding fails.
pub fn decode<T: AsRef<[u8]>>(input: T) -> Result<Vec<u8>, Error> {
    URL_SAFE.decode(input).map_err(Error::from)
}

#[cfg(test)]
mod tests {
    #[test]
    fn encoding_round_trip() {
        let input = vec![9, 8, 7, 6, 5, 4, 3, 2, 1, 0];
        let encoded = super::encode(&input);
        let decoded = super::decode(&encoded).unwrap();
        assert_eq!(input, decoded);
    }
}
