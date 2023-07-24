use std::sync::{
    atomic::{
        AtomicU32,
        Ordering,
    },
    Arc,
};

use astria_sequencer::{
    accounts::types::Address as SequencerAddress,
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
    ensure,
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

// #[derive(Debug)]
pub struct Searcher {
    // The client for getting new pending transactions from the ethereum JSON RPC.
    eth_client: Provider<providers::Ws>,
    // The client for submitting swrapped pending eth transactions to the astria sequencer.
    sequencer_client: astria_sequencer_client::Client,
    nonce: Arc<AtomicU32>,
    rollup_chain_id: String,
    sequencer_key: Arc<SigningKey>,
    // Set of currently running jobs converting pending eth transactions to signed sequencer
    // transactions.
    conversion_tasks: JoinSet<Signed>,
    // Set of in-flight RPCs submitting signed transactions to the sequencer.
    // submission_tasks: JoinSet<eyre::Result<tx_sync::Response>>,
    submission_tasks: JoinSet<eyre::Result<()>>,
}

impl Searcher {
    /// Constructs a new Searcher service from config.
    ///
    /// # Errors
    ///
    /// - `Error::CollectorError` if there is an error initializing the tx collector.
    /// - `Error::BundlerError` if there is an error initializing the tx bundler.
    /// - `Error::SequencerClientInit` if there is an error initializing the sequencer client.
    pub async fn new(cfg: &Config) -> eyre::Result<Self> {
        let eth_client = Provider::connect(&cfg.execution_ws_url)
            .await
            .wrap_err("failed connecting to ethereum json rpc server")?;

        // configure rollup tx bundler
        let sequencer_client = SequencerClient::new(&cfg.sequencer_url)
            .wrap_err("failed constructing sequencer client")?;
        let sequencer_address = SequencerAddress::try_from_str(&cfg.sequencer_address)
            .map_err(|e| eyre!(Box::new(e)))
            .wrap_err("failed constructing sequencer address from string")?;
        let nonce = Arc::new(AtomicU32::new(
            sequencer_client
                .get_nonce(&sequencer_address, None)
                .await
                .wrap_err("failed getting current nonce from sequencer")?
                .into(),
        ));

        let rollup_chain_id = cfg.chain_id.clone();

        let secret_bytes =
            hex::decode(&cfg.sequencer_secret).wrap_err("failed decoding string as hexadecimal")?;
        let sequencer_key = Arc::new(
            SigningKey::try_from(&*secret_bytes)
                .wrap_err("failed to construct signing key from decoded sequencer secret")?,
        );

        Ok(Self {
            eth_client,
            sequencer_client,
            nonce,
            rollup_chain_id,
            sequencer_key,
            conversion_tasks: JoinSet::new(),
            submission_tasks: JoinSet::new(),
        })
    }

    fn handle_pending_tx(&mut self, tx: Transaction) {
        let nonce = self.nonce.clone();
        let chain_id = self.rollup_chain_id.clone();
        let sequencer_key = self.sequencer_key.clone();
        self.conversion_tasks.spawn_blocking(move || {
            let data = tx.rlp().to_vec();
            let chain_id = chain_id.into_bytes();
            let seq_action = SequencerAction::SequenceAction(SequenceAction::new(chain_id, data));
            let nonce = nonce.fetch_add(1, Ordering::Relaxed).into();
            Unsigned::new_with_actions(nonce, vec![seq_action]).into_signed(&*sequencer_key)
        });
    }

    fn handle_signed_tx(&mut self, tx: Signed) {
        let client = self.sequencer_client.clone();
        self.submission_tasks.spawn(async move {
            let rsp = client
                .submit_transaction_sync(tx)
                .await
                .wrap_err("failed to submit transaction to sequencer")?;
            // TODO: provide a jsonrpc error server msg?
            ensure!(rsp.code.is_ok(), "sequencer responded with non zero code",);
            Ok(())
        });
    }

    /// Runs the Searcher and blocks until all subtasks have exited:
    /// - api server
    /// - tx collector
    /// - bundler
    /// - executor
    ///
    /// # Errors
    ///
    /// - `searcher::Error` if the Searcher fails to start or if any of the subtasks fail
    /// and cannot be recovered.
    pub async fn run(mut self) -> eyre::Result<()> {
        let eth_client = self.eth_client.clone();
        let mut tx_stream = eth_client
            .subscribe_full_pending_txs()
            .await
            .wrap_err("failed to subscriber eth client to full pending transactions")?;

        loop {
            select!(
                Some(rsp) = tx_stream.next() => self.handle_pending_tx(rsp),
                Some(signed_tx) = self.conversion_tasks.join_next(), if !self.conversion_tasks.is_empty() => {
                    match signed_tx {
                        Ok(signed_tx) => self.handle_signed_tx(signed_tx),
                        Err(e) => warn!(
                            error.message = %e,
                            error.cause_chain = ?e,
                            "conversion task failed while trying to convert pending eth transaction to signed sequencer transaction",
                        ),
                    }
                }
                // TODO: do smth with the response. log?
                Some(_submission_rsp) = self.submission_tasks.join_next(), if !self.submission_tasks.is_empty() => {},
            )
        }

        // FIXME: ensure that we can get here
        #[allow(unreachable_code)]
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::searcher::Config,
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
