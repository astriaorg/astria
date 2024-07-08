use std::{
    sync::Arc,
    time::Duration,
};

use astria_core::{
    bridge::{
        self,
        Ics20WithdrawalFromRollupMemo,
    },
    primitive::v1::asset,
    protocol::{
        asset::v1alpha1::AllowedFeeAssetsResponse,
        bridge::v1alpha1::BridgeAccountLastTxHashResponse,
        transaction::v1alpha1::{
            Action,
            TransactionParams,
            UnsignedTransaction,
        },
    },
};
use astria_eyre::eyre::{
    self,
    ensure,
    eyre,
    Context,
    OptionExt,
};
pub(crate) use builder::Builder;
pub(super) use builder::Handle;
use prost::Message as _;
use sequencer_client::{
    tendermint_rpc::{
        self,
        endpoint::broadcast::tx_commit,
    },
    Address,
    BalanceResponse,
    SequencerClientExt,
    SignedTransaction,
};
use signer::SequencerKey;
use state::State;
use tendermint_rpc::{
    endpoint::tx,
    Client,
};
use tokio::{
    select,
    sync::{
        mpsc,
        oneshot::{
            self,
        },
    },
    time::Instant,
};
use tokio_util::sync::CancellationToken;
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

use super::{
    batch::Batch,
    state,
    SequencerStartupInfo,
};
use crate::metrics::Metrics;

mod builder;
mod signer;
#[cfg(test)]
mod tests;

pub(super) struct Submitter {
    shutdown_token: CancellationToken,
    state: Arc<State>,
    batches_rx: mpsc::Receiver<Batch>,
    sequencer_cometbft_client: sequencer_client::HttpClient,
    signer: SequencerKey,
    sequencer_chain_id: String,
    startup_tx: oneshot::Sender<SequencerStartupInfo>,
    expected_fee_asset: asset::Denom,
    min_expected_fee_asset_balance: u128,
    metrics: &'static Metrics,
}

impl Submitter {
    pub(super) async fn run(mut self) -> eyre::Result<()> {
        // call startup
        let startup = self
            .startup()
            .await
            .wrap_err("submitter failed to start up")?;
        self.startup_tx
            .send(startup)
            .map_err(|_startup| eyre!("failed to send startup info to watcher"))?;

        let reason = loop {
            select!(
                biased;

                () = self.shutdown_token.cancelled() => {
                    info!("received shutdown signal");
                    break Ok("shutdown requested");
                }

                batch = self.batches_rx.recv() => {
                    let Some(Batch { actions, rollup_height }) = batch else {
                        info!("received None from batch channel, shutting down");
                        break Err(eyre!("batch channel closed"));
                    };
                    // if batch submission fails, halt the submitter
                    if let Err(e) = process_batch(
                        self.sequencer_cometbft_client.clone(),
                        &self.signer,
                        self.state.clone(),
                        &self.sequencer_chain_id,
                        actions,
                        rollup_height,
                        self.metrics
                    ).await {
                        break Err(e);
                    }
                }
            );
        };

        // update status
        self.state.set_sequencer_connected(false);

        // close the channel to signal to batcher that the submitter is shutting down
        self.batches_rx.close();

        match reason {
            Ok(reason) => info!(reason, "submitter shutting down"),
            Err(reason) => {
                error!(%reason, "submitter shutting down");
            }
        }

        Ok(())
    }

