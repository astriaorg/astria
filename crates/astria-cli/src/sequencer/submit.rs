// use prost::message::Message;
// use astria_core::protocol::transaction::v1::TransactionBody;
use astria_core::{
    self,
    generated::protocol::transaction::v1::TransactionBody as TransactionBodyProto,
    protocol::transaction::v1::TransactionBody,
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
    /// The private key of account being sent from
    #[arg(long, env = "SEQUENCER_PRIVATE_KEY")]
    // TODO: https://github.com/astriaorg/astria/issues/594
    // Don't use a plain text private, prefer wrapper like from
    // the secrecy crate with specialized `Debug` and `Drop` implementations
    // that overwrite the key on drop and don't reveal it when printing.
    private_key: String,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        let sequencer_client = HttpClient::new(self.sequencer_url.as_str())
            .wrap_err("failed constructing http sequencer client")?;

        let sequencer_key = signing_key_from_private_key(self.private_key.as_str())?;

        let tx_body: TransactionBodyProto = serde_json::from_str(self.pbjson.as_str())
            .wrap_err("failed to parse pbjson into TransactionBody")?;

        let tx = TransactionBody::try_from_raw(tx_body.clone())
            .wrap_err("failed to convert to TransactionBody from raw")?
            .sign(&sequencer_key);

        // println!(
        //     "{}",
        //     serde_json::to_string_pretty(&tx_body)
        //         .wrap_err("failed to serialize TransactionBody")?
        // );

        let res = sequencer_client
            .submit_transaction_sync(tx)
            .await
            .wrap_err("failed to submit transaction")?;

        ensure!(res.code.is_ok(), "failed to check tx: {}", res.log);

        let tx_response = sequencer_client.wait_for_tx_inclusion(res.hash).await;

        ensure!(
            tx_response.tx_result.code.is_ok(),
            "failed to execute tx: {}",
            tx_response.tx_result.log
        );

        println!("Submission completed!");
        println!("Included in block: {}", tx_response.height);
        Ok(())
    }
}
