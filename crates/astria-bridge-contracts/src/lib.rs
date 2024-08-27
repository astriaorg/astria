#[rustfmt::skip]
#[allow(clippy::pedantic)]
mod generated;
use std::{
    borrow::Cow,
    sync::Arc,
};

use astria_core::{
    primitive::v1::{
        asset,
        Address,
        AddressError,
    },
    protocol::{
        memos,
        transaction::v1alpha1::{
            action::Ics20Withdrawal,
            Action,
        },
    },
};
use astria_withdrawer::{
    Ics20WithdrawalFilter,
    SequencerWithdrawalFilter,
};
use ethers::{
    abi::AbiEncode as _,
    contract::EthEvent,
    providers::Middleware,
    types::{
        Filter,
        Log,
        H256,
    },
};
pub use generated::*;

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct BuildError(BuildErrorKind);

impl BuildError {
    #[must_use]
    fn bad_divisor(base_chain_asset_precision: u32) -> Self {
        Self(BuildErrorKind::BadDivisor {
            base_chain_asset_precision,
        })
    }

    #[must_use]
    fn call_base_chain_asset_precision<
        T: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    >(
        source: T,
    ) -> Self {
        Self(BuildErrorKind::CallBaseChainAssetPrecision {
            source: source.into(),
        })
    }

    #[must_use]
    pub fn no_withdraws_configured() -> Self {
        Self(BuildErrorKind::NoWithdrawsConfigured)
    }

    #[must_use]
    fn not_set(field: &'static str) -> Self {
        Self(BuildErrorKind::NotSet {
            field,
        })
    }

    #[must_use]
    fn ics20_asset_without_channel() -> Self {
        Self(BuildErrorKind::Ics20AssetWithoutChannel)
    }

    #[must_use]
    fn parse_ics20_asset_source_channel(source: ibc_types::IdentifierError) -> Self {
        Self(BuildErrorKind::ParseIcs20AssetSourceChannel {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum BuildErrorKind {
    #[error(
        "failed calculating asset divisor. The base chain asset precision should be <= 18 as \
         that's enforced by the contract, so the construction should work. Did the precision \
         change? Precision returned by contract: `{base_chain_asset_precision}`"
    )]
    BadDivisor { base_chain_asset_precision: u32 },
    #[error("required option `{field}` not set")]
    NotSet { field: &'static str },
    #[error(
        "getting withdraws actions must be configured for one of sequencer or ics20 (or both); \
         neither was set"
    )]
    NoWithdrawsConfigured,
    #[error("failed to call the `BASE_CHAIN_ASSET_PRECISION` of the provided contract")]
    CallBaseChainAssetPrecision {
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
    #[error("ics20 asset must have a channel to be withdrawn via IBC")]
    Ics20AssetWithoutChannel,
    #[error("could not parse ics20 asset channel as channel ID")]
    ParseIcs20AssetSourceChannel { source: ibc_types::IdentifierError },
}

pub struct NoProvider;
pub struct WithProvider<P>(Arc<P>);

pub struct GetWithdrawalActionsBuilder<TProvider = NoProvider> {
    provider: TProvider,
    contract_address: Option<ethers::types::Address>,
    bridge_address: Option<Address>,
    fee_asset: Option<asset::Denom>,
    sequencer_asset_to_withdraw: Option<asset::Denom>,
    ics20_asset_to_withdraw: Option<asset::TracePrefixed>,
}

impl Default for GetWithdrawalActionsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GetWithdrawalActionsBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            provider: NoProvider,
            contract_address: None,
            bridge_address: None,
            fee_asset: None,
            sequencer_asset_to_withdraw: None,
            ics20_asset_to_withdraw: None,
        }
    }
}

impl<P> GetWithdrawalActionsBuilder<P> {
    #[must_use]
    pub fn provider<Q>(self, provider: Arc<Q>) -> GetWithdrawalActionsBuilder<WithProvider<Q>> {
        let Self {
            contract_address,
            bridge_address,
            fee_asset,
            sequencer_asset_to_withdraw,
            ics20_asset_to_withdraw,
            ..
        } = self;
        GetWithdrawalActionsBuilder {
            provider: WithProvider(provider),
            contract_address,
            bridge_address,
            fee_asset,
            sequencer_asset_to_withdraw,
            ics20_asset_to_withdraw,
        }
    }

    #[must_use]
    pub fn contract_address(self, contract_address: ethers::types::Address) -> Self {
        Self {
            contract_address: Some(contract_address),
            ..self
        }
    }

    #[must_use]
    pub fn bridge_address(self, bridge_address: Address) -> Self {
        Self {
            bridge_address: Some(bridge_address),
            ..self
        }
    }

