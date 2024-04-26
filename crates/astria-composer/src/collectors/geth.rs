//! `GethCollector` fetches pending transactions from a Geth Rollup.
//!
//! //! [`Geth`] subscribes to pending transactions from a [go-ethereum](https://geth.ethereum.org) rollup node,
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

use std::time::Duration;

use astria_core::{
    primitive::v1::{
        asset::default_native_asset_id,
        RollupId,
    },
    protocol::transaction::v1alpha1::action::SequenceAction,
};
use astria_eyre::eyre::{
    self,
    eyre,
    Report,
    WrapErr as _,
};
use ethers::providers::{
    Provider,
    ProviderError,
    Ws,
};
use tokio::{
    select,
    sync::{
        mpsc::error::SendTimeoutError,
        watch,
    },
};
use tokio_util::sync::CancellationToken;
use tracing::{
    debug,
    error,
    info,
    instrument,
    warn,
};

use crate::{
    collectors::{
        CollectorType,
        EXECUTOR_SEND_TIMEOUT,
    },
    executor,
    metrics_init::{
        COLLECTOR_TYPE_LABEL,
        ROLLUP_ID_LABEL,
    },
};

type StdError = dyn std::error::Error;

const WSS_UNSUBSCRIBE_TIMEOUT: Duration = Duration::from_secs(2);

/// `GethCollector` Collects transactions submitted to a Geth rollup node and passes
/// them downstream for further processing.
///
/// It is responsible for fetching pending transactions submitted to the rollup Geth nodes and then
/// passing them downstream for the executor to process. Thus, a composer can have multiple
/// collectors running at the same time funneling data from multiple rollup nodes.
pub(crate) struct Geth {
    // Chain ID to identify in the astria sequencer block which rollup a serialized sequencer
    // action belongs to. Created from `chain_name`.
    rollup_id: RollupId,
    // Name of the chain the transactions are read from.
    chain_name: String,
    // The channel on which the collector sends new txs to the executor.
    executor_handle: executor::Handle,
    // The status of this collector instance.
    status: watch::Sender<Status>,
    // Rollup URL
    url: String,
    // Token to signal the geth collector to stop upon shutdown.
    shutdown_token: CancellationToken,
}

#[derive(Debug)]
pub(crate) struct Status {
    pub(crate) is_connected: bool,
}

impl Status {
    fn new() -> Self {
        Self {
            is_connected: false,
        }
    }

    pub(crate) fn is_connected(&self) -> bool {
        self.is_connected
    }
}

pub(crate) struct Builder {
    pub(crate) chain_name: String,
    pub(crate) url: String,
    pub(crate) executor_handle: executor::Handle,
    pub(crate) shutdown_token: CancellationToken,
}

impl Builder {
    pub(crate) fn build(self) -> Geth {
        let Self {
            chain_name,
            url,
            executor_handle,
            shutdown_token,
        } = self;
        let (status, _) = watch::channel(Status::new());
        let rollup_id = RollupId::from_unhashed_bytes(&chain_name);
        info!(
            rollup_name = %chain_name,
            rollup_id = %rollup_id,
            "created new geth collector for rollup",
        );
        Geth {
            rollup_id,
            chain_name,
            executor_handle,
            status,
            url,
            shutdown_token,
        }
    }
}

impl Geth {
    /// Subscribe to the collector's status.
    pub(crate) fn subscribe(&self) -> watch::Receiver<Status> {
        self.status.subscribe()
    }

    /// Starts the collector instance and runs until failure or until
    /// explicitly closed
    #[instrument(skip_all, fields(chain_name = self.chain_name, rollup_id = %self.rollup_id))]
    pub(crate) async fn run_until_stopped(self) -> eyre::Result<()> {
        use std::time::Duration;

        use ethers::providers::Middleware as _;
        use futures::stream::StreamExt as _;

        let Self {
            rollup_id,
            executor_handle,
            status,
            url,
            shutdown_token,
            chain_name,
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

        let reason = loop {
            select! {
                biased;
                () = shutdown_token.cancelled() => {
                    break Ok("shutdown signal received");
                },
                tx_res = tx_stream.next() => {
                    if let Some(tx) = tx_res {
                        let tx_hash = tx.hash;
                        debug!(transaction.hash = %tx_hash, "collected transaction from rollup");
                        let data = tx.rlp().to_vec();
                        let seq_action = SequenceAction {
                            rollup_id,
                            data,
                            fee_asset_id: default_native_asset_id(),
                        };

                        metrics::counter!(
                            crate::metrics_init::TRANSACTIONS_COLLECTED,
                            &[
                                (ROLLUP_ID_LABEL, chain_name.clone()),
                                (COLLECTOR_TYPE_LABEL, CollectorType::Geth.to_string())
                            ]).increment(1);

                        match executor_handle
                            .send_timeout(seq_action, EXECUTOR_SEND_TIMEOUT)
                            .await
                        {
                            Ok(()) => {
                                metrics::counter!(
                                    crate::metrics_init::TRANSACTIONS_FORWARDED,
                                    &[
                                        (ROLLUP_ID_LABEL, chain_name.clone()),
                                        (COLLECTOR_TYPE_LABEL, CollectorType::Geth.to_string())
                                    ]
                                ).increment(1);
                            },
                            Err(SendTimeoutError::Timeout(_seq_action)) => {
                                warn!(
                                    transaction.hash = %tx_hash,
                                    timeout_ms = EXECUTOR_SEND_TIMEOUT.as_millis(),
                                    "timed out sending new transaction to executor; dropping tx",
                                );
                                metrics::counter!(
                                    crate::metrics_init::TRANSACTIONS_DROPPED,
                                    &[
                                        (ROLLUP_ID_LABEL, chain_name.clone()),
                                        (COLLECTOR_TYPE_LABEL, CollectorType::Geth.to_string())
                                    ]
                                ).increment(1);
                            }
                            Err(SendTimeoutError::Closed(_seq_action)) => {
                                warn!(
                                    transaction.hash = %tx_hash,
                                    "executor channel closed while sending transaction; dropping transaction \
                                     and exiting event loop"
                                );
                                metrics::counter!(
                                    crate::metrics_init::TRANSACTIONS_DROPPED,
                                    &[
                                        (ROLLUP_ID_LABEL, chain_name.clone()),
                                        (COLLECTOR_TYPE_LABEL, CollectorType::Geth.to_string())
                                    ]
                                ).increment(1);
                                break Err(eyre!("executor channel closed while sending transaction"));
                            }
                        }
                    } else {
                        break Err(eyre!("geth tx stream ended"));
                    }
                }
            }
        };

        match &reason {
            Ok(reason) => {
                info!(reason, "shutting down");
            }
            Err(reason) => {
                error!(%reason, "shutting down");
            }
        };

        status.send_modify(|status| status.is_connected = false);

        // if the loop exits with an error, we can still proceed with unsubscribing the WSS
        // stream as we could have exited due to an error in sending messages via the executor
        // channel.

        // give 2s for the websocket connection to be unsubscribed as we want to avoid having
        // this hang for too long
        match tokio::time::timeout(WSS_UNSUBSCRIBE_TIMEOUT, tx_stream.unsubscribe()).await {
            Ok(Ok(true)) => info!("unsubscribed from geth tx stream"),
            Ok(Ok(false)) => warn!("failed to unsubscribe from geth tx stream"),
            Ok(Err(err)) => {
                error!(error = %Report::new(err), "failed unsubscribing from the geth tx stream");
            }
            Err(err) => {
                error!(error = %Report::new(err), "timed out while unsubscribing from the geth tx stream");
            }
        }

        reason.map(|_| ())
    }
}
