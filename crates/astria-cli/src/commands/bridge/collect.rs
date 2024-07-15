use std::{
    collections::BTreeMap,
    path::{
        Path,
        PathBuf,
    },
    sync::Arc,
    time::Duration,
};

use astria_bridge_contracts::i_astria_withdrawer::{
    IAstriaWithdrawer,
    Ics20WithdrawalFilter,
    SequencerWithdrawalFilter,
};
use astria_core::{
    bridge::{
        self,
        Ics20WithdrawalFromRollupMemo,
    },
    primitive::v1::{
        asset::{
            self,
            TracePrefixed,
        },
        Address,
    },
    protocol::transaction::v1alpha1::{
        action::{
            BridgeUnlockAction,
            Ics20Withdrawal,
        },
        Action,
    },
};
use clap::Args;
use color_eyre::eyre::{
    self,
    bail,
    ensure,
    eyre,
    OptionExt as _,
    WrapErr as _,
};
use ethers::{
    contract::EthEvent,
    core::types::Block,
    providers::{
        Middleware,
        Provider,
        ProviderError,
        StreamExt as _,
        Ws,
    },
    types::{
        Filter,
        Log,
        H256,
    },
};
use futures::stream::BoxStream;
use tracing::{
    error,
    info,
    instrument,
    warn,
};

#[derive(Args, Debug)]
pub(crate) struct WithdrawalEvents {
    /// The websocket endpoint of a geth compatible rollup.
    #[arg(long)]
    rollup_endpoint: String,
    /// The eth address of the astria bridge contracts.
    #[arg(long)]
    contract_address: ethers::types::Address,
    /// The start rollup height from which blocks will be checked for withdrawal events.
    #[arg(long)]
    from_rollup_height: u64,
    /// The end rollup height from which blocks will be checked for withdrawal events.
    /// If not set, then this tool will stream blocks until SIGINT is received.
    #[arg(long)]
    to_rollup_height: Option<u64>,
    /// The asset that will be used to pay the Sequencer fees (should the generated
    /// actions be submitted to the Sequencer).
    #[arg(long, default_value = "nria")]
    fee_asset: asset::Denom,
    /// The asset denomination of the asset that's withdrawn from the bridge.
    #[arg(long)]
    rollup_asset_denom: asset::Denom,
    /// The bech32-encoded bridge address corresponding to the bridged rollup
    ///  asset on the sequencer. Should match the bridge address in the geth
    /// rollup's bridge configuration for that asset.
    #[arg(long)]
    bridge_address: Address,
    /// The path to write the collected withdrawal events converted
    /// to Sequencer actions.
    #[arg(long, short)]
    output: PathBuf,
}

