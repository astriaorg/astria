use std::{
    path::PathBuf,
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
use tokio::select;
use tracing::warn;

#[derive(Args, Debug)]
pub struct CollectWithdrawalEvents {
    #[arg(long)]
    rollup_endpoint: String,
    #[arg(long)]
    contract_address: ethers::types::Address,
    #[arg(long)]
    next_rollup_block_height: u64,
    #[arg(long, default_value = "nria")]
    fee_asset: asset::Denom,
    #[arg(long)]
    rollup_asset_denom: asset::Denom,
    #[arg(long)]
    bridge_address: Address,
    #[arg(long, default_value = "astria")]
    sequencer_address_prefix: String,
    #[arg(long, short)]
    output: PathBuf,
}

impl CollectWithdrawalEvents {
    pub(crate) async fn run(self) -> eyre::Result<()> {
        let Self {
            rollup_endpoint,
            contract_address,
            next_rollup_block_height,
            fee_asset,
            rollup_asset_denom,
            bridge_address,
            sequencer_address_prefix,
            output,
        } = self;

        let output_file = std::fs::File::options()
            .write(true)
            .create_new(true)
            .open(&output)
            .wrap_err("failed to open specified file for writing")?;

        let block_provider = connect_to_rollup(&rollup_endpoint)
            .await
            .wrap_err("failed to connect to rollup")?;

        let asset_withdrawal_divisor =
            get_asset_withdrawal_divisor(contract_address, block_provider.clone())
                .await
                .wrap_err("failed determining asset withdrawal divisor")?;

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

        let incoming_blocks =
            futures::stream::iter(next_rollup_block_height..current_rollup_block_height.as_u64())
                .then(|height| {
                    let block_provider = block_provider.clone();
                    async move {
                        block_provider
                            .get_block(height)
                            .await
                            .wrap_err("failed to get block")?
                            .ok_or_else(|| eyre!("block with number {height} missing"))
                    }
                })
                .chain(futures::stream::once(
                    async move { Ok(current_rollup_block) },
                ))
                .chain(block_subscription.map(Ok));

        tokio::pin!(incoming_blocks);

        let mut actions = Vec::new();
        loop {
            select! {
                biased;

                _ = tokio::signal::ctrl_c() => {
                    break;
                }

                block = incoming_blocks.next() => {
                    match block {
                        Some(Ok(block)) =>
                            actions.append(&mut BlockToActions {
                                block_provider: block_provider.clone(),
                                contract_address,
                                block,
                                fee_asset: fee_asset.clone(),
                                rollup_asset_denom: rollup_asset_denom.clone(),
                                bridge_address,
                                asset_withdrawal_divisor,
                                sequencer_address_prefix: sequencer_address_prefix.clone(),
                             }.run().await),
                        Some(Err(error)) => warn!(
                            error = AsRef::<dyn std::error::Error>::as_ref(&error),
                            "encountered an error getting block; skipping"
                        ),
                        None => bail!("block subscription ended"),
                    }
                }
            }
        }

        write_collected_actions(output_file, &actions).wrap_err("failed to write actions to file")
    }
}

fn write_collected_actions(output_file: std::fs::File, actions: &[Action]) -> eyre::Result<()> {
    let writer = std::io::BufWriter::new(output_file);
    serde_json::to_writer(writer, actions).wrap_err("failed writing actions to file")
}

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

async fn get_asset_withdrawal_divisor(
    address: ethers::types::Address,
    provider: Arc<Provider<Ws>>,
) -> eyre::Result<u128> {
    let contract = IAstriaWithdrawer::new(address, provider);

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
    sequencer_address_prefix: String,
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
            sequencer_address_prefix: self.sequencer_address_prefix.clone(),
        }
        .try_convert()
        .wrap_err("failed converting log to ics20 withdrawal action")
    }

    fn log_to_sequencer_withdrawal_action(&self, log: Log) -> eyre::Result<Action> {
        LogToSequencerWithdrawalAction {
            log,
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
    sequencer_address_prefix: String,
}

impl LogToIcs20WithdrawalAction {
    fn try_convert(self) -> eyre::Result<Action> {
        let Self {
            log,
            fee_asset,
            rollup_asset_denom,
            asset_withdrawal_divisor,
            bridge_address,
            sequencer_address_prefix,
        } = self;

        let (event, block_number, transaction_hash) =
            action_inputs_from_log::<Ics20WithdrawalFilter>(log)
                .wrap_err("failed getting required data from log")?;

        let sender = event.sender.to_fixed_bytes();

        let source_channel = rollup_asset_denom
            .as_trace_prefixed()
            .and_then(TracePrefixed::last_channel)
            .ok_or_eyre("rollup asset denom must have a channel to be withdrawn via IBC")?
            .parse()
            .wrap_err("failed to parse channel from rollup asset denom")?;

        let memo = Ics20WithdrawalFromRollupMemo {
            memo: event.memo,
            bridge_address,
            block_number,
            transaction_hash,
        };

        let action = Ics20Withdrawal {
            denom: rollup_asset_denom,
            destination_chain_address: event.destination_chain_address,
            // note: this is actually a rollup address; we expect failed ics20 withdrawals to be
            // returned to the rollup.
            // this is only ok for now because addresses on the sequencer and the rollup are both 20
            // bytes, but this won't work otherwise.
            return_address: Address::builder()
                .array(sender)
                .prefix(sequencer_address_prefix)
                .try_build()
                .wrap_err("failed to construct return address")?,
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
}

impl LogToSequencerWithdrawalAction {
    fn try_into_action(self) -> eyre::Result<Action> {
        let Self {
            log,
            fee_asset,
            asset_withdrawal_divisor,
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
