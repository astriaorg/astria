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

use std::{
    collections::{
        BTreeMap,
        HashMap,
    },
    time::Duration,
};

use astria_core::{
    primitive::v1::{
        asset,
        RollupId,
    },
    protocol::transaction::v1::action::RollupDataSubmission,
};
use astria_eyre::eyre::{
    self,
    eyre,
    Report,
    WrapErr as _,
};
use ethers::{
    providers::{
        Middleware as _,
        Provider,
        ProviderError,
        Ws,
    },
    types::U256,
};
use futures::stream;
use itertools::Itertools as _;
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
    info_span,
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

    /// Starts the collector instance and runs until failure or until explicitly closed.
    ///
    /// The following steps are performed:
    /// 1. Connect to Geth node and subscribe to new pending transactions.
    /// 2. Retrieve any existing pending transactions and add them to a cache.
    /// 3. Stream existing pending transactions followed by subscription to new pending
    ///    transactions, sending each to the executor. If the transaction has already been cached as
    ///    sent, it is skipped. This is to account for any transactions which are received between
    ///    steps 1 and 2, and hence are included in both.
    pub(crate) async fn run_until_stopped(self) -> eyre::Result<()> {
        use ethers::providers::Middleware as _;
        use futures::stream::StreamExt as _;

        let Self {
            status,
            url,
            shutdown_token,
            chain_name,
            metrics,
            ..
        } = self;

        let txs_received_counter = txs_received_counter(metrics, &chain_name);
        let txs_dropped_counter = txs_dropped_counter(metrics, &chain_name);

        let client = connect_to_geth_node(url.clone())
            .await
            .wrap_err("failed to connect to geth node")?;

        // Subscribe to pending transactions immediately so that there is no gap between retrieving
        // the current pending transactions and subscribing to new ones.
        let new_tx_stream = client
            .subscribe_full_pending_txs()
            .await
            .wrap_err("failed to subscribe eth client to full pending transactions")?;
        let new_tx_subscription_id = new_tx_stream.id;

        // Get current pending transactions, since the subscription will only return new ones.
        // Using `sorted_by_key` instead of `sorted_unstable_by_key` to ensure that the order of the
        // transactions is determinisic.
        let existing_pending_txs = client
            .txpool_content()
            .await
            .wrap_err("failed to get current tx pool")?
            .pending
            .into_values()
            .flat_map(BTreeMap::into_values)
            .sorted_by_key(|tx| tx.nonce)
            .collect::<Vec<_>>();

        if existing_pending_txs.is_empty() {
            info_span!("fetch_pending_rollup_txs")
                .in_scope(|| info!("no pending transactions in tx pool"));
        } else {
            info_span!("fetch_pending_rollup_txs").in_scope(|| {
                info!(
                    num_existing_txs = existing_pending_txs.len(),
                    "fetched pending transactions in tx pool that will be sent prior to newly \
                     submitted transactions",
                );
            });
        };

        // Create a cache for existing pending transactions to avoid sending the same transaction if
        // it is also streamed via `new_tx_stream`.
        let mut existing_pending_tx_cache = existing_pending_txs
            .iter()
            .map(|tx| (tx.hash.0, false))
            .collect::<HashMap<_, _>>();

        // Chain current pending transactions with new transactions.
        let existing_pending_txs_stream = stream::iter(existing_pending_txs.into_iter());
        let mut tx_stream = existing_pending_txs_stream.chain(new_tx_stream);

        status.send_modify(|status| status.is_connected = true);

        let reason = loop {
            select! {
                biased;
                () = shutdown_token.cancelled() => {
                    break Ok("shutdown signal received");
                },
                tx_res = tx_stream.next() => {
                    if let Some(tx) = tx_res {
                        // Check cache for previously sent transactions.
                        if let Some(previously_sent) = existing_pending_tx_cache.get(&tx.hash.0) {
                            if *previously_sent {
                                // this transaction was already sent to the executor
                                continue;
                            }
                            // update value in cache to represent that it has already been sent
                            existing_pending_tx_cache.insert(tx.hash.0, true);
                        };

                        txs_received_counter.increment(1);

                        let tx_hash = tx.hash;
                        let data = tx.rlp().to_vec();
                        let seq_action = RollupDataSubmission {
                            rollup_id: self.rollup_id,
                            data: data.into(),
                            fee_asset: self.fee_asset.clone(),
                        };

                        if let Err(err) = forward_geth_tx(
                            &self.executor_handle,
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

        status.send_modify(|status| status.is_connected = false);

        // if the loop exits with an error, we can still proceed with unsubscribing the WSS
        // stream as we could have exited due to an error in sending messages via the executor
        // channel.
        unsubscribe_from_rollup(&client, &new_tx_subscription_id).await;

        report_exit_reason(reason.as_deref());

        reason.map(|_| ())
    }
}

#[instrument(skip_all)]
async fn forward_geth_tx(
    executor_handle: &Handle,
    seq_action: RollupDataSubmission,
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
async fn unsubscribe_from_rollup(tx_stream: &Provider<Ws>, subscription_id: &U256) {
    // give 2s for the websocket connection to be unsubscribed as we want to avoid having
    // this hang for too long
    match tokio::time::timeout(
        WSS_UNSUBSCRIBE_TIMEOUT,
        tx_stream.unsubscribe(subscription_id),
    )
    .await
    {
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
                    .map(telemetry::display::format_duration)
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