impl WithdrawalEvents {
    pub(crate) async fn run(self) -> eyre::Result<()> {
        let Self {
            rollup_endpoint,
            contract_address,
            from_rollup_height,
            to_rollup_height,
            fee_asset,
            rollup_asset_denom,
            bridge_address,
            output,
        } = self;

        let output = open_output(&output).wrap_err("failed to open output for writing")?;

        let block_provider = connect_to_rollup(&rollup_endpoint)
            .await
            .wrap_err("failed to connect to rollup")?;

        let asset_withdrawal_divisor =
            get_asset_withdrawal_divisor(contract_address, block_provider.clone())
                .await
                .wrap_err("failed determining asset withdrawal divisor")?;

        let mut incoming_blocks =
            create_stream_of_blocks(&block_provider, from_rollup_height, to_rollup_height)
                .await
                .wrap_err("failed initializing stream of rollup blocks")?;

        let mut actions_by_rollup_height = ActionsByRollupHeight::new();
        loop {
            tokio::select! {
                biased;

                _ = tokio::signal::ctrl_c() => {
                    break;
                }

                block = incoming_blocks.next() => {
                    match block {
                        Some(Ok(block)) =>
                            if let Err(err) = actions_by_rollup_height.convert_and_insert(BlockToActions {
                                block_provider: block_provider.clone(),
                                contract_address,
                                block,
                                fee_asset: fee_asset.clone(),
                                rollup_asset_denom: rollup_asset_denom.clone(),
                                bridge_address,
                                asset_withdrawal_divisor,
                             }).await {
                                 error!(
                                     err = AsRef::<dyn std::error::Error>::as_ref(&err),
                                     "failed converting contract block to Sequencer actions and storing them; exiting stream");
                                 break;
                             }
                        Some(Err(error)) => {
                            error!(
                                error = AsRef::<dyn std::error::Error>::as_ref(&error),
                                "encountered an error getting block; exiting stream",
                            );
                            break;
                        },
                        None => {
                            info!("block subscription ended");
                            break;
                        }
                    }
                }
            }
        }

        actions_by_rollup_height
            .write_to_output(output)
            .wrap_err("failed to write actions to file")
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub(crate) struct ActionsByRollupHeight(BTreeMap<u64, Vec<Action>>);

impl ActionsByRollupHeight {
    fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub(crate) fn into_inner(self) -> BTreeMap<u64, Vec<Action>> {
        self.0
    }

    #[instrument(skip_all, err)]
    async fn convert_and_insert(&mut self, block_to_actions: BlockToActions) -> eyre::Result<()> {
        let rollup_height = block_to_actions
            .block
            .number
            .ok_or_eyre("block was missing a number")?
            .as_u64();
        let actions = block_to_actions.run().await;
        ensure!(
            self.0.insert(rollup_height, actions).is_none(),
            "already collected actions for block at rollup height `{rollup_height}`; no 2 blocks \
             with the same height should have been seen",
        );
        Ok(())
    }

    #[instrument(skip_all, fields(target = %output.path.display()), err)]
    fn write_to_output(self, output: Output) -> eyre::Result<()> {
        let writer = std::io::BufWriter::new(output.handle);
        serde_json::to_writer(writer, &self.0).wrap_err("failed writing actions to file")
    }
}

/// Constructs a block stream from `start` until `maybe_end`, if `Some`.
/// Constructs an open ended stream from `start` if `None`.
#[instrument(skip_all, fields(start, end = maybe_end), err)]
async fn create_stream_of_blocks(
    block_provider: &Provider<Ws>,
    start: u64,
    maybe_end: Option<u64>,
) -> eyre::Result<BoxStream<'_, eyre::Result<Block<H256>>>> {
    let subscription = if let Some(end) = maybe_end {
        futures::stream::iter(start..=end)
            .then(move |height| async move {
                block_provider
                    .get_block(height)
                    .await
                    .wrap_err("failed to get block")?
                    .ok_or_else(|| eyre!("block with number {height} missing"))
            })
            .boxed()
    } else {
        let mut block_subscription = block_provider
            .subscribe_blocks()
            .await
            .wrap_err("failed to subscribe to blocks from rollup")?
            .boxed();

        let Some(current_rollup_block) = block_subscription.next().await else {
            bail!("failed to get current rollup block from subscription")
        };

        let Some(current_rollup_block_height) = current_rollup_block.number else {
            bail!(
                "couldn't determine current rollup block height; value was not set on current on \
                 most recent block",
            );
        };

        futures::stream::iter(start..current_rollup_block_height.as_u64())
            .then(move |height| async move {
                block_provider
                    .get_block(height)
                    .await
                    .wrap_err("failed to get block")?
                    .ok_or_else(|| eyre!("block with number {height} missing"))
            })
            .chain(futures::stream::once(
                async move { Ok(current_rollup_block) },
            ))
            .chain(block_subscription.map(Ok))
            .boxed()
    };
    Ok(subscription)
}

#[derive(Debug)]
struct Output {
    handle: std::fs::File,
    path: PathBuf,
}

#[instrument(skip_all, fields(target = %target.as_ref().display()), err)]
fn open_output<P: AsRef<Path>>(target: P) -> eyre::Result<Output> {
    let handle = std::fs::File::options()
        .write(true)
        .create_new(true)
        .open(&target)
        .wrap_err("failed to open specified fil}e for writing")?;
    Ok(Output {
        handle,
        path: target.as_ref().to_path_buf(),
    })
}

#[instrument(err)]
async fn connect_to_rollup(rollup_endpoint: &str) -> eyre::Result<Arc<Provider<Ws>>> {
    let retry_config = tryhard::RetryFutureConfig::new(10)
        .fixed_backoff(Duration::from_secs(2))
        .on_retry(
            |attempt, next_delay: Option<Duration>, error: &ProviderError| {
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    attempt,
                    wait_duration,
                    error = error as &dyn std::error::Error,
                    "attempt to connect to rollup node failed; retrying after backoff",
                );
                std::future::ready(())
            },
        );

    let provider = tryhard::retry_fn(|| Provider::<Ws>::connect(rollup_endpoint))
        .with_config(retry_config)
        .await
        .wrap_err("failed connecting to rollup after several retries; giving up")?;
    Ok(Arc::new(provider))
}

