use std::{
    sync::Arc,
    time::Duration,
};

use astria_core::{
    bridge::Ics20WithdrawalFromRollupMemo,
    primitive::v1::asset,
    protocol::{
        asset::v1alpha1::AllowedFeeAssetIdsResponse,
        bridge::v1alpha1::BridgeAccountLastTxHashResponse,
        transaction::v1alpha1::Action,
    },
};
use astria_eyre::eyre::{
    self,
    ensure,
    eyre,
    Context as _,
    OptionExt as _,
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
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    info,
    instrument,
    warn,
};

use super::state::State;
use crate::bridge_withdrawer::ethereum::convert::BridgeUnlockMemo;

pub(super) struct Builder {
    pub(super) shutdown_token: CancellationToken,
    pub(super) state: Arc<State>,
    pub(super) sequencer_chain_id: String,
    pub(super) sequencer_cometbft_endpoint: String,
    pub(super) sequencer_key_path: String,
    pub(super) expected_fee_asset_id: asset::Id,
    // TODO: change the name of this config var
    pub(super) expected_min_fee_asset_balance: u128,
}

impl Builder {
    pub(super) fn build(self) -> eyre::Result<(Startup, SubmitterHandle, WatcherHandle)> {
        let Self {
            shutdown_token,
            state,
            sequencer_chain_id,
            sequencer_cometbft_endpoint,
            sequencer_key_path,
            expected_fee_asset_id,
            expected_min_fee_asset_balance,
        } = self;

        let signer = super::submitter::signer::SequencerKey::try_from_path(sequencer_key_path)
            .wrap_err("failed to load sequencer private ky")?;
        let address = signer.address;
        info!(address = %telemetry::display::hex(&address), "loaded sequencer signer");

        let sequencer_cometbft_client =
            sequencer_client::HttpClient::new(&*sequencer_cometbft_endpoint)
                .wrap_err("failed constructing cometbft http client")?;

        let (submitter_tx, submitter_rx) = oneshot::channel();
        let submitter_handle = SubmitterHandle::new(submitter_rx);

        let (watcher_tx, watcher_rx) = oneshot::channel();
        let watcher_handle = WatcherHandle::new(watcher_rx);

        let startup = Startup {
            shutdown_token,
            state,
            submitter_tx,
            watcher_tx,
            sequencer_chain_id,
            sequencer_cometbft_client,
            address,
            expected_fee_asset_id,
            expected_min_fee_asset_balance,
        };

        Ok((startup, submitter_handle, watcher_handle))
    }
}

#[derive(Debug)]
pub(super) struct SubmitterInfo {
    pub(super) sequencer_chain_id: String,
}

#[derive(Debug)]
pub(super) struct SubmitterHandle {
    info_rx: Option<oneshot::Receiver<SubmitterInfo>>,
}

impl SubmitterHandle {
    pub(super) fn new(info_rx: oneshot::Receiver<SubmitterInfo>) -> Self {
        Self {
            info_rx: Some(info_rx),
        }
    }

    pub(super) async fn recv(&mut self) -> eyre::Result<SubmitterInfo> {
        self.info_rx
            .take()
            .expect("startup info should only be taken once - this is a bug")
            .await
            .wrap_err("failed to get startup info from submitter. channel was dropped.")
    }
}

#[derive(Debug)]
pub(super) struct WatcherInfo {
    pub(super) fee_asset_id: asset::Id,
    pub(super) starting_rollup_height: u64,
}

#[derive(Debug)]
pub(super) struct WatcherHandle {
    info_rx: Option<oneshot::Receiver<WatcherInfo>>,
}

impl WatcherHandle {
    pub(super) fn new(info_rx: oneshot::Receiver<WatcherInfo>) -> Self {
        Self {
            info_rx: Some(info_rx),
        }
    }

    pub(super) async fn recv(&mut self) -> eyre::Result<WatcherInfo> {
        self.info_rx
            .take()
            .expect("startup info should only be taken once - this is a bug")
            .await
            .wrap_err("failed to get startup info from watcher. channel was dropped.")
    }
}

pub(super) struct Startup {
    shutdown_token: CancellationToken,
    state: Arc<State>,
    submitter_tx: oneshot::Sender<SubmitterInfo>,
    watcher_tx: oneshot::Sender<WatcherInfo>,
    sequencer_chain_id: String,
    sequencer_cometbft_client: sequencer_client::HttpClient,
    address: Address,
    expected_fee_asset_id: asset::Id,
    expected_min_fee_asset_balance: u128,
}

