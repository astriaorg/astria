use astria_sequencer::{
    accounts::types::Nonce,
    sequence::Action as SequenceAction,
    transaction::{
        action::Action as SequencerAction,
        Signed as SignedSequencerTx,
        Unsigned as UnsignedSequencerTx,
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
        JsonRpcClient,
        Middleware,
        Provider,
        PubsubClient,
        StreamExt,
        Ws,
    },
    types::Transaction as RollupTx,
};
use tokio::{
    select,
    task::JoinSet,
};
use tracing::warn;

use crate::Config;

mod bundler;
mod collector;
mod executor;
mod searcher;

#[cfg(test)]
mod tests;

impl<T> Searcher<T>
where
    T: PubsubClient + JsonRpcClient + Clone,
{
    pub fn build(
        eth_client: Provider<T>,
        sequencer_client: astria_sequencer_client::Client,
        rollup_chain_id: String,
    ) -> Searcher<T> {
        Searcher {
            eth_client,
            sequencer_client,
            rollup_chain_id,
            conversion_tasks: JoinSet::new(),
            submission_tasks: JoinSet::new(),
        }
    }

    /// Constructs a new Searcher service from config.
    pub async fn new_ws(cfg: &Config) -> eyre::Result<Searcher<Ws>> {
        // connect to eth node
        let eth_client = Provider::connect(&cfg.execution_ws_url)
            .await
            .wrap_err("failed connecting to ethereum json rpc server")?;

        // connect to sequencer node
        let sequencer_client = SequencerClient::new(&cfg.sequencer_url)
            .wrap_err("failed constructing sequencer client")?;

        let rollup_chain_id = cfg.chain_id.clone();

        Ok(Searcher {
            eth_client,
            sequencer_client,
            rollup_chain_id,
            conversion_tasks: JoinSet::new(),
            submission_tasks: JoinSet::new(),
        })
    }

    /// Serializes and signs a sequencer tx from a rollup tx.
    async fn handle_pending_tx(&mut self, rollup_tx: Transaction) -> eyre::Result<()> {
        let chain_id = self.rollup_chain_id.clone();

        self.conversion_tasks.spawn_blocking(move || {
            // FIXME: Needs to be altered when nonces are implemented in the sequencer
            // For now, each transaction is transmitted from a new account with nonce 0
            let sequencer_key = SigningKey::new(rand::thread_rng());
            let nonce = Nonce::from(0);

            // Pack into sequencer tx
            let data = rollup_tx.rlp().to_vec();
            let chain_id = chain_id.into_bytes();
            let seq_action = SequencerAction::SequenceAction(SequenceAction::new(chain_id, data));
            let unsigned_tx = UnsignedSequencerTx::new_with_actions(nonce, vec![seq_action]);

            // Sign transaction
            Ok(unsigned_tx.into_signed(&sequencer_key))
        });

        Ok(())
    }

    fn handle_signed_tx(&mut self, tx: SignedSequencerTx) {
        let client = self.sequencer_client.clone();
        self.submission_tasks.spawn(async move {
            let rsp = client
                .submit_transaction_sync(tx.clone())
                .await
                .wrap_err("failed to submit transaction to sequencer")?;
            if !rsp.code.is_ok() {
                Err(eyre!("transaction submission response error: {:?}", rsp))
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
                                Err(e) => {
                                    warn!(error.message = %e, error.cause_chain = ?e, "failed to submit signed sequencer transaction to sequencer");
                                    todo!("handle sequencer failed CheckTx")
                                },
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

#[derive(Debug, Clone)]
pub(crate) enum Event {
    NewRollupTx(RollupTx),
}

#[derive(Debug, Clone)]
pub(crate) enum Action {
    SendSequencerTx(SignedSequencerTx),
}
