//! `GethCollector` fetches pending transactions from a Geth Rollup.
//!
//! //! [`GethCollector`] subscribes to pending transactions from a [go-ethereum](https://geth.ethereum.org) rollup node,
//! and forwards them for more processing. and then passing them downstream for the executor to
//! process.
//!
//! ## Note
//! This collector is likely specific to go-ethereum and only checked to work wit. It makes use of
//! the [`eth_subscribe`](https://geth.ethereum.org/docs/interacting-with-geth/rpc/pubsub#newpendingtransactions)
//! JSON-RPC with arguments shown below. It appears as if go-ethereum is the only ethereum node that
//! documents this.
//! ``` json
//! { "id": 1, "jsonrpc": "2.0", "method": "eth_subscribe", "params": ["newPendingTransactions"] }
//! ```

use astria_core::sequencer::v1::{
    asset::default_native_asset_id,
    transaction::action::SequenceAction,
    RollupId,
};
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use ethers::providers::{
    Provider,
    ProviderError,
    Ws,
};
use tokio::sync::{
    mpsc::{
        error::SendTimeoutError,
        Sender,
    },
    watch,
};
use tracing::{
    debug,
    instrument,
    warn,
};

type StdError = dyn std::error::Error;

/// `GethCollector` Collects transactions submitted to a Geth rollup node and passes
/// them downstream for further processing.
///
/// It is responsible for fetching pending transactions submitted to the rollup Geth nodes and then
/// passing them downstream for the executor to process. Thus, a composer can have multiple
/// collectors running at the same time funneling data from multiple rollup nodes.
#[derive(Debug)]
pub(super) struct GethCollector {
    // Chain ID to identify in the astria sequencer block which rollup a serialized sequencer
    // action belongs to. Created from `chain_name`.
    rollup_id: RollupId,
    // Name of the chain the transactions are read from.
    chain_name: String,
    // The channel on which the collector sends new txs to the executor.
    new_bundles: Sender<SequenceAction>,
    // The status of this collector instance.
    status: watch::Sender<Status>,
    /// Rollup URL
    url: String,
}

#[derive(Debug)]
pub(super) struct Status {
    is_connected: bool,
}

impl Status {
    fn new() -> Self {
        Self {
            is_connected: false,
        }
    }

    pub(super) fn is_connected(&self) -> bool {
        self.is_connected
    }
}

impl GethCollector {
    /// Initializes a new collector instance
    pub(super) fn new(
        chain_name: String,
        url: String,
        new_bundles: Sender<SequenceAction>,
    ) -> Self {
        let (status, _) = watch::channel(Status::new());
        Self {
            rollup_id: RollupId::from_unhashed_bytes(&chain_name),
            chain_name,
            new_bundles,
            status,
            url,
        }
    }

    /// Subscribe to the collector's status.
    pub(super) fn subscribe(&self) -> watch::Receiver<Status> {
        self.status.subscribe()
    }

    /// Starts the collector instance and runs until failure or until
    /// explicitly closed
    #[instrument(skip_all, fields(chain_name = self.chain_name))]
    pub(super) async fn run_until_stopped(self) -> eyre::Result<()> {
        use std::time::Duration;

        use ethers::providers::Middleware as _;
        use futures::stream::StreamExt as _;

        let Self {
            rollup_id,
            new_bundles,
            status,
            url,
            ..
        } = self;

        let retry_config = tryhard::RetryFutureConfig::new(1024)
            .exponential_backoff(Duration::from_millis(500))
            .max_delay(Duration::from_secs(60))
            .on_retry(
                |attempt, next_delay: Option<Duration>, error: &ProviderError| {
                    let wait_duration = next_delay
                        .map(humantime::format_duration)
                        .map(tracing::field::display);
                    warn!(
                        attempt,
                        wait_duration,
                        error = error as &StdError,
                        "attempt to connect to geth node failed; retrying after backoff",
                    );
                    futures::future::ready(())
                },
            );

        let client = tryhard::retry_fn(|| {
            let url = url.clone();
            async move {
                let websocket_client = Ws::connect_with_reconnects(url, 0).await?;
                Ok(Provider::new(websocket_client))
            }
        })
        .with_config(retry_config)
        .await
        .wrap_err("failed connecting to geth after several retries; giving up")?;

        let mut tx_stream = client
            .subscribe_full_pending_txs()
            .await
            .wrap_err("failed to subscribe eth client to full pending transactions")?;

        status.send_modify(|status| status.is_connected = true);

        while let Some(tx) = tx_stream.next().await {
            let tx_hash = tx.hash;
            debug!(transaction.hash = %tx_hash, "collected transaction from rollup");
            let data = tx.rlp().to_vec();
            let seq_action = SequenceAction {
                rollup_id,
                data,
                fee_asset_id: default_native_asset_id(),
            };

            match new_bundles
                .send_timeout(seq_action, Duration::from_millis(500))
                .await
            {
                Ok(()) => {}
                Err(SendTimeoutError::Timeout(_seq_action)) => {
                    warn!(
                        transaction.hash = %tx_hash,
                        "timed out sending new transaction to executor after 500ms; dropping tx"
                    );
                }
                Err(SendTimeoutError::Closed(_seq_action)) => {
                    warn!(
                        transaction.hash = %tx_hash,
                        "executor channel closed while sending transaction; dropping transaction \
                         and exiting event loop"
                    );
                    break;
                }
            }
        }
        Ok(())
    }
}