impl Startup {
    pub(super) async fn run(mut self) -> eyre::Result<()> {
        let shutdown_token = self.shutdown_token.clone();

        let startup_task = tokio::spawn(async {
            self.confirm_sequencer_config()
                .await
                .wrap_err("failed to confirm sequencer config")?;
            let starting_rollup_height = self
                .get_starting_rollup_height()
                .await
                .wrap_err("failed to get next rollup block height")?;

            // send the startup info to the submitter
            let submitter_info = SubmitterInfo {
                sequencer_chain_id: self.sequencer_chain_id.clone(),
            };

            let watcher_info = WatcherInfo {
                fee_asset_id: self.expected_fee_asset_id,
                starting_rollup_height,
            };

            self.submitter_tx
                .send(submitter_info)
                .map_err(|_submitter_info| eyre!("failed to send submitter info"))?;
            self.watcher_tx
                .send(watcher_info)
                .map_err(|_watcher_info| eyre!("failed to send watcher info"))?;

            Ok(())
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
    /// - `self.sequencer_key.address` has a sufficient balance of `self.fee_asset_id`
    ///
    /// # Errors
    ///
    /// - `self.chain_id` does not match the value returned from the sequencer node
    /// - `self.fee_asset_id` is not a valid fee asset on the sequencer node
    /// - `self.sequencer_key.address` does not have a sufficient balance of `self.fee_asset_id`.
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
                .fee_asset_ids
                .contains(&self.expected_fee_asset_id),
            "fee_asset_id provided in config is not a valid fee asset on the sequencer"
        );

        // confirm that the sequencer key has a sufficient balance of the fee asset
        let fee_asset_balances = get_latest_balance(
            self.sequencer_cometbft_client.clone(),
            self.state.clone(),
            self.address,
        )
        .await
        .wrap_err("failed to get latest balance")?;
        let fee_asset_balance = fee_asset_balances
            .balances
            .into_iter()
            .find(|balance| balance.denom.id() == self.expected_fee_asset_id)
            .ok_or_eyre("withdrawer's account does not have the minimum balance of the fee asset")?
            .balance;
        ensure!(
            fee_asset_balance >= self.expected_min_fee_asset_balance,
            "sequencer key does not have a sufficient balance of the fee asset"
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
            self.address,
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
            rollup_height_from_signed_transaction(&signed_transaction).wrap_err(
                "failed to extract rollup height from last transaction by the bridge account",
            )?
        } else {
            1
        };
        Ok(starting_rollup_height)
    }
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
    let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(20))
        .on_retry(
            |attempt: u32,
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
                    "attempt to fetch last bridge account's transaction hash; retrying after \
                     backoff",
                );
                futures::future::ready(())
            },
        );

    let res = tryhard::retry_fn(|| client.get_bridge_account_last_transaction_hash(address))
        .with_config(retry_config)
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
    let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(20))
        .on_retry(
            |attempt: u32, next_delay: Option<Duration>, error: &tendermint_rpc::Error| {
                let state = Arc::clone(&state);
                state.set_sequencer_connected(false);

                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    attempt,
                    wait_duration,
                    error = error as &dyn std::error::Error,
                    "attempt to get transaction from Sequencer; retrying after backoff",
                );
                futures::future::ready(())
            },
        );

    let res = tryhard::retry_fn(|| client.tx(tx_hash, false))
        .with_config(retry_config)
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
    use sequencer_client::Client as _;

    let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(20))
        .on_retry(
            |attempt: u32, next_delay: Option<Duration>, error: &tendermint_rpc::Error| {
                let state = Arc::clone(&state);
                state.set_sequencer_connected(false);

                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    attempt,
                    wait_duration,
                    error = error as &dyn std::error::Error,
                    "attempt to fetch sequencer genesis info; retrying after backoff",
                );
                futures::future::ready(())
            },
        );

    let genesis: tendermint::Genesis = tryhard::retry_fn(|| client.genesis())
        .with_config(retry_config)
        .await
        .wrap_err("failed to get genesis info from Sequencer after a lot of attempts")?;

    state.set_sequencer_connected(true);

    Ok(genesis.chain_id)
}

#[instrument(skip_all)]
async fn get_allowed_fee_asset_ids(
    client: sequencer_client::HttpClient,
    state: Arc<State>,
) -> eyre::Result<AllowedFeeAssetIdsResponse> {
    let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(20))
        .on_retry(
            |attempt: u32,
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
                    "attempt to fetch sequencer allowed fee asset ids; retrying after backoff",
                );
                futures::future::ready(())
            },
        );

    let res = tryhard::retry_fn(|| client.get_allowed_fee_asset_ids())
        .with_config(retry_config)
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
    let retry_config = tryhard::RetryFutureConfig::new(u32::MAX)
        .exponential_backoff(Duration::from_millis(100))
        .max_delay(Duration::from_secs(20))
        .on_retry(
            |attempt: u32,
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
                    "attempt to get latest balance; retrying after backoff",
                );
                futures::future::ready(())
            },
        );

    let res = tryhard::retry_fn(|| client.get_latest_balance(address))
        .with_config(retry_config)
        .await
        .wrap_err("failed to get latest balance from Sequencer after a lot of attempts");

    state.set_sequencer_connected(res.is_ok());

    res
}
