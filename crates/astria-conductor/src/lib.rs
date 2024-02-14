//! # Astria Conductor
//! The Astria conductor connects the shared sequencer layer and the execution layer.
//! When a block is received from the sequencer layer, the conductor pushes it to the execution
//! layer. There are two ways for a block to be received:
//! - pushed from the shared-sequencer
//! - via the data availability layer
//! In the first case, the block is pushed to the execution layer, executed, and added to the
//! blockchain. It's marked as a soft commitment; the block is not regarded as finalized on the
//! execution layer until it's received from the data availability layer. In the second case, the
//! execution layer is notified to mark the block as finalized.
pub(crate) mod block_cache;
mod build_info;
pub(crate) mod celestia;
pub(crate) mod client_provider;
pub mod conductor;
pub mod config;
pub(crate) mod executor;
pub(crate) mod sequencer;
pub(crate) mod utils;

use std::fmt::Write;

pub use build_info::BUILD_INFO;
pub use conductor::Conductor;
pub use config::Config;

/// Installs an eyre error handler to print display-formatted errors.
///
/// # Errors
/// Returns an error if the error handler could not be installed.
/// See [`eyre::set_hook`] for more information.
pub fn install_error_handler() -> Result<(), eyre::InstallError> {
    eyre::set_hook(Box::new(|_| Box::new(ErrorHandler)))?;
    Ok(())
}

struct ErrorHandler;

impl eyre::EyreHandler for ErrorHandler {
    fn debug(
        &self,
        mut error: &(dyn std::error::Error + 'static),
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        f.write_char('{')?;
        f.write_fmt(format_args!("\"0\": \"{error}\""))?;
        let mut level: u32 = 1;
        while let Some(source) = error.source() {
            f.write_fmt(format_args!(", \"{level}\": \"{source}\""))?;
            level += 1;
            error = source;
        }
        f.write_char('}')?;
        Ok(())
    }

    fn display(
        &self,
        mut error: &(dyn std::error::Error + 'static),
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        f.write_char('{')?;
        f.write_fmt(format_args!("\"0\": \"{error}\""))?;
        let mut level: u32 = 1;
        while let Some(source) = error.source() {
            f.write_fmt(format_args!(", \"{level}\": \"{source}\""))?;
            level += 1;
            error = source;
        }
        f.write_char('}')?;
        Ok(())
    }
}
