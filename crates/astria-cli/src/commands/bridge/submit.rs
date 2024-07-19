use std::path::{
    Path,
    PathBuf,
};

use astria_core::{
    crypto::SigningKey,
    protocol::transaction::v1alpha1::{
        Action,
        TransactionParams,
        UnsignedTransaction,
    },
};
use astria_sequencer_client::{
    tendermint_rpc::endpoint,
    HttpClient,
    SequencerClientExt as _,
};
use clap::Args;
use color_eyre::eyre::{
    self,
    ensure,
    WrapErr as _,
};
use tracing::{
    error,
    info,
    instrument,
    warn,
};

use crate::utils::read_signing_key;

#[derive(Args, Debug)]
pub(crate) struct WithdrawalEvents {
    #[arg(long, short)]
    input: PathBuf,
    #[arg(long)]
    signing_key: PathBuf,
    #[arg(long, default_value = "astria")]
    sequencer_address_prefix: String,
    #[arg(long)]
    sequencer_chain_id: String,
    #[arg(long)]
    sequencer_url: String,
}

impl WithdrawalEvents {
    pub(crate) async fn run(self) -> eyre::Result<()> {
        let signing_key = read_signing_key(&self.signing_key).wrap_err_with(|| {
            format!(
                "failed reading signing key from file: {}",
                self.signing_key.display()
            )
        })?;

        let actions_by_rollup_number = read_actions(&self.input).wrap_err_with(|| {
            format!("failed reading actions from file: {}", self.input.display())
        })?;

        let sequencer_client = HttpClient::new(&*self.sequencer_url)
            .wrap_err("failed constructing http sequencer client")?;

        for (rollup_height, actions) in actions_by_rollup_number.into_inner() {
            if actions.is_empty() {
                warn!(
                    rollup_height,
                    "entry for rollup height exists, but actions were empty; skipping"
                );
                continue;
            }
            match submit_transaction(
                sequencer_client.clone(),
                &self.sequencer_chain_id,
                &self.sequencer_address_prefix,
                &signing_key,
                actions,
            )
            .await
            .wrap_err_with(|| {
                format!("submitting withdrawal actions for rollup height `{rollup_height}` failed")
            }) {
                Err(e) => {
                    error!(
                        rollup_height,
                        "failed submitting actions; bailing and not submitting the rest"
                    );
                    return Err(e);
                }
                Ok(response) => info!(
                    sequencer_height = %response.height,
                    rollup_height,
                    "actions derived from rollup succesfully submitted to sequencer"
                ),
            }
        }
        Ok(())
    }
}

fn read_actions<P: AsRef<Path>>(path: P) -> eyre::Result<super::collect::ActionsByRollupHeight> {
    let s = std::fs::read_to_string(path).wrap_err("failed buffering file contents as string")?;
    serde_json::from_str(&s)
        .wrap_err("failed deserializing file contents height-to-sequencer-actions serde object")
}

#[instrument(skip_all, fields(actions = actions.len()), err)]
async fn submit_transaction(
    client: HttpClient,
    chain_id: &str,
    prefix: &str,
    signing_key: &SigningKey,
    actions: Vec<Action>,
) -> eyre::Result<endpoint::broadcast::tx_commit::Response> {
    let from_address =
        crate::utils::make_address(prefix, &signing_key.verification_key().address_bytes())
            .wrap_err("failed constructing a valid from address from the provided prefix")?;

    let nonce_res = client
        .get_latest_nonce(from_address)
        .await
        .wrap_err("failed to get nonce")?;

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(nonce_res.nonce)
            .chain_id(chain_id)
            .build(),
        actions,
    }
    .into_signed(signing_key);
    let res = client
        .submit_transaction_commit(tx)
        .await
        .wrap_err("failed to submit transaction")?;
    ensure!(
        res.check_tx.code.is_ok(),
        "failed to check tx: {}",
        res.check_tx.log
    );
    ensure!(
        res.tx_result.code.is_ok(),
        "failed to execute tx: {}",
        res.tx_result.log
    );
    Ok(res)
}
