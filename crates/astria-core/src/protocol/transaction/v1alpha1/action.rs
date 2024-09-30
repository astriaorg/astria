use bytes::Bytes;
use ibc_types::{
    core::{
        channel::ChannelId,
        client::Height as IbcHeight,
    },
    IdentifierError,
};
use penumbra_ibc::IbcRelay;

use super::raw;
use crate::{
    primitive::v1::{
        asset::{
            self,
            Denom,
        },
        Address,
        AddressError,
        IncorrectRollupIdLength,
        RollupId,
    },
    Protobuf,
};

#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Deserialize, ::serde::Serialize),
    serde(into = "raw::Action", try_from = "raw::Action")
)]
pub enum Action {
    Sequence(SequenceAction),
    Transfer(TransferAction),
    ValidatorUpdate(ValidatorUpdate),
    SudoAddressChange(SudoAddressChangeAction),
    Ibc(IbcRelay),
    IbcSudoChange(IbcSudoChangeAction),
    Ics20Withdrawal(Ics20Withdrawal),
    IbcRelayerChange(IbcRelayerChangeAction),
    FeeAssetChange(FeeAssetChangeAction),
    InitBridgeAccount(InitBridgeAccountAction),
    BridgeLock(BridgeLockAction),
    BridgeUnlock(BridgeUnlockAction),
    BridgeSudoChange(BridgeSudoChangeAction),
    FeeChange(FeeChangeAction),
}

impl Protobuf for Action {
    type Error = ActionError;
    type Raw = raw::Action;

    #[must_use]
    fn to_raw(&self) -> Self::Raw {
        use raw::action::Value;
        let kind = match self {
            Action::Sequence(act) => Value::SequenceAction(act.to_raw()),
            Action::Transfer(act) => Value::TransferAction(act.to_raw()),
            Action::ValidatorUpdate(act) => Value::ValidatorUpdateAction(act.to_raw()),
            Action::SudoAddressChange(act) => {
                Value::SudoAddressChangeAction(act.clone().into_raw())
            }
            Action::Ibc(act) => Value::IbcAction(act.clone().into()),
            Action::IbcSudoChange(act) => Value::IbcSudoChangeAction(act.clone().into_raw()),
            Action::Ics20Withdrawal(act) => Value::Ics20Withdrawal(act.to_raw()),
            Action::IbcRelayerChange(act) => Value::IbcRelayerChangeAction(act.to_raw()),
            Action::FeeAssetChange(act) => Value::FeeAssetChangeAction(act.to_raw()),
            Action::InitBridgeAccount(act) => Value::InitBridgeAccountAction(act.to_raw()),
            Action::BridgeLock(act) => Value::BridgeLockAction(act.to_raw()),
            Action::BridgeUnlock(act) => Value::BridgeUnlockAction(act.to_raw()),
            Action::BridgeSudoChange(act) => Value::BridgeSudoChangeAction(act.to_raw()),
            Action::FeeChange(act) => Value::FeeChangeAction(act.to_raw()),
        };
        raw::Action {
            value: Some(kind),
        }
    }

    /// Attempt to convert from a reference to raw, unchecked protobuf [`raw::Action`].
    ///
    /// # Errors
    ///
    /// Returns an error if conversion of one of the inner raw action variants
    /// to a native action ([`SequenceAction`] or [`TransferAction`]) fails.
    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, ActionError> {
        Self::try_from_raw(raw.clone())
    }

    /// Attempt to convert from a raw, unchecked protobuf [`raw::Action`].
    ///
    /// # Errors
    ///
    /// Returns an error if conversion of one of the inner raw action variants
    /// to a native action ([`SequenceAction`] or [`TransferAction`]) fails.
    fn try_from_raw(proto: raw::Action) -> Result<Self, ActionError> {
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
            Value::ValidatorUpdateAction(act) => Self::ValidatorUpdate(
                ValidatorUpdate::try_from_raw(act).map_err(ActionError::validator_update)?,
            ),
            Value::SudoAddressChangeAction(act) => Self::SudoAddressChange(
                SudoAddressChangeAction::try_from_raw(act)
                    .map_err(ActionError::sudo_address_change)?,
            ),
            Value::IbcSudoChangeAction(act) => Self::IbcSudoChange(
                IbcSudoChangeAction::try_from_raw(act).map_err(ActionError::ibc_sudo_change)?,
            ),
            Value::IbcAction(act) => {
                Self::Ibc(IbcRelay::try_from(act).map_err(|e| ActionError::ibc(e.into()))?)
            }
            Value::Ics20Withdrawal(act) => Self::Ics20Withdrawal(
                Ics20Withdrawal::try_from_raw(act).map_err(ActionError::ics20_withdrawal)?,
            ),
            Value::IbcRelayerChangeAction(act) => Self::IbcRelayerChange(
                IbcRelayerChangeAction::try_from_raw_ref(&act)
                    .map_err(ActionError::ibc_relayer_change)?,
            ),
            Value::FeeAssetChangeAction(act) => Self::FeeAssetChange(
                FeeAssetChangeAction::try_from_raw_ref(&act)
                    .map_err(ActionError::fee_asset_change)?,
            ),
            Value::InitBridgeAccountAction(act) => Self::InitBridgeAccount(
                InitBridgeAccountAction::try_from_raw(act)
                    .map_err(ActionError::init_bridge_account)?,
            ),
            Value::BridgeLockAction(act) => Self::BridgeLock(
                BridgeLockAction::try_from_raw(act).map_err(ActionError::bridge_lock)?,
            ),
            Value::BridgeUnlockAction(act) => Self::BridgeUnlock(
                BridgeUnlockAction::try_from_raw(act).map_err(ActionError::bridge_unlock)?,
            ),
            Value::BridgeSudoChangeAction(act) => Self::BridgeSudoChange(
                BridgeSudoChangeAction::try_from_raw(act)
                    .map_err(ActionError::bridge_sudo_change)?,
            ),
            Value::FeeChangeAction(act) => Self::FeeChange(
                FeeChangeAction::try_from_raw_ref(&act).map_err(ActionError::fee_change)?,
            ),
        };
        Ok(action)
    }
}

impl Action {
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

impl From<IbcSudoChangeAction> for Action {
    fn from(value: IbcSudoChangeAction) -> Self {
        Self::IbcSudoChange(value)
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

impl From<BridgeUnlockAction> for Action {
    fn from(value: BridgeUnlockAction) -> Self {
        Self::BridgeUnlock(value)
    }
}

impl From<BridgeSudoChangeAction> for Action {
    fn from(value: BridgeSudoChangeAction) -> Self {
        Self::BridgeSudoChange(value)
    }
}

impl From<FeeChangeAction> for Action {
    fn from(value: FeeChangeAction) -> Self {
        Self::FeeChange(value)
    }
}

impl From<Action> for raw::Action {
    fn from(value: Action) -> Self {
        value.into_raw()
    }
}

impl TryFrom<raw::Action> for Action {
    type Error = ActionError;

    fn try_from(value: raw::Action) -> Result<Self, Self::Error> {
        Self::try_from_raw(value)
    }
}

#[expect(
    clippy::module_name_repetitions,
    reason = "for parity with the Protobuf spec"
)]
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

    fn validator_update(inner: ValidatorUpdateError) -> Self {
        Self(ActionErrorKind::ValidatorUpdate(inner))
    }

    fn sudo_address_change(inner: SudoAddressChangeActionError) -> Self {
        Self(ActionErrorKind::SudoAddressChange(inner))
    }

