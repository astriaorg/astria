use std::time::Duration;

use astria_sequencer::{
    accounts::types::Nonce,
    sequence::Action as SequenceAction,
    transaction::{
        action::Action as SequencerAction, Signed as SignedSequencerTx,
        Unsigned as UnsignedSequencerTx,
    },
};
use color_eyre::eyre::{self, bail, WrapErr as _};
use ethers::{
    providers::{Provider, ProviderError, Ws},
    types::Transaction,
};
use humantime::format_duration;
use tendermint::abci;
use tokio::{select, sync::watch, task::JoinSet};
use tracing::{debug, info, instrument, warn};


impl Searcher {

    /// Runs the Searcher
    pub(super) async fn run(mut self) -> eyre::Result<()> {
        use ethers::providers::{Middleware as _, StreamExt as _};
        let wait_for_eth = self.wait_for_eth(5, Duration::from_secs(5), 2.0);
        let wait_for_seq = self.wait_for_sequencer(5, Duration::from_secs(5), 2.0);
        match tokio::try_join!(wait_for_eth, wait_for_seq) {
            Ok(((), ())) => {}
            Err(err) => return Err(err).wrap_err("failed to start searcher"),
        }
        let eth_client = self.eth_client.inner.clone();
        let mut tx_stream = eth_client
            .subscribe_full_pending_txs()
            .await
            .wrap_err("failed to subscribe eth client to full pending transactions")?;

        loop {
            select!(
                // serialize and sign sequencer tx for incoming pending rollup txs
                Some(rollup_tx) = tx_stream.next() => self.handle_pending_tx(rollup_tx),

                // submit signed sequencer txs to sequencer
                Some(join_result) = self.conversion_tasks.join_next(), if !self.conversion_tasks.is_empty() => {
                    match join_result {
                        Ok(Ok(signed_tx)) => self.handle_signed_tx(signed_tx),
                        Ok(Err(e)) => warn!(error.message = %e, error.cause_chain = ?e, "failed to sign sequencer transaction"),
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
                        Ok(Ok(())) => {}
                        Ok(Err(e)) =>
                            // TODO: Decide what to do if submitting to sequencer failed. Should it be resubmitted?
                            warn!(error.message = %e, error.cause_chain = ?e, "failed to submit signed sequencer transaction to sequencer"),
                        Err(e) => warn!(
                            error.message = %e,
                            error.cause_chain = ?e,
                            "submission task failed while trying to submit signed sequencer transaction to sequencer",
                        ),
                    }
                }
            );
        }

        // FIXME: ensure that we can get here
        #[allow(unreachable_code)]
        Ok(())
    }
}
