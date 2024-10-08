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
        asset,
        RollupId,
    },
    protocol::transaction::v1alpha1::action::Sequence,
};
use astria_eyre::eyre::{
    self,
    eyre,
    Report,
    WrapErr as _,
};
use ethers::{
    providers::{
        Provider,
        ProviderError,
        SubscriptionStream,
        Ws,
    },
    types::Transaction,
};
use telemetry::metrics::Counter;
use tokio::{
    select,
    sync::{
        mpsc::error::SendTimeoutError,
        watch,
    },
};
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    info,
    instrument,
    warn,
};

use crate::{
    collectors::EXECUTOR_SEND_TIMEOUT,
    executor::{
        self,
        Handle,
    },
    metrics::Metrics,
    utils::report_exit_reason,
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
    metrics: &'static Metrics,
    fee_asset: asset::Denom,
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
    pub(crate) metrics: &'static Metrics,
    pub(crate) fee_asset: asset::Denom,
}

impl Builder {
    pub(crate) fn build(self) -> Geth {
        let Self {
            chain_name,
            url,
            executor_handle,
            shutdown_token,
            metrics,
            fee_asset,
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
            metrics,
            fee_asset,
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
    pub(crate) async fn run_until_stopped(self) -> eyre::Result<()> {
        use ethers::providers::Middleware as _;
        use futures::stream::StreamExt as _;

        let Self {
            rollup_id,
            executor_handle,
            status,
            url,
            shutdown_token,
            chain_name,
            metrics,
            fee_asset,
        } = self;

        let txs_received_counter = txs_received_counter(metrics, &chain_name);
        let txs_dropped_counter = txs_dropped_counter(metrics, &chain_name);

        let client = connect_to_geth_node(url)
            .await
            .wrap_err("failed to connect to geth node")?;

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
                        let data = tx.rlp().to_vec();
                        let seq_action = Sequence {
                            rollup_id,
                            data: data.into(),
                            fee_asset: fee_asset.clone(),
                        };

                        txs_received_counter.increment(1);

                        if let Err(err) = forward_geth_tx(
                            &executor_handle,
                            seq_action,
                            tx_hash,
                            &txs_dropped_counter,
                        ).await {
                            break Err(err);
                        }

                    } else {
                        break Err(eyre!("geth tx stream ended"));
                    }
                }
            }
        };

        report_exit_reason(reason.as_deref());

        status.send_modify(|status| status.is_connected = false);

        // if the loop exits with an error, we can still proceed with unsubscribing the WSS
        // stream as we could have exited due to an error in sending messages via the executor
        // channel.
        unsubscribe_from_rollup(&tx_stream).await;

        reason.map(|_| ())
    }
}

#[instrument(skip_all)]
async fn forward_geth_tx(
    executor_handle: &Handle,
    seq_action: Sequence,
    tx_hash: ethers::types::H256,
    txs_dropped_counter: &Counter,
) -> eyre::Result<()> {
    match executor_handle
        .send_timeout(seq_action, EXECUTOR_SEND_TIMEOUT)
        .await
    {
        Ok(()) => Ok(()),
        Err(SendTimeoutError::Timeout(_seq_action)) => {
            warn!(
                transaction.hash = %tx_hash,
                timeout_ms = EXECUTOR_SEND_TIMEOUT.as_millis(),
                "timed out sending new transaction to executor; dropping tx",
            );
            txs_dropped_counter.increment(1);
            Ok(())
        }
        Err(SendTimeoutError::Closed(_seq_action)) => {
            warn!(
                transaction.hash = %tx_hash,
                "executor channel closed while sending transaction; dropping transaction \
                    and exiting event loop"
            );
            txs_dropped_counter.increment(1);
            Err(eyre!("executor channel closed while sending transaction"))
        }
    }
}

#[instrument(skip_all)]
async fn unsubscribe_from_rollup(tx_stream: &SubscriptionStream<'_, Ws, Transaction>) {
    // give 2s for the websocket connection to be unsubscribed as we want to avoid having
    // this hang for too long
    match tokio::time::timeout(WSS_UNSUBSCRIBE_TIMEOUT, tx_stream.unsubscribe()).await {
        Ok(Ok(true)) => info!("unsubscribed from geth tx stream"),
        Ok(Ok(false)) => warn!("geth responded to unsubscribe request but returned `false`"),
        Ok(Err(err)) => {
            error!(error = %Report::new(err), "failed unsubscribing from the geth tx stream");
        }
        Err(_) => {
            error!("timed out while unsubscribing from the geth tx stream");
        }
    }
}

#[instrument(skip_all)]
fn txs_received_counter(metrics: &'static Metrics, chain_name: &String) -> Counter {
    metrics
        .geth_txs_received(chain_name)
        .cloned()
        .unwrap_or_else(|| {
            error!(
                rollup_chain_name = %chain_name,
                "failed to get geth transactions_received counter"
            );
            Counter::noop()
        })
}

#[instrument(skip_all)]
fn txs_dropped_counter(metrics: &'static Metrics, chain_name: &String) -> Counter {
    metrics
        .geth_txs_dropped(chain_name)
        .cloned()
        .unwrap_or_else(|| {
            error!(
                rollup_chain_name = %chain_name,
                "failed to get geth transactions_dropped counter"
            );
            Counter::noop()
        })
}

#[instrument(skip_all, err)]
async fn connect_to_geth_node(url: String) -> eyre::Result<Provider<Ws>> {
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

    Ok(client)
}