    #[must_use]
    pub fn fee_asset(self, fee_asset: asset::Denom) -> Self {
        Self {
            fee_asset: Some(fee_asset),
            ..self
        }
    }

    #[must_use]
    pub fn sequencer_asset_to_withdraw(self, sequencer_asset_to_withdraw: asset::Denom) -> Self {
        self.set_sequencer_asset_to_withdraw(Some(sequencer_asset_to_withdraw))
    }

    #[must_use]
    pub fn set_sequencer_asset_to_withdraw(
        self,
        sequencer_asset_to_withdraw: Option<asset::Denom>,
    ) -> Self {
        Self {
            sequencer_asset_to_withdraw,
            ..self
        }
    }

    #[must_use]
    pub fn ics20_asset_to_withdraw(self, ics20_asset_to_withdraw: asset::TracePrefixed) -> Self {
        self.set_ics20_asset_to_withdraw(Some(ics20_asset_to_withdraw))
    }

    #[must_use]
    pub fn set_ics20_asset_to_withdraw(
        self,
        ics20_asset_to_withdraw: Option<asset::TracePrefixed>,
    ) -> Self {
        Self {
            ics20_asset_to_withdraw,
            ..self
        }
    }
}

impl<P> GetWithdrawalActionsBuilder<WithProvider<P>>
where
    P: Middleware + 'static,
    P::Error: std::error::Error + 'static,
{
    /// Constructs a [`GetWithdrawalActions`] fetcher.
    ///
    /// # Errors
    /// Returns an error in one of these cases:
    /// + `contract_address` is not set
    /// + `bridge_address` is not set
    /// + `fee_asset` is not set
    /// + neither `source_asset_to_withdraw` nor `ics20_asset_to_withdraw` are set
    /// + `ics20_asset_to_withdraw` is set, but does not contain a ics20 channel
    /// + the `BASE_CHAIN_ASSET_PRECISION` call on the provided `contract_address` cannot be
    ///   executed
    /// + the base chain asset precision retrieved from the contract at `contract_address` is
    ///   greater than 18 (this is currently hardcoded in the smart contract).
    pub async fn try_build(self) -> Result<GetWithdrawalActions<P>, BuildError> {
        let Self {
            provider: WithProvider(provider),
            contract_address,
            bridge_address,
            fee_asset,
            sequencer_asset_to_withdraw,
            ics20_asset_to_withdraw,
        } = self;

        let Some(contract_address) = contract_address else {
            return Err(BuildError::not_set("contract_address"));
        };
        let Some(bridge_address) = bridge_address else {
            return Err(BuildError::not_set("bridge_address"));
        };
        let Some(fee_asset) = fee_asset else {
            return Err(BuildError::not_set("fee_asset"));
        };

        if sequencer_asset_to_withdraw.is_none() && ics20_asset_to_withdraw.is_none() {
            return Err(BuildError::no_withdraws_configured());
        }

        let mut ics20_source_channel = None;
        if let Some(ics20_asset_to_withdraw) = &ics20_asset_to_withdraw {
            ics20_source_channel.replace(
                ics20_asset_to_withdraw
                    .last_channel()
                    .ok_or(BuildError::ics20_asset_without_channel())?
                    .parse()
                    .map_err(BuildError::parse_ics20_asset_source_channel)?,
            );
        };

        let contract =
            i_astria_withdrawer::IAstriaWithdrawer::new(contract_address, provider.clone());

        let base_chain_asset_precision = contract
            .base_chain_asset_precision()
            .call()
            .await
            .map_err(BuildError::call_base_chain_asset_precision)?;

        let exponent = 18u32
            .checked_sub(base_chain_asset_precision)
            .ok_or_else(|| BuildError::bad_divisor(base_chain_asset_precision))?;

        let asset_withdrawal_divisor = 10u128.pow(exponent);

        Ok(GetWithdrawalActions {
            provider,
            contract_address,
            asset_withdrawal_divisor,
            bridge_address,
            fee_asset,
            sequencer_asset_to_withdraw,
            ics20_asset_to_withdraw,
            ics20_source_channel,
        })
    }
}

pub struct GetWithdrawalActions<P> {
    provider: Arc<P>,
    contract_address: ethers::types::Address,
    asset_withdrawal_divisor: u128,
    bridge_address: Address,
    fee_asset: asset::Denom,
    sequencer_asset_to_withdraw: Option<asset::Denom>,
    ics20_asset_to_withdraw: Option<asset::TracePrefixed>,
    ics20_source_channel: Option<ibc_types::core::channel::ChannelId>,
}

