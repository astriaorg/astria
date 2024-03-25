use ibc_types::{
    core::{
        channel::ChannelId,
        client::Height as IbcHeight,
    },
    IdentifierError,
};
use penumbra_ibc::IbcRelay;
use penumbra_proto::penumbra::core::component::ibc::v1::FungibleTokenPacketData;

use super::raw;
use crate::{
    sequencer::v1::{
        asset::{
            self,
            Denom,
        },
        Address,
        IncorrectAddressLength,
        IncorrectRollupIdLength,
        RollupId,
    },
    Protobuf,
};

#[derive(Clone, Debug)]
pub enum Action {
    Sequence(SequenceAction),
    Transfer(TransferAction),
    ValidatorUpdate(tendermint::validator::Update),
    SudoAddressChange(SudoAddressChangeAction),
    Mint(MintAction),
    Ibc(IbcRelay),
    Ics20Withdrawal(Ics20Withdrawal),
    IbcRelayerChange(IbcRelayerChangeAction),
    FeeAssetChange(FeeAssetChangeAction),
    InitBridgeAccount(InitBridgeAccountAction),
    BridgeLock(BridgeLockAction),
}

impl Action {
    #[must_use]
    pub fn into_raw(self) -> raw::Action {
        use raw::action::Value;
        let kind = match self {
            Action::Sequence(act) => Value::SequenceAction(act.into_raw()),
            Action::Transfer(act) => Value::TransferAction(act.into_raw()),
            Action::ValidatorUpdate(act) => Value::ValidatorUpdateAction(act.into()),
            Action::SudoAddressChange(act) => Value::SudoAddressChangeAction(act.into_raw()),
            Action::Mint(act) => Value::MintAction(act.into_raw()),
            Action::Ibc(act) => Value::IbcAction(act.into()),
            Action::Ics20Withdrawal(act) => Value::Ics20Withdrawal(act.into_raw()),
            Action::IbcRelayerChange(act) => Value::IbcRelayerChangeAction(act.into_raw()),
            Action::FeeAssetChange(act) => Value::FeeAssetChangeAction(act.into_raw()),
            Action::InitBridgeAccount(act) => Value::InitBridgeAccountAction(act.into_raw()),
            Action::BridgeLock(act) => Value::BridgeLockAction(act.into_raw()),
        };
        raw::Action {
            value: Some(kind),
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::Action {
        use raw::action::Value;
        let kind = match self {
            Action::Sequence(act) => Value::SequenceAction(act.to_raw()),
            Action::Transfer(act) => Value::TransferAction(act.to_raw()),
            Action::ValidatorUpdate(act) => Value::ValidatorUpdateAction(act.clone().into()),
            Action::SudoAddressChange(act) => {
                Value::SudoAddressChangeAction(act.clone().into_raw())
            }
            Action::Mint(act) => Value::MintAction(act.to_raw()),
            Action::Ibc(act) => Value::IbcAction(act.clone().into()),
            Action::Ics20Withdrawal(act) => Value::Ics20Withdrawal(act.to_raw()),
            Action::IbcRelayerChange(act) => Value::IbcRelayerChangeAction(act.to_raw()),
            Action::FeeAssetChange(act) => Value::FeeAssetChangeAction(act.to_raw()),
            Action::InitBridgeAccount(act) => Value::InitBridgeAccountAction(act.to_raw()),
            Action::BridgeLock(act) => Value::BridgeLockAction(act.to_raw()),
        };
        raw::Action {
            value: Some(kind),
        }
    }

    /// Attempt to convert from a raw, unchecked protobuf [`raw::Action`].
    ///
    /// # Errors
    ///
    /// Returns an error if conversion of one of the inner raw action variants
    /// to a native action ([`SequenceAction`] or [`TransferAction`]) fails.
    pub fn try_from_raw(proto: raw::Action) -> Result<Self, ActionError> {
        use raw::action::Value;
        let raw::Action {
            value,
        } = proto;
        let Some(action) = value else {
            return Err(ActionError::unset());
        };
        let action = match action {
            Value::SequenceAction(act) => {
                Self::Sequence(SequenceAction::try_from_raw(act).map_err(ActionError::sequence)?)
            }
            Value::TransferAction(act) => {
                Self::Transfer(TransferAction::try_from_raw(act).map_err(ActionError::transfer)?)
            }
            Value::ValidatorUpdateAction(act) => {
                Self::ValidatorUpdate(act.try_into().map_err(ActionError::validator_update)?)
            }
            Value::SudoAddressChangeAction(act) => Self::SudoAddressChange(
                SudoAddressChangeAction::try_from_raw(act)
                    .map_err(ActionError::sudo_address_change)?,
            ),
            Value::MintAction(act) => {
                Self::Mint(MintAction::try_from_raw(act).map_err(ActionError::mint)?)
            }
            Value::IbcAction(act) => {
                Self::Ibc(IbcRelay::try_from(act).map_err(|e| ActionError::ibc(e.into()))?)
            }
            Value::Ics20Withdrawal(act) => Self::Ics20Withdrawal(
                Ics20Withdrawal::try_from_raw(act).map_err(ActionError::ics20_withdrawal)?,
            ),
            Value::IbcRelayerChangeAction(act) => Self::IbcRelayerChange(
                IbcRelayerChangeAction::try_from_raw(&act)
                    .map_err(ActionError::ibc_relayer_change)?,
            ),
            Value::FeeAssetChangeAction(act) => Self::FeeAssetChange(
                FeeAssetChangeAction::try_from_raw(&act).map_err(ActionError::fee_asset_change)?,
            ),
            Value::InitBridgeAccountAction(act) => Self::InitBridgeAccount(
                InitBridgeAccountAction::try_from_raw(act)
                    .map_err(ActionError::init_bridge_account)?,
            ),
            Value::BridgeLockAction(act) => Self::BridgeLock(
                BridgeLockAction::try_from_raw(act).map_err(ActionError::bridge_lock)?,
            ),
        };
        Ok(action)
    }

    #[must_use]
    pub fn as_sequence(&self) -> Option<&SequenceAction> {
        let Self::Sequence(sequence_action) = self else {
            return None;
        };
        Some(sequence_action)
    }

    #[must_use]
    pub fn as_transfer(&self) -> Option<&TransferAction> {
        let Self::Transfer(transfer_action) = self else {
            return None;
        };
        Some(transfer_action)
    }
}

impl From<SequenceAction> for Action {
    fn from(value: SequenceAction) -> Self {
        Self::Sequence(value)
    }
}

impl From<TransferAction> for Action {
    fn from(value: TransferAction) -> Self {
        Self::Transfer(value)
    }
}

impl From<SudoAddressChangeAction> for Action {
    fn from(value: SudoAddressChangeAction) -> Self {
        Self::SudoAddressChange(value)
    }
}

impl From<MintAction> for Action {
    fn from(value: MintAction) -> Self {
        Self::Mint(value)
    }
}

impl From<IbcRelay> for Action {
    fn from(value: IbcRelay) -> Self {
        Self::Ibc(value)
    }
}

impl From<Ics20Withdrawal> for Action {
    fn from(value: Ics20Withdrawal) -> Self {
        Self::Ics20Withdrawal(value)
    }
}

impl From<IbcRelayerChangeAction> for Action {
    fn from(value: IbcRelayerChangeAction) -> Self {
        Self::IbcRelayerChange(value)
    }
}

impl From<FeeAssetChangeAction> for Action {
    fn from(value: FeeAssetChangeAction) -> Self {
        Self::FeeAssetChange(value)
    }
}

impl From<InitBridgeAccountAction> for Action {
    fn from(value: InitBridgeAccountAction) -> Self {
        Self::InitBridgeAccount(value)
    }
}

impl From<BridgeLockAction> for Action {
    fn from(value: BridgeLockAction) -> Self {
        Self::BridgeLock(value)
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ActionError(ActionErrorKind);

impl ActionError {
    fn unset() -> Self {
        Self(ActionErrorKind::Unset)
    }

    fn sequence(inner: SequenceActionError) -> Self {
        Self(ActionErrorKind::Sequence(inner))
    }

    fn transfer(inner: TransferActionError) -> Self {
        Self(ActionErrorKind::Transfer(inner))
    }

    fn validator_update(inner: tendermint::error::Error) -> Self {
        Self(ActionErrorKind::ValidatorUpdate(inner))
    }

    fn sudo_address_change(inner: SudoAddressChangeActionError) -> Self {
        Self(ActionErrorKind::SudoAddressChange(inner))
    }

    fn mint(inner: MintActionError) -> Self {
        Self(ActionErrorKind::Mint(inner))
    }

    fn ibc(inner: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self(ActionErrorKind::Ibc(inner))
    }

    fn ics20_withdrawal(inner: Ics20WithdrawalError) -> Self {
        Self(ActionErrorKind::Ics20Withdrawal(inner))
    }

    fn ibc_relayer_change(inner: IbcRelayerChangeActionError) -> Self {
        Self(ActionErrorKind::IbcRelayerChange(inner))
    }

    fn fee_asset_change(inner: FeeAssetChangeActionError) -> Self {
        Self(ActionErrorKind::FeeAssetChange(inner))
    }

    fn init_bridge_account(inner: InitBridgeAccountActionError) -> Self {
        Self(ActionErrorKind::InitBridgeAccount(inner))
    }

    fn bridge_lock(inner: BridgeLockActionError) -> Self {
        Self(ActionErrorKind::BridgeLock(inner))
    }
}

#[derive(Debug, thiserror::Error)]
enum ActionErrorKind {
    #[error("required action value was not set")]
    Unset,
    #[error("sequence action was not valid")]
    Sequence(#[source] SequenceActionError),
    #[error("transfer action was not valid")]
    Transfer(#[source] TransferActionError),
    #[error("validator update action was not valid")]
    ValidatorUpdate(#[source] tendermint::error::Error),
    #[error("sudo address change action was not valid")]
    SudoAddressChange(#[source] SudoAddressChangeActionError),
    #[error("mint action was not valid")]
    Mint(#[source] MintActionError),
    #[error("ibc action was not valid")]
    Ibc(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("ics20 withdrawal action was not valid")]
    Ics20Withdrawal(#[source] Ics20WithdrawalError),
    #[error("ibc relayer change action was not valid")]
    IbcRelayerChange(#[source] IbcRelayerChangeActionError),
    #[error("fee asset change action was not valid")]
    FeeAssetChange(#[source] FeeAssetChangeActionError),
    #[error("init bridge account action was not valid")]
    InitBridgeAccount(#[source] InitBridgeAccountActionError),
    #[error("bridge lock action was not valid")]
    BridgeLock(#[source] BridgeLockActionError),
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct SequenceActionError(SequenceActionErrorKind);

impl SequenceActionError {
    fn rollup_id(inner: IncorrectRollupIdLength) -> Self {
        Self(SequenceActionErrorKind::RollupId(inner))
    }

    fn fee_asset_id(inner: asset::IncorrectAssetIdLength) -> Self {
        Self(SequenceActionErrorKind::FeeAssetId(inner))
    }
}

#[derive(Debug, thiserror::Error)]
enum SequenceActionErrorKind {
    #[error("`rollup_id` field did not contain a valid rollup ID")]
    RollupId(IncorrectRollupIdLength),
    #[error("`fee_asset_id` field did not contain a valid asset ID")]
    FeeAssetId(asset::IncorrectAssetIdLength),
}

#[derive(Clone, Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct SequenceAction {
    pub rollup_id: RollupId,
    pub data: Vec<u8>,
    /// asset to use for fee payment.
    pub fee_asset_id: asset::Id,
}

impl SequenceAction {
    #[must_use]
    pub fn into_raw(self) -> raw::SequenceAction {
        let Self {
            rollup_id,
            data,
            fee_asset_id,
        } = self;
        raw::SequenceAction {
            rollup_id: rollup_id.to_vec(),
            data,
            fee_asset_id: fee_asset_id.as_ref().to_vec(),
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::SequenceAction {
        let Self {
            rollup_id,
            data,
            fee_asset_id,
        } = self;
        raw::SequenceAction {
            rollup_id: rollup_id.to_vec(),
            data: data.clone(),
            fee_asset_id: fee_asset_id.as_ref().to_vec(),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::SequenceAction`].
    ///
    /// # Errors
    /// Returns an error if the `proto.rollup_id` field was not 32 bytes.
    pub fn try_from_raw(proto: raw::SequenceAction) -> Result<Self, SequenceActionError> {
        let raw::SequenceAction {
            rollup_id,
            data,
            fee_asset_id,
        } = proto;
        let rollup_id =
            RollupId::try_from_slice(&rollup_id).map_err(SequenceActionError::rollup_id)?;
        let fee_asset_id =
            asset::Id::try_from_slice(&fee_asset_id).map_err(SequenceActionError::fee_asset_id)?;
        Ok(Self {
            rollup_id,
            data,
            fee_asset_id,
        })
    }
}

#[derive(Clone, Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct TransferAction {
    pub to: Address,
    pub amount: u128,
    // asset to be transferred.
    pub asset_id: asset::Id,
    /// asset to use for fee payment.
    pub fee_asset_id: asset::Id,
}

impl TransferAction {
    #[must_use]
    pub fn into_raw(self) -> raw::TransferAction {
        let Self {
            to,
            amount,
            asset_id,
            fee_asset_id,
        } = self;
        raw::TransferAction {
            to: to.to_vec(),
            amount: Some(amount.into()),
            asset_id: asset_id.get().to_vec(),
            fee_asset_id: fee_asset_id.as_ref().to_vec(),
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::TransferAction {
        let Self {
            to,
            amount,
            asset_id,
            fee_asset_id,
        } = self;
        raw::TransferAction {
            to: to.to_vec(),
            amount: Some((*amount).into()),
            asset_id: asset_id.get().to_vec(),
            fee_asset_id: fee_asset_id.as_ref().to_vec(),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::TransferAction`].
    ///
    /// # Errors
    ///
    /// Returns an error if the raw action's `to` address did not have the expected
    /// length.
    pub fn try_from_raw(proto: raw::TransferAction) -> Result<Self, TransferActionError> {
        let raw::TransferAction {
            to,
            amount,
            asset_id,
            fee_asset_id,
        } = proto;
        let to = Address::try_from_slice(&to).map_err(TransferActionError::address)?;
        let amount = amount.map_or(0, Into::into);
        let asset_id =
            asset::Id::try_from_slice(&asset_id).map_err(TransferActionError::asset_id)?;
        let fee_asset_id =
            asset::Id::try_from_slice(&fee_asset_id).map_err(TransferActionError::fee_asset_id)?;

        Ok(Self {
            to,
            amount,
            asset_id,
            fee_asset_id,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct TransferActionError(TransferActionErrorKind);

impl TransferActionError {
    fn address(inner: IncorrectAddressLength) -> Self {
        Self(TransferActionErrorKind::Address(inner))
    }

    fn asset_id(inner: asset::IncorrectAssetIdLength) -> Self {
        Self(TransferActionErrorKind::Asset(inner))
    }

    fn fee_asset_id(inner: asset::IncorrectAssetIdLength) -> Self {
        Self(TransferActionErrorKind::FeeAsset(inner))
    }
}

#[derive(Debug, thiserror::Error)]
enum TransferActionErrorKind {
    #[error("`to` field did not contain a valid address")]
    Address(#[source] IncorrectAddressLength),
    #[error("`asset_id` field did not contain a valid asset ID")]
    Asset(#[source] asset::IncorrectAssetIdLength),
    #[error("`fee_asset_id` field did not contain a valid asset ID")]
    FeeAsset(#[source] asset::IncorrectAssetIdLength),
}

#[derive(Clone, Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct SudoAddressChangeAction {
    pub new_address: Address,
}

impl SudoAddressChangeAction {
    #[must_use]
    pub fn into_raw(self) -> raw::SudoAddressChangeAction {
        let Self {
            new_address,
        } = self;
        raw::SudoAddressChangeAction {
            new_address: new_address.to_vec(),
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::SudoAddressChangeAction {
        let Self {
            new_address,
        } = self;
        raw::SudoAddressChangeAction {
            new_address: new_address.to_vec(),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::SudoAddressChangeAction`].
    ///
    /// # Errors
    ///
    /// Returns an error if the raw action's `new_address` did not have the expected
    /// length.
    pub fn try_from_raw(
        proto: raw::SudoAddressChangeAction,
    ) -> Result<Self, SudoAddressChangeActionError> {
        let raw::SudoAddressChangeAction {
            new_address,
        } = proto;
        let new_address =
            Address::try_from_slice(&new_address).map_err(SudoAddressChangeActionError::address)?;
        Ok(Self {
            new_address,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct SudoAddressChangeActionError(SudoAddressChangeActionErrorKind);

impl SudoAddressChangeActionError {
    fn address(inner: IncorrectAddressLength) -> Self {
        Self(SudoAddressChangeActionErrorKind::Address(inner))
    }
}

#[derive(Debug, thiserror::Error)]
enum SudoAddressChangeActionErrorKind {
    #[error("`new_address` field did not contain a valid address")]
    Address(#[source] IncorrectAddressLength),
}

#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug)]
pub struct MintAction {
    pub to: Address,
    pub amount: u128,
}

impl MintAction {
    #[must_use]
    pub fn into_raw(self) -> raw::MintAction {
        let Self {
            to,
            amount,
        } = self;
        raw::MintAction {
            to: to.to_vec(),
            amount: Some(amount.into()),
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::MintAction {
        let Self {
            to,
            amount,
        } = self;
        raw::MintAction {
            to: to.to_vec(),
            amount: Some((*amount).into()),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::MintAction`].
    ///
    /// # Errors
    ///
    /// Returns an error if the raw action's `to` address did not have the expected
    /// length.
    pub fn try_from_raw(proto: raw::MintAction) -> Result<Self, MintActionError> {
        let raw::MintAction {
            to,
            amount,
        } = proto;
        let to = Address::try_from_slice(&to).map_err(MintActionError::address)?;
        let amount = amount.map_or(0, Into::into);
        Ok(Self {
            to,
            amount,
        })
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct MintActionError(MintActionErrorKind);

impl MintActionError {
    fn address(inner: IncorrectAddressLength) -> Self {
        Self(MintActionErrorKind::Address(inner))
    }
}

#[derive(Debug, thiserror::Error)]
enum MintActionErrorKind {
    #[error("`to` field did not contain a valid address")]
    Address(#[source] IncorrectAddressLength),
}

/// Represents an IBC withdrawal of an asset from a source chain to a destination chain.
///
/// The parameters match the arguments to the `sendFungibleTokens` function in the
/// [ICS 20 spec](https://github.com/cosmos/ibc/blob/fe150abb629de5c1a598e8c7896a7568f2083681/spec/app/ics-020-fungible-token-transfer/README.md#packet-relay).
///
/// Note that it does not contain `source_port` as that is implicit (it uses the `transfer`) port.
///
/// It also contains a `return_address` field which may or may not be the same as the signer
/// of the packet. The funds will be returned to the `return_address` in the case of a timeout.
#[derive(Debug, Clone)]
pub struct Ics20Withdrawal {
    // a transparent value consisting of an amount and a denom.
    amount: u128,
    denom: Denom,
    // the address on the destination chain to send the transfer to.
    destination_chain_address: String,
    // an Astria address to use to return funds from this withdrawal
    // in the case it fails.
    return_address: Address,
    // the height (on Astria) at which this transfer expires.
    timeout_height: IbcHeight,
    // the unix timestamp (in nanoseconds) at which this transfer expires.
    timeout_time: u64,
    // the source channel used for the withdrawal.
    source_channel: ChannelId,
    // the asset to use for fee payment.
    fee_asset_id: asset::Id,
}

impl Ics20Withdrawal {
    #[must_use]
    pub fn amount(&self) -> u128 {
        self.amount
    }

    #[must_use]
    pub fn denom(&self) -> &Denom {
        &self.denom
    }

    #[must_use]
    pub fn destination_chain_address(&self) -> &str {
        &self.destination_chain_address
    }

    #[must_use]
    pub fn return_address(&self) -> &Address {
        &self.return_address
    }

    #[must_use]
    pub fn timeout_height(&self) -> &IbcHeight {
        &self.timeout_height
    }

    #[must_use]
    pub fn timeout_time(&self) -> u64 {
        self.timeout_time
    }

    #[must_use]
    pub fn source_channel(&self) -> &ChannelId {
        &self.source_channel
    }

    #[must_use]
    pub fn fee_asset_id(&self) -> &asset::Id {
        &self.fee_asset_id
    }

    #[must_use]
    pub fn to_fungible_token_packet_data(&self) -> FungibleTokenPacketData {
        FungibleTokenPacketData {
            amount: self.amount.to_string(),
            denom: self.denom.to_string(),
            sender: self.return_address.to_string(),
            receiver: self.destination_chain_address.clone(),
            memo: String::new(),
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::Ics20Withdrawal {
        raw::Ics20Withdrawal {
            amount: Some(self.amount.into()),
            denom: self.denom.to_string(),
            destination_chain_address: self.destination_chain_address.clone(),
            return_address: self.return_address.to_vec(),
            timeout_height: Some(self.timeout_height.into_raw()),
            timeout_time: self.timeout_time,
            source_channel: self.source_channel.to_string(),
            fee_asset_id: self.fee_asset_id.get().to_vec(),
        }
    }

    #[must_use]
    pub fn into_raw(self) -> raw::Ics20Withdrawal {
        raw::Ics20Withdrawal {
            amount: Some(self.amount.into()),
            denom: self.denom.to_string(),
            destination_chain_address: self.destination_chain_address,
            return_address: self.return_address.to_vec(),
            timeout_height: Some(self.timeout_height.into_raw()),
            timeout_time: self.timeout_time,
            source_channel: self.source_channel.to_string(),
            fee_asset_id: self.fee_asset_id.get().to_vec(),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::Ics20Withdrawal`].
    ///
    /// # Errors
    ///
    /// - if the `amount` field is missing
    /// - if the `denom` field is invalid
    /// - if the `return_address` field is invalid
    /// - if the `timeout_height` field is missing
    /// - if the `source_channel` field is invalid
    pub fn try_from_raw(proto: raw::Ics20Withdrawal) -> Result<Self, Ics20WithdrawalError> {
        let amount = proto.amount.ok_or(Ics20WithdrawalError::missing_amount())?;
        let return_address = Address::try_from_slice(&proto.return_address)
            .map_err(Ics20WithdrawalError::invalid_return_address)?;
        let timeout_height = proto
            .timeout_height
            .ok_or(Ics20WithdrawalError::missing_timeout_height())?
            .into();

        Ok(Self {
            amount: amount.into(),
            denom: proto.denom.clone().into(),
            destination_chain_address: proto.destination_chain_address,
            return_address,
            timeout_height,
            timeout_time: proto.timeout_time,
            source_channel: proto
                .source_channel
                .parse()
                .map_err(Ics20WithdrawalError::invalid_source_channel)?,
            fee_asset_id: asset::Id::try_from_slice(&proto.fee_asset_id)
                .map_err(Ics20WithdrawalError::invalid_fee_asset_id)?,
        })
    }
}

impl From<raw::IbcHeight> for IbcHeight {
    fn from(h: raw::IbcHeight) -> Self {
        Self {
            revision_number: h.revision_number,
            revision_height: h.revision_height,
        }
    }
}

impl Protobuf for IbcHeight {
    type Error = ::std::convert::Infallible;
    type Raw = raw::IbcHeight;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        Ok(Self {
            revision_number: raw.revision_number,
            revision_height: raw.revision_height,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        Self::Raw {
            revision_number: self.revision_number,
            revision_height: self.revision_height,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Ics20WithdrawalError(Ics20WithdrawalErrorKind);

impl Ics20WithdrawalError {
    #[must_use]
    fn missing_amount() -> Self {
        Self(Ics20WithdrawalErrorKind::MissingAmount)
    }

    #[must_use]
    fn invalid_return_address(err: IncorrectAddressLength) -> Self {
        Self(Ics20WithdrawalErrorKind::InvalidReturnAddress(err))
    }

    #[must_use]
    fn missing_timeout_height() -> Self {
        Self(Ics20WithdrawalErrorKind::MissingTimeoutHeight)
    }

    #[must_use]
    fn invalid_source_channel(err: IdentifierError) -> Self {
        Self(Ics20WithdrawalErrorKind::InvalidSourceChannel(err))
    }

    #[must_use]
    fn invalid_fee_asset_id(err: asset::IncorrectAssetIdLength) -> Self {
        Self(Ics20WithdrawalErrorKind::InvalidFeeAssetId(err))
    }
}

#[derive(Debug, thiserror::Error)]
enum Ics20WithdrawalErrorKind {
    #[error("`amount` field was missing")]
    MissingAmount,
    #[error("`return_address` field was invalid")]
    InvalidReturnAddress(#[source] IncorrectAddressLength),
    #[error("`timeout_height` field was missing")]
    MissingTimeoutHeight,
    #[error("`source_channel` field was invalid")]
    InvalidSourceChannel(#[source] IdentifierError),
    #[error("`fee_asset_id` field was invalid")]
    InvalidFeeAssetId(#[source] asset::IncorrectAssetIdLength),
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone)]
pub enum IbcRelayerChangeAction {
    Addition(Address),
    Removal(Address),
}

impl IbcRelayerChangeAction {
    #[must_use]
    pub fn into_raw(self) -> raw::IbcRelayerChangeAction {
        match self {
            IbcRelayerChangeAction::Addition(address) => raw::IbcRelayerChangeAction {
                value: Some(raw::ibc_relayer_change_action::Value::Addition(
                    address.to_vec(),
                )),
            },
            IbcRelayerChangeAction::Removal(address) => raw::IbcRelayerChangeAction {
                value: Some(raw::ibc_relayer_change_action::Value::Removal(
                    address.to_vec(),
                )),
            },
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::IbcRelayerChangeAction {
        match self {
            IbcRelayerChangeAction::Addition(address) => raw::IbcRelayerChangeAction {
                value: Some(raw::ibc_relayer_change_action::Value::Addition(
                    address.to_vec(),
                )),
            },
            IbcRelayerChangeAction::Removal(address) => raw::IbcRelayerChangeAction {
                value: Some(raw::ibc_relayer_change_action::Value::Removal(
                    address.to_vec(),
                )),
            },
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::IbcRelayerChangeAction`].
    ///
    /// # Errors
    ///
    /// - if the `address` field is invalid
    pub fn try_from_raw(
        raw: &raw::IbcRelayerChangeAction,
    ) -> Result<Self, IbcRelayerChangeActionError> {
        match raw {
            raw::IbcRelayerChangeAction {
                value: Some(raw::ibc_relayer_change_action::Value::Addition(address)),
            } => {
                let address = Address::try_from_slice(address)
                    .map_err(IbcRelayerChangeActionError::invalid_address)?;
                Ok(IbcRelayerChangeAction::Addition(address))
            }
            raw::IbcRelayerChangeAction {
                value: Some(raw::ibc_relayer_change_action::Value::Removal(address)),
            } => {
                let address = Address::try_from_slice(address)
                    .map_err(IbcRelayerChangeActionError::invalid_address)?;
                Ok(IbcRelayerChangeAction::Removal(address))
            }
            _ => Err(IbcRelayerChangeActionError::missing_address()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct IbcRelayerChangeActionError(IbcRelayerChangeActionErrorKind);

impl IbcRelayerChangeActionError {
    #[must_use]
    fn invalid_address(err: IncorrectAddressLength) -> Self {
        Self(IbcRelayerChangeActionErrorKind::InvalidAddress(err))
    }

    #[must_use]
    fn missing_address() -> Self {
        Self(IbcRelayerChangeActionErrorKind::MissingAddress)
    }
}

#[derive(Debug, thiserror::Error)]
enum IbcRelayerChangeActionErrorKind {
    #[error("the address was invalid")]
    InvalidAddress(#[source] IncorrectAddressLength),
    #[error("the address was missing")]
    MissingAddress,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone)]
pub enum FeeAssetChangeAction {
    Addition(asset::Id),
    Removal(asset::Id),
}

impl FeeAssetChangeAction {
    #[must_use]
    pub fn into_raw(self) -> raw::FeeAssetChangeAction {
        match self {
            FeeAssetChangeAction::Addition(asset_id) => raw::FeeAssetChangeAction {
                value: Some(raw::fee_asset_change_action::Value::Addition(
                    asset_id.get().to_vec(),
                )),
            },
            FeeAssetChangeAction::Removal(asset_id) => raw::FeeAssetChangeAction {
                value: Some(raw::fee_asset_change_action::Value::Removal(
                    asset_id.get().to_vec(),
                )),
            },
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::FeeAssetChangeAction {
        match self {
            FeeAssetChangeAction::Addition(asset_id) => raw::FeeAssetChangeAction {
                value: Some(raw::fee_asset_change_action::Value::Addition(
                    asset_id.get().to_vec(),
                )),
            },
            FeeAssetChangeAction::Removal(asset_id) => raw::FeeAssetChangeAction {
                value: Some(raw::fee_asset_change_action::Value::Removal(
                    asset_id.get().to_vec(),
                )),
            },
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::FeeAssetChangeAction`].
    ///
    /// # Errors
    ///
    /// - if the `asset_id` field is invalid
    pub fn try_from_raw(
        raw: &raw::FeeAssetChangeAction,
    ) -> Result<Self, FeeAssetChangeActionError> {
        match raw {
            raw::FeeAssetChangeAction {
                value: Some(raw::fee_asset_change_action::Value::Addition(asset_id)),
            } => {
                let asset_id = asset::Id::try_from_slice(asset_id)
                    .map_err(FeeAssetChangeActionError::invalid_asset_id)?;
                Ok(FeeAssetChangeAction::Addition(asset_id))
            }
            raw::FeeAssetChangeAction {
                value: Some(raw::fee_asset_change_action::Value::Removal(asset_id)),
            } => {
                let asset_id = asset::Id::try_from_slice(asset_id)
                    .map_err(FeeAssetChangeActionError::invalid_asset_id)?;
                Ok(FeeAssetChangeAction::Removal(asset_id))
            }
            _ => Err(FeeAssetChangeActionError::missing_asset_id()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct FeeAssetChangeActionError(FeeAssetChangeActionErrorKind);

impl FeeAssetChangeActionError {
    #[must_use]
    fn invalid_asset_id(err: asset::IncorrectAssetIdLength) -> Self {
        Self(FeeAssetChangeActionErrorKind::InvalidAssetId(err))
    }

    #[must_use]
    fn missing_asset_id() -> Self {
        Self(FeeAssetChangeActionErrorKind::MissingAssetId)
    }
}

#[derive(Debug, thiserror::Error)]
enum FeeAssetChangeActionErrorKind {
    #[error("the asset_id was invalid")]
    InvalidAssetId(#[source] asset::IncorrectAssetIdLength),
    #[error("the asset_id was missing")]
    MissingAssetId,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone)]
pub struct InitBridgeAccountAction {
    // the rollup ID to register for the sender of this action
    pub rollup_id: RollupId,
    // the assets accepted by the bridge account
    pub asset_ids: Vec<asset::Id>,
    // the fee asset which to pay this action's fees with
    pub fee_asset_id: asset::Id,
}

impl InitBridgeAccountAction {
    #[must_use]
    pub fn into_raw(self) -> raw::InitBridgeAccountAction {
        raw::InitBridgeAccountAction {
            rollup_id: self.rollup_id.to_vec(),
            asset_ids: self.asset_ids.iter().map(|id| id.get().to_vec()).collect(),
            fee_asset_id: self.fee_asset_id.get().to_vec(),
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::InitBridgeAccountAction {
        raw::InitBridgeAccountAction {
            rollup_id: self.rollup_id.to_vec(),
            asset_ids: self.asset_ids.iter().map(|id| id.get().to_vec()).collect(),
            fee_asset_id: self.fee_asset_id.get().to_vec(),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::InitBridgeAccountAction`].
    ///
    /// # Errors
    ///
    /// - if the `rollup_id` field is invalid
    pub fn try_from_raw(
        proto: raw::InitBridgeAccountAction,
    ) -> Result<Self, InitBridgeAccountActionError> {
        let rollup_id = RollupId::try_from_slice(&proto.rollup_id)
            .map_err(InitBridgeAccountActionError::invalid_rollup_id)?;
        let asset_ids = proto
            .asset_ids
            .into_iter()
            .map(|bytes| asset::Id::try_from_slice(&bytes))
            .collect::<Result<Vec<asset::Id>, asset::IncorrectAssetIdLength>>()
            .map_err(InitBridgeAccountActionError::invalid_asset_id)?;
        let fee_asset_id = asset::Id::try_from_slice(&proto.fee_asset_id)
            .map_err(InitBridgeAccountActionError::invalid_fee_asset_id)?;

        Ok(Self {
            rollup_id,
            asset_ids,
            fee_asset_id,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct InitBridgeAccountActionError(InitBridgeAccountActionErrorKind);

impl InitBridgeAccountActionError {
    #[must_use]
    fn invalid_rollup_id(err: IncorrectRollupIdLength) -> Self {
        Self(InitBridgeAccountActionErrorKind::InvalidRollupId(err))
    }

    #[must_use]
    fn invalid_asset_id(err: asset::IncorrectAssetIdLength) -> Self {
        Self(InitBridgeAccountActionErrorKind::InvalidAssetId(err))
    }

    #[must_use]
    fn invalid_fee_asset_id(err: asset::IncorrectAssetIdLength) -> Self {
        Self(InitBridgeAccountActionErrorKind::InvalidFeeAssetId(err))
    }
}

// allow pedantic clippy as the errors have the same prefix (for consistency
// with other error types) as well as the same postfix (due to the types the
// errors are referencing), both of which cause clippy to complain.
#[derive(Debug, thiserror::Error)]
#[allow(clippy::enum_variant_names)]
enum InitBridgeAccountActionErrorKind {
    #[error("the `rollup_id` field was invalid")]
    InvalidRollupId(#[source] IncorrectRollupIdLength),
    #[error("an asset ID was invalid")]
    InvalidAssetId(#[source] asset::IncorrectAssetIdLength),
    #[error("the `fee_asset_id` field was invalid")]
    InvalidFeeAssetId(#[source] asset::IncorrectAssetIdLength),
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone)]
pub struct BridgeLockAction {
    pub to: Address,
    pub amount: u128,
    // asset to be transferred.
    pub asset_id: asset::Id,
    // asset to use for fee payment.
    pub fee_asset_id: asset::Id,
    // the address on the destination chain to send the transfer to.
    pub destination_chain_address: String,
}

impl BridgeLockAction {
    #[must_use]
    pub fn into_raw(self) -> raw::BridgeLockAction {
        raw::BridgeLockAction {
            to: self.to.to_vec(),
            amount: Some(self.amount.into()),
            asset_id: self.asset_id.get().to_vec(),
            fee_asset_id: self.fee_asset_id.as_ref().to_vec(),
            destination_chain_address: self.destination_chain_address,
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::BridgeLockAction {
        raw::BridgeLockAction {
            to: self.to.to_vec(),
            amount: Some(self.amount.into()),
            asset_id: self.asset_id.get().to_vec(),
            fee_asset_id: self.fee_asset_id.as_ref().to_vec(),
            destination_chain_address: self.destination_chain_address.clone(),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::BridgeLockAction`].
    ///
    /// # Errors
    ///
    /// - if the `to` field is invalid
    /// - if the `asset_id` field is invalid
    /// - if the `fee_asset_id` field is invalid
    pub fn try_from_raw(proto: raw::BridgeLockAction) -> Result<Self, BridgeLockActionError> {
        let to =
            Address::try_from_slice(&proto.to).map_err(BridgeLockActionError::invalid_address)?;
        let amount = proto
            .amount
            .ok_or(BridgeLockActionError::missing_amount())?;
        let asset_id = asset::Id::try_from_slice(&proto.asset_id)
            .map_err(BridgeLockActionError::invalid_asset_id)?;
        let fee_asset_id = asset::Id::try_from_slice(&proto.fee_asset_id)
            .map_err(BridgeLockActionError::invalid_fee_asset_id)?;
        Ok(Self {
            to,
            amount: amount.into(),
            asset_id,
            fee_asset_id,
            destination_chain_address: proto.destination_chain_address,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct BridgeLockActionError(BridgeLockActionErrorKind);

impl BridgeLockActionError {
    #[must_use]
    fn invalid_address(err: IncorrectAddressLength) -> Self {
        Self(BridgeLockActionErrorKind::InvalidAddress(err))
    }

    #[must_use]
    fn missing_amount() -> Self {
        Self(BridgeLockActionErrorKind::MissingAmount)
    }

    #[must_use]
    fn invalid_asset_id(err: asset::IncorrectAssetIdLength) -> Self {
        Self(BridgeLockActionErrorKind::InvalidAssetId(err))
    }

    #[must_use]
    fn invalid_fee_asset_id(err: asset::IncorrectAssetIdLength) -> Self {
        Self(BridgeLockActionErrorKind::InvalidFeeAssetId(err))
    }
}

#[derive(Debug, thiserror::Error)]
enum BridgeLockActionErrorKind {
    #[error("the `address` field was invalid")]
    InvalidAddress(#[source] IncorrectAddressLength),
    #[error("the `amount` field was not set")]
    MissingAmount,
    #[error("the `asset_id` field was invalid")]
    InvalidAssetId(#[source] asset::IncorrectAssetIdLength),
    #[error("the `fee_asset_id` field was invalid")]
    InvalidFeeAssetId(#[source] asset::IncorrectAssetIdLength),
}