#[instrument(skip_all, fields(%contract_address), err(Display))]
async fn get_asset_withdrawal_divisor(
    contract_address: ethers::types::Address,
    provider: Arc<Provider<Ws>>,
) -> eyre::Result<u128> {
    let contract = IAstriaWithdrawer::new(contract_address, provider);

    let base_chain_asset_precision = contract
        .base_chain_asset_precision()
        .call()
        .await
        .wrap_err("failed to get asset withdrawal decimals")?;

    let exponent = 18u32.checked_sub(base_chain_asset_precision).ok_or_eyre(
        "failed calculating asset divisor. The base chain asset precision should be <= 18 as \
         that's enforced by the contract, so the construction should work. Did the precision \
         change?",
    )?;
    Ok(10u128.pow(exponent))
}

fn packet_timeout_time() -> eyre::Result<u64> {
    tendermint::Time::now()
        .checked_add(Duration::from_secs(300))
        .ok_or_eyre("adding 5 minutes to current time caused overflow")?
        .unix_timestamp_nanos()
        .try_into()
        .wrap_err("failed to i128 nanoseconds to u64")
}

struct BlockToActions {
    block_provider: Arc<Provider<Ws>>,
    contract_address: ethers::types::Address,
    block: Block<H256>,
    fee_asset: asset::Denom,
    rollup_asset_denom: asset::Denom,
    bridge_address: Address,
    asset_withdrawal_divisor: u128,
}

impl BlockToActions {
    async fn run(self) -> Vec<Action> {
        let mut actions = Vec::new();

        let Some(block_hash) = self.block.hash else {
            warn!("block hash missing; skipping");
            return actions;
        };

        match get_log::<SequencerWithdrawalFilter>(
            self.block_provider.clone(),
            self.contract_address,
            block_hash,
        )
        .await
        {
            Err(error) => warn!(
                error = AsRef::<dyn std::error::Error>::as_ref(&error),
                "encountered an error getting logs for sequencer withdrawal events",
            ),
            Ok(logs) => {
                for log in logs {
                    match self.log_to_sequencer_withdrawal_action(log) {
                        Ok(action) => actions.push(action),
                        Err(error) => {
                            warn!(
                                error = AsRef::<dyn std::error::Error>::as_ref(&error),
                                "failed converting ethers contract log to sequencer withdrawal \
                                 action; skipping"
                            );
                        }
                    }
                }
            }
        }
        match get_log::<Ics20WithdrawalFilter>(
            self.block_provider.clone(),
            self.contract_address,
            block_hash,
        )
        .await
        {
            Err(error) => warn!(
                error = AsRef::<dyn std::error::Error>::as_ref(&error),
                "encountered an error getting logs for ics20 withdrawal events",
            ),
            Ok(logs) => {
                for log in logs {
                    match self.log_to_ics20_withdrawal_action(log) {
                        Ok(action) => actions.push(action),
                        Err(error) => {
                            warn!(
                                error = AsRef::<dyn std::error::Error>::as_ref(&error),
                                "failed converting ethers contract log to ics20 withdrawal \
                                 action; skipping"
                            );
                        }
                    }
                }
            }
        }
        actions
    }

    fn log_to_ics20_withdrawal_action(&self, log: Log) -> eyre::Result<Action> {
        LogToIcs20WithdrawalAction {
            log,
            fee_asset: self.fee_asset.clone(),
            rollup_asset_denom: self.rollup_asset_denom.clone(),
            asset_withdrawal_divisor: self.asset_withdrawal_divisor,
            bridge_address: self.bridge_address,
        }
        .try_convert()
        .wrap_err("failed converting log to ics20 withdrawal action")
    }

    fn log_to_sequencer_withdrawal_action(&self, log: Log) -> eyre::Result<Action> {
        LogToSequencerWithdrawalAction {
            log,
            bridge_address: self.bridge_address,
            fee_asset: self.fee_asset.clone(),
            asset_withdrawal_divisor: self.asset_withdrawal_divisor,
        }
        .try_into_action()
        .wrap_err("failed converting log to sequencer withdrawal action")
    }
}

