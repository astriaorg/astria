#![doc = include_str!("../README.md")]

use std::{
    error::Error,
    fmt::Write as _,
};

#[cfg(feature = "anyhow")]
pub use anyhow;
#[cfg(feature = "anyhow")]
pub use anyhow_conversion::{
    anyhow_to_eyre,
    eyre_to_anyhow,
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
        level = level.saturating_add(1);
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

#[cfg(feature = "anyhow")]
mod anyhow_conversion {
    pub fn anyhow_to_eyre(anyhow_error: anyhow::Error) -> eyre::Report {
        let boxed: Box<dyn std::error::Error + Send + Sync> = anyhow_error.into();
        eyre::eyre!(boxed)
    }

    #[must_use]
    pub fn eyre_to_anyhow(eyre_error: eyre::Report) -> anyhow::Error {
        let boxed: Box<dyn std::error::Error + Send + Sync> = eyre_error.into();
        anyhow::anyhow!(boxed)
    }

    #[cfg(test)]
    mod test {
        #[test]
        fn anyhow_to_eyre_preserves_source_chain() {
            let mut errs = ["foo", "bar", "baz", "qux"];
            let anyhow_error = anyhow::anyhow!(errs[0]).context(errs[1]).context(errs[2]);
            let eyre_from_anyhow = super::anyhow_to_eyre(anyhow_error).wrap_err(errs[3]);

            errs.reverse();
            for (i, err) in eyre_from_anyhow.chain().enumerate() {
                assert_eq!(errs[i], &err.to_string());
            }
        }

        #[test]
        fn eyre_to_anyhow_preserves_source_chain() {
            let mut errs = ["foo", "bar", "baz", "qux"];
            let eyre_error = eyre::eyre!(errs[0]).wrap_err(errs[1]).wrap_err(errs[2]);
            let anyhow_from_eyre = super::eyre_to_anyhow(eyre_error).context(errs[3]);

            errs.reverse();
            for (i, err) in anyhow_from_eyre.chain().enumerate() {
                assert_eq!(errs[i], &err.to_string());
            }
        }
    }
}
