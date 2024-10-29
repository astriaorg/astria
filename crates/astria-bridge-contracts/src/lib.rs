#[rustfmt::skip]
#[expect(clippy::pedantic, clippy::allow_attributes, clippy::allow_attributes_without_reason)]
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
        transaction::v1::{
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
    self,
    abi::AbiEncode,
    contract::EthEvent,
    providers::Middleware,
    types::{
        Filter,
        Log,
        H256,
    },
};
pub use generated::*;

const NON_ERC20_CONTRACT_DECIMALS: u32 = 18u32;

macro_rules! warn {
    ($($tt:tt)*) => {
        #[cfg(feature = "tracing")]
        {
            #![cfg_attr(
                feature = "tracing",
                expect(
                    clippy::used_underscore_binding,
                    reason = "underscore is needed to quiet `unused-variables` warning if `tracing` feature is not set",
            ))]
            ::tracing::warn!($($tt)*);
        }
    }
}

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
    use_compat_address: bool,
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
            use_compat_address: false,
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
            use_compat_address,
            ..
        } = self;
        GetWithdrawalActionsBuilder {
            provider: WithProvider(provider),
            contract_address,
            bridge_address,
            fee_asset,
            sequencer_asset_to_withdraw,
            ics20_asset_to_withdraw,
            use_compat_address,
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

    #[must_use]
    pub fn use_compat_address(self, use_compat_address: bool) -> Self {
        Self {
            use_compat_address,
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
            use_compat_address,
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

        let contract_decimals = {
            let erc_20_contract = astria_bridgeable_erc20::AstriaBridgeableERC20::new(
                contract_address,
                provider.clone(),
            );
            match erc_20_contract.decimals().call().await {
                Ok(decimals) => decimals.into(),
                Err(_error) => {
                    warn!(
                        error = &_error as &dyn std::error::Error,
                        "failed reading decimals from contract; assuming it is not an ERC20 \
                         contract and falling back to `{NON_ERC20_CONTRACT_DECIMALS}`"
                    );
                    NON_ERC20_CONTRACT_DECIMALS
                }
            }
        };

        let exponent = contract_decimals
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
            use_compat_address,
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
    use_compat_address: bool,
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
    ) -> Result<Vec<Result<Action, GetWithdrawalActionsError>>, GetWithdrawalActionsError> {
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
                .map_err(GetWithdrawalActionsError::get_logs)?;

        // XXX: The calls to `log_to_*_action` rely on only be called if `GetWithdrawalActions`
        // is configured for either ics20 or sequencer withdrawals (or both). They would panic
        // otherwise.
        Ok(ics20_logs
            .into_iter()
            .map(|log| self.log_to_ics20_withdrawal_action(log))
            .chain(
                sequencer_logs
                    .into_iter()
                    .map(|log| self.log_to_sequencer_withdrawal_action(log)),
            )
            .collect())
    }

    fn log_to_ics20_withdrawal_action(
        &self,
        log: Log,
    ) -> Result<Action, GetWithdrawalActionsError> {
        let rollup_block_number = log
            .block_number
            .ok_or_else(|| GetWithdrawalActionsError::log_without_block_number(&log))?
            .as_u64();

        let transaction_hash = log
            .transaction_hash
            .ok_or_else(|| GetWithdrawalActionsError::log_without_transaction_hash(&log))?
            .encode_hex();
        let event_index = log
            .log_index
            .ok_or_else(|| GetWithdrawalActionsError::log_without_log_index(&log))?
            .encode_hex();

        let rollup_withdrawal_event_id = format!("{transaction_hash}.{event_index}");

        let event = decode_log::<Ics20WithdrawalFilter>(log)
            .map_err(GetWithdrawalActionsError::decode_log)?;

        let (denom, source_channel) = (
            self.ics20_asset_to_withdraw
                .clone()
                .expect("must be set if this method is entered")
                .into(),
            self.ics20_source_channel
                .clone()
                .expect("must be set if this method is entered"),
        );

        let memo = memo_to_json(&memos::v1::Ics20WithdrawalFromRollup {
            memo: event.memo.clone(),
            rollup_block_number,
            rollup_return_address: event.sender.encode_hex(),
            rollup_withdrawal_event_id,
        })
        .map_err(GetWithdrawalActionsError::encode_memo)?;

        let amount = calculate_amount(&event, self.asset_withdrawal_divisor)
            .map_err(GetWithdrawalActionsError::calculate_withdrawal_amount)?;

        let action = Ics20Withdrawal {
            denom,
            destination_chain_address: event.destination_chain_address,
            return_address: self.bridge_address,
            amount,
            memo,
            fee_asset: self.fee_asset.clone(),
            // note: this refers to the timeout on the destination chain, which we are unaware of.
            // thus, we set it to the maximum possible value.
            timeout_height: max_timeout_height(),
            timeout_time: timeout_in_5_min(),
            source_channel,
            bridge_address: Some(self.bridge_address),
            use_compat_address: self.use_compat_address,
        };
        Ok(Action::Ics20Withdrawal(action))
    }

    fn log_to_sequencer_withdrawal_action(
        &self,
        log: Log,
    ) -> Result<Action, GetWithdrawalActionsError> {
        let rollup_block_number = log
            .block_number
            .ok_or_else(|| GetWithdrawalActionsError::log_without_block_number(&log))?
            .as_u64();

        let transaction_hash = log
            .transaction_hash
            .ok_or_else(|| GetWithdrawalActionsError::log_without_transaction_hash(&log))?
            .encode_hex();
        let event_index = log
            .log_index
            .ok_or_else(|| GetWithdrawalActionsError::log_without_log_index(&log))?
            .encode_hex();

        let rollup_withdrawal_event_id = format!("{transaction_hash}.{event_index}");

        let event = decode_log::<SequencerWithdrawalFilter>(log)
            .map_err(GetWithdrawalActionsError::decode_log)?;

        let amount = calculate_amount(&event, self.asset_withdrawal_divisor)
            .map_err(GetWithdrawalActionsError::calculate_withdrawal_amount)?;

        let to = parse_destination_chain_as_address(&event)
            .map_err(GetWithdrawalActionsError::destination_chain_as_address)?;

        let action = astria_core::protocol::transaction::v1::action::BridgeUnlock {
            to,
            amount,
            rollup_block_number,
            rollup_withdrawal_event_id,
            memo: String::new(),
            fee_asset: self.fee_asset.clone(),
            bridge_address: self.bridge_address,
        };

        Ok(Action::BridgeUnlock(action))
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct GetWithdrawalActionsError(GetWithdrawalActionsErrorKind);

impl GetWithdrawalActionsError {
    fn calculate_withdrawal_amount(source: CalculateWithdrawalAmountError) -> Self {
        Self(GetWithdrawalActionsErrorKind::CalculateWithdrawalAmount(
            source,
        ))
    }

    fn decode_log(source: DecodeLogError) -> Self {
        Self(GetWithdrawalActionsErrorKind::DecodeLog(source))
    }

    fn destination_chain_as_address(source: DestinationChainAsAddressError) -> Self {
        Self(GetWithdrawalActionsErrorKind::DestinationChainAsAddress(
            source,
        ))
    }

    fn encode_memo(source: EncodeMemoError) -> Self {
        Self(GetWithdrawalActionsErrorKind::EncodeMemo(source))
    }

    fn get_logs(source: GetLogsError) -> Self {
        Self(GetWithdrawalActionsErrorKind::GetLogs(source))
    }

    // XXX: Somehow identify the log?
    fn log_without_block_number(_log: &Log) -> Self {
        Self(GetWithdrawalActionsErrorKind::LogWithoutBlockNumber)
    }

    // XXX: Somehow identify the log?
    fn log_without_transaction_hash(_log: &Log) -> Self {
        Self(GetWithdrawalActionsErrorKind::LogWithoutTransactionHash)
    }

    // XXX: Somehow identify the log?
    fn log_without_log_index(_log: &Log) -> Self {
        Self(GetWithdrawalActionsErrorKind::LogWithoutLogIndex)
    }
}

#[derive(Debug, thiserror::Error)]
enum GetWithdrawalActionsErrorKind {
    #[error(transparent)]
    DecodeLog(DecodeLogError),
    #[error(transparent)]
    DestinationChainAsAddress(DestinationChainAsAddressError),
    #[error(transparent)]
    EncodeMemo(EncodeMemoError),
    #[error(transparent)]
    GetLogs(GetLogsError),
    #[error("log did not contain a block number")]
    LogWithoutBlockNumber,
    #[error("log did not contain a transaction hash")]
    LogWithoutTransactionHash,
    #[error("log did not contain a log index")]
    LogWithoutLogIndex,
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
    use super::max_timeout_height;
    #[test]
    fn max_timeout_height_does_not_panic() {
        max_timeout_height();
    }
}