    /// Confirms configuration values against the sequencer node and then syncs the next sequencer
    /// nonce and rollup block according to the latest on-chain state.
    ///
    /// Configuration values checked:
    /// - `self.chain_id` matches the value returned from the sequencer node's genesis
    /// - `self.fee_asset_id` is a valid fee asset on the sequencer node
    /// - `self.sequencer_key.address` has a sufficient balance of `self.fee_asset_id`
    ///
    /// Sync process:
    /// - Fetch the last transaction hash by the bridge account from the sequencer
    /// - Fetch the corresponding transaction
    /// - Extract the last nonce used from the transaction
    /// - Extract the rollup block height from the memo of one of the withdraw actions in the
    ///   transaction
    ///
    /// # Returns
    /// A struct with the information collected and validated during startup:
    /// - `fee_asset_id`
    /// - `next_batch_rollup_height`
    ///
    /// # Errors
    ///
    /// - `self.chain_id` does not match the value returned from the sequencer node
    /// - `self.fee_asset_id` is not a valid fee asset on the sequencer node
    /// - `self.sequencer_key.address` does not have a sufficient balance of `self.fee_asset_id`.
    async fn startup(&mut self) -> eyre::Result<SequencerStartupInfo> {
        let actual_chain_id =
            get_sequencer_chain_id(self.sequencer_cometbft_client.clone(), self.state.clone())
                .await
                .wrap_err("failed to get chain id from sequencer")?;
        ensure!(
            self.sequencer_chain_id == actual_chain_id.to_string(),
            "sequencer_chain_id provided in config does not match chain_id returned from sequencer"
        );

        let expected_fee_asset_ibc = self.expected_fee_asset.to_ibc_prefixed();
        // confirm that the fee asset ID is valid
        let allowed_fee_assets_resp =
            get_allowed_fee_assets(self.sequencer_cometbft_client.clone(), self.state.clone())
                .await
                .wrap_err("failed to get allowed fee asset ids from sequencer")?;
        ensure!(
            allowed_fee_assets_resp
                .fee_assets
                .iter()
                .any(|asset| asset.to_ibc_prefixed() == expected_fee_asset_ibc),
            "fee_asset_id provided in config is not a valid fee asset on the sequencer"
        );

        // confirm that the sequencer key has a sufficient balance of the fee asset
        let fee_asset_balances = get_latest_balance(
            self.sequencer_cometbft_client.clone(),
            self.state.clone(),
            *self.signer.address(),
        )
        .await
        .wrap_err("failed to get latest balance")?;
        let fee_asset_balance = fee_asset_balances
            .balances
            .into_iter()
            .find(|balance| balance.denom.to_ibc_prefixed() == expected_fee_asset_ibc)
            .ok_or_eyre("withdrawer's account does not have the minimum balance of the fee asset")?
            .balance;
        ensure!(
            fee_asset_balance >= self.min_expected_fee_asset_balance,
            "sequencer key does not have a sufficient balance of the fee asset"
        );

        // sync to latest on-chain state
        let next_batch_rollup_height = self
            .get_next_rollup_height()
            .await
            .wrap_err("failed to get next rollup block height")?;

        self.state.set_submitter_ready();

        // send startup info to watcher
        let startup = SequencerStartupInfo {
            fee_asset: self.expected_fee_asset.clone(),
            next_batch_rollup_height,
        };
        Ok(startup)
    }

    /// Gets the data necessary for syncing to the latest on-chain state from the sequencer. Since
    /// we batch all events from a given rollup block into a single sequencer transaction, we
    /// get the last tx finalized by the bridge account on the sequencer and extract the rollup
    /// height from it.
    ///
    /// The rollup height is extracted from the block height value in the memo of one of the actions
    /// in the batch.
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
    async fn get_next_rollup_height(&mut self) -> eyre::Result<u64> {
        let signed_transaction = self
            .get_last_transaction()
            .await
            .wrap_err("failed to get the bridge account's last sequencer transaction")?;
        let next_batch_rollup_height = if let Some(signed_transaction) = signed_transaction {
            rollup_height_from_signed_transaction(&signed_transaction).wrap_err(
                "failed to extract rollup height from last transaction by the bridge account",
            )?
        } else {
            1
        };
        Ok(next_batch_rollup_height)
    }

