//! Utilities to emit fields using their [`std::fmt::Display`] implementation.
use std::{
    fmt::{
        self,
        Display,
        Formatter,
        Result,
    },
    io,
    str,
    time::Duration,
};

use base64_serde::base64_serde_type;
use serde_with::SerializeDisplay;

/// Format `bytes` using standard base64 formatting.
///
/// See the [`base64::engine::general_purpose::STANDARD`] for the formatting definition.
pub fn base64<T: AsRef<[u8]>>(bytes: T) -> Base64<T> {
    Base64(bytes)
}

pub struct Base64<T>(T);

impl<T> Display for Base64<T>
where
    T: AsRef<[u8]>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        use base64::{
            display::Base64Display,
            engine::general_purpose::STANDARD,
        };
        Base64Display::new(self.0.as_ref(), &STANDARD).fmt(f)
    }
}

impl<T> serde::Serialize for Base64<T>
where
    T: AsRef<[u8]>,
{
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        base64_serde_type!(Base64Standard, base64::engine::general_purpose::STANDARD);
        Base64Standard::serialize(self.0.as_ref(), serializer)
    }
}

/// Format `bytes` as lower-cased hex.
///
/// # Example
/// ```
/// use astria_telemetry::display;
/// let signature = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
/// tracing::info!(signature = %display::hex(&signature), "received signature");
/// ```
pub fn hex<T: AsRef<[u8]>>(bytes: T) -> Hex<T> {
    Hex(bytes)
}

/// A newtype wrapper of a byte slice that implements [`std::fmt::Display`].
///
/// To be used in tracing contexts. See the [`self::hex`] utility.
#[derive(SerializeDisplay)]
pub struct Hex<T>(T);

impl<T> Display for Hex<T>
where
    T: AsRef<[u8]>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for byte in self.0.as_ref() {
            f.write_fmt(format_args!("{byte:02x}"))?;
        }
        Ok(())
    }
}

pub fn json<T>(serializable: &T) -> Json<'_, T>
where
    T: serde::Serialize,
{
    Json(serializable)
}

/// A newtype wrapper of a serializable type that implements [`std::fmt::Display`].
///
/// To be used in tracing contexts. See the [`self::json`] utility.
///
/// # Panics
/// The type must not contain non-utf8 fields, nor can any of the type's fields or variants
/// have [`Serialize`] implementations that are fallible. The [`Display`] implementation will
/// panic otherwise.
pub struct Json<'a, T>(&'a T);

// NOTE: This implementation is lifted straight from serde_json:
// https://docs.rs/serde_json/1.0.114/src/serde_json/value/mod.rs.html#197
impl<T> Display for Json<'_, T>
where
    T: serde::Serialize,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        struct WriterFormatter<'a, 'b: 'a> {
            inner: &'a mut Formatter<'b>,
        }

        impl io::Write for WriterFormatter<'_, '_> {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                // NOTE: Same argument for safety as in
                // https://docs.rs/serde_json/1.0.114/src/serde_json/value/mod.rs.html#229
                // Safety: the serializer below only emits valid utf8 when using
                // the default formatter.
                let s = unsafe { str::from_utf8_unchecked(buf) };
                self.inner.write_str(s).map_err(io_error)?;
                Ok(buf.len())
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        fn io_error(_: fmt::Error) -> io::Error {
            // Error value does not matter because Display impl just maps it
            // back to fmt::Error.
            io::Error::new(io::ErrorKind::Other, "fmt error")
        }

        let mut wr = WriterFormatter {
            inner: f,
        };
        serde_json::to_writer(&mut wr, self.0).map_err(|_| fmt::Error)
    }
}

/// Format `duration` using human-readable format.
///
/// See the [`base64::engine::general_purpose::STANDARD`] for the formatting definition.
#[must_use]
pub fn format_duration(duration: Duration) -> FormattedDuration {
    FormattedDuration(duration)
}

pub struct FormattedDuration(Duration);

impl Display for FormattedDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        use jiff::{
            fmt::{
                friendly::SpanPrinter,
                StdFmtWrite,
            },
            SignedDuration,
        };

        static PRINTER: SpanPrinter = SpanPrinter::new();
        match SignedDuration::try_from(self.0) {
            Ok(signed_duration) => PRINTER
                .print_duration(&signed_duration, StdFmtWrite(f))
                .map_err(|_| fmt::Error),
            Err(_) => {
                write!(f, "<duration greater than {:#}>", SignedDuration::MAX)
            }
        }
    }
}
