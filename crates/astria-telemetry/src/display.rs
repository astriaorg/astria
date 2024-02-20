//! Utilities to emit fields using their [`std::fmt::Display`] implementation.
use base64::{
    display::Base64Display,
    engine::general_purpose::GeneralPurpose,
};

/// Format `bytes` using standard base64 formatting.
///
/// See the [`base64::engine::general_purpose::STANDARD`] for the formatting definition.
pub fn base64<T: AsRef<[u8]>>(bytes: &T) -> Base64Display<'_, 'static, GeneralPurpose> {
    Base64Display::new(bytes.as_ref(), &base64::engine::general_purpose::STANDARD)
}

/// Format `bytes` as lower-cased hex.
///
/// # Example
/// ```
/// use astria_telemetry::display;
/// let signature = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
/// tracing::info!(signature = %display::hex(&signature), "received signature");
/// ```
pub fn hex<T: AsRef<[u8]>>(bytes: &T) -> Hex<'_> {
    Hex(bytes.as_ref())
}

/// A newtype wrapper of a byte slice that implements [`std::fmt::Display`].
///
/// To be used in tracing contexts. See the [`self::hex`] utility.
pub struct Hex<'a>(pub &'a [u8]);

impl<'a> std::fmt::Display for Hex<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            f.write_fmt(format_args!("{byte:02x}"))?;
        }
        Ok(())
    }
}
