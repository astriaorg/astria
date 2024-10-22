use std::{
    io::Write,
    path::{
        Path,
        PathBuf,
    },
};

use astria_core::{
    protocol::transaction::v1::TransactionBody,
    Protobuf,
};
use clap_stdin::FileOrStdin;
use color_eyre::eyre::{
    self,
    WrapErr as _,
};

use crate::utils::signing_key_from_private_key;

#[derive(clap::Args, Debug)]
pub(super) struct Command {
    /// The private key of account being sent from
    #[arg(long, env = "SEQUENCER_PRIVATE_KEY")]
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    private_key: String,
    /// Target to write the signed transaction in pbjson format (omit to write to STDOUT).
    #[arg(long, short)]
    output: Option<PathBuf>,
    /// Forces an overwrite of `--output` if a file at that location exists.
    #[arg(long, short)]
    force: bool,
    /// The source to read the pbjson formatted astra.protocol.transaction.v1.Transaction (use `-`
    /// to pass via STDIN).
    input: FileOrStdin,
}

// The goal of the `sign` CLI command is to take in a `TransactionBody` and to sign with a private
// key to create a `Transaction`. This signed `Transaction` should be printed to the console in
// pbjson format.
impl Command {
    pub(super) fn run(self) -> eyre::Result<()> {
        let key = signing_key_from_private_key(self.private_key.as_str())?;

        let filename = self.input.filename().to_string();
        let transaction_body = read_transaction_body(self.input)
            .wrap_err_with(|| format!("failed to read transaction body from `{filename}`"))?;
        let transaction = transaction_body.sign(&key);

        serde_json::to_writer(
            stdout_or_file(self.output.as_ref(), self.force)
                .wrap_err("failed to determine output target")?,
            &transaction.to_raw(),
        )
        .wrap_err("failed to write signed transaction")?;
        Ok(())
    }
}

fn read_transaction_body(input: FileOrStdin) -> eyre::Result<TransactionBody> {
    let wire_body: <TransactionBody as Protobuf>::Raw = serde_json::from_reader(
        std::io::BufReader::new(input.into_reader()?),
    )
    .wrap_err_with(|| {
        format!(
            "failed to parse input as json `{}`",
            TransactionBody::full_name()
        )
    })?;
    TransactionBody::try_from_raw(wire_body).wrap_err("failed to validate transaction body")
}

fn stdout_or_file<P: AsRef<Path>>(
    output: Option<P>,
    force_overwrite: bool,
) -> eyre::Result<Box<dyn Write>> {
    let writer = match output {
        Some(path) => {
            let file = if force_overwrite {
                std::fs::File::options()
                    .write(true)
                    .truncate(true)
                    .open(path)
            } else {
                std::fs::File::options()
                    .create_new(true)
                    .write(true)
                    .open(path)
            }
            .wrap_err("failed to open file for writing")?;
            Box::new(file) as Box<dyn Write>
        }
        None => Box::new(std::io::stdout()),
    };
    Ok(writer)
}
