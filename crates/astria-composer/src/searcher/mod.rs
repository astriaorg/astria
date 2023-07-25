use std::sync::Arc;

use astria_sequencer::{
    accounts::types::{
        Address as SequencerAddress,
        Nonce,
    },
    sequence::Action as SequenceAction,
    transaction::{
        action::Action as SequencerAction,
        Signed,
        Unsigned,
    },
};
use astria_sequencer_client::Client as SequencerClient;
use color_eyre::eyre::{
    self,
    eyre,
    WrapErr as _,
};
use ed25519_consensus::SigningKey;
use ethers::{
    providers::{
        self,
        Middleware as _,
        Provider,
        StreamExt as _,
    },
    types::Transaction,
};
use tokio::{
    select,
    task::JoinSet,
};
use tracing::{
    error,
    warn,
};

use crate::Config;

pub struct Searcher {
    // The client for getting new pending transactions from the ethereum JSON RPC.
    eth_client: Provider<providers::Ws>,
    // The client for submitting swrapped pending eth transactions to the astria sequencer.
    sequencer_client: astria_sequencer_client::Client,
    nonce: Option<Nonce>,
    rollup_chain_id: String,
    sequencer_address: SequencerAddress,
    sequencer_key: Arc<SigningKey>,
    // Set of currently running jobs converting pending eth transactions to signed sequencer
    // transactions.
    conversion_tasks: JoinSet<eyre::Result<Signed>>,
    // Set of in-flight RPCs submitting signed transactions to the sequencer.
    // submission_tasks: JoinSet<eyre::Result<tx_sync::Response>>,
    submission_tasks: JoinSet<Result<(), SubmissionError>>,
}

impl Searcher {
    /// Constructs a new Searcher service from config.
    pub async fn new(cfg: &Config) -> eyre::Result<Self> {
        // connect to eth node
        let eth_client = Provider::connect(&cfg.execution_ws_url)
            .await
            .wrap_err("failed connecting to ethereum json rpc server")?;

        // connect to sequencer node
        let sequencer_client = SequencerClient::new(&cfg.sequencer_url)
            .wrap_err("failed constructing sequencer client")?;

        // construct sequencer address and private key
        let sequencer_address = SequencerAddress::try_from_str(&cfg.sequencer_address)
            .map_err(|e| eyre!(Box::new(e)))
            .wrap_err("failed constructing sequencer address from string")?;
        let secret_bytes =
            hex::decode(&cfg.sequencer_secret).wrap_err("failed decoding string as hexadecimal")?;
        let sequencer_key = Arc::new(
            SigningKey::try_from(&*secret_bytes)
                .wrap_err("failed to construct signing key from decoded sequencer secret")?,
        );

        // get current nonce for sequencer address
        let nonce = Some(
            sequencer_client
                .get_nonce(&sequencer_address, None)
                .await
                .wrap_err("failed getting current nonce from sequencer")?
                .into(),
        );

        let rollup_chain_id = cfg.chain_id.clone();

        Ok(Self {
            eth_client,
            sequencer_client,
            nonce,
            rollup_chain_id,
            sequencer_address,
            sequencer_key,
            conversion_tasks: JoinSet::new(),
            submission_tasks: JoinSet::new(),
        })
    }

    /// Serializes and signs a sequencer tx from a rollup tx.
    async fn handle_pending_tx(&mut self, rollup_tx: Transaction) -> eyre::Result<()> {
        let chain_id = self.rollup_chain_id.clone();
        let sequencer_key = self.sequencer_key.clone();

        // get next nonce for sequencer address
        let nonce = self
            .get_next_nonce()
            .await
            .ok_or_else(|| eyre!("get nonce failed"))?;

        self.conversion_tasks.spawn_blocking(move || {
            // pack into sequencer tx and sign
            let data = rollup_tx.rlp().to_vec();
            let chain_id = chain_id.into_bytes();

            // pack into sequencer tx and sign
            let seq_action = SequencerAction::SequenceAction(SequenceAction::new(chain_id, data));
            Ok(Unsigned::new_with_actions(nonce, vec![seq_action]).into_signed(&sequencer_key))
        });

        Ok(())
    }

