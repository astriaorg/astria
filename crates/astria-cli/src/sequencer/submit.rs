use astria_core::{
    self,
    protocol::transaction::v1::Transaction,
    Protobuf,
};
use astria_sequencer_client::{
    HttpClient,
    SequencerClientExt as _,
};
use clap_stdin::FileOrStdin;
use color_eyre::eyre::{
    self,
    ensure,
    WrapErr as _,
};

#[derive(clap::Args, Debug)]
pub(super) struct Command {
    /// The URL at which the Sequencer node is listening for ABCI commands.
    #[arg(long, env = "SEQUENCER_URL")]
    sequencer_url: String,
    /// The source to read the pbjson formatted astra.protocol.transaction.v1.Transaction (use `-`
    /// to pass via STDIN).
    input: FileOrStdin,
}

// The 'submit' command takes a 'Transaction' in pbjson form and submits it to the sequencer
impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        let sequencer_client = HttpClient::new(self.sequencer_url.as_str())
            .wrap_err("failed constructing http sequencer client")?;

        let filename = self.input.filename().to_string();
        let transaction = read_transaction(self.input)
            .wrap_err_with(|| format!("to signed transaction from `{filename}`"))?;

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

fn read_transaction(input: FileOrStdin) -> eyre::Result<Transaction> {
    let wire_body: <Transaction as Protobuf>::Raw = serde_json::from_reader(
        std::io::BufReader::new(input.into_reader()?),
    )
    .wrap_err_with(|| {
        format!(
            "failed to parse input as json `{}`",
            Transaction::full_name()
        )
    })?;
    Transaction::try_from_raw(wire_body).wrap_err("failed to validate transaction body")
}
