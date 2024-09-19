use std::fmt::{
    Display,
    Formatter,
    Result,
};

/// Format `bytes` using standard base64 formatting.
///
/// See the [`base64::engine::general_purpose::STANDARD`] for the formatting definition.
///
/// # Example
/// ```
/// use astria_core::display;
/// let signature = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
/// println!("received signature: {}", display::base64(&signature));
/// ```
pub fn base64<T: AsRef<[u8]> + ?Sized>(bytes: &T) -> Base64<'_> {
    Base64(bytes.as_ref())
}

pub struct Base64<'a>(&'a [u8]);

impl<'a> Display for Base64<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        use base64::{
            display::Base64Display,
            engine::general_purpose::STANDARD,
        };
        Base64Display::new(self.0, &STANDARD).fmt(f)
    }
}

/// A newtype wrapper of a byte slice that implements [`std::fmt::Display`].
///
/// To be used in tracing contexts. See the [`self::hex`] utility.
pub struct Hex<'a>(&'a [u8]);

/// Format `bytes` as lower-cased hex.
///
/// # Example
/// ```
/// use astria_core::display;
/// let signature = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
/// println!("received signature: {}", display::hex(&signature));
/// ```
pub fn hex<T: AsRef<[u8]> + ?Sized>(bytes: &T) -> Hex<'_> {
    Hex(bytes.as_ref())
}

impl<'a> Display for Hex<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for byte in self.0 {
            f.write_fmt(format_args!("{byte:02x}"))?;
        }
        Ok(())
    }
}
