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
    sequencer::v1alpha1::{
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
            asset_id: asset_id.as_bytes().to_vec(),
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
            asset_id: asset_id.as_bytes().to_vec(),
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
    pub fn to_fungible_token_packet_data(&self) -> FungibleTokenPacketData {
        FungibleTokenPacketData {
            amount: self.amount.to_string(),
            denom: self.denom.to_string(),
            sender: self.return_address.to_string(),
            receiver: self.destination_chain_address.clone(),
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
            denom: proto.denom.as_str().into(),
            destination_chain_address: proto.destination_chain_address,
            return_address,
            timeout_height,
            timeout_time: proto.timeout_time,
            source_channel: proto
                .source_channel
                .parse()
                .map_err(Ics20WithdrawalError::invalid_source_channel)?,
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

#[allow(clippy::module_name_repetitions)]
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
}

#[derive(Debug, thiserror::Error)]
enum Ics20WithdrawalErrorKind {
    #[error("`amount` field was missing")]
    MissingAmount,
    #[error("`return_address` field was invalid")]
    InvalidReturnAddress(IncorrectAddressLength),
    #[error("`timeout_height` field was missing")]
    MissingTimeoutHeight,
    #[error("`source_channel` field was invalid")]
    InvalidSourceChannel(IdentifierError),
}
