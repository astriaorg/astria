#[rustfmt::skip]
#[allow(clippy::pedantic)]
mod generated;
use std::{
    borrow::Cow,
    sync::Arc,
};

use astria_core::{
    primitive::v1::{
        asset::{
            self,
            TracePrefixed,
        },
        Address,
        AddressError,
    },
    protocol::transaction::v1alpha1::{
        action::Ics20Withdrawal,
        Action,
    },
};
use astria_withdrawer::{
    Ics20WithdrawalFilter,
    SequencerWithdrawalFilter,
};
use ethers::{
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
    fn bad_divisor(base_chain_asset_precision: u32) -> Self {
        Self(BuildErrorKind::BadDivisor {
            base_chain_asset_precision,
        })
    }

    fn call_base_chain_asset_precision<
        T: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    >(
        source: T,
    ) -> Self {
        Self(BuildErrorKind::CallBaseChainAssetPrecision {
            source: source.into(),
        })
    }

    fn not_set(field: &'static str) -> Self {
        Self(BuildErrorKind::NotSet {
            field,
        })
    }

    fn rollup_asset_without_channel() -> Self {
        Self(BuildErrorKind::RollupAssetWithoutChannel)
    }

    fn parse_rollup_asset_source_channel(source: ibc_types::IdentifierError) -> Self {
        Self(BuildErrorKind::ParseRollupAssetSourceChannel {
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
    #[error("failed to call the `BASE_CHAIN_ASSET_PRECISION` of the provided contract")]
    CallBaseChainAssetPrecision {
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
    #[error("rollup asset denom must have a channel to be withdrawn via IBC")]
    RollupAssetWithoutChannel,
    #[error("could not parse rollup asset channel as channel ID")]
    ParseRollupAssetSourceChannel { source: ibc_types::IdentifierError },
}

pub struct NoProvider;
pub struct WithProvider<P>(Arc<P>);

pub struct GetWithdrawalActionsBuilder<TProvider = NoProvider> {
    provider: TProvider,
    contract_address: Option<ethers::types::Address>,
    bridge_address: Option<Address>,
    fee_asset: Option<asset::Denom>,
    rollup_asset_denom: Option<asset::Denom>,
}

impl Default for GetWithdrawalActionsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GetWithdrawalActionsBuilder {
    pub fn new() -> Self {
        Self {
            provider: NoProvider,
            contract_address: None,
            bridge_address: None,
            fee_asset: None,
            rollup_asset_denom: None,
        }
    }
}

impl<P> GetWithdrawalActionsBuilder<P> {
    pub fn provider<Q>(self, provider: Arc<Q>) -> GetWithdrawalActionsBuilder<WithProvider<Q>> {
        let Self {
            contract_address,
            bridge_address,
            fee_asset,
            rollup_asset_denom,
            ..
        } = self;
        GetWithdrawalActionsBuilder {
            provider: WithProvider(provider),
            contract_address,
            bridge_address,
            fee_asset,
            rollup_asset_denom,
        }
    }

    pub fn contract_address(self, contract_address: ethers::types::Address) -> Self {
        Self {
            contract_address: Some(contract_address),
            ..self
        }
    }

    pub fn bridge_address(self, bridge_address: Address) -> Self {
        Self {
            bridge_address: Some(bridge_address),
            ..self
        }
    }

    pub fn fee_asset(self, fee_asset: asset::Denom) -> Self {
        Self {
            fee_asset: Some(fee_asset),
            ..self
        }
    }

    pub fn rollup_asset_denom(self, rollup_asset_denom: asset::Denom) -> Self {
        Self {
            rollup_asset_denom: Some(rollup_asset_denom),
            ..self
        }
    }
}

impl<P> GetWithdrawalActionsBuilder<WithProvider<P>>
where
    P: Middleware + 'static,
    P::Error: std::error::Error + 'static,
{
    pub async fn try_build(self) -> Result<GetWithdrawalActions<P>, BuildError> {
        let Self {
            provider: WithProvider(provider),
            contract_address,
            bridge_address,
            fee_asset,
            rollup_asset_denom,
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
        let Some(rollup_asset_denom) = rollup_asset_denom else {
            return Err(BuildError::not_set("rollup_asset_denom"));
        };

        let rollup_asset_source_channel = rollup_asset_denom
            .as_trace_prefixed()
            .and_then(TracePrefixed::last_channel)
            .ok_or(BuildError::rollup_asset_without_channel())?
            .parse()
            .map_err(BuildError::parse_rollup_asset_source_channel)?;

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
            rollup_asset_denom,
            rollup_asset_source_channel,
        })
    }
}

pub struct GetWithdrawalActions<P> {
    provider: Arc<P>,
    contract_address: ethers::types::Address,
    asset_withdrawal_divisor: u128,
    bridge_address: Address,
    fee_asset: asset::Denom,
    rollup_asset_denom: asset::Denom,
    rollup_asset_source_channel: ibc_types::core::channel::ChannelId,
}

impl<P> GetWithdrawalActions<P>
where
    P: Middleware,
    P::Error: std::error::Error + 'static,
{
    pub async fn get_for_block_hash(
        &self,
        block_hash: H256,
    ) -> Result<Vec<Action>, GetWithdrawalActionsError> {
        let (ics20_logs, sequencer_logs) = futures::future::try_join(
            get_logs::<Ics20WithdrawalFilter, _>(&self.provider, self.contract_address, block_hash),
            get_logs::<SequencerWithdrawalFilter, _>(
                &self.provider,
                self.contract_address,
                block_hash,
            ),
        )
        .await
        .map_err(GetWithdrawalActionsError::get_logs)?;

        ics20_logs
            .into_iter()
            .map(|log| self.log_to_ics20_withdrawal_action(log))
            .chain(
                sequencer_logs
                    .into_iter()
                    .map(|log| self.log_to_sequencer_withdrawal_action(log)),
            )
            .collect()
    }

    fn log_to_ics20_withdrawal_action(
        &self,
        log: Log,
    ) -> Result<Action, GetWithdrawalActionsError> {
        let block_number = log
            .block_number
            .ok_or_else(|| GetWithdrawalActionsError::log_without_block_number(&log))?
            .as_u64();

        let transaction_hash = log
            .transaction_hash
            .ok_or_else(|| GetWithdrawalActionsError::log_without_transaction_hash(&log))?
            .into();

        let event = decode_log::<Ics20WithdrawalFilter>(log)
            .map_err(GetWithdrawalActionsError::decode_log)?;

        let source_channel = self.rollup_asset_source_channel.clone();

        let memo = serde_json::to_string(&astria_core::bridge::Ics20WithdrawalFromRollupMemo {
            memo: event.memo.clone(),
            block_number,
            rollup_return_address: event.sender.to_string(),
            transaction_hash,
        })
        .map_err(|source| {
            GetWithdrawalActionsError::encode_memo("Ics20WithdrawalFromRollupMemo", source)
        })?;

        let amount = calculate_amount(&event, self.asset_withdrawal_divisor)
            .map_err(GetWithdrawalActionsError::calculate_withdrawal_amount)?;

        let action = Ics20Withdrawal {
            denom: self.rollup_asset_denom.clone(),
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
        };
        Ok(Action::Ics20Withdrawal(action))
    }

    fn log_to_sequencer_withdrawal_action(
        &self,
        log: Log,
    ) -> Result<Action, GetWithdrawalActionsError> {
        let block_number = log
            .block_number
            .ok_or_else(|| GetWithdrawalActionsError::log_without_block_number(&log))?
            .as_u64();

        let transaction_hash = log
            .transaction_hash
            .ok_or_else(|| GetWithdrawalActionsError::log_without_transaction_hash(&log))?
            .into();

        let event = decode_log::<SequencerWithdrawalFilter>(log)
            .map_err(GetWithdrawalActionsError::decode_log)?;

        let memo = serde_json::to_string(&astria_core::bridge::UnlockMemo {
            block_number,
            transaction_hash,
        })
        .map_err(|err| GetWithdrawalActionsError::encode_memo("bridge::UnlockMemo", err))?;

        let amount = calculate_amount(&event, self.asset_withdrawal_divisor)
            .map_err(GetWithdrawalActionsError::calculate_withdrawal_amount)?;

        let to = parse_destination_chain_as_address(&event)
            .map_err(GetWithdrawalActionsError::destination_chain_as_address)?;

        let action = astria_core::protocol::transaction::v1alpha1::action::BridgeUnlockAction {
            to,
            amount,
            memo,
            fee_asset: self.fee_asset.clone(),
            bridge_address: Some(self.bridge_address),
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

    fn encode_memo(which: &'static str, source: serde_json::Error) -> Self {
        Self(GetWithdrawalActionsErrorKind::EncodeMemo {
            which,
            source,
        })
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
}

#[derive(Debug, thiserror::Error)]
enum GetWithdrawalActionsErrorKind {
    #[error(transparent)]
    DecodeLog(DecodeLogError),
    #[error(transparent)]
    DestinationChainAsAddress(DestinationChainAsAddressError),
    #[error("failed encoding memo `{which}`")]
    EncodeMemo {
        which: &'static str,
        source: serde_json::Error,
    },
    #[error(transparent)]
    GetLogs(GetLogsError),
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