fn action_inputs_from_log<T: EthEvent>(log: Log) -> eyre::Result<(T, u64, [u8; 32])> {
    let block_number = log
        .block_number
        .ok_or_eyre("log did not contain block number")?
        .as_u64();
    let transaction_hash = log
        .transaction_hash
        .ok_or_eyre("log did not contain transaction hash")?
        .into();

    let event = T::decode_log(&log.into())
        .wrap_err_with(|| format!("failed decoding contract log as `{}`", T::name()))?;
    Ok((event, block_number, transaction_hash))
}

#[derive(Debug)]
struct LogToIcs20WithdrawalAction {
    log: Log,
    fee_asset: asset::Denom,
    rollup_asset_denom: asset::Denom,
    asset_withdrawal_divisor: u128,
    bridge_address: Address,
}

impl LogToIcs20WithdrawalAction {
    fn try_convert(self) -> eyre::Result<Action> {
        let Self {
            log,
            fee_asset,
            rollup_asset_denom,
            asset_withdrawal_divisor,
            bridge_address,
        } = self;

        let (event, block_number, transaction_hash) =
            action_inputs_from_log::<Ics20WithdrawalFilter>(log)
                .wrap_err("failed getting required data from log")?;

        let source_channel = rollup_asset_denom
            .as_trace_prefixed()
            .and_then(TracePrefixed::last_channel)
            .ok_or_eyre("rollup asset denom must have a channel to be withdrawn via IBC")?
            .parse()
            .wrap_err("failed to parse channel from rollup asset denom")?;

        let memo = Ics20WithdrawalFromRollupMemo {
            memo: event.memo,
            block_number,
            rollup_return_address: event.sender.to_string(),
            transaction_hash,
        };

        let action = Ics20Withdrawal {
            denom: rollup_asset_denom,
            destination_chain_address: event.destination_chain_address,
            // note: this is actually a rollup address; we expect failed ics20 withdrawals to be
            // returned to the rollup.
            // this is only ok for now because addresses on the sequencer and the rollup are both 20
            // bytes, but this won't work otherwise.
            return_address: bridge_address,
            amount: event
                .amount
                .as_u128()
                .checked_div(asset_withdrawal_divisor)
                .ok_or(eyre::eyre!(
                    "failed to divide amount by asset withdrawal multiplier"
                ))?,
            memo: serde_json::to_string(&memo).wrap_err("failed to serialize memo to json")?,
            fee_asset,
            // note: this refers to the timeout on the destination chain, which we are unaware of.
            // thus, we set it to the maximum possible value.
            timeout_height: ibc_types::core::client::Height::new(u64::MAX, u64::MAX)
                .wrap_err("failed to generate timeout height")?,
            timeout_time: packet_timeout_time()
                .wrap_err("failed to calculate packet timeout time")?,
            source_channel,
            bridge_address: Some(bridge_address),
        };
        Ok(Action::Ics20Withdrawal(action))
    }
}

#[derive(Debug)]
struct LogToSequencerWithdrawalAction {
    log: Log,
    fee_asset: asset::Denom,
    asset_withdrawal_divisor: u128,
    bridge_address: Address,
}

impl LogToSequencerWithdrawalAction {
    fn try_into_action(self) -> eyre::Result<Action> {
        let Self {
            log,
            fee_asset,
            asset_withdrawal_divisor,
            bridge_address,
        } = self;
        let (event, block_number, transaction_hash) =
            action_inputs_from_log::<SequencerWithdrawalFilter>(log)
                .wrap_err("failed getting required data from log")?;

        let memo = bridge::UnlockMemo {
            block_number,
            transaction_hash,
        };

        let action = BridgeUnlockAction {
            to: event
                .destination_chain_address
                .parse()
                .wrap_err("failed to parse destination chain address")?,
            amount: event
                .amount
                .as_u128()
                .checked_div(asset_withdrawal_divisor)
                .ok_or_eyre("failed to divide amount by asset withdrawal multiplier")?,
            memo: serde_json::to_string(&memo).wrap_err("failed to serialize memo to json")?,
            fee_asset,
            bridge_address: Some(bridge_address),
        };

        Ok(Action::BridgeUnlock(action))
    }
}

async fn get_log<T: EthEvent>(
    provider: Arc<Provider<Ws>>,
    contract_address: ethers::types::Address,
    block_hash: H256,
) -> eyre::Result<Vec<Log>> {
    let event_sig = T::signature();
    let filter = Filter::new()
        .at_block_hash(block_hash)
        .address(contract_address)
        .topic0(event_sig);

    provider
        .get_logs(&filter)
        .await
        .wrap_err("failed to get sequencer withdrawal events")
}
