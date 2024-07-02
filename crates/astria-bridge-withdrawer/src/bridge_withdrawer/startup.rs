use std::{
    sync::Arc,
    time::Duration,
};

use astria_core::{
    bridge::Ics20WithdrawalFromRollupMemo,
    generated::sequencerblock::v1alpha1::{
        sequencer_service_client,
        GetPendingNonceRequest,
    },
    primitive::v1::asset,
    protocol::{
        asset::v1alpha1::AllowedFeeAssetsResponse,
        bridge::v1alpha1::BridgeAccountLastTxHashResponse,
        transaction::v1alpha1::Action,
    },
};
use astria_eyre::eyre::{
    self,
    ensure,
    eyre,
    OptionExt as _,
    WrapErr as _,
};
use prost::Message as _;
use sequencer_client::{
    tendermint_rpc,
    Address,
    BalanceResponse,
    SequencerClientExt as _,
    SignedTransaction,
};
use tendermint_rpc::{
    endpoint::tx,
    Client as _,
};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tonic::transport::Channel;
use tracing::{
    debug,
    error,
    info,
    info_span,
    instrument,
    warn,
    Instrument as _,
    Span,
};
use tryhard::backoff_strategies::ExponentialBackoff;

use super::state::{
    self,
    State,
};
use crate::bridge_withdrawer::ethereum::convert::BridgeUnlockMemo;

pub(super) struct Builder {
    pub(super) shutdown_token: CancellationToken,
    pub(super) state: Arc<State>,
    pub(super) sequencer_chain_id: String,
    pub(super) sequencer_cometbft_endpoint: String,
    pub(super) sequencer_bridge_address: Address,
    pub(super) sequencer_grpc_endpoint: String,
    pub(super) expected_fee_asset: asset::Denom,
    // TODO: change the name of this config var
    pub(super) expected_min_fee_asset_balance: u128,
}