    fn ibc_sudo_change(inner: IbcSudoChangeActionError) -> Self {
        Self(ActionErrorKind::IbcSudoChange(inner))
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

    fn bridge_unlock(inner: BridgeUnlockActionError) -> Self {
        Self(ActionErrorKind::BridgeUnlock(inner))
    }

    fn bridge_sudo_change(inner: BridgeSudoChangeActionError) -> Self {
        Self(ActionErrorKind::BridgeSudoChange(inner))
    }

    fn fee_change(inner: FeeChangeActionError) -> Self {
        Self(ActionErrorKind::FeeChange(inner))
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
    ValidatorUpdate(#[source] ValidatorUpdateError),
    #[error("sudo address change action was not valid")]
    SudoAddressChange(#[source] SudoAddressChangeActionError),
    #[error("ibc sudo address change action was not valid")]
    IbcSudoChange(#[source] IbcSudoChangeActionError),
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
    #[error("bridge unlock action was not valid")]
    BridgeUnlock(#[source] BridgeUnlockActionError),
    #[error("bridge sudo change action was not valid")]
    BridgeSudoChange(#[source] BridgeSudoChangeActionError),
    #[error("fee change action was not valid")]
    FeeChange(#[source] FeeChangeActionError),
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct SequenceActionError(SequenceActionErrorKind);

impl SequenceActionError {
    fn field_not_set(field: &'static str) -> Self {
        Self(SequenceActionErrorKind::FieldNotSet(field))
    }

    fn rollup_id_length(inner: IncorrectRollupIdLength) -> Self {
        Self(SequenceActionErrorKind::RollupIdLength(inner))
    }

    fn fee_asset(inner: asset::ParseDenomError) -> Self {
        Self(SequenceActionErrorKind::FeeAsset(inner))
    }
}

#[derive(Debug, thiserror::Error)]
enum SequenceActionErrorKind {
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("`rollup_id` field did not contain a valid rollup ID")]
    RollupIdLength(IncorrectRollupIdLength),
    #[error("`fee_asset` field did not contain a valid asset ID")]
    FeeAsset(#[source] asset::ParseDenomError),
}

#[derive(Clone, Debug)]
#[expect(
    clippy::module_name_repetitions,
    reason = "for parity with the Protobuf spec"
)]
pub struct SequenceAction {
    pub rollup_id: RollupId,
    pub data: Bytes,
    /// asset to use for fee payment.
    pub fee_asset: asset::Denom,
}

impl Protobuf for SequenceAction {
    type Error = SequenceActionError;
    type Raw = raw::SequenceAction;

    #[must_use]
    fn to_raw(&self) -> raw::SequenceAction {
        let Self {
            rollup_id,
            data,
            fee_asset,
        } = self;
        raw::SequenceAction {
            rollup_id: Some(rollup_id.to_raw()),
            data: data.clone(),
            fee_asset: fee_asset.to_string(),
        }
    }

    /// Convert from a reference to the raw protobuf type.
    ///
    /// # Errors
    /// Returns `SequenceActionError` if the `proto.rollup_id` field was not 32 bytes.
    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let raw::SequenceAction {
            rollup_id,
            data,
            fee_asset,
        } = raw;
        let Some(rollup_id) = rollup_id else {
            return Err(SequenceActionError::field_not_set("rollup_id"));
        };
        let rollup_id =
            RollupId::try_from_raw(rollup_id).map_err(SequenceActionError::rollup_id_length)?;
        let fee_asset = fee_asset.parse().map_err(SequenceActionError::fee_asset)?;
        let data = data.clone();
        Ok(Self {
            rollup_id,
            data,
            fee_asset,
        })
    }
}

#[derive(Clone, Debug)]
#[expect(
    clippy::module_name_repetitions,
    reason = "for parity with the Protobuf spec"
)]
pub struct TransferAction {
    pub to: Address,
    pub amount: u128,
    /// asset to be transferred.
    pub asset: asset::Denom,
    /// asset to use for fee payment.
    pub fee_asset: asset::Denom,
}

impl Protobuf for TransferAction {
    type Error = TransferActionError;
    type Raw = raw::TransferAction;

    #[must_use]
    fn to_raw(&self) -> raw::TransferAction {
        let Self {
            to,
            amount,
            asset,
            fee_asset,
        } = self;
        raw::TransferAction {
            to: Some(to.to_raw()),
            amount: Some((*amount).into()),
            asset: asset.to_string(),
            fee_asset: fee_asset.to_string(),
        }
    }

    /// Convert from a reference to the raw protobuf type.
    ///
    /// # Errors
    /// Returns `TransferActionError` if the raw action's `to` address did not have the expected
    /// length.
    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let raw::TransferAction {
            to,
            amount,
            asset,
            fee_asset,
        } = raw;
        let Some(to) = to else {
            return Err(TransferActionError::field_not_set("to"));
        };
        let to = Address::try_from_raw(to).map_err(TransferActionError::address)?;
        let amount = amount.map_or(0, Into::into);
        let asset = asset.parse().map_err(TransferActionError::asset)?;
        let fee_asset = fee_asset.parse().map_err(TransferActionError::fee_asset)?;

        Ok(Self {
            to,
            amount,
            asset,
            fee_asset,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct TransferActionError(TransferActionErrorKind);

impl TransferActionError {
    fn field_not_set(field: &'static str) -> Self {
        Self(TransferActionErrorKind::FieldNotSet(field))
    }

    fn address(inner: AddressError) -> Self {
        Self(TransferActionErrorKind::Address(inner))
    }

    fn asset(inner: asset::ParseDenomError) -> Self {
        Self(TransferActionErrorKind::Asset(inner))
    }

    fn fee_asset(inner: asset::ParseDenomError) -> Self {
        Self(TransferActionErrorKind::FeeAsset(inner))
    }
}

#[derive(Debug, thiserror::Error)]
enum TransferActionErrorKind {
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("`to` field did not contain a valid address")]
    Address(#[source] AddressError),
    #[error("`asset` field did not contain a valid asset ID")]
    Asset(#[source] asset::ParseDenomError),
    #[error("`fee_asset` field did not contain a valid asset ID")]
    FeeAsset(#[source] asset::ParseDenomError),
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ValidatorUpdateError(ValidatorUpdateErrorKind);

impl ValidatorUpdateError {
    fn negative_power(power: i64) -> Self {
        Self(ValidatorUpdateErrorKind::NegativePower {
            power,
        })
    }

    fn public_key_not_set() -> Self {
        Self(ValidatorUpdateErrorKind::PublicKeyNotSet)
    }

    fn secp256k1_not_supported() -> Self {
        Self(ValidatorUpdateErrorKind::Secp256k1NotSupported)
    }

    fn verification_key(source: crate::crypto::Error) -> Self {
        Self(ValidatorUpdateErrorKind::VerificationKey {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum ValidatorUpdateErrorKind {
    #[error("field .power had negative value `{power}`, which is not permitted")]
    NegativePower { power: i64 },
    #[error(".pub_key field was not set")]
    PublicKeyNotSet,
    #[error(".pub_key field was set to secp256k1, but only ed25519 keys are supported")]
    Secp256k1NotSupported,
    #[error("bytes stored in the .pub_key field could not be read as an ed25519 verification key")]
    VerificationKey { source: crate::crypto::Error },
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Deserialize, ::serde::Serialize),
    serde(
        into = "crate::generated::astria_vendored::tendermint::abci::ValidatorUpdate",
        try_from = "crate::generated::astria_vendored::tendermint::abci::ValidatorUpdate",
    )
)]
pub struct ValidatorUpdate {
    pub power: u32,
    pub verification_key: crate::crypto::VerificationKey,
}

impl Protobuf for ValidatorUpdate {
    type Error = ValidatorUpdateError;
    type Raw = crate::generated::astria_vendored::tendermint::abci::ValidatorUpdate;

    /// Create a validator update by verifying a raw protobuf-decoded
    /// [`crate::generated::astria_vendored::tendermint::abci::ValidatorUpdate`].
    ///
    /// # Errors
    /// Returns an error if the `.power` field is negative, if `.pub_key`
    /// is not set, or if `.pub_key` contains a non-ed25519 variant, or
    /// if the ed25519 has invalid bytes (that is, bytes from which an
    /// ed25519 public key cannot be constructed).
    fn try_from_raw(
        value: crate::generated::astria_vendored::tendermint::abci::ValidatorUpdate,
    ) -> Result<Self, ValidatorUpdateError> {
        use crate::generated::astria_vendored::tendermint::crypto::{
            public_key,
            PublicKey,
        };
        let crate::generated::astria_vendored::tendermint::abci::ValidatorUpdate {
            pub_key,
            power,
        } = value;
        let power = power
            .try_into()
            .map_err(|_| ValidatorUpdateError::negative_power(power))?;
        let verification_key = match pub_key {
            None
            | Some(PublicKey {
                sum: None,
            }) => Err(ValidatorUpdateError::public_key_not_set()),
            Some(PublicKey {
                sum: Some(public_key::Sum::Secp256k1(..)),
            }) => Err(ValidatorUpdateError::secp256k1_not_supported()),

            Some(PublicKey {
                sum: Some(public_key::Sum::Ed25519(bytes)),
            }) => crate::crypto::VerificationKey::try_from(&*bytes)
                .map_err(ValidatorUpdateError::verification_key),
        }?;
        Ok(Self {
            power,
            verification_key,
        })
    }

    /// Create a validator update by verifying a reference to raw protobuf-decoded
    /// [`crate::generated::astria_vendored::tendermint::abci::ValidatorUpdate`].
    ///
    /// # Errors
    /// Returns an error if the `.power` field is negative, if `.pub_key`
    /// is not set, or if `.pub_key` contains a non-ed25519 variant, or
    /// if the ed25519 has invalid bytes (that is, bytes from which an
    /// ed25519 public key cannot be constructed).
    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, ValidatorUpdateError> {
        Self::try_from_raw(raw.clone())
    }

    #[must_use]
    fn to_raw(&self) -> crate::generated::astria_vendored::tendermint::abci::ValidatorUpdate {
        use crate::generated::astria_vendored::tendermint::crypto::{
            public_key,
            PublicKey,
        };
        let Self {
            power,
            verification_key,
        } = self;

        crate::generated::astria_vendored::tendermint::abci::ValidatorUpdate {
            power: (*power).into(),
            pub_key: Some(PublicKey {
                sum: Some(public_key::Sum::Ed25519(
                    verification_key.to_bytes().to_vec(),
                )),
            }),
        }
    }
}

impl From<ValidatorUpdate>
    for crate::generated::astria_vendored::tendermint::abci::ValidatorUpdate
{
    fn from(value: ValidatorUpdate) -> Self {
        value.into_raw()
    }
}

impl TryFrom<crate::generated::astria_vendored::tendermint::abci::ValidatorUpdate>
    for ValidatorUpdate
{
    type Error = ValidatorUpdateError;

    fn try_from(
        value: crate::generated::astria_vendored::tendermint::abci::ValidatorUpdate,
    ) -> Result<Self, Self::Error> {
        Self::try_from_raw(value)
    }
}

#[derive(Clone, Debug)]
#[expect(
    clippy::module_name_repetitions,
    reason = "for parity with the Protobuf spec"
)]
pub struct SudoAddressChangeAction {
    pub new_address: Address,
}

impl Protobuf for SudoAddressChangeAction {
    type Error = SudoAddressChangeActionError;
    type Raw = raw::SudoAddressChangeAction;

    fn into_raw(self) -> raw::SudoAddressChangeAction {
        let Self {
            new_address,
        } = self;
        raw::SudoAddressChangeAction {
            new_address: Some(new_address.into_raw()),
        }
    }

    #[must_use]
    fn to_raw(&self) -> raw::SudoAddressChangeAction {
        let Self {
            new_address,
        } = self;
        raw::SudoAddressChangeAction {
            new_address: Some(new_address.to_raw()),
        }
    }

    /// Convert from a reference to a raw, unchecked protobuf [`raw::SudoAddressChangeAction`].
    ///
    /// # Errors
    ///
    /// Returns an error if the raw action's `new_address` did not have the expected
    /// length.
    fn try_from_raw_ref(proto: &Self::Raw) -> Result<Self, SudoAddressChangeActionError> {
        let raw::SudoAddressChangeAction {
            new_address,
        } = proto;
        let Some(new_address) = new_address else {
            return Err(SudoAddressChangeActionError::field_not_set("new_address"));
        };
        let new_address =
            Address::try_from_raw(new_address).map_err(SudoAddressChangeActionError::address)?;
        Ok(Self {
            new_address,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct SudoAddressChangeActionError(SudoAddressChangeActionErrorKind);

impl SudoAddressChangeActionError {
    fn field_not_set(field: &'static str) -> Self {
        Self(SudoAddressChangeActionErrorKind::FieldNotSet(field))
    }

    fn address(source: AddressError) -> Self {
        Self(SudoAddressChangeActionErrorKind::Address {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum SudoAddressChangeActionErrorKind {
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("`new_address` field did not contain a valid address")]
    Address { source: AddressError },
}

#[derive(Debug, Clone)]
#[expect(
    clippy::module_name_repetitions,
    reason = "for parity with the Protobuf spec"
)]
pub struct IbcSudoChangeAction {
    pub new_address: Address,
}

impl Protobuf for IbcSudoChangeAction {
    type Error = IbcSudoChangeActionError;
    type Raw = raw::IbcSudoChangeAction;

    fn into_raw(self) -> raw::IbcSudoChangeAction {
        raw::IbcSudoChangeAction {
            new_address: Some(self.new_address.into_raw()),
        }
    }

    #[must_use]
    fn to_raw(&self) -> raw::IbcSudoChangeAction {
        raw::IbcSudoChangeAction {
            new_address: Some(self.new_address.to_raw()),
        }
    }

    /// Convert from a reference to a raw, unchecked protobuf [`raw::IbcSudoChangeAction`].
    ///
    /// # Errors
    ///
    /// Returns an error if the raw action's `new_address` did not have the expected
    /// length or if the field was not set.
    fn try_from_raw_ref(proto: &Self::Raw) -> Result<Self, IbcSudoChangeActionError> {
        let raw::IbcSudoChangeAction {
            new_address,
        } = proto;
        let Some(new_address) = new_address else {
            return Err(IbcSudoChangeActionError::field_not_set("new_address"));
        };
        let new_address =
            Address::try_from_raw(new_address).map_err(IbcSudoChangeActionError::address)?;
        Ok(Self {
            new_address,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct IbcSudoChangeActionError(IbcSudoChangeActionErrorKind);

impl IbcSudoChangeActionError {
    fn field_not_set(field: &'static str) -> Self {
        Self(IbcSudoChangeActionErrorKind::FieldNotSet(field))
    }

    fn address(source: AddressError) -> Self {
        Self(IbcSudoChangeActionErrorKind::Address {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum IbcSudoChangeActionErrorKind {
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("`new_sudo` field did not contain a valid address")]
    Address { source: AddressError },
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ics20Withdrawal {
    // a transparent value consisting of an amount and a denom.
    pub amount: u128,
    pub denom: Denom,
    // the address on the destination chain to send the transfer to.
    pub destination_chain_address: String,
    // an Astria address to use to return funds from this withdrawal
    // in the case it fails.
    pub return_address: Address,
    // the height (on Astria) at which this transfer expires.
    pub timeout_height: IbcHeight,
    // the unix timestamp (in nanoseconds) at which this transfer expires.
    pub timeout_time: u64,
    // the source channel used for the withdrawal.
    pub source_channel: ChannelId,
    // the asset to use for fee payment.
    pub fee_asset: asset::Denom,
    // a memo to include with the transfer
    pub memo: String,
    // the address of the bridge account to transfer from, if this is a withdrawal
    // from a bridge account and the sender of the tx is the bridge's withdrawer,
    // which differs from the bridge account's address.
    //
    // if unset, and the transaction sender is not a bridge account, the withdrawal
    // is treated as a user (non-bridge) withdrawal.
    //
    // if unset, and the transaction sender is a bridge account, the withdrawal is
    // treated as a bridge withdrawal (ie. the bridge account's withdrawer address is checked).
    pub bridge_address: Option<Address>,

    // whether to use a bech32-compatible format of the `.return_address` when generating
    // fungible token packets (as opposed to Astria-native bech32m addresses). This is
    // necessary for chains like noble which enforce a strict bech32 format.
    pub use_compat_address: bool,
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
    pub fn fee_asset(&self) -> &asset::Denom {
        &self.fee_asset
    }

    #[must_use]
    pub fn memo(&self) -> &str {
        &self.memo
    }
}

impl Protobuf for Ics20Withdrawal {
    type Error = Ics20WithdrawalError;
    type Raw = raw::Ics20Withdrawal;

    #[must_use]
    fn to_raw(&self) -> raw::Ics20Withdrawal {
        raw::Ics20Withdrawal {
            amount: Some(self.amount.into()),
            denom: self.denom.to_string(),
            destination_chain_address: self.destination_chain_address.clone(),
            return_address: Some(self.return_address.into_raw()),
            timeout_height: Some(self.timeout_height.into_raw()),
            timeout_time: self.timeout_time,
            source_channel: self.source_channel.to_string(),
            fee_asset: self.fee_asset.to_string(),
            memo: self.memo.clone(),
            bridge_address: self.bridge_address.as_ref().map(Address::to_raw),
            use_compat_address: self.use_compat_address,
        }
    }

    #[must_use]
    fn into_raw(self) -> raw::Ics20Withdrawal {
        raw::Ics20Withdrawal {
            amount: Some(self.amount.into()),
            denom: self.denom.to_string(),
            destination_chain_address: self.destination_chain_address,
            return_address: Some(self.return_address.into_raw()),
            timeout_height: Some(self.timeout_height.into_raw()),
            timeout_time: self.timeout_time,
            source_channel: self.source_channel.to_string(),
            fee_asset: self.fee_asset.to_string(),
            memo: self.memo,
            bridge_address: self.bridge_address.map(Address::into_raw),
            use_compat_address: self.use_compat_address,
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::Ics20Withdrawal`].
    ///
    /// # Errors
    ///
    /// - if the `amount` field is missing
    /// - if the `denom` field is invalid
    /// - if the `return_address` field is invalid or missing
    /// - if the `timeout_height` field is missing
    /// - if the `source_channel` field is invalid
    fn try_from_raw(proto: raw::Ics20Withdrawal) -> Result<Self, Ics20WithdrawalError> {
        let raw::Ics20Withdrawal {
            amount,
            denom,
            destination_chain_address,
            return_address,
            timeout_height,
            timeout_time,
            source_channel,
            fee_asset,
            memo,
            bridge_address,
            use_compat_address,
        } = proto;
        let amount = amount.ok_or(Ics20WithdrawalError::field_not_set("amount"))?;
        let return_address = Address::try_from_raw(
            &return_address.ok_or(Ics20WithdrawalError::field_not_set("return_address"))?,
        )
        .map_err(Ics20WithdrawalError::return_address)?;

        let timeout_height = timeout_height
            .ok_or(Ics20WithdrawalError::field_not_set("timeout_height"))?
            .into();
        let bridge_address = bridge_address
            .as_ref()
            .map(Address::try_from_raw)
            .transpose()
            .map_err(Ics20WithdrawalError::invalid_bridge_address)?;

        Ok(Self {
            amount: amount.into(),
            denom: denom.parse().map_err(Ics20WithdrawalError::invalid_denom)?,
            destination_chain_address,
            return_address,
            timeout_height,
            timeout_time,
            source_channel: source_channel
                .parse()
                .map_err(Ics20WithdrawalError::invalid_source_channel)?,
            fee_asset: fee_asset
                .parse()
                .map_err(Ics20WithdrawalError::invalid_fee_asset)?,
            memo,
            bridge_address,
            use_compat_address,
        })
    }

    /// Convert from a reference to raw, unchecked protobuf [`raw::Ics20Withdrawal`].
    ///
    /// # Errors
    ///
    /// - if the `amount` field is missing
    /// - if the `denom` field is invalid
    /// - if the `return_address` field is invalid or missing
    /// - if the `timeout_height` field is missing
    /// - if the `source_channel` field is invalid
    fn try_from_raw_ref(proto: &raw::Ics20Withdrawal) -> Result<Self, Ics20WithdrawalError> {
        let raw::Ics20Withdrawal {
            amount,
            denom,
            destination_chain_address,
            return_address,
            timeout_height,
            timeout_time,
            source_channel,
            fee_asset,
            memo,
            bridge_address,
            use_compat_address,
        } = proto;
        let amount = amount.ok_or(Ics20WithdrawalError::field_not_set("amount"))?;
        let return_address = Address::try_from_raw(
            return_address
                .as_ref()
                .ok_or(Ics20WithdrawalError::field_not_set("return_address"))?,
        )
        .map_err(Ics20WithdrawalError::return_address)?;

        let timeout_height = timeout_height
            .clone()
            .ok_or(Ics20WithdrawalError::field_not_set("timeout_height"))?
            .into();
        let bridge_address = bridge_address
            .as_ref()
            .map(Address::try_from_raw)
            .transpose()
            .map_err(Ics20WithdrawalError::invalid_bridge_address)?;

        Ok(Self {
            amount: amount.into(),
            denom: denom.parse().map_err(Ics20WithdrawalError::invalid_denom)?,
            destination_chain_address: destination_chain_address.clone(),
            return_address,
            timeout_height,
            timeout_time: *timeout_time,
            source_channel: source_channel
                .parse()
                .map_err(Ics20WithdrawalError::invalid_source_channel)?,
            fee_asset: fee_asset
                .parse()
                .map_err(Ics20WithdrawalError::invalid_fee_asset)?,
            memo: memo.clone(),
            bridge_address,
            use_compat_address: *use_compat_address,
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
    fn field_not_set(field: &'static str) -> Self {
        Self(Ics20WithdrawalErrorKind::FieldNotSet {
            field,
        })
    }

    #[must_use]
    fn return_address(source: AddressError) -> Self {
        Self(Ics20WithdrawalErrorKind::ReturnAddress {
            source,
        })
    }

    #[must_use]
    fn invalid_source_channel(err: IdentifierError) -> Self {
        Self(Ics20WithdrawalErrorKind::InvalidSourceChannel(err))
    }

    #[must_use]
    fn invalid_fee_asset(err: asset::ParseDenomError) -> Self {
        Self(Ics20WithdrawalErrorKind::InvalidFeeAsset(err))
    }

    #[must_use]
    fn invalid_bridge_address(err: AddressError) -> Self {
        Self(Ics20WithdrawalErrorKind::InvalidBridgeAddress(err))
    }

    fn invalid_denom(source: asset::ParseDenomError) -> Self {
        Self(Ics20WithdrawalErrorKind::InvalidDenom {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum Ics20WithdrawalErrorKind {
    #[error("expected field `{field}` was not set`")]
    FieldNotSet { field: &'static str },
    #[error("`return_address` field was invalid")]
    ReturnAddress { source: AddressError },
    #[error("`source_channel` field was invalid")]
    InvalidSourceChannel(#[source] IdentifierError),
    #[error("field `fee_asset` could not be parsed")]
    InvalidFeeAsset(#[source] asset::ParseDenomError),
    #[error("`bridge_address` field was invalid")]
    InvalidBridgeAddress(#[source] AddressError),
    #[error("`denom` field was invalid")]
    InvalidDenom { source: asset::ParseDenomError },
}

#[expect(
    clippy::module_name_repetitions,
    reason = "for parity with the Protobuf spec"
)]
#[derive(Debug, Clone)]
pub enum IbcRelayerChangeAction {
    Addition(Address),
    Removal(Address),
}

impl Protobuf for IbcRelayerChangeAction {
    type Error = IbcRelayerChangeActionError;
    type Raw = raw::IbcRelayerChangeAction;

    #[must_use]
    fn to_raw(&self) -> raw::IbcRelayerChangeAction {
        match self {
            IbcRelayerChangeAction::Addition(address) => raw::IbcRelayerChangeAction {
                value: Some(raw::ibc_relayer_change_action::Value::Addition(
                    address.to_raw(),
                )),
            },
            IbcRelayerChangeAction::Removal(address) => raw::IbcRelayerChangeAction {
                value: Some(raw::ibc_relayer_change_action::Value::Removal(
                    address.to_raw(),
                )),
            },
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::IbcRelayerChangeAction`].
    ///
    /// # Errors
    ///
    /// - if the `address` field is invalid
    fn try_from_raw_ref(
        raw: &raw::IbcRelayerChangeAction,
    ) -> Result<Self, IbcRelayerChangeActionError> {
        match raw {
            raw::IbcRelayerChangeAction {
                value: Some(raw::ibc_relayer_change_action::Value::Addition(address)),
            } => {
                let address =
                    Address::try_from_raw(address).map_err(IbcRelayerChangeActionError::address)?;
                Ok(IbcRelayerChangeAction::Addition(address))
            }
            raw::IbcRelayerChangeAction {
                value: Some(raw::ibc_relayer_change_action::Value::Removal(address)),
            } => {
                let address =
                    Address::try_from_raw(address).map_err(IbcRelayerChangeActionError::address)?;
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
    fn address(source: AddressError) -> Self {
        Self(IbcRelayerChangeActionErrorKind::Address {
            source,
        })
    }

    #[must_use]
    fn missing_address() -> Self {
        Self(IbcRelayerChangeActionErrorKind::MissingAddress)
    }
}

#[derive(Debug, thiserror::Error)]
enum IbcRelayerChangeActionErrorKind {
    #[error("the `address` was invalid")]
    Address { source: AddressError },
    #[error("the `address` was not set")]
    MissingAddress,
}

#[expect(
    clippy::module_name_repetitions,
    reason = "for parity with the Protobuf spec"
)]
#[derive(Debug, Clone)]
pub enum FeeAssetChangeAction {
    Addition(asset::Denom),
    Removal(asset::Denom),
}

impl Protobuf for FeeAssetChangeAction {
    type Error = FeeAssetChangeActionError;
    type Raw = raw::FeeAssetChangeAction;

    #[must_use]
    fn to_raw(&self) -> raw::FeeAssetChangeAction {
        match self {
            FeeAssetChangeAction::Addition(asset) => raw::FeeAssetChangeAction {
                value: Some(raw::fee_asset_change_action::Value::Addition(
                    asset.to_string(),
                )),
            },
            FeeAssetChangeAction::Removal(asset) => raw::FeeAssetChangeAction {
                value: Some(raw::fee_asset_change_action::Value::Removal(
                    asset.to_string(),
                )),
            },
        }
    }

    /// Convert from a reference to a raw, unchecked protobuf [`raw::FeeAssetChangeAction`].
    ///
    /// # Errors
    ///
    /// - if the `asset` field is invalid
    fn try_from_raw_ref(
        raw: &raw::FeeAssetChangeAction,
    ) -> Result<Self, FeeAssetChangeActionError> {
        match raw {
            raw::FeeAssetChangeAction {
                value: Some(raw::fee_asset_change_action::Value::Addition(asset)),
            } => {
                let asset = asset
                    .parse()
                    .map_err(FeeAssetChangeActionError::invalid_asset)?;
                Ok(FeeAssetChangeAction::Addition(asset))
            }
            raw::FeeAssetChangeAction {
                value: Some(raw::fee_asset_change_action::Value::Removal(asset)),
            } => {
                let asset = asset
                    .parse()
                    .map_err(FeeAssetChangeActionError::invalid_asset)?;
                Ok(FeeAssetChangeAction::Removal(asset))
            }
            _ => Err(FeeAssetChangeActionError::missing_asset()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct FeeAssetChangeActionError(FeeAssetChangeActionErrorKind);

impl FeeAssetChangeActionError {
    #[must_use]
    fn invalid_asset(err: asset::ParseDenomError) -> Self {
        Self(FeeAssetChangeActionErrorKind::InvalidAsset(err))
    }

    #[must_use]
    fn missing_asset() -> Self {
        Self(FeeAssetChangeActionErrorKind::MissingAsset)
    }
}

#[derive(Debug, thiserror::Error)]
enum FeeAssetChangeActionErrorKind {
    #[error("the `asset` field was invalid")]
    InvalidAsset(#[source] asset::ParseDenomError),
    #[error("the `asset` field was not set")]
    MissingAsset,
}

#[expect(
    clippy::module_name_repetitions,
    reason = "for parity with the Protobuf spec"
)]
#[derive(Debug, Clone)]
pub struct InitBridgeAccountAction {
    // the rollup ID to register for the sender of this action
    pub rollup_id: RollupId,
    // the assets accepted by the bridge account
    pub asset: asset::Denom,
    // the fee asset which to pay this action's fees with
    pub fee_asset: asset::Denom,
    // the address corresponding to the key which has sudo capabilities;
    // ie. can change the sudo and withdrawer addresses for this bridge account.
    // if unset, this is set to the sender of the transaction.
    pub sudo_address: Option<Address>,
    // the address corresponding to the key which can withdraw funds from this bridge account.
    // if unset, this is set to the sender of the transaction.
    pub withdrawer_address: Option<Address>,
}

impl Protobuf for InitBridgeAccountAction {
    type Error = InitBridgeAccountActionError;
    type Raw = raw::InitBridgeAccountAction;

    #[must_use]
    fn into_raw(self) -> raw::InitBridgeAccountAction {
        raw::InitBridgeAccountAction {
            rollup_id: Some(self.rollup_id.to_raw()),
            asset: self.asset.to_string(),
            fee_asset: self.fee_asset.to_string(),
            sudo_address: self.sudo_address.map(Address::into_raw),
            withdrawer_address: self.withdrawer_address.map(Address::into_raw),
        }
    }

    #[must_use]
    fn to_raw(&self) -> raw::InitBridgeAccountAction {
        raw::InitBridgeAccountAction {
            rollup_id: Some(self.rollup_id.to_raw()),
            asset: self.asset.to_string(),
            fee_asset: self.fee_asset.to_string(),
            sudo_address: self.sudo_address.as_ref().map(Address::to_raw),
            withdrawer_address: self.withdrawer_address.as_ref().map(Address::to_raw),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::InitBridgeAccountAction`].
    ///
    /// # Errors
    ///
    /// - if the `rollup_id` field is not set
    /// - if the `rollup_id` field is invalid
    /// - if the `sudo_address` field is invalid
    /// - if the `withdrawer_address` field is invalid
    fn try_from_raw(
        proto: raw::InitBridgeAccountAction,
    ) -> Result<Self, InitBridgeAccountActionError> {
        let Some(rollup_id) = proto.rollup_id else {
            return Err(InitBridgeAccountActionError::field_not_set("rollup_id"));
        };
        let rollup_id = RollupId::try_from_raw(&rollup_id)
            .map_err(InitBridgeAccountActionError::invalid_rollup_id)?;
        let asset = proto
            .asset
            .parse()
            .map_err(InitBridgeAccountActionError::invalid_asset)?;
        let fee_asset = proto
            .fee_asset
            .parse()
            .map_err(InitBridgeAccountActionError::invalid_fee_asset)?;
        let sudo_address = proto
            .sudo_address
            .as_ref()
            .map(Address::try_from_raw)
            .transpose()
            .map_err(InitBridgeAccountActionError::invalid_sudo_address)?;
        let withdrawer_address = proto
            .withdrawer_address
            .as_ref()
            .map(Address::try_from_raw)
            .transpose()
            .map_err(InitBridgeAccountActionError::invalid_withdrawer_address)?;

        Ok(Self {
            rollup_id,
            asset,
            fee_asset,
            sudo_address,
            withdrawer_address,
        })
    }

    /// Convert from a reference to a raw, unchecked protobuf [`raw::InitBridgeAccountAction`].
    ///
    /// # Errors
    ///
    /// - if the `rollup_id` field is not set
    /// - if the `rollup_id` field is invalid
    /// - if the `sudo_address` field is invalid
    /// - if the `withdrawer_address` field is invalid
    fn try_from_raw_ref(proto: &Self::Raw) -> Result<Self, InitBridgeAccountActionError> {
        Self::try_from_raw(proto.clone())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct InitBridgeAccountActionError(InitBridgeAccountActionErrorKind);

impl InitBridgeAccountActionError {
    #[must_use]
    fn field_not_set(field: &'static str) -> Self {
        Self(InitBridgeAccountActionErrorKind::FieldNotSet(field))
    }

    #[must_use]
    fn invalid_rollup_id(err: IncorrectRollupIdLength) -> Self {
        Self(InitBridgeAccountActionErrorKind::InvalidRollupId(err))
    }

    #[must_use]
    fn invalid_asset(err: asset::ParseDenomError) -> Self {
        Self(InitBridgeAccountActionErrorKind::InvalidAsset(err))
    }

    #[must_use]
    fn invalid_fee_asset(err: asset::ParseDenomError) -> Self {
        Self(InitBridgeAccountActionErrorKind::InvalidFeeAsset(err))
    }

    #[must_use]
    fn invalid_sudo_address(err: AddressError) -> Self {
        Self(InitBridgeAccountActionErrorKind::InvalidSudoAddress(err))
    }

    #[must_use]
    fn invalid_withdrawer_address(err: AddressError) -> Self {
        Self(InitBridgeAccountActionErrorKind::InvalidWithdrawerAddress(
            err,
        ))
    }
}

#[derive(Debug, thiserror::Error)]
enum InitBridgeAccountActionErrorKind {
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("the `rollup_id` field was invalid")]
    InvalidRollupId(#[source] IncorrectRollupIdLength),
    #[error("an asset ID was invalid")]
    InvalidAsset(#[source] asset::ParseDenomError),
    #[error("the `fee_asset` field was invalid")]
    InvalidFeeAsset(#[source] asset::ParseDenomError),
    #[error("the `sudo_address` field was invalid")]
    InvalidSudoAddress(#[source] AddressError),
    #[error("the `withdrawer_address` field was invalid")]
    InvalidWithdrawerAddress(#[source] AddressError),
}

#[expect(
    clippy::module_name_repetitions,
    reason = "for parity with the Protobuf spec"
)]
#[derive(Debug, Clone)]
pub struct BridgeLockAction {
    pub to: Address,
    pub amount: u128,
    // asset to be transferred.
    pub asset: asset::Denom,
    // asset to use for fee payment.
    pub fee_asset: asset::Denom,
    // the address on the destination chain to send the transfer to.
    pub destination_chain_address: String,
}

impl Protobuf for BridgeLockAction {
    type Error = BridgeLockActionError;
    type Raw = raw::BridgeLockAction;

    #[must_use]
    fn into_raw(self) -> raw::BridgeLockAction {
        raw::BridgeLockAction {
            to: Some(self.to.to_raw()),
            amount: Some(self.amount.into()),
            asset: self.asset.to_string(),
            fee_asset: self.fee_asset.to_string(),
            destination_chain_address: self.destination_chain_address,
        }
    }

    #[must_use]
    fn to_raw(&self) -> raw::BridgeLockAction {
        raw::BridgeLockAction {
            to: Some(self.to.to_raw()),
            amount: Some(self.amount.into()),
            asset: self.asset.to_string(),
            fee_asset: self.fee_asset.to_string(),
            destination_chain_address: self.destination_chain_address.clone(),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::BridgeLockAction`].
    ///
    /// # Errors
    ///
    /// - if the `to` field is not set
    /// - if the `to` field is invalid
    /// - if the `asset` field is invalid
    /// - if the `fee_asset` field is invalid
    fn try_from_raw(proto: raw::BridgeLockAction) -> Result<Self, BridgeLockActionError> {
        let Some(to) = proto.to else {
            return Err(BridgeLockActionError::field_not_set("to"));
        };
        let to = Address::try_from_raw(&to).map_err(BridgeLockActionError::address)?;
        let amount = proto
            .amount
            .ok_or(BridgeLockActionError::missing_amount())?;
        let asset = proto
            .asset
            .parse()
            .map_err(BridgeLockActionError::invalid_asset)?;
        let fee_asset = proto
            .fee_asset
            .parse()
            .map_err(BridgeLockActionError::invalid_fee_asset)?;
        Ok(Self {
            to,
            amount: amount.into(),
            asset,
            fee_asset,
            destination_chain_address: proto.destination_chain_address,
        })
    }

    /// Convert from a reference to a raw, unchecked protobuf [`raw::BridgeLockAction`].
    ///
    /// # Errors
    ///
    /// - if the `to` field is not set
    /// - if the `to` field is invalid
    /// - if the `asset` field is invalid
    /// - if the `fee_asset` field is invalid
    fn try_from_raw_ref(proto: &raw::BridgeLockAction) -> Result<Self, BridgeLockActionError> {
        Self::try_from_raw(proto.clone())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct BridgeLockActionError(BridgeLockActionErrorKind);

impl BridgeLockActionError {
    #[must_use]
    fn field_not_set(field: &'static str) -> Self {
        Self(BridgeLockActionErrorKind::FieldNotSet(field))
    }

    #[must_use]
    fn address(source: AddressError) -> Self {
        Self(BridgeLockActionErrorKind::Address {
            source,
        })
    }

    #[must_use]
    fn missing_amount() -> Self {
        Self(BridgeLockActionErrorKind::MissingAmount)
    }

    #[must_use]
    fn invalid_asset(err: asset::ParseDenomError) -> Self {
        Self(BridgeLockActionErrorKind::InvalidAsset(err))
    }

    #[must_use]
    fn invalid_fee_asset(err: asset::ParseDenomError) -> Self {
        Self(BridgeLockActionErrorKind::InvalidFeeAsset(err))
    }
}

#[derive(Debug, thiserror::Error)]
enum BridgeLockActionErrorKind {
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("the `to` field was invalid")]
    Address { source: AddressError },
    #[error("the `amount` field was not set")]
    MissingAmount,
    #[error("the `asset` field was invalid")]
    InvalidAsset(#[source] asset::ParseDenomError),
    #[error("the `fee_asset` field was invalid")]
    InvalidFeeAsset(#[source] asset::ParseDenomError),
}

#[expect(
    clippy::module_name_repetitions,
    reason = "for parity with the Protobuf spec"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeUnlockAction {
    pub to: Address,
    pub amount: u128,
    // asset to use for fee payment.
    pub fee_asset: asset::Denom,
    // the address of the bridge account to transfer from.
    pub bridge_address: Address,
    // A field for users to additional identifying information
    pub memo: String,
    // The block number of the rollup block containing the withdrawal event.
    pub rollup_block_number: u64,
    // The identifier of the withdrawal event in the rollup block.
    pub rollup_withdrawal_event_id: String,
}

impl Protobuf for BridgeUnlockAction {
    type Error = BridgeUnlockActionError;
    type Raw = raw::BridgeUnlockAction;

    #[must_use]
    fn into_raw(self) -> raw::BridgeUnlockAction {
        raw::BridgeUnlockAction {
            to: Some(self.to.into_raw()),
            amount: Some(self.amount.into()),
            fee_asset: self.fee_asset.to_string(),
            memo: self.memo,
            bridge_address: Some(self.bridge_address.into_raw()),
            rollup_block_number: self.rollup_block_number,
            rollup_withdrawal_event_id: self.rollup_withdrawal_event_id,
        }
    }

    #[must_use]
    fn to_raw(&self) -> raw::BridgeUnlockAction {
        raw::BridgeUnlockAction {
            to: Some(self.to.to_raw()),
            amount: Some(self.amount.into()),
            fee_asset: self.fee_asset.to_string(),
            memo: self.memo.clone(),
            bridge_address: Some(self.bridge_address.to_raw()),
            rollup_block_number: self.rollup_block_number,
            rollup_withdrawal_event_id: self.rollup_withdrawal_event_id.clone(),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::BridgeUnlockAction`].
    ///
    /// # Errors
    ///
    /// - if the `to` field is not set
    /// - if the `to` field is invalid
    /// - if the `amount` field is invalid
    /// - if the `fee_asset` field is invalid
    /// - if the `from` field is invalid
    fn try_from_raw(proto: raw::BridgeUnlockAction) -> Result<Self, Self::Error> {
        let raw::BridgeUnlockAction {
            to,
            amount,
            fee_asset,
            memo,
            bridge_address,
            rollup_block_number,
            rollup_withdrawal_event_id,
        } = proto;
        let to = to
            .ok_or_else(|| BridgeUnlockActionError::field_not_set("to"))
            .and_then(|to| Address::try_from_raw(&to).map_err(BridgeUnlockActionError::address))?;
        let amount = amount.ok_or_else(|| BridgeUnlockActionError::field_not_set("amount"))?;
        let fee_asset = fee_asset
            .parse()
            .map_err(BridgeUnlockActionError::fee_asset)?;

        let bridge_address = bridge_address
            .ok_or_else(|| BridgeUnlockActionError::field_not_set("bridge_address"))
            .and_then(|to| {
                Address::try_from_raw(&to).map_err(BridgeUnlockActionError::bridge_address)
            })?;
        Ok(Self {
            to,
            amount: amount.into(),
            fee_asset,
            memo,
            bridge_address,
            rollup_block_number,
            rollup_withdrawal_event_id,
        })
    }

    /// Convert from a reference to a raw, unchecked protobuf [`raw::BridgeUnlockAction`].
    ///
    /// # Errors
    ///
    /// - if the `to` field is not set
    /// - if the `to` field is invalid
    /// - if the `amount` field is invalid
    /// - if the `fee_asset` field is invalid
    /// - if the `from` field is invalid
    fn try_from_raw_ref(proto: &raw::BridgeUnlockAction) -> Result<Self, BridgeUnlockActionError> {
        Self::try_from_raw(proto.clone())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct BridgeUnlockActionError(BridgeUnlockActionErrorKind);

impl BridgeUnlockActionError {
    #[must_use]
    fn field_not_set(field: &'static str) -> Self {
        Self(BridgeUnlockActionErrorKind::FieldNotSet(field))
    }

    #[must_use]
    fn address(source: AddressError) -> Self {
        Self(BridgeUnlockActionErrorKind::Address {
            source,
        })
    }

    #[must_use]
    fn fee_asset(source: asset::ParseDenomError) -> Self {
        Self(BridgeUnlockActionErrorKind::FeeAsset {
            source,
        })
    }

    #[must_use]
    fn bridge_address(source: AddressError) -> Self {
        Self(BridgeUnlockActionErrorKind::BridgeAddress {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum BridgeUnlockActionErrorKind {
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("the `to` field was invalid")]
    Address { source: AddressError },
    #[error("the `fee_asset` field was invalid")]
    FeeAsset { source: asset::ParseDenomError },
    #[error("the `bridge_address` field was invalid")]
    BridgeAddress { source: AddressError },
}

#[expect(
    clippy::module_name_repetitions,
    reason = "for parity with the Protobuf spec"
)]
#[derive(Debug, Clone)]
pub struct BridgeSudoChangeAction {
    pub bridge_address: Address,
    pub new_sudo_address: Option<Address>,
    pub new_withdrawer_address: Option<Address>,
    pub fee_asset: asset::Denom,
}

impl Protobuf for BridgeSudoChangeAction {
    type Error = BridgeSudoChangeActionError;
    type Raw = raw::BridgeSudoChangeAction;

    #[must_use]
    fn into_raw(self) -> raw::BridgeSudoChangeAction {
        raw::BridgeSudoChangeAction {
            bridge_address: Some(self.bridge_address.to_raw()),
            new_sudo_address: self.new_sudo_address.map(Address::into_raw),
            new_withdrawer_address: self.new_withdrawer_address.map(Address::into_raw),
            fee_asset: self.fee_asset.to_string(),
        }
    }

    #[must_use]
    fn to_raw(&self) -> raw::BridgeSudoChangeAction {
        raw::BridgeSudoChangeAction {
            bridge_address: Some(self.bridge_address.to_raw()),
            new_sudo_address: self.new_sudo_address.as_ref().map(Address::to_raw),
            new_withdrawer_address: self.new_withdrawer_address.as_ref().map(Address::to_raw),
            fee_asset: self.fee_asset.to_string(),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::BridgeSudoChangeAction`].
    ///
    /// # Errors
    ///
    /// - if the `bridge_address` field is not set
    /// - if the `bridge_address` field is invalid
    /// - if the `new_sudo_address` field is invalid
    /// - if the `new_withdrawer_address` field is invalid
    /// - if the `fee_asset` field is invalid
    fn try_from_raw(
        proto: raw::BridgeSudoChangeAction,
    ) -> Result<Self, BridgeSudoChangeActionError> {
        let Some(bridge_address) = proto.bridge_address else {
            return Err(BridgeSudoChangeActionError::field_not_set("bridge_address"));
        };
        let bridge_address = Address::try_from_raw(&bridge_address)
            .map_err(BridgeSudoChangeActionError::invalid_bridge_address)?;
        let new_sudo_address = proto
            .new_sudo_address
            .as_ref()
            .map(Address::try_from_raw)
            .transpose()
            .map_err(BridgeSudoChangeActionError::invalid_new_sudo_address)?;
        let new_withdrawer_address = proto
            .new_withdrawer_address
            .as_ref()
            .map(Address::try_from_raw)
            .transpose()
            .map_err(BridgeSudoChangeActionError::invalid_new_withdrawer_address)?;
        let fee_asset = proto
            .fee_asset
            .parse()
            .map_err(BridgeSudoChangeActionError::invalid_fee_asset)?;

        Ok(Self {
            bridge_address,
            new_sudo_address,
            new_withdrawer_address,
            fee_asset,
        })
    }

    /// Convert from a reference to a raw, unchecked protobuf [`raw::BridgeSudoChangeAction`].
    ///
    /// # Errors
    ///
    /// - if the `bridge_address` field is not set
    /// - if the `bridge_address` field is invalid
    /// - if the `new_sudo_address` field is invalid
    /// - if the `new_withdrawer_address` field is invalid
    /// - if the `fee_asset` field is invalid
    fn try_from_raw_ref(
        proto: &raw::BridgeSudoChangeAction,
    ) -> Result<Self, BridgeSudoChangeActionError> {
        Self::try_from_raw(proto.clone())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct BridgeSudoChangeActionError(BridgeSudoChangeActionErrorKind);

impl BridgeSudoChangeActionError {
    #[must_use]
    fn field_not_set(field: &'static str) -> Self {
        Self(BridgeSudoChangeActionErrorKind::FieldNotSet(field))
    }

    #[must_use]
    fn invalid_bridge_address(err: AddressError) -> Self {
        Self(BridgeSudoChangeActionErrorKind::InvalidBridgeAddress(err))
    }

    #[must_use]
    fn invalid_new_sudo_address(err: AddressError) -> Self {
        Self(BridgeSudoChangeActionErrorKind::InvalidNewSudoAddress(err))
    }

    #[must_use]
    fn invalid_new_withdrawer_address(err: AddressError) -> Self {
        Self(BridgeSudoChangeActionErrorKind::InvalidNewWithdrawerAddress(err))
    }

    #[must_use]
    fn invalid_fee_asset(err: asset::ParseDenomError) -> Self {
        Self(BridgeSudoChangeActionErrorKind::InvalidFeeAsset(err))
    }
}

#[derive(Debug, thiserror::Error)]
enum BridgeSudoChangeActionErrorKind {
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("the `bridge_address` field was invalid")]
    InvalidBridgeAddress(#[source] AddressError),
    #[error("the `new_sudo_address` field was invalid")]
    InvalidNewSudoAddress(#[source] AddressError),
    #[error("the `new_withdrawer_address` field was invalid")]
    InvalidNewWithdrawerAddress(#[source] AddressError),
    #[error("the `fee_asset` field was invalid")]
    InvalidFeeAsset(#[source] asset::ParseDenomError),
}

#[derive(Debug, Clone)]
pub enum FeeChange {
    TransferBaseFee,
    SequenceBaseFee,
    SequenceByteCostMultiplier,
    InitBridgeAccountBaseFee,
    BridgeLockByteCostMultiplier,
    BridgeSudoChangeBaseFee,
    Ics20WithdrawalBaseFee,
}

#[expect(
    clippy::module_name_repetitions,
    reason = "for parity with the Protobuf spec"
)]
#[derive(Debug, Clone)]
pub struct FeeChangeAction {
    pub fee_change: FeeChange,
    pub new_value: u128,
}

impl Protobuf for FeeChangeAction {
    type Error = FeeChangeActionError;
    type Raw = raw::FeeChangeAction;

    #[must_use]
    fn to_raw(&self) -> raw::FeeChangeAction {
        raw::FeeChangeAction {
            value: Some(match self.fee_change {
                FeeChange::TransferBaseFee => {
                    raw::fee_change_action::Value::TransferBaseFee(self.new_value.into())
                }
                FeeChange::SequenceBaseFee => {
                    raw::fee_change_action::Value::SequenceBaseFee(self.new_value.into())
                }
                FeeChange::SequenceByteCostMultiplier => {
                    raw::fee_change_action::Value::SequenceByteCostMultiplier(self.new_value.into())
                }
                FeeChange::InitBridgeAccountBaseFee => {
                    raw::fee_change_action::Value::InitBridgeAccountBaseFee(self.new_value.into())
                }
                FeeChange::BridgeLockByteCostMultiplier => {
                    raw::fee_change_action::Value::BridgeLockByteCostMultiplier(
                        self.new_value.into(),
                    )
                }
                FeeChange::BridgeSudoChangeBaseFee => {
                    raw::fee_change_action::Value::BridgeSudoChangeBaseFee(self.new_value.into())
                }
                FeeChange::Ics20WithdrawalBaseFee => {
                    raw::fee_change_action::Value::Ics20WithdrawalBaseFee(self.new_value.into())
                }
            }),
        }
    }

    /// Convert from a reference to a raw, unchecked protobuf [`raw::FeeChangeAction`].
    ///
    /// # Errors
    ///
    /// - if the fee change `value` field is missing
    /// - if the `new_value` field is missing
    fn try_from_raw_ref(proto: &raw::FeeChangeAction) -> Result<Self, FeeChangeActionError> {
        let (fee_change, new_value) = match proto.value {
            Some(raw::fee_change_action::Value::TransferBaseFee(new_value)) => {
                (FeeChange::TransferBaseFee, new_value)
            }
            Some(raw::fee_change_action::Value::SequenceBaseFee(new_value)) => {
                (FeeChange::SequenceBaseFee, new_value)
            }
            Some(raw::fee_change_action::Value::SequenceByteCostMultiplier(new_value)) => {
                (FeeChange::SequenceByteCostMultiplier, new_value)
            }
            Some(raw::fee_change_action::Value::InitBridgeAccountBaseFee(new_value)) => {
                (FeeChange::InitBridgeAccountBaseFee, new_value)
            }
            Some(raw::fee_change_action::Value::BridgeLockByteCostMultiplier(new_value)) => {
                (FeeChange::BridgeLockByteCostMultiplier, new_value)
            }
            Some(raw::fee_change_action::Value::BridgeSudoChangeBaseFee(new_value)) => {
                (FeeChange::BridgeSudoChangeBaseFee, new_value)
            }
            Some(raw::fee_change_action::Value::Ics20WithdrawalBaseFee(new_value)) => {
                (FeeChange::Ics20WithdrawalBaseFee, new_value)
            }
            None => return Err(FeeChangeActionError::missing_value_to_change()),
        };

        Ok(Self {
            fee_change,
            new_value: new_value.into(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct FeeChangeActionError(FeeChangeActionErrorKind);

impl FeeChangeActionError {
    fn missing_value_to_change() -> Self {
        Self(FeeChangeActionErrorKind::MissingValueToChange)
    }
}

#[derive(Debug, thiserror::Error)]
enum FeeChangeActionErrorKind {
    #[error("the value which to change was missing")]
    MissingValueToChange,
}