    async fn get_last_transaction(&self) -> eyre::Result<Option<SignedTransaction>> {
        // get last transaction hash by the bridge account, if it exists
        let last_transaction_hash_resp = get_bridge_account_last_transaction_hash(
            self.sequencer_cometbft_client.clone(),
            self.state.clone(),
            *self.signer.address(),
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
}

async fn process_batch(
    sequencer_cometbft_client: sequencer_client::HttpClient,
    sequencer_key: &SequencerKey,
    state: Arc<State>,
    sequencer_chain_id: &str,
    actions: Vec<Action>,
    rollup_height: u64,
    metrics: &'static Metrics,
) -> eyre::Result<()> {
    // get nonce and make unsigned transaction
    let nonce = get_latest_nonce(
        sequencer_cometbft_client.clone(),
        *sequencer_key.address(),
        state.clone(),
        metrics,
    )
    .await
    .wrap_err("failed to get nonce from sequencer")?;
    debug!(nonce, "fetched latest nonce");

    let unsigned = UnsignedTransaction {
        actions,
        params: TransactionParams::builder()
            .nonce(nonce)
            .chain_id(sequencer_chain_id)
            .build(),
    };

    // sign transaction
    let signed = unsigned.into_signed(sequencer_key.signing_key());
    debug!(tx_hash = %telemetry::display::hex(&signed.sha256_of_proto_encoding()), "signed transaction");

    // submit transaction and handle response
    let rsp = submit_tx(
        sequencer_cometbft_client.clone(),
        signed,
        state.clone(),
        metrics,
    )
    .await
    .context("failed to submit transaction to to cometbft")?;
    if let tendermint::abci::Code::Err(check_tx_code) = rsp.check_tx.code {
        error!(
            abci.code = check_tx_code,
            abci.log = rsp.check_tx.log,
            rollup.height = rollup_height,
            "transaction failed to be included in the mempool, aborting."
        );
        Err(eyre!(
            "check_tx failure upon submitting transaction to sequencer"
        ))
    } else if let tendermint::abci::Code::Err(deliver_tx_code) = rsp.tx_result.code {
        error!(
            abci.code = deliver_tx_code,
            abci.log = rsp.tx_result.log,
            rollup.height = rollup_height,
            "transaction failed to be executed in a block, aborting."
        );
        Err(eyre!(
            "deliver_tx failure upon submitting transaction to sequencer"
        ))
    } else {
        // update state after successful submission
        info!(
            sequencer.block = rsp.height.value(),
            sequencer.tx_hash = %rsp.hash,
            rollup.height = rollup_height,
            "withdraw batch successfully executed."
        );
        state.set_last_rollup_height_submitted(rollup_height);
        state.set_last_sequencer_height(rsp.height.value());
        state.set_last_sequencer_tx_hash(rsp.hash);
        Ok(())
    }
}

async fn get_latest_nonce(
    client: sequencer_client::HttpClient,
    address: Address,
    state: Arc<State>,
    metrics: &'static Metrics,
) -> eyre::Result<u32> {
    debug!("fetching latest nonce from sequencer");
    metrics.increment_nonce_fetch_count();
    let span = Span::current();
    let start = Instant::now();
    let retry_config = tryhard::RetryFutureConfig::new(1024)
        .exponential_backoff(Duration::from_millis(200))
        .max_delay(Duration::from_secs(60))
        .on_retry(
            |attempt,
             next_delay: Option<Duration>,
             err: &sequencer_client::extension_trait::Error| {
                metrics.increment_nonce_fetch_failure_count();

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
        let span = info_span!(parent: span.clone(), "attempt get nonce");
        async move { client.get_latest_nonce(address).await.map(|rsp| rsp.nonce) }.instrument(span)
    })
    .with_config(retry_config)
    .await
    .wrap_err("failed getting latest nonce from sequencer after 1024 attempts");

    state.set_sequencer_connected(res.is_ok());

    metrics.record_nonce_fetch_latency(start.elapsed());

    res
}

/// Submits a `SignedTransaction` to the sequencer with an exponential backoff
#[instrument(
    name = "submit_tx",
    skip_all,
    fields(
        nonce = tx.nonce(),
        transaction.hash = %telemetry::display::hex(&tx.sha256_of_proto_encoding()),
    )
)]
async fn submit_tx(
    client: sequencer_client::HttpClient,
    tx: SignedTransaction,
    state: Arc<State>,
    metrics: &'static Metrics,
) -> eyre::Result<tx_commit::Response> {
    let nonce = tx.nonce();
    metrics.set_current_nonce(nonce);
    let start = std::time::Instant::now();
    debug!("submitting signed transaction to sequencer");
    let span = Span::current();
    let retry_config = tryhard::RetryFutureConfig::new(1024)
        .exponential_backoff(Duration::from_millis(200))
        .max_delay(Duration::from_secs(60))
        .on_retry(
            |attempt,
             next_delay: Option<Duration>,
             err: &sequencer_client::extension_trait::Error| {
                metrics.increment_sequencer_submission_failure_count();

                let state = Arc::clone(&state);
                state.set_sequencer_connected(false);

                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    parent: span.clone(),
                    attempt,
                    wait_duration,
                    error = err as &dyn std::error::Error,
                    "failed sending transaction to sequencer; retrying after backoff",
                );
                async move {}
            },
        );
    let res = tryhard::retry_fn(|| {
        let client = client.clone();
        let tx = tx.clone();
        let span = info_span!(parent: span.clone(), "attempt send");
        async move { client.submit_transaction_commit(tx).await }.instrument(span)
    })
    .with_config(retry_config)
    .await
    .wrap_err("failed sending transaction after 1024 attempts");

    state.set_sequencer_connected(res.is_ok());

    metrics.record_sequencer_submission_latency(start.elapsed());

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
async fn get_allowed_fee_assets(
    client: sequencer_client::HttpClient,
    state: Arc<State>,
) -> eyre::Result<AllowedFeeAssetsResponse> {
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

    let res = tryhard::retry_fn(|| client.get_allowed_fee_assets())
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
            let memo: bridge::UnlockMemo = serde_json::from_slice(&action.memo)
                .wrap_err("failed to parse memo from last transaction by the bridge account")?;
            Some(memo.block_number)
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