impl Builder {
    pub(super) fn build(self) -> eyre::Result<Startup> {
        let Self {
            shutdown_token,
            state,
            sequencer_chain_id,
            sequencer_cometbft_endpoint,
            sequencer_bridge_address,
            sequencer_grpc_endpoint,
            expected_fee_asset,
            expected_min_fee_asset_balance,
        } = self;

        let sequencer_cometbft_client =
            sequencer_client::HttpClient::new(&*sequencer_cometbft_endpoint)
                .wrap_err("failed constructing cometbft http client")?;

        Ok(Startup {
            shutdown_token,
            state,
            sequencer_chain_id,
            sequencer_cometbft_client,
            sequencer_bridge_address,
            sequencer_grpc_endpoint,
            expected_fee_asset,
            expected_min_fee_asset_balance,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(super) struct Info {
    pub(super) starting_rollup_height: u64,
    pub(super) fee_asset: asset::Denom,
    pub(super) chain_id: String,
}

#[derive(Debug, Clone)]
pub(super) struct InfoHandle {
    rx: watch::Receiver<state::StateSnapshot>,
}

impl InfoHandle {
    pub(super) fn new(rx: watch::Receiver<state::StateSnapshot>) -> Self {
        Self {
            rx,
        }
    }

    pub(super) async fn get_info(&mut self) -> eyre::Result<Info> {
        let state = self
            .rx
            .wait_for(|state| state.get_startup_info().is_some())
            .await
            .wrap_err("")?;

        Ok(state
            .get_startup_info()
            .expect("the previous line guarantes that the state is intialized")
            .clone())
    }
}

pub(super) struct Startup {
    shutdown_token: CancellationToken,
    state: Arc<State>,
    sequencer_chain_id: String,
    sequencer_cometbft_client: sequencer_client::HttpClient,
    sequencer_bridge_address: Address,
    sequencer_grpc_endpoint: String,
    expected_fee_asset: asset::Denom,
    expected_min_fee_asset_balance: u128,
}

impl Startup {
    pub(super) async fn run(mut self) -> eyre::Result<()> {
        let shutdown_token = self.shutdown_token.clone();

        let startup_task = tokio::spawn({
            let state = self.state.clone();
            async move {
                self.confirm_sequencer_config()
                    .await
                    .wrap_err("failed to confirm sequencer config")?;

                let sequencer_grpc_client =
                    sequencer_service_client::SequencerServiceClient::connect(format!(
                        "http://{}",
                        self.sequencer_grpc_endpoint
                    ))
                    .await
                    .wrap_err("sequencer grpc failed to connect client")?;

                wait_for_empty_mempool(
                    self.sequencer_cometbft_client.clone(),
                    sequencer_grpc_client,
                    self.sequencer_bridge_address,
                    self.state.clone(),
                )
                .await
                .wrap_err("failed to wait for mempool to be empty")?;

                let starting_rollup_height = self
                    .get_starting_rollup_height()
                    .await
                    .wrap_err("failed to get next rollup block height")?;

                // send the startup info to the submitter
                let info = Info {
                    chain_id: self.sequencer_chain_id.clone(),
                    fee_asset: self.expected_fee_asset,
                    starting_rollup_height,
                };

                state.set_startup_info(info);

                Ok(())
            }
        });

        tokio::select!(
            () = shutdown_token.cancelled() => {
                Err(eyre!("startup was cancelled"))
            }
            res = startup_task => {
                match res {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(err)) => {
                        error!(%err, "startup failed");
                        Err(err)},
                    Err(reason) => {
                        Err(reason.into())
                    }
                }
            }
        )
    }

    /// Confirms configuration values against the sequencer node. Values checked:
    ///
    /// - `self.sequencer_chain_id` matches the value returned from the sequencer node's genesis
    /// - `self.fee_asset_id` is a valid fee asset on the sequencer node
    /// - `self.sequencer_bridge_address` has a sufficient balance of `self.fee_asset_id`
    ///
    /// # Errors
    ///
    /// - `self.chain_id` does not match the value returned from the sequencer node
    /// - `self.fee_asset_id` is not a valid fee asset on the sequencer node
    /// - `self.sequencer_bridge_address` does not have a sufficient balance of `self.fee_asset_id`.
    async fn confirm_sequencer_config(&mut self) -> eyre::Result<()> {
        // confirm the sequencer chain id
        let actual_chain_id =
            get_sequencer_chain_id(self.sequencer_cometbft_client.clone(), self.state.clone())
                .await
                .wrap_err("failed to get chain id from sequencer")?;
        ensure!(
            self.sequencer_chain_id == actual_chain_id.to_string(),
            "sequencer_chain_id provided in config does not match chain_id returned from sequencer"
        );

        // confirm that the fee asset ID is valid
        let allowed_fee_asset_ids_resp =
            get_allowed_fee_asset_ids(self.sequencer_cometbft_client.clone(), self.state.clone())
                .await
                .wrap_err("failed to get allowed fee asset ids from sequencer")?;
        ensure!(
            allowed_fee_asset_ids_resp
                .fee_assets
                .contains(&self.expected_fee_asset),
            "fee_asset_id provided in config is not a valid fee asset on the sequencer"
        );

        // confirm that the sequencer key has a sufficient balance of the fee asset
        let fee_asset_balances = get_latest_balance(
            self.sequencer_cometbft_client.clone(),
            self.state.clone(),
            self.sequencer_bridge_address,
        )
        .await
        .wrap_err("failed to get latest balance")?;
        let fee_asset_balance = fee_asset_balances
            .balances
            .into_iter()
            .find(|balance| balance.denom == self.expected_fee_asset)
            .ok_or_eyre("withdrawer's account balance of the fee asset is zero")?
            .balance;
        ensure!(
            fee_asset_balance >= self.expected_min_fee_asset_balance,
            "withdrawer account does not have a sufficient balance of the fee asset"
        );

        Ok(())
    }

    /// Gets the last transaction by the bridge account on the sequencer. This is used to
    /// determine the starting rollup height for syncing to the latest on-chain state.
    ///
    /// # Returns
    /// The last transaction by the bridge account on the sequencer, if it exists.
    ///
    /// # Errors
    ///
    /// 1. Failing to fetch the last transaction hash by the bridge account.
    /// 2. Failing to convert the last transaction hash to a tendermint hash.
    /// 3. Failing to fetch the last transaction by the bridge account.
    /// 4. The last transaction by the bridge account failed to execute (this should not happen
    ///   in the sequencer logic).
    /// 5. Failing to convert the transaction data from bytes to proto.
    /// 6. Failing to convert the transaction data from proto to `SignedTransaction`.
    async fn get_last_transaction(&self) -> eyre::Result<Option<SignedTransaction>> {
        // get last transaction hash by the bridge account, if it exists
        let last_transaction_hash_resp = get_bridge_account_last_transaction_hash(
            self.sequencer_cometbft_client.clone(),
            self.state.clone(),
            self.sequencer_bridge_address,
        )
        .await
        .wrap_err("failed to fetch last transaction hash by the bridge account")?;

        let Some(tx_hash) = last_transaction_hash_resp.tx_hash else {
            return Ok(None);
        };

        let tx_hash = tendermint::Hash::try_from(tx_hash.to_vec())
            .wrap_err("failed to convert last transaction hash to tendermint hash")?;

        // get the corresponding transaction
        let last_transaction = get_tx(
            self.sequencer_cometbft_client.clone(),
            self.state.clone(),
            tx_hash,
        )
        .await
        .wrap_err("failed to fetch last transaction by the bridge account")?;

        // check that the transaction actually executed
        ensure!(
            last_transaction.tx_result.code == tendermint::abci::Code::Ok,
            "last transaction by the bridge account failed to execute. this should not happen in \
             the sequencer logic."
        );

        let proto_tx =
            astria_core::generated::protocol::transaction::v1alpha1::SignedTransaction::decode(
                &*last_transaction.tx,
            )
            .wrap_err("failed to convert transaction data from CometBFT to proto")?;

        let tx = SignedTransaction::try_from_raw(proto_tx)
            .wrap_err("failed to convert transaction data from proto to SignedTransaction")?;

        info!(
            last_bridge_account_tx.hash = %telemetry::display::hex(&tx_hash),
            last_bridge_account_tx.height = i64::from(last_transaction.height),
            "fetched last transaction by the bridge account"
        );

        Ok(Some(tx))
    }

    /// Gets the data necessary for syncing to the latest on-chain state from the sequencer.
    /// Since we batch all events from a given rollup block into a single sequencer
    /// transaction, we get the last tx finalized by the bridge account on the sequencer
    /// and extract the rollup height from it.
    ///
    /// The rollup height is extracted from the block height value in the memo of one of the
    /// actions in the batch.
    ///
    /// # Returns
    /// The next batch rollup height to process.
    ///
    /// # Errors
    ///
    /// 1. Failing to get and deserialize a valid last transaction by the bridge account from the
    ///    sequencer.
    /// 2. The last transaction by the bridge account failed to execute (this should not happen in
    ///    the sequencer logic)
    /// 3. The last transaction by the bridge account did not contain a withdrawal action
    /// 4. The memo of the last transaction by the bridge account could not be parsed
    async fn get_starting_rollup_height(&mut self) -> eyre::Result<u64> {
        let signed_transaction = self
            .get_last_transaction()
            .await
            .wrap_err("failed to get the bridge account's last sequencer transaction")?;
        let starting_rollup_height = if let Some(signed_transaction) = signed_transaction {
            rollup_height_from_signed_transaction(&signed_transaction)
                .wrap_err(
                    "failed to extract rollup height from last transaction by the bridge account",
                )?
                .checked_add(1)
                .ok_or_eyre("failed to increment rollup height by 1")?
        } else {
            1
        };
        Ok(starting_rollup_height)
    }
}

async fn check_for_empty_mempool(
    cometbft_client: sequencer_client::HttpClient,
    sequencer_client: sequencer_service_client::SequencerServiceClient<Channel>,
    address: Address,
    state: Arc<State>,
) -> eyre::Result<()> {
    // get pending nonce from sequencer mempool
    let pending = get_pending_nonce(sequencer_client, address, state.clone())
        .await
        .wrap_err("failed to get pending nonce")?;
    // get next nonce from cometbft mempool
    let latest = get_latest_nonce(cometbft_client, address, state)
        .await
        .wrap_err("failed to get latest nonce")?;
    // if not equal, wait for a bit and try again
    if pending == latest {
        Ok(())
    } else {
        Err(eyre::eyre!("mempool is not empty"))
    }
}

/// Waits for the mempool to be empty of transactions by the given address (i.e. the bridge
/// withdrawer's). This is used to make sure that batches are submitted under the correct nonce.
///
/// This function checks that the mempool is empty by querying:
/// 1. the pending nonce from the Sequencer's app-side mempool
/// 2. the latest nonce from cometBFT's mempool.
/// If the pending nonce is equal to the latest nonce, then the mempool has no unexecuted
/// transactions by the address.
///
/// This ensures that future submitted batches will continue to maintain the one-to-one
/// relationship between rollup block and withdrawer nonce that is needed to simplify the sync
/// process.
///
/// This function runs the above check with an exponential backoff until the nonces match and the
/// mempool can be considered empty. The backoff starts at 1 second and is capped at 60 seconds.
///
/// # Errors
///
/// 1. Failing to get the pending nonce from the Sequencer's app-side mempool.
/// 2. Failing to get the latest nonce from cometBFT's mempool.
/// 3. The pending nonce from the Sequencer's app-side mempool does not match the latest nonce from
///    cometBFT's mempool after the exponential backoff times out.
async fn wait_for_empty_mempool(
    cometbft_client: sequencer_client::HttpClient,
    sequencer_client: sequencer_service_client::SequencerServiceClient<Channel>,
    address: Address,
    state: Arc<State>,
) -> eyre::Result<()> {
    let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_secs(1))
        .max_delay(Duration::from_secs(60))
        .on_retry(
            |attempt: u32, next_delay: Option<Duration>, error: &eyre::Report| {
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    error = error.as_ref() as &dyn std::error::Error,
                    attempt,
                    wait_duration,
                    "failed getting pending nonce from sequencing; retrying after backoff",
                );

                // TODO: update metrics here?
                futures::future::ready(())
            },
        );

    tryhard::retry_fn(|| {
        let sequencer_client = sequencer_client.clone();
        let cometbft_client = cometbft_client.clone();
        let state = state.clone();

        check_for_empty_mempool(cometbft_client, sequencer_client, address, state)
    })
    .with_config(retry_config)
    .await
    .wrap_err("failed to wait for empty mempool")?;

    Ok(())
}

/// Extracts the rollup height from the last transaction by the bridge account on the sequencer.
/// Since all the withdrawals from a rollup block are batched into a single sequencer transaction,
/// he rollup height can be extracted from the memo of any withdrawal action in the batch.
///
/// # Returns
///
/// The rollup height of the last batch of withdrawals.
///
/// # Errors
///
/// 1. The last transaction by the bridge account did not contain a withdrawal action.
/// 2. The memo of the last transaction by the bridge account could not be parsed.
/// 3. The block number in the memo of the last transaction by the bridge account could not be
///   converted to a u64.
fn rollup_height_from_signed_transaction(
    signed_transaction: &SignedTransaction,
) -> eyre::Result<u64> {
    // find the last batch's rollup block height
    let withdrawal_action = signed_transaction
        .actions()
        .iter()
        .find(|action| matches!(action, Action::BridgeUnlock(_) | Action::Ics20Withdrawal(_)))
        .ok_or_eyre("last transaction by the bridge account did not contain a withdrawal action")?;

    let last_batch_rollup_height = match withdrawal_action {
        Action::BridgeUnlock(action) => {
            let memo: BridgeUnlockMemo = serde_json::from_slice(&action.memo)
                .wrap_err("failed to parse memo from last transaction by the bridge account")?;
            Some(memo.block_number.as_u64())
        }
        Action::Ics20Withdrawal(action) => {
            let memo: Ics20WithdrawalFromRollupMemo = serde_json::from_str(&action.memo)
                .wrap_err("failed to parse memo from last transaction by the bridge account")?;
            Some(memo.block_number)
        }
        _ => None,
    }
    .expect("action is already checked to be either BridgeUnlock or Ics20Withdrawal");

    info!(
        last_batch.tx_hash = %telemetry::display::hex(&signed_transaction.sha256_of_proto_encoding()),
        last_batch.rollup_height = last_batch_rollup_height,
        "extracted rollup height from last batch of withdrawals",
    );

    Ok(last_batch_rollup_height)
}

#[instrument(skip_all)]
async fn get_bridge_account_last_transaction_hash(
    client: sequencer_client::HttpClient,
    state: Arc<State>,
    address: Address,
) -> eyre::Result<BridgeAccountLastTxHashResponse> {
    let res = tryhard::retry_fn(|| client.get_bridge_account_last_transaction_hash(address))
        .with_config(make_sequencer_retry_config(
            state.clone(),
            "attempt to fetch last bridge account's transaction hash from Sequencer; retrying \
             after backoff"
                .to_string(),
        ))
        .await
        .wrap_err(
            "failed to fetch last bridge account's transaction hash from Sequencer after a lot of \
             attempts",
        );

    state.set_sequencer_connected(res.is_ok());

    res
}

#[instrument(skip_all)]
async fn get_tx(
    client: sequencer_client::HttpClient,
    state: Arc<State>,
    tx_hash: tendermint::Hash,
) -> eyre::Result<tx::Response> {
    let res = tryhard::retry_fn(|| client.tx(tx_hash, false))
        .with_config(make_cometbft_retry_config(
            state.clone(),
            "attempt to get transaction from CometBFT; retrying after backoff".to_string(),
        ))
        .await
        .wrap_err("failed to get transaction from Sequencer after a lot of attempts");

    state.set_sequencer_connected(res.is_ok());

    res
}

#[instrument(skip_all)]
async fn get_sequencer_chain_id(
    client: sequencer_client::HttpClient,
    state: Arc<State>,
) -> eyre::Result<tendermint::chain::Id> {
    let genesis: tendermint::Genesis = tryhard::retry_fn(|| client.genesis())
        .with_config(make_cometbft_retry_config(
            state.clone(),
            "attempt to get genesis from CometBFT; retrying after backoff".to_string(),
        ))
        .await
        .wrap_err("failed to get genesis info from Sequencer after a lot of attempts")?;

    state.set_sequencer_connected(true);

    Ok(genesis.chain_id)
}

#[instrument(skip_all)]
async fn get_allowed_fee_asset_ids(
    client: sequencer_client::HttpClient,
    state: Arc<State>,
) -> eyre::Result<AllowedFeeAssetsResponse> {
    let res = tryhard::retry_fn(|| client.get_allowed_fee_assets())
        .with_config(make_sequencer_retry_config(
            state.clone(),
            "attempt to get allowed fee assets from Sequencer; retrying after backoff".to_string(),
        ))
        .await
        .wrap_err("failed to get allowed fee asset ids from Sequencer after a lot of attempts");

    state.set_sequencer_connected(res.is_ok());

    res
}

#[instrument(skip_all)]
async fn get_latest_balance(
    client: sequencer_client::HttpClient,
    state: Arc<State>,
    address: Address,
) -> eyre::Result<BalanceResponse> {
    let res = tryhard::retry_fn(|| {
        // only retry on tendermint_rpc errors, not deserialization or native conversion
        let client = client.clone();
        async move {
            match client.get_latest_balance(address).await {
                Ok(res) => Ok(Ok(res)),
                Err(err) => {
                    if let Some(tendermint_err) = err.as_tendermint_rpc() {
                        Err(tendermint_err.inner().clone())
                    } else {
                        Ok(Err(err))
                    }
                }
            }
        }
    })
    .with_config(make_cometbft_retry_config(
        state.clone(),
        "attempt to get latest balance from CometBFT; retrying after backoff".to_string(),
    ))
    .await
    .wrap_err("failed to get latest balance from Sequencer after a lot of attempts");

    let res = res?.wrap_err("failed to deserialize the latest balance response from CometBFT");

    // set cometbft as connected if received a response
    state.set_sequencer_connected(res.is_ok());

    res
}

fn make_cometbft_retry_config(
    state: Arc<State>,
    retry_message: String,
) -> tryhard::RetryFutureConfig<
    ExponentialBackoff,
    impl Fn(u32, Option<Duration>, &tendermint_rpc::Error) -> futures::future::Ready<()>,
> {
    tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(20))
        .on_retry(
            move |attempt: u32, next_delay: Option<Duration>, error: &tendermint_rpc::Error| {
                let state = Arc::clone(&state);
                state.set_cometbft_connected(false);

                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    attempt,
                    wait_duration,
                    error = error as &dyn std::error::Error,
                    retry_message,
                );
                futures::future::ready(())
            },
        )
}