    /// Resubmits a failed sequencer tx with a new nonce.
    async fn resubmit_tx_with_new_nonce(&mut self, old_tx: Signed) -> eyre::Result<()> {
        // reset nonce and try again
        self.reset_nonce();
        let sequencer_key = self.sequencer_key.clone();

        // get next nonce for sequencer address
        let nonce = self
            .get_next_nonce()
            .await
            .ok_or_else(|| eyre!("get nonce failed"))?;

        self.conversion_tasks.spawn(async move {
            // grab actions from the old tx
            let actions = old_tx.transaction().actions().to_vec();

            Ok(Unsigned::new_with_actions(nonce, actions).into_signed(&sequencer_key))
        });

        Ok(())
    }

    /// Fetches the current next nonce from the sequencer node.
    async fn get_next_nonce(&mut self) -> Option<Nonce> {
        // get nonce from sequencer ndoe if None otherwise increment it
        if let Some(nonce) = self.nonce {
            self.nonce = Some(nonce + Nonce::from(1));
        } else {
            self.nonce = self
                .sequencer_client
                .get_nonce(&self.sequencer_address, None)
                .await
                .ok();
        }
        self.nonce
    }

    /// Resets the stored nonce to None.
    fn reset_nonce(&mut self) {
        warn!("resetting sequencer nonce, was {:?}", self.nonce);
        self.nonce = None;
    }

    fn handle_signed_tx(&mut self, tx: Signed) {
        let client = self.sequencer_client.clone();
        self.submission_tasks.spawn(async move {
            let rsp = client
                .submit_transaction_sync(tx.clone())
                .await
                .map_err(|_e| SubmissionError::CheckTxFailed(tx.clone()))?;
            if !rsp.code.is_ok() {
                error!("failed to submit transaction to sequencer: {:?}", rsp);
                // TODO: return error with the failed tx
                Err(SubmissionError::InvalidNonce(tx))
            } else {
                Ok(())
            }
        });
    }

    /// Runs the Searcher
    pub async fn run(mut self) -> eyre::Result<()> {
        // set up connection to eth node
        let eth_client = self.eth_client.clone();
        let mut tx_stream = eth_client
            .subscribe_full_pending_txs()
            .await
            .wrap_err("failed to subscriber eth client to full pending transactions")?;

        loop {
            select!(
                // serialize and sign sequencer tx for incoming pending rollup txs
                Some(rollup_tx) = tx_stream.next() => self.handle_pending_tx(rollup_tx).await?,
                // submit signed sequencer txs to sequencer
                Some(join_result) = self.conversion_tasks.join_next(), if !self.conversion_tasks.is_empty() => {
                    match join_result {
                        Ok(signing_result) => {
                            match signing_result {
                                Ok(signed_tx) => self.handle_signed_tx(signed_tx),
                                Err(e) => warn!(error.message = %e, error.cause_chain = ?e, "failed to sign sequencer transaction")
                            }
                        },
                        Err(e) => warn!(
                            error.message = %e,
                            error.cause_chain = ?e,
                            "conversion task failed while trying to convert pending eth transaction to signed sequencer transaction",
                        ),
                    }
                }
                // handle failed sequencer tx submissions
                Some(join_result) = self.submission_tasks.join_next(), if !self.submission_tasks.is_empty() => {
                    match join_result {
                        Ok(signing_result) => {
                            match signing_result {
                                Err(SubmissionError::InvalidNonce(failed_tx)) => {
                                    self.resubmit_tx_with_new_nonce(failed_tx).await?;
                                }
                                Err(SubmissionError::CheckTxFailed(_failed_tx)) => { todo!("handle sequencer failed CheckTx") },
                                _ => {}
                            }
                        },
                        Err(e) => warn!(
                            error.message = %e,
                            error.cause_chain = ?e,
                            "submission task failed while trying to submit signed sequencer transaction to sequencer",
                        ),
                }
            }
            )
        }

        // FIXME: ensure that we can get here
        #[allow(unreachable_code)]
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
enum SubmissionError {
    #[error("sequencer transaction failed CheckTx: {0:?}")]
    CheckTxFailed(Signed),
    #[error("sequencer transaction submission failed due to invalid nonce: {0:?}")]
    InvalidNonce(Signed),
}

#[cfg(test)]
mod tests {
    use crate::{
        config::Config,
        searcher::Searcher,
    };

    #[tokio::test]
    async fn new_from_valid_config() {
        let cfg = Config::default();
        let searcher = Searcher::new(&cfg).await;
        assert!(searcher.is_ok());
        // assert!(dbg!(searcher).is_ok());
    }
}
