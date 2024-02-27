#![doc = include_str!("../README.md")]

use std::{
    error::Error,
    fmt::Write as _,
};

pub use eyre;
#[doc(hidden)]
pub use eyre::Result;

/// Installs the `astria-eyre` hook as the global error report hook.
///
/// # Details
///
/// This function must be called to enable the customization of `eyre::Report`
/// provided by `astria-eyre`.
///
/// **NOTE**: It must be called before any `eyre::Report`s are constructed
/// to prevent the default handler from being installed.
///
/// # Errors
///
/// Calling this function after another handler has been installed will cause
/// an error.
pub fn install() -> Result<()> {
    eyre::set_hook(Box::new(|_| Box::new(ErrorHandler)))?;
    Ok(())
}

struct ErrorHandler;

impl eyre::EyreHandler for ErrorHandler {
    fn debug(
        &self,
        error: &(dyn Error + 'static),
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        display(error, f)
    }

    fn display(
        &self,
        error: &(dyn Error + 'static),
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        display(error, f)
    }
}

fn display(
    mut error: &(dyn Error + 'static),
    f: &mut core::fmt::Formatter<'_>,
) -> core::fmt::Result {
    f.write_char('{')?;
    let mut level = 0;
    write_layer(level, error, f)?;
    while let Some(cause) = error.source() {
        level += 1;
        f.write_str(", ")?;
        write_layer(level, cause, f)?;
        error = cause;
    }
    f.write_char('}')?;
    Ok(())
}

fn write_layer(key: u32, err: &dyn Error, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write_key(key, f)?;
    write_value(err, f)?;
    Ok(())
}

fn write_key(key: u32, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.write_char('"')?;
    let mut buf = itoa::Buffer::new();
    f.write_str(buf.format(key))?;
    f.write_str("\": ")?;
    Ok(())
}

fn write_value(err: &dyn Error, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.write_fmt(format_args!("\"{err}\""))?;
    Ok(())
}
