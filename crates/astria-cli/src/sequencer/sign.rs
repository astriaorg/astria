// use prost::message::Message;
// use astria_core::protocol::transaction::v1::TransactionBody;
use astria_core::{
    self,
    generated::protocol::transaction::v1::{
        Transaction as TransactionProto,
        TransactionBody as TransactionBodyProto,
    },
    protocol::transaction::v1::{
        Transaction,
        TransactionBody,
    },
};
use astria_sequencer_client::{
    HttpClient,
    SequencerClientExt as _,
};
use color_eyre::eyre::{
    self,
    ensure,
    WrapErr as _,
};

use crate::utils::signing_key_from_private_key;

#[derive(clap::Args, Debug)]
pub(super) struct Command {
    /// The pbjson for submission
    #[arg(long)]
    pbjson: String,
    /// The url of the Sequencer node
    #[arg(
        long,
        env = "SEQUENCER_URL",
        default_value = crate::DEFAULT_SEQUENCER_RPC
    )]
    sequencer_url: String,
    // /// The private key of account being sent from
    #[arg(long, env = "SEQUENCER_PRIVATE_KEY")]
    // // TODO: https://github.com/astriaorg/astria/issues/594
    // // Don't use a plain text private, prefer wrapper like from
    // // the secrecy crate with specialized `Debug` and `Drop` implementations
    // // that overwrite the key on drop and don't reveal it when printing.
    private_key: String,
}

// The goal of the `sign` CLI command is to take in a `TransactionBody` and to sign with a private key to create a `Transaction`. 
// This signed `Transaction` should be printed to the console in pbjson format.
impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {

        let sequencer_key = signing_key_from_private_key(self.private_key.as_str())?;

        let tx_body: TransactionBodyProto = serde_json::from_str(self.pbjson.as_str())
            .wrap_err("failed to parse pbjson into TransactionBody")?;

        let tx = TransactionBody::try_from_raw(tx_body.clone())
            .wrap_err("failed to convert to TransactionBody from raw")?
            .sign(&sequencer_key);

        // Copied code from Jordan's PR to print stuff in JSON
        println!("Transaction:");
        println!(
            "{}",
            serde_json::to_string_pretty(&tx.to_raw()).wrap_err("failed to json-encode")?
        );
        println!();
        println!("Transaction Body:");
        println!("{}", serde_json::to_string_pretty(&tx.unsigned_transaction().to_raw()).wrap_err("failed to json-encode")?);

        Ok(())
    }
}