impl<P> GetWithdrawalActions<P>
where
    P: Middleware,
    P::Error: std::error::Error + 'static,
{
    fn configured_for_sequencer_withdrawals(&self) -> bool {
        self.sequencer_asset_to_withdraw.is_some()
    }

    fn configured_for_ics20_withdrawals(&self) -> bool {
        self.ics20_asset_to_withdraw.is_some()
    }

    /// Gets all withdrawal events for `block_hash` and converts them to astria sequencer actions.
    ///
    /// # Errors
    /// Returns an error in one of the following cases:
    /// + fetching logs for either ics20 or sequencer withdrawal events fails
    /// + converting either event to Sequencer actions fails due to the events being malformed.
    pub async fn get_for_block_hash(
        &self,
        block_hash: H256,
    ) -> Result<LogsToActionsConverter, GetWithdrawalLogsError> {
        use futures::FutureExt as _;
        let get_ics20_logs = if self.configured_for_ics20_withdrawals() {
            get_logs::<Ics20WithdrawalFilter, _>(&self.provider, self.contract_address, block_hash)
                .boxed()
        } else {
            futures::future::ready(Ok(vec![])).boxed()
        };
        let get_sequencer_logs = if self.configured_for_sequencer_withdrawals() {
            get_logs::<SequencerWithdrawalFilter, _>(
                &self.provider,
                self.contract_address,
                block_hash,
            )
            .boxed()
        } else {
            futures::future::ready(Ok(vec![])).boxed()
        };
        let (ics20_logs, sequencer_logs) =
            futures::future::try_join(get_ics20_logs, get_sequencer_logs)
                .await
                .map_err(GetWithdrawalLogsError::get_logs)?;

        Ok(LogsToActionsConverter {
            ics20_logs,
            sequencer_logs,
            asset_withdrawal_divisor: self.asset_withdrawal_divisor,
            bridge_address: self.bridge_address,
            fee_asset: self.fee_asset.clone(),
            ics20_asset_to_withdraw: self.ics20_asset_to_withdraw.clone(),
            ics20_source_channel: self.ics20_source_channel.clone(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct GetWithdrawalLogsError(GetWithdrawalLogsErrorKind);

impl GetWithdrawalLogsError {
    fn get_logs(source: GetLogsError) -> Self {
        Self(GetWithdrawalLogsErrorKind::GetLogs(source))
    }
}

#[derive(Debug, thiserror::Error)]
enum GetWithdrawalLogsErrorKind {
    #[error(transparent)]
    GetLogs(GetLogsError),
}

#[derive(Debug, thiserror::Error)]
#[error("failed getting the eth logs for event `{event_name}`")]
struct GetLogsError {
    event_name: Cow<'static, str>,
    // use a trait object instead of the error to not force the middleware
    // type parameter into the error.
    source: Box<dyn std::error::Error + Send + Sync + 'static>,
}

async fn get_logs<T: EthEvent, M>(
    provider: &M,
    contract_address: ethers::types::Address,
    block_hash: H256,
) -> Result<Vec<Log>, GetLogsError>
where
    M: Middleware,
    M::Error: std::error::Error + 'static,
{
    let event_sig = T::signature();
    let filter = Filter::new()
        .at_block_hash(block_hash)
        .address(contract_address)
        .topic0(event_sig);

    provider
        .get_logs(&filter)
        .await
        .map_err(|err| GetLogsError {
            event_name: T::name(),
            source: err.into(),
        })
}

pub struct LogsToActionsConverter {
    ics20_logs: Vec<Log>,
    sequencer_logs: Vec<Log>,
    asset_withdrawal_divisor: u128,
    bridge_address: Address,
    fee_asset: asset::Denom,
    ics20_asset_to_withdraw: Option<asset::TracePrefixed>,
    ics20_source_channel: Option<ibc_types::core::channel::ChannelId>,
}

impl LogsToActionsConverter {
    /// Converts all logs to withdrawal actions. Returns a
    ///
    /// # Panics
    /// - Panics if collected ICS20 withdrawal events without being configured for ICS20
    ///   withdrawals.
    #[must_use]
    pub fn convert_logs_to_actions(self) -> Vec<Result<Action, WithdrawalConversionError>> {
        let Self {
            ics20_logs,
            sequencer_logs,
            asset_withdrawal_divisor,
            bridge_address,
            fee_asset,
            ics20_asset_to_withdraw,
            ics20_source_channel,
        } = self;
        // XXX: The calls to `log_to_*_action` rely on only be called if `GetWithdrawalActions`
        // is configured for either ics20 or sequencer withdrawals (or both). They would panic
        // otherwise.
        ics20_logs
            .into_iter()
            .map(|log| {
                log_to_ics20_withdrawal_action(
                    log,
                    asset_withdrawal_divisor,
                    bridge_address,
                    &fee_asset,
                    ics20_asset_to_withdraw
                        .clone()
                        .expect("ics20_asset_to_withdraw must be configured for ics20 withdrawals"),
                    ics20_source_channel
                        .clone()
                        .expect("ics20_source_channel must be configured for ics20 withdrawals"),
                )
            })
            .chain(sequencer_logs.into_iter().map(|log| {
                log_to_sequencer_withdrawal_action(
                    log,
                    asset_withdrawal_divisor,
                    bridge_address,
                    &fee_asset,
                )
            }))
            .collect()
    }
}

/// Converts a rollup-side smart contract event log to a sequencer-side ics20 withdrawal action.
///
/// # Errors
/// - If the log does not contain a block number.
/// - If the log does not contain a transaction hash.
/// - If the log cannot be decoded as a sequencer withdrawal event.
/// - If the log does not contain a `recipient` field.
/// - If the memo cannot be encoded to json.
/// - If calculating the amount using the asset withdrawal divisor overflows.
pub fn log_to_ics20_withdrawal_action(
    log: Log,
    asset_withdrawal_divisor: u128,
    bridge_address: Address,
    fee_asset: &asset::Denom,
    asset_to_withdraw: asset::TracePrefixed,
    source_channel: ibc_types::core::channel::ChannelId,
) -> Result<Action, WithdrawalConversionError> {
    let rollup_block_number = log
        .block_number
        .ok_or_else(|| WithdrawalConversionError::log_without_block_number(&log))?
        .as_u64();

    let rollup_transaction_hash = log
        .transaction_hash
        .ok_or_else(|| WithdrawalConversionError::log_without_transaction_hash(&log))?
        .encode_hex();

    let event =
        decode_log::<Ics20WithdrawalFilter>(log).map_err(WithdrawalConversionError::decode_log)?;

    let (denom, source_channel) = (asset_to_withdraw.into(), source_channel);

    let memo = memo_to_json(&memos::v1alpha1::Ics20WithdrawalFromRollup {
        memo: event.memo.clone(),
        rollup_block_number,
        rollup_return_address: event.sender.to_string(),
        rollup_transaction_hash,
    })
    .map_err(WithdrawalConversionError::encode_memo)?;

    let amount = calculate_amount(&event, asset_withdrawal_divisor)
        .map_err(WithdrawalConversionError::calculate_withdrawal_amount)?;

    let action = Ics20Withdrawal {
        denom,
        destination_chain_address: event.destination_chain_address,
        return_address: bridge_address,
        amount,
        memo,
        fee_asset: fee_asset.clone(),
        // note: this refers to the timeout on the destination chain, which we are unaware of.
        // thus, we set it to the maximum possible value.
        timeout_height: max_timeout_height(),
        timeout_time: timeout_in_5_min(),
        source_channel,
        bridge_address: Some(bridge_address),
    };
    Ok(Action::Ics20Withdrawal(action))
}

fn log_to_sequencer_withdrawal_action(
    log: Log,
    asset_withdrawal_divisor: u128,
    bridge_address: Address,
    fee_asset: &asset::Denom,
) -> Result<Action, WithdrawalConversionError> {
    let rollup_block_number = log
        .block_number
        .ok_or_else(|| WithdrawalConversionError::log_without_block_number(&log))?
        .as_u64();

    let rollup_transaction_hash = log
        .transaction_hash
        .ok_or_else(|| WithdrawalConversionError::log_without_transaction_hash(&log))?
        .encode_hex();

    let event = decode_log::<SequencerWithdrawalFilter>(log)
        .map_err(WithdrawalConversionError::decode_log)?;

    let memo = memo_to_json(&memos::v1alpha1::BridgeUnlock {
        rollup_block_number,
        rollup_transaction_hash,
    })
    .map_err(WithdrawalConversionError::encode_memo)?;

    let amount = calculate_amount(&event, asset_withdrawal_divisor)
        .map_err(WithdrawalConversionError::calculate_withdrawal_amount)?;

    let to = parse_destination_chain_as_address(&event)
        .map_err(WithdrawalConversionError::destination_chain_as_address)?;

    let action = astria_core::protocol::transaction::v1alpha1::action::BridgeUnlockAction {
        to,
        amount,
        memo,
        fee_asset: fee_asset.clone(),
        bridge_address,
    };

    Ok(Action::BridgeUnlock(action))
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct WithdrawalConversionError(WithdrawalConversionErrorKind);

impl WithdrawalConversionError {
    fn calculate_withdrawal_amount(source: CalculateWithdrawalAmountError) -> Self {
        Self(WithdrawalConversionErrorKind::CalculateWithdrawalAmount(
            source,
        ))
    }

    fn decode_log(source: DecodeLogError) -> Self {
        Self(WithdrawalConversionErrorKind::DecodeLog(source))
    }

    fn destination_chain_as_address(source: DestinationChainAsAddressError) -> Self {
        Self(WithdrawalConversionErrorKind::DestinationChainAsAddress(
            source,
        ))
    }

    fn encode_memo(source: EncodeMemoError) -> Self {
        Self(WithdrawalConversionErrorKind::EncodeMemo(source))
    }

    // XXX: Somehow identify the log?
    fn log_without_block_number(_log: &Log) -> Self {
        Self(WithdrawalConversionErrorKind::LogWithoutBlockNumber)
    }

    // XXX: Somehow identify the log?
    fn log_without_transaction_hash(_log: &Log) -> Self {
        Self(WithdrawalConversionErrorKind::LogWithoutTransactionHash)
    }
}

#[derive(Debug, thiserror::Error)]
enum WithdrawalConversionErrorKind {
    #[error(transparent)]
    DecodeLog(DecodeLogError),
    #[error(transparent)]
    DestinationChainAsAddress(DestinationChainAsAddressError),
    #[error(transparent)]
    EncodeMemo(EncodeMemoError),
    #[error("log did not contain a block number")]
    LogWithoutBlockNumber,
    #[error("log did not contain a transaction hash")]
    LogWithoutTransactionHash,
    #[error(transparent)]
    CalculateWithdrawalAmount(CalculateWithdrawalAmountError),
}

#[derive(Debug, thiserror::Error)]
#[error("failed decoding a log into an Astria bridge contract event `{event_name}`")]
struct DecodeLogError {
    event_name: Cow<'static, str>,
    // use a trait object instead of the error to not force the middleware
    // type parameter into the error.
    source: Box<dyn std::error::Error + Send + Sync + 'static>,
}

fn decode_log<T: EthEvent>(log: Log) -> Result<T, DecodeLogError> {
    T::decode_log(&log.into()).map_err(|err| DecodeLogError {
        event_name: T::name(),
        source: err.into(),
    })
}

trait GetAmount {
    fn get_amount(&self) -> u128;
}

impl GetAmount for Ics20WithdrawalFilter {
    fn get_amount(&self) -> u128 {
        self.amount.as_u128()
    }
}

impl GetAmount for SequencerWithdrawalFilter {
    fn get_amount(&self) -> u128 {
        self.amount.as_u128()
    }
}

#[derive(Debug, thiserror::Error)]
#[error(
    "failed calculate amount to withdraw because mount in event could not be divided by the asset \
     withdrawal divisor; amount: `{amount}`, divisor: `{divisor}`"
)]
struct CalculateWithdrawalAmountError {
    amount: u128,
    divisor: u128,
}

fn calculate_amount<T: GetAmount>(
    event: &T,
    asset_withdrawal_divisor: u128,
) -> Result<u128, CalculateWithdrawalAmountError> {
    event
        .get_amount()
        .checked_div(asset_withdrawal_divisor)
        .ok_or_else(|| CalculateWithdrawalAmountError {
            amount: event.get_amount(),
            divisor: asset_withdrawal_divisor,
        })
}

fn max_timeout_height() -> ibc_types::core::client::Height {
    ibc_types::core::client::Height::new(u64::MAX, u64::MAX)
        .expect("non-zero arguments should never fail")
}

#[derive(Debug, thiserror::Error)]
#[error("failed to parse destination chain address as Astria address for a bridge unlock")]
struct DestinationChainAsAddressError {
    #[from]
    source: AddressError,
}

#[derive(Debug, thiserror::Error)]
#[error("failed encoding memo `{proto_message}` as JSON")]
struct EncodeMemoError {
    proto_message: String,
    source: serde_json::Error,
}

fn memo_to_json<T: prost::Name + serde::Serialize>(memo: &T) -> Result<String, EncodeMemoError> {
    serde_json::to_string(memo).map_err(|source| EncodeMemoError {
        proto_message: T::full_name(),
        source,
    })
}

fn parse_destination_chain_as_address(
    event: &SequencerWithdrawalFilter,
) -> Result<Address, DestinationChainAsAddressError> {
    event.destination_chain_address.parse().map_err(Into::into)
}

fn timeout_in_5_min() -> u64 {
    use std::time::Duration;
    tendermint::Time::now()
        .checked_add(Duration::from_secs(300))
        .expect("adding 5 minutes to the current time should never fail")
        .unix_timestamp_nanos()
        .try_into()
        .expect("timestamp must be positive, so this conversion would only fail if negative")
}

#[cfg(test)]
mod tests {
    use std::{
        str::FromStr,
        time::Duration,
    };

    use astria_core::{
        self,
        primitive::v1::{
            asset,
            Address,
        },
        protocol::{
            memos,
            transaction::v1alpha1::{
                action::Ics20Withdrawal,
                Action,
            },
        },
    };
    use ethers::{
        abi::AbiEncode,
        types::{
            Log,
            H256,
            U256,
        },
    };
    use ibc_types::core::client::Height as IbcHeight;

    use super::max_timeout_height;
    use crate::{
        astria_withdrawer,
        log_to_ics20_withdrawal_action,
        log_to_sequencer_withdrawal_action,
    };

    #[test]
    fn max_timeout_height_does_not_panic() {
        max_timeout_height();
    }

    const ASTRIA_ADDRESS_PREFIX: &str = "astria";
    /// Constructs an [`Address`] prefixed by `"astria"`.
    #[must_use]
    pub(crate) fn astria_address(
        array: [u8; astria_core::primitive::v1::ADDRESS_LEN],
    ) -> astria_core::primitive::v1::Address {
        astria_core::primitive::v1::Address::builder()
            .array(array)
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .try_build()
            .unwrap()
    }

    fn default_bridge_address() -> Address {
        astria_address([0u8; 20])
    }

    fn default_sequencer_address() -> Address {
        astria_address([1u8; 20])
    }

    fn default_fee_asset() -> asset::Denom {
        "nria".parse().unwrap()
    }

    fn default_sequencer_withdrawal_denom() -> asset::Denom {
        "nria".parse().unwrap()
    }

    fn default_sequencer_withdrawal_memo() -> memos::v1alpha1::BridgeUnlock {
        memos::v1alpha1::BridgeUnlock {
            rollup_block_number: 1,
            rollup_transaction_hash: H256::from_str(
                "0x1234567890123456789012345678901234567890123456789012345678901234",
            )
            .unwrap()
            .encode_hex(),
        }
    }

    fn default_sender_rollup_address() -> ethers::types::Address {
        "0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap()
    }

    const DEFAULT_IBC_DENOM: &str = "transfer/channel-0/utia";
    #[must_use]
    fn default_ibc_asset() -> asset::Denom {
        DEFAULT_IBC_DENOM.parse::<asset::Denom>().unwrap()
    }

    fn default_ics20_withdrawal_memo() -> memos::v1alpha1::Ics20WithdrawalFromRollup {
        memos::v1alpha1::Ics20WithdrawalFromRollup {
            rollup_block_number: 1,
            rollup_transaction_hash: H256::from_str(
                "0x1234567890123456789012345678901234567890123456789012345678901234",
            )
            .unwrap()
            .encode_hex(),
            rollup_return_address: default_sender_rollup_address().to_string(),
            memo: "foo".to_string(),
        }
    }

    struct SequencerWithdrawalTestConfig {
        asset_withdrawal_divisor: u128,
        bridge_address: Address,
        fee_asset: asset::Denom,
        memo: memos::v1alpha1::BridgeUnlock,
        event: astria_withdrawer::SequencerWithdrawalFilter,
    }

    fn default_sequencer_withdrawal_test_config() -> SequencerWithdrawalTestConfig {
        SequencerWithdrawalTestConfig {
            asset_withdrawal_divisor: 10u128.pow(18),
            bridge_address: default_bridge_address(),
            fee_asset: default_sequencer_withdrawal_denom(),
            memo: default_sequencer_withdrawal_memo(),
            event: astria_withdrawer::SequencerWithdrawalFilter {
                sender: default_sender_rollup_address(),
                amount: U256::from(10u128.pow(18)),
                destination_chain_address: default_sequencer_address().to_string(),
            },
        }
    }

    struct Ics20WithdrawalTestConfig {
        asset_withdrawal_divisor: u128,
        bridge_address: Address,
        fee_asset: asset::Denom,
        memo: memos::v1alpha1::Ics20WithdrawalFromRollup,
        event: astria_withdrawer::Ics20WithdrawalFilter,
        ibc_asset: asset::Denom,
        source_channel: ibc_types::core::channel::ChannelId,
    }

    fn default_ics20_withdrawal_test_config() -> Ics20WithdrawalTestConfig {
        Ics20WithdrawalTestConfig {
            asset_withdrawal_divisor: 10u128.pow(18),
            bridge_address: default_bridge_address(),
            fee_asset: default_fee_asset(),
            memo: default_ics20_withdrawal_memo(),
            event: astria_withdrawer::Ics20WithdrawalFilter {
                sender: default_sender_rollup_address(),
                amount: U256::from(10u128.pow(18)),
                destination_chain_address: default_sequencer_address().to_string(),
                memo: "foo".to_string(),
            },
            ibc_asset: default_ibc_asset(),
            source_channel: "channel-0".parse().unwrap(),
        }
    }

    fn make_sequencer_withdrawal_log(
        event: astria_withdrawer::SequencerWithdrawalFilter,
        memo: memos::v1alpha1::BridgeUnlock,
    ) -> Log {
        use ethers::{
            abi::Tokenizable as _,
            contract::EthEvent as _,
        };

        let topics = vec![
            astria_withdrawer::SequencerWithdrawalFilter::signature(),
            H256::from_slice(&ethers::abi::encode(&[event.sender.into_token()])),
            H256::from_slice(&ethers::abi::encode(&[event.amount.into_token()])),
        ];

        let data = ethers::abi::encode(&[event.destination_chain_address.to_string().into_token()]);

        Log {
            block_number: Some(memo.rollup_block_number.into()),
            transaction_hash: Some(memo.rollup_transaction_hash.parse().unwrap()),
            data: data.into(),
            topics,
            address: event.sender,
            log_index: Some(1.into()),
            transaction_index: Some(1.into()),
            removed: Some(false),
            block_hash: Some(
                "0x8e38b4dbf6b11fcc3b9dee84fb7986e29ca0a02cecd8977c161ff7333329681e"
                    .parse()
                    .unwrap(),
            ),
            transaction_log_index: Some(1.into()),
            log_type: None,
        }
    }

    fn make_sequencer_withdrawal_action(
        to: Address,
        amount: u128,
        memo: memos::v1alpha1::BridgeUnlock,
        bridge_address: Address,
        fee_asset: asset::Denom,
    ) -> Action {
        let action = astria_core::protocol::transaction::v1alpha1::action::BridgeUnlockAction {
            to,
            amount,
            memo: serde_json::to_string(&memo).unwrap(),
            bridge_address,
            fee_asset,
        };
        Action::BridgeUnlock(action)
    }

    fn make_ics20_withdrawal_log(
        event: astria_withdrawer::Ics20WithdrawalFilter,
        memo: memos::v1alpha1::Ics20WithdrawalFromRollup,
    ) -> Log {
        use ethers::contract::EthEvent as _;
        let topics = vec![
            astria_withdrawer::Ics20WithdrawalFilter::signature(),
            H256::from_slice(&ethers::abi::encode(&[event.sender.into_token()])),
            H256::from_slice(&ethers::abi::encode(&[event.amount.into_token()])),
        ];

        use ethers::abi::Tokenizable as _;
        let data = ethers::abi::encode(&[
            event.destination_chain_address.to_string().into_token(),
            event.memo.into_token(),
        ]);

        Log {
            block_number: Some(memo.rollup_block_number.into()),
            transaction_hash: Some(memo.rollup_transaction_hash.parse().unwrap()),
            data: data.into(),
            topics,
            address: event.sender,
            log_index: Some(1.into()),
            transaction_index: Some(1.into()),
            removed: Some(false),
            block_hash: Some(
                "0x8e38b4dbf6b11fcc3b9dee84fb7986e29ca0a02cecd8977c161ff7333329681e"
                    .parse()
                    .unwrap(),
            ),
            transaction_log_index: Some(1.into()),
            log_type: None,
        }
    }

    #[must_use]
    fn make_ibc_timeout_time() -> u64 {
        // this is copied from `bridge_withdrawer::ethereum::convert`
        const ICS20_WITHDRAWAL_TIMEOUT: Duration = Duration::from_secs(300);

        tendermint::Time::now()
            .checked_add(ICS20_WITHDRAWAL_TIMEOUT)
            .unwrap()
            .unix_timestamp_nanos()
            .try_into()
            .unwrap()
    }

    fn make_ics20_withdrawal_action(
        amount: u128,
        bridge_address: Address,
        fee_asset: &asset::Denom,
        source_channel: ibc_types::core::channel::ChannelId,
        memo: memos::v1alpha1::Ics20WithdrawalFromRollup,
        event: astria_withdrawer::Ics20WithdrawalFilter,
    ) -> Action {
        let timeout_height = IbcHeight::new(u64::MAX, u64::MAX).unwrap();
        let timeout_time = make_ibc_timeout_time();
        let denom = default_ibc_asset();
        let action = astria_core::protocol::transaction::v1alpha1::action::Ics20Withdrawal {
            amount,
            denom: denom.clone(),
            destination_chain_address: event.destination_chain_address,
            return_address: bridge_address,
            timeout_height,
            timeout_time,
            source_channel,
            fee_asset: fee_asset.clone(),
            memo: serde_json::to_string(&memo).unwrap(),
            bridge_address: Some(bridge_address),
        };
        Action::Ics20Withdrawal(action)
    }

    #[track_caller]
    fn assert_actions_eq(expected: &Action, actual: &Action) {
        match (expected.clone(), actual.clone()) {
            (Action::BridgeUnlock(expected), Action::BridgeUnlock(actual)) => {
                assert_eq!(expected, actual, "BridgeUnlock actions do not match");
            }
            (Action::Ics20Withdrawal(expected), Action::Ics20Withdrawal(actual)) => {
                assert_eq!(
                    SubsetOfIcs20Withdrawal::from(expected),
                    SubsetOfIcs20Withdrawal::from(actual),
                    "Ics20Withdrawal actions do not match"
                );
            }
            _ => {
                panic!(
                    "actions have a differing variants:\nexpected: {expected:?}\nactual: \
                     {actual:?}"
                )
            }
        }
    }

    /// A test wrapper around the `BridgeWithdrawer` for comparing the type without taking into
    /// account the timout timestamp (which is based on the current `tendermint::Time::now()` in
    /// the implementation)
    #[derive(Debug, PartialEq)]
    struct SubsetOfIcs20Withdrawal {
        amount: u128,
        denom: asset::Denom,
        destination_chain_address: String,
        return_address: Address,
        timeout_height: IbcHeight,
        source_channel: ibc_types::core::channel::ChannelId,
        fee_asset: asset::Denom,
        memo: String,
        bridge_address: Option<Address>,
    }

    impl From<Ics20Withdrawal> for SubsetOfIcs20Withdrawal {
        fn from(value: Ics20Withdrawal) -> Self {
            let Ics20Withdrawal {
                amount,
                denom,
                destination_chain_address,
                return_address,
                timeout_height,
                timeout_time: _timeout_time,
                source_channel,
                fee_asset,
                memo,
                bridge_address,
            } = value;
            Self {
                amount,
                denom,
                destination_chain_address,
                return_address,
                timeout_height,
                source_channel,
                fee_asset,
                memo,
                bridge_address,
            }
        }
    }

    #[test]
    fn sequencer_withdrawal_conversion_correct() {
        // create log from default config
        let SequencerWithdrawalTestConfig {
            asset_withdrawal_divisor,
            bridge_address,
            fee_asset,
            memo,
            event,
        } = default_sequencer_withdrawal_test_config();
        let log = make_sequencer_withdrawal_log(event.clone(), memo.clone());

        // convert to action
        let action = log_to_sequencer_withdrawal_action(
            log,
            asset_withdrawal_divisor,
            bridge_address,
            &fee_asset,
        )
        .unwrap();

        // compare against action created from default config values
        let expected_action = make_sequencer_withdrawal_action(
            event.destination_chain_address.parse().unwrap(),
            event
                .amount
                .as_u128()
                .checked_div(asset_withdrawal_divisor)
                .unwrap(),
            memo,
            bridge_address,
            fee_asset,
        );

        assert_actions_eq(&expected_action, &action)
    }

    #[test]
    fn ics20_withdrawal_conversion_correct() {
        // create log form default config
        let Ics20WithdrawalTestConfig {
            asset_withdrawal_divisor,
            bridge_address,
            fee_asset,
            memo,
            event,
            ibc_asset,
            source_channel,
        } = default_ics20_withdrawal_test_config();
        let log = make_ics20_withdrawal_log(event.clone(), memo.clone());

        // convert to action
        let action = log_to_ics20_withdrawal_action(
            log,
            asset_withdrawal_divisor,
            bridge_address,
            &fee_asset,
            ibc_asset.unwrap_trace_prefixed(),
            source_channel.clone(),
        )
        .unwrap();

        // compare against action created from default config values
        let expected_action = make_ics20_withdrawal_action(
            event
                .amount
                .as_u128()
                .checked_div(asset_withdrawal_divisor)
                .unwrap(),
            bridge_address,
            &fee_asset,
            source_channel,
            memo,
            event,
        );
        assert_actions_eq(&expected_action, &action)
    }
}