fn make_sequencer_retry_config(
    state: Arc<State>,
    retry_message: String,
) -> tryhard::RetryFutureConfig<
    ExponentialBackoff,
    impl Fn(
        u32,
        Option<Duration>,
        &sequencer_client::extension_trait::Error,
    ) -> futures::future::Ready<()>,
> {
    tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(20))
        .on_retry(
            move |attempt: u32,
                  next_delay: Option<Duration>,
                  error: &sequencer_client::extension_trait::Error| {
                let state = Arc::clone(&state);
                state.set_sequencer_connected(false);

                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    attempt,
                    wait_duration,
                    error = error as &dyn std::error::Error,
                    retry_message,
                );
                futures::future::ready(())
            },
        )
}

async fn get_latest_nonce(
    client: sequencer_client::HttpClient,
    address: Address,
    state: Arc<State>,
    // metrics: &'static Metrics,
) -> eyre::Result<u32> {
    debug!("fetching latest nonce from sequencer");
    // metrics.increment_nonce_fetch_count();
    let span = Span::current();
    // let start = Instant::now();
    let retry_config = tryhard::RetryFutureConfig::new(1024)
        .exponential_backoff(Duration::from_millis(200))
        .max_delay(Duration::from_secs(60))
        .on_retry(
            |attempt,
             next_delay: Option<Duration>,
             err: &sequencer_client::extension_trait::Error| {
                // metrics.increment_nonce_fetch_failure_count();

                let state = Arc::clone(&state);
                state.set_sequencer_connected(false);

                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    parent: span.clone(),
                    error = err as &dyn std::error::Error,
                    attempt,
                    wait_duration,
                    "failed getting latest nonce from sequencer; retrying after backoff",
                );
                async move {}
            },
        );
    let res = tryhard::retry_fn(|| {
        let client = client.clone();
        let span = info_span!(parent: span.clone(), "attempt get latest nonce");
        async move { client.get_latest_nonce(address).await.map(|rsp| rsp.nonce) }.instrument(span)
    })
    .with_config(retry_config)
    .await
    .wrap_err("failed getting latest nonce from sequencer after 1024 attempts");

    state.set_sequencer_connected(res.is_ok());

    // metrics.record_nonce_fetch_latency(start.elapsed());

    res
}

