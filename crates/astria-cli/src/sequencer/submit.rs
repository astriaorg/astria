use astria_core::{
    self,
    generated::protocol::transaction::v1::Transaction as TransactionProto,
    protocol::transaction::v1::Transaction,
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

#[derive(clap::Args, Debug)]
pub(super) struct Command {
    /// The pbjson for submission
    pbjson: String,
    /// The url of the Sequencer node
    #[arg(
        long,
        env = "SEQUENCER_URL",
        default_value = crate::DEFAULT_SEQUENCER_RPC
    )]
    sequencer_url: String,
}

// The 'submit' command takes a 'Transaction' in pbjson form and submits it to the sequencer
impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        let sequencer_client = HttpClient::new(self.sequencer_url.as_str())
            .wrap_err("failed constructing http sequencer client")?;

        let tx_raw: TransactionProto = serde_json::from_str(self.pbjson.as_str())
            .wrap_err("failed to parse pbjson into raw Transaction")?;

        let transaction = Transaction::try_from_raw(tx_raw.clone())
            .wrap_err("failed to convert to transaction")?;

        println!(
            "{}",
            serde_json::to_string_pretty(&tx_raw)
                .wrap_err("failed to serialize TransactionBody")?
        );

        let res = sequencer_client
            .submit_transaction_sync(transaction)
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
