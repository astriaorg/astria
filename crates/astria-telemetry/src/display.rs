//! Utilities to emit fields using their [`std::fmt::Display`] implementation.

/// Format `bytes` as lower-cased hex.
///
/// # Example
/// ```
/// use astria_telemetry::display;
/// let signature = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
/// tracing::info!(signature = %display::hex(&signature), "received signature");
/// ```
pub fn hex<T: AsRef<[u8]>>(bytes: &T) -> DisplayHex<'_> {
    DisplayHex(bytes.as_ref())
}

/// A newtype wrapper of a byte slice that implements [`std::fmt::Display`].
///
/// To be used in tracing contexts. See the [`self::hex`] utility.
pub struct DisplayHex<'a>(pub &'a [u8]);

impl<'a> std::fmt::Display for DisplayHex<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            f.write_fmt(format_args!("{byte:02x}"))?;
        }
        Ok(())
    }
}