async fn get_pending_nonce(
    client: sequencer_service_client::SequencerServiceClient<Channel>,
    address: Address,
    state: Arc<State>,
    // metrics: &'static Metrics,
) -> eyre::Result<u32> {
    debug!("fetching pending nonce from sequencing");
    // TODO: add metric and start time
    let span = Span::current();
    let retry_config = tryhard::RetryFutureConfig::new(1024)
        .exponential_backoff(Duration::from_millis(200))
        .max_delay(Duration::from_secs(60))
        .on_retry(
            |attempt, next_delay: Option<Duration>, err: &tonic::Status| {
                // TODO: update metrics here
                let state = Arc::clone(&state);
                state.set_sequencer_connected(false);

                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    error = err as &dyn std::error::Error,
                    attempt,
                    wait_duration,
                    "failed getting pending nonce from sequencing; retrying after backoff",
                );
                futures::future::ready(())
            },
        );

    let res = tryhard::retry_fn(|| {
        let mut client = client.clone();
        let span = info_span!(parent: span.clone(), "attempt get pending nonce");
        async move {
            client
                .get_pending_nonce(GetPendingNonceRequest {
                    address: Some(address.into_raw()),
                })
                .await
                .map(|rsp| rsp.into_inner().inner)
        }
        .instrument(span)
    })
    .with_config(retry_config)
    .await
    .wrap_err("failed getting pending nonce from sequencing after 1024 attempts");

    state.set_sequencer_connected(res.is_ok());

    // TODO: record latency metric

    res
}
