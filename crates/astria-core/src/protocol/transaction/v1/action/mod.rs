use std::{
    fmt::Display,
    str::FromStr,
};

use bytes::Bytes;
use ibc_types::{
    core::{
        channel::ChannelId,
        client::Height as IbcHeight,
    },
    IdentifierError,
};
use penumbra_ibc::IbcRelay;
use prost::Name as _;

use super::raw;
use crate::{
    oracles::price_feed::{
        market_map::v2::{
            Market,
            MarketError,
        },
        types::v2::{
            CurrencyPair,
            CurrencyPairError,
        },
    },
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
    protocol::fees::v1::{
        FeeComponentError,
        FeeComponents,
    },
    Protobuf,
};

pub mod group;

const MAX_VALIDATOR_NAME_LENGTH: usize = 32;

#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(::serde::Deserialize, ::serde::Serialize),
    serde(into = "raw::Action", try_from = "raw::Action")
)]
pub enum Action {
    RollupDataSubmission(RollupDataSubmission),
    Transfer(Transfer),
    ValidatorUpdate(ValidatorUpdate),
    SudoAddressChange(SudoAddressChange),
    Ibc(IbcRelay),
    IbcSudoChange(IbcSudoChange),
    Ics20Withdrawal(Ics20Withdrawal),
    IbcRelayerChange(IbcRelayerChange),
    FeeAssetChange(FeeAssetChange),
    InitBridgeAccount(InitBridgeAccount),
    BridgeLock(BridgeLock),
    BridgeUnlock(BridgeUnlock),
    BridgeSudoChange(BridgeSudoChange),
    BridgeTransfer(BridgeTransfer),
    FeeChange(FeeChange),
    RecoverIbcClient(RecoverIbcClient),
    CurrencyPairsChange(CurrencyPairsChange),
    MarketsChange(MarketsChange),
}

impl Protobuf for Action {
    type Error = Error;
    type Raw = raw::Action;

    #[must_use]
    fn to_raw(&self) -> Self::Raw {
        use raw::action::Value;
        let kind = match self {
            Action::RollupDataSubmission(act) => Value::RollupDataSubmission(act.to_raw()),
            Action::Transfer(act) => Value::Transfer(act.to_raw()),
            Action::ValidatorUpdate(act) => Value::ValidatorUpdate(act.to_raw()),
            Action::SudoAddressChange(act) => Value::SudoAddressChange(act.clone().into_raw()),
            Action::Ibc(act) => Value::Ibc(act.clone().into()),
            Action::IbcSudoChange(act) => Value::IbcSudoChange(act.clone().into_raw()),
            Action::Ics20Withdrawal(act) => Value::Ics20Withdrawal(act.to_raw()),
            Action::IbcRelayerChange(act) => Value::IbcRelayerChange(act.to_raw()),
            Action::FeeAssetChange(act) => Value::FeeAssetChange(act.to_raw()),
            Action::InitBridgeAccount(act) => Value::InitBridgeAccount(act.to_raw()),
            Action::BridgeLock(act) => Value::BridgeLock(act.to_raw()),
            Action::BridgeUnlock(act) => Value::BridgeUnlock(act.to_raw()),
            Action::BridgeSudoChange(act) => Value::BridgeSudoChange(act.to_raw()),
            Action::BridgeTransfer(act) => Value::BridgeTransfer(act.to_raw()),
            Action::FeeChange(act) => Value::FeeChange(act.to_raw()),
            Action::RecoverIbcClient(act) => Value::RecoverIbcClient(act.to_raw()),
            Action::CurrencyPairsChange(act) => Value::CurrencyPairsChange(act.to_raw()),
            Action::MarketsChange(act) => Value::MarketsChange(act.to_raw()),
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
    /// to a native action fails.
    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Error> {
        Self::try_from_raw(raw.clone())
    }

    /// Attempt to convert from a raw, unchecked protobuf [`raw::Action`].
    ///
    /// # Errors
    ///
    /// Returns an error if conversion of one of the inner raw action variants
    /// to a native action fails.
    fn try_from_raw(proto: raw::Action) -> Result<Self, Error> {
        use raw::action::Value;
        let raw::Action {
            value,
        } = proto;
        let Some(action) = value else {
            return Err(Error::unset());
        };
        let action = match action {
            Value::RollupDataSubmission(act) => Self::RollupDataSubmission(
                RollupDataSubmission::try_from_raw(act).map_err(Error::rollup_data_submission)?,
            ),
            Value::Transfer(act) => {
                Self::Transfer(Transfer::try_from_raw(act).map_err(Error::transfer)?)
            }
            Value::ValidatorUpdate(act) => Self::ValidatorUpdate(
                ValidatorUpdate::try_from_raw(act).map_err(Error::validator_update)?,
            ),
            Value::SudoAddressChange(act) => Self::SudoAddressChange(
                SudoAddressChange::try_from_raw(act).map_err(Error::sudo_address_change)?,
            ),
            Value::IbcSudoChange(act) => Self::IbcSudoChange(
                IbcSudoChange::try_from_raw(act).map_err(Error::ibc_sudo_change)?,
            ),
            Value::Ibc(act) => {
                Self::Ibc(IbcRelay::try_from(act).map_err(|e| Error::ibc(e.into()))?)
            }
            Value::Ics20Withdrawal(act) => Self::Ics20Withdrawal(
                Ics20Withdrawal::try_from_raw(act).map_err(Error::ics20_withdrawal)?,
            ),
            Value::IbcRelayerChange(act) => Self::IbcRelayerChange(
                IbcRelayerChange::try_from_raw_ref(&act).map_err(Error::ibc_relayer_change)?,
            ),
            Value::FeeAssetChange(act) => Self::FeeAssetChange(
                FeeAssetChange::try_from_raw_ref(&act).map_err(Error::fee_asset_change)?,
            ),
            Value::InitBridgeAccount(act) => Self::InitBridgeAccount(
                InitBridgeAccount::try_from_raw(act).map_err(Error::init_bridge_account)?,
            ),
            Value::BridgeLock(act) => {
                Self::BridgeLock(BridgeLock::try_from_raw(act).map_err(Error::bridge_lock)?)
            }
            Value::BridgeUnlock(act) => {
                Self::BridgeUnlock(BridgeUnlock::try_from_raw(act).map_err(Error::bridge_unlock)?)
            }
            Value::BridgeSudoChange(act) => Self::BridgeSudoChange(
                BridgeSudoChange::try_from_raw(act).map_err(Error::bridge_sudo_change)?,
            ),
            Value::BridgeTransfer(act) => Self::BridgeTransfer(
                BridgeTransfer::try_from_raw(act).map_err(Error::bridge_transfer)?,
            ),
            Value::FeeChange(act) => {
                Self::FeeChange(FeeChange::try_from_raw_ref(&act).map_err(Error::fee_change)?)
            }
            Value::RecoverIbcClient(act) => Self::RecoverIbcClient(
                RecoverIbcClient::try_from_raw(act).map_err(Error::recover_ibc_client)?,
            ),
            Value::CurrencyPairsChange(act) => Self::CurrencyPairsChange(
                CurrencyPairsChange::try_from_raw(act).map_err(Error::currency_pairs_change)?,
            ),
            Value::MarketsChange(act) => Self::MarketsChange(
                MarketsChange::try_from_raw(act).map_err(Error::markets_change)?,
            ),
        };
        Ok(action)
    }
}

// TODO: add unit tests for these methods (https://github.com/astriaorg/astria/issues/1593)
impl Action {
    #[must_use]
    pub fn as_rollup_data_submission(&self) -> Option<&RollupDataSubmission> {
        let Self::RollupDataSubmission(sequence_action) = self else {
            return None;
        };
        Some(sequence_action)
    }

    #[must_use]
    pub fn as_transfer(&self) -> Option<&Transfer> {
        let Self::Transfer(transfer_action) = self else {
            return None;
        };
        Some(transfer_action)
    }

    pub fn is_fee_asset_change(&self) -> bool {
        matches!(self, Self::FeeAssetChange(_))
    }

    pub fn is_fee_change(&self) -> bool {
        matches!(self, Self::FeeChange(_))
    }
}

impl From<RollupDataSubmission> for Action {
    fn from(value: RollupDataSubmission) -> Self {
        Self::RollupDataSubmission(value)
    }
}

impl From<Transfer> for Action {
    fn from(value: Transfer) -> Self {
        Self::Transfer(value)
    }
}

impl From<ValidatorUpdate> for Action {
    fn from(value: ValidatorUpdate) -> Self {
        Self::ValidatorUpdate(value)
    }
}

impl From<SudoAddressChange> for Action {
    fn from(value: SudoAddressChange) -> Self {
        Self::SudoAddressChange(value)
    }
}

impl From<IbcSudoChange> for Action {
    fn from(value: IbcSudoChange) -> Self {
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

impl From<IbcRelayerChange> for Action {
    fn from(value: IbcRelayerChange) -> Self {
        Self::IbcRelayerChange(value)
    }
}

impl From<FeeAssetChange> for Action {
    fn from(value: FeeAssetChange) -> Self {
        Self::FeeAssetChange(value)
    }
}

impl From<InitBridgeAccount> for Action {
    fn from(value: InitBridgeAccount) -> Self {
        Self::InitBridgeAccount(value)
    }
}

impl From<BridgeLock> for Action {
    fn from(value: BridgeLock) -> Self {
        Self::BridgeLock(value)
    }
}

impl From<BridgeUnlock> for Action {
    fn from(value: BridgeUnlock) -> Self {
        Self::BridgeUnlock(value)
    }
}

impl From<BridgeSudoChange> for Action {
    fn from(value: BridgeSudoChange) -> Self {
        Self::BridgeSudoChange(value)
    }
}

impl From<BridgeTransfer> for Action {
    fn from(value: BridgeTransfer) -> Self {
        Self::BridgeTransfer(value)
    }
}

impl From<FeeChange> for Action {
    fn from(value: FeeChange) -> Self {
        Self::FeeChange(value)
    }
}

impl From<RecoverIbcClient> for Action {
    fn from(value: RecoverIbcClient) -> Self {
        Self::RecoverIbcClient(value)
    }
}

impl From<CurrencyPairsChange> for Action {
    fn from(value: CurrencyPairsChange) -> Self {
        Self::CurrencyPairsChange(value)
    }
}

impl From<MarketsChange> for Action {
    fn from(value: MarketsChange) -> Self {
        Self::MarketsChange(value)
    }
}

impl From<Action> for raw::Action {
    fn from(value: Action) -> Self {
        value.into_raw()
    }
}

impl TryFrom<raw::Action> for Action {
    type Error = Error;

    fn try_from(value: raw::Action) -> Result<Self, Self::Error> {
        Self::try_from_raw(value)
    }
}

// TODO: replace this trait with a Protobuf:FullName implementation.
// Issue tracked in #1567
pub(super) trait ActionName {
    fn name(&self) -> &'static str;
}

impl ActionName for Action {
    fn name(&self) -> &'static str {
        match self {
            Action::RollupDataSubmission(_) => "RollupDataSubmission",
            Action::Transfer(_) => "Transfer",
            Action::ValidatorUpdate(_) => "ValidatorUpdate",
            Action::SudoAddressChange(_) => "SudoAddressChange",
            Action::Ibc(_) => "Ibc",
            Action::IbcSudoChange(_) => "IbcSudoChange",
            Action::Ics20Withdrawal(_) => "Ics20Withdrawal",
            Action::IbcRelayerChange(_) => "IbcRelayerChange",
            Action::FeeAssetChange(_) => "FeeAssetChange",
            Action::InitBridgeAccount(_) => "InitBridgeAccount",
            Action::BridgeLock(_) => "BridgeLock",
            Action::BridgeUnlock(_) => "BridgeUnlock",
            Action::BridgeSudoChange(_) => "BridgeSudoChange",
            Action::BridgeTransfer(_) => "BridgeTransfer",
            Action::FeeChange(_) => "FeeChange",
            Action::RecoverIbcClient(_) => "RecoverIbcClient",
            Action::CurrencyPairsChange(_) => "CurrencyPairsChange",
            Action::MarketsChange(_) => "MarketsChange",
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error(ActionErrorKind);

impl Error {
    fn unset() -> Self {
        Self(ActionErrorKind::Unset)
    }

    fn rollup_data_submission(inner: RollupDataSubmissionError) -> Self {
        Self(ActionErrorKind::RollupDataSubmission(inner))
    }

    fn transfer(inner: TransferError) -> Self {
        Self(ActionErrorKind::Transfer(inner))
    }

    fn validator_update(inner: ValidatorUpdateError) -> Self {
        Self(ActionErrorKind::ValidatorUpdate(inner))
    }

    fn sudo_address_change(inner: SudoAddressChangeError) -> Self {
        Self(ActionErrorKind::SudoAddressChange(inner))
    }

    fn ibc_sudo_change(inner: IbcSudoChangeError) -> Self {
        Self(ActionErrorKind::IbcSudoChange(inner))
    }

    fn ibc(inner: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self(ActionErrorKind::Ibc(inner))
    }

    fn ics20_withdrawal(inner: Ics20WithdrawalError) -> Self {
        Self(ActionErrorKind::Ics20Withdrawal(inner))
    }

    fn ibc_relayer_change(inner: IbcRelayerChangeError) -> Self {
        Self(ActionErrorKind::IbcRelayerChange(inner))
    }

    fn fee_asset_change(inner: FeeAssetChangeError) -> Self {
        Self(ActionErrorKind::FeeAssetChange(inner))
    }

    fn init_bridge_account(inner: InitBridgeAccountError) -> Self {
        Self(ActionErrorKind::InitBridgeAccount(inner))
    }

    fn bridge_lock(inner: BridgeLockError) -> Self {
        Self(ActionErrorKind::BridgeLock(inner))
    }

    fn bridge_unlock(inner: BridgeUnlockError) -> Self {
        Self(ActionErrorKind::BridgeUnlock(inner))
    }

    fn bridge_sudo_change(inner: BridgeSudoChangeError) -> Self {
        Self(ActionErrorKind::BridgeSudoChange(inner))
    }

    fn bridge_transfer(inner: BridgeTransferError) -> Self {
        Self(ActionErrorKind::BridgeTransfer(inner))
    }

    fn fee_change(inner: FeeChangeError) -> Self {
        Self(ActionErrorKind::FeeChange(inner))
    }

    fn recover_ibc_client(inner: RecoverIbcClientError) -> Self {
        Self(ActionErrorKind::RecoverIbcClient(inner))
    }

    fn currency_pairs_change(inner: CurrencyPairsChangeError) -> Self {
        Self(ActionErrorKind::CurrencyPairsChange(inner))
    }

    fn markets_change(inner: MarketsChangeError) -> Self {
        Self(ActionErrorKind::MarketsChange(inner))
    }
}

#[derive(Debug, thiserror::Error)]
enum ActionErrorKind {
    #[error("required action value was not set")]
    Unset,
    #[error("rollup data submission action was not valid")]
    RollupDataSubmission(#[source] RollupDataSubmissionError),
    #[error("transfer action was not valid")]
    Transfer(#[source] TransferError),
    #[error("validator update action was not valid")]
    ValidatorUpdate(#[source] ValidatorUpdateError),
    #[error("sudo address change action was not valid")]
    SudoAddressChange(#[source] SudoAddressChangeError),
    #[error("ibc sudo address change action was not valid")]
    IbcSudoChange(#[source] IbcSudoChangeError),
    #[error("ibc action was not valid")]
    Ibc(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("ics20 withdrawal action was not valid")]
    Ics20Withdrawal(#[source] Ics20WithdrawalError),
    #[error("ibc relayer change action was not valid")]
    IbcRelayerChange(#[source] IbcRelayerChangeError),
    #[error("fee asset change action was not valid")]
    FeeAssetChange(#[source] FeeAssetChangeError),
    #[error("init bridge account action was not valid")]
    InitBridgeAccount(#[source] InitBridgeAccountError),
    #[error("bridge lock action was not valid")]
    BridgeLock(#[source] BridgeLockError),
    #[error("bridge unlock action was not valid")]
    BridgeUnlock(#[source] BridgeUnlockError),
    #[error("bridge sudo change action was not valid")]
    BridgeSudoChange(#[source] BridgeSudoChangeError),
    #[error("bridge transfer action was not valid")]
    BridgeTransfer(#[source] BridgeTransferError),
    #[error("fee change action was not valid")]
    FeeChange(#[source] FeeChangeError),
    #[error("recover ibc client action was not valid")]
    RecoverIbcClient(#[source] RecoverIbcClientError),
    #[error("currency pairs change action was not valid")]
    CurrencyPairsChange(#[source] CurrencyPairsChangeError),
    #[error("markets change action was not valid")]
    MarketsChange(#[source] MarketsChangeError),
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct RollupDataSubmissionError(RollupDataSubmissionErrorKind);

impl RollupDataSubmissionError {
    fn field_not_set(field: &'static str) -> Self {
        Self(RollupDataSubmissionErrorKind::FieldNotSet(field))
    }

    fn rollup_id_length(inner: IncorrectRollupIdLength) -> Self {
        Self(RollupDataSubmissionErrorKind::RollupIdLength(inner))
    }

    fn fee_asset(inner: asset::ParseDenomError) -> Self {
        Self(RollupDataSubmissionErrorKind::FeeAsset(inner))
    }
}

#[derive(Debug, thiserror::Error)]
enum RollupDataSubmissionErrorKind {
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("`rollup_id` field did not contain a valid rollup ID")]
    RollupIdLength(IncorrectRollupIdLength),
    #[error("`fee_asset` field did not contain a valid asset ID")]
    FeeAsset(#[source] asset::ParseDenomError),
}

#[derive(Clone, Debug)]
pub struct RollupDataSubmission {
    pub rollup_id: RollupId,
    pub data: Bytes,
    /// asset to use for fee payment.
    pub fee_asset: asset::Denom,
}

impl Protobuf for RollupDataSubmission {
    type Error = RollupDataSubmissionError;
    type Raw = raw::RollupDataSubmission;

    #[must_use]
    fn to_raw(&self) -> raw::RollupDataSubmission {
        let Self {
            rollup_id,
            data,
            fee_asset,
        } = self;
        raw::RollupDataSubmission {
            rollup_id: Some(rollup_id.to_raw()),
            data: data.clone(),
            fee_asset: fee_asset.to_string(),
        }
    }

    /// Convert from a reference to the raw protobuf type.
    ///
    /// # Errors
    /// Returns [`RollupDataSubmissionError`] if the on-wire data type could not be validated.
    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let raw::RollupDataSubmission {
            rollup_id,
            data,
            fee_asset,
        } = raw;
        let Some(rollup_id) = rollup_id else {
            return Err(RollupDataSubmissionError::field_not_set("rollup_id"));
        };
        let rollup_id = RollupId::try_from_raw_ref(rollup_id)
            .map_err(RollupDataSubmissionError::rollup_id_length)?;
        let fee_asset = fee_asset
            .parse()
            .map_err(RollupDataSubmissionError::fee_asset)?;
        let data = data.clone();
        Ok(Self {
            rollup_id,
            data,
            fee_asset,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Transfer {
    pub to: Address,
    pub amount: u128,
    /// asset to be transferred.
    pub asset: asset::Denom,
    /// asset to use for fee payment.
    pub fee_asset: asset::Denom,
}

impl Protobuf for Transfer {
    type Error = TransferError;
    type Raw = raw::Transfer;

    #[must_use]
    fn to_raw(&self) -> raw::Transfer {
        let Self {
            to,
            amount,
            asset,
            fee_asset,
        } = self;
        raw::Transfer {
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
        let raw::Transfer {
            to,
            amount,
            asset,
            fee_asset,
        } = raw;
        let Some(to) = to else {
            return Err(TransferError::field_not_set("to"));
        };
        let to = Address::try_from_raw_ref(to).map_err(TransferError::address)?;
        let amount = amount.map_or(0, Into::into);
        let asset = asset.parse().map_err(TransferError::asset)?;
        let fee_asset = fee_asset.parse().map_err(TransferError::fee_asset)?;

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
pub struct TransferError(TransferActionErrorKind);

impl TransferError {
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
#[error(
    "input was `{length}` bytes but validator names can only be up to \
     `{MAX_VALIDATOR_NAME_LENGTH}` bytes long"
)]
pub struct ValidatorNameError {
    length: usize,
}

/// Wrapper for validator name field of [`ValidatorUpdate`]. Cannot be longer than
/// [`MAX_VALIDATOR_NAME_LENGTH`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ValidatorName(String);

impl ValidatorName {
    #[must_use]
    pub fn empty() -> Self {
        Self(String::new())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl FromStr for ValidatorName {
    type Err = ValidatorNameError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() > MAX_VALIDATOR_NAME_LENGTH {
            return Err(ValidatorNameError {
                length: value.len(),
            });
        }
        Ok(Self(value.to_string()))
    }
}

impl Display for ValidatorName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
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

    fn invalid_name(source: ValidatorNameError) -> Self {
        Self(ValidatorUpdateErrorKind::InvalidName {
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
    #[error("field `.name` was invalid")]
    InvalidName { source: ValidatorNameError },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatorUpdate {
    pub power: u32,
    pub verification_key: crate::crypto::VerificationKey,
    pub name: ValidatorName,
}

impl Protobuf for ValidatorUpdate {
    type Error = ValidatorUpdateError;
    type Raw = raw::ValidatorUpdate;

    /// Create a validator update by verifying a raw protobuf-decoded
    /// [`crate::generated::protocol::transaction::v1alpha1::ValidatorUpdate`].
    ///
    /// # Errors
    /// Returns an error if the `.power` field is negative, if `.pub_key`
    /// is not set, or if `.pub_key` contains a non-ed25519 variant, or
    /// if the ed25519 has invalid bytes (that is, bytes from which an
    /// ed25519 public key cannot be constructed).
    fn try_from_raw(value: Self::Raw) -> Result<Self, Self::Error> {
        use crate::generated::astria_vendored::tendermint::crypto::{
            public_key,
            PublicKey,
        };
        let Self::Raw {
            pub_key,
            power,
            name,
        } = value;
        let name = name.parse().map_err(ValidatorUpdateError::invalid_name)?;
        let power = power
            .try_into()
            .map_err(|_| Self::Error::negative_power(power))?;
        let verification_key = match pub_key {
            None
            | Some(PublicKey {
                sum: None,
            }) => Err(Self::Error::public_key_not_set()),
            Some(PublicKey {
                sum: Some(public_key::Sum::Secp256k1(..)),
            }) => Err(Self::Error::secp256k1_not_supported()),

            Some(PublicKey {
                sum: Some(public_key::Sum::Ed25519(bytes)),
            }) => crate::crypto::VerificationKey::try_from(&*bytes)
                .map_err(Self::Error::verification_key),
        }?;
        Ok(Self {
            power,
            verification_key,
            name,
        })
    }

    /// Create a validator update by verifying a reference to raw protobuf-decoded
    /// [`crate::generated::protocol::transaction::v1alpha1::ValidatorUpdate`].
    ///
    /// # Errors
    /// Returns an error if the `.power` field is negative, if `.pub_key`
    /// is not set, or if `.pub_key` contains a non-ed25519 variant, or
    /// if the ed25519 has invalid bytes (that is, bytes from which an
    /// ed25519 public key cannot be constructed).
    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        Self::try_from_raw(raw.clone())
    }

    #[must_use]
    fn to_raw(&self) -> Self::Raw {
        use crate::generated::astria_vendored::tendermint::crypto::{
            public_key,
            PublicKey,
        };
        let Self {
            power,
            verification_key,
            name,
        } = self;

        Self::Raw {
            power: (*power).into(),
            pub_key: Some(PublicKey {
                sum: Some(public_key::Sum::Ed25519(
                    verification_key.to_bytes().to_vec(),
                )),
            }),
            name: name.clone().into_inner(),
        }
    }
}

impl From<ValidatorUpdate>
    for crate::generated::astria::protocol::transaction::v1::ValidatorUpdate
{
    fn from(value: ValidatorUpdate) -> Self {
        value.into_raw()
    }
}

impl TryFrom<crate::generated::astria::protocol::transaction::v1::ValidatorUpdate>
    for ValidatorUpdate
{
    type Error = ValidatorUpdateError;

    fn try_from(
        value: crate::generated::astria::protocol::transaction::v1::ValidatorUpdate,
    ) -> Result<Self, Self::Error> {
        Self::try_from_raw(value)
    }
}

#[derive(Clone, Debug)]
pub struct SudoAddressChange {
    pub new_address: Address,
}

impl Protobuf for SudoAddressChange {
    type Error = SudoAddressChangeError;
    type Raw = raw::SudoAddressChange;

    fn into_raw(self) -> raw::SudoAddressChange {
        let Self {
            new_address,
        } = self;
        raw::SudoAddressChange {
            new_address: Some(new_address.into_raw()),
        }
    }

    #[must_use]
    fn to_raw(&self) -> raw::SudoAddressChange {
        let Self {
            new_address,
        } = self;
        raw::SudoAddressChange {
            new_address: Some(new_address.to_raw()),
        }
    }

    /// Convert from a reference to a raw, unchecked protobuf [`raw::SudoAddressChange`].
    ///
    /// # Errors
    ///
    /// Returns an error if the raw action's `new_address` did not have the expected
    /// length.
    fn try_from_raw_ref(proto: &Self::Raw) -> Result<Self, SudoAddressChangeError> {
        let raw::SudoAddressChange {
            new_address,
        } = proto;
        let Some(new_address) = new_address else {
            return Err(SudoAddressChangeError::field_not_set("new_address"));
        };
        let new_address =
            Address::try_from_raw_ref(new_address).map_err(SudoAddressChangeError::address)?;
        Ok(Self {
            new_address,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct SudoAddressChangeError(SudoAddressChangeErrorKind);

impl SudoAddressChangeError {
    fn field_not_set(field: &'static str) -> Self {
        Self(SudoAddressChangeErrorKind::FieldNotSet(field))
    }

    fn address(source: AddressError) -> Self {
        Self(SudoAddressChangeErrorKind::Address {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum SudoAddressChangeErrorKind {
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("`new_address` field did not contain a valid address")]
    Address { source: AddressError },
}

#[derive(Debug, Clone)]
pub struct IbcSudoChange {
    pub new_address: Address,
}

impl Protobuf for IbcSudoChange {
    type Error = IbcSudoChangeError;
    type Raw = raw::IbcSudoChange;

    fn into_raw(self) -> raw::IbcSudoChange {
        raw::IbcSudoChange {
            new_address: Some(self.new_address.into_raw()),
        }
    }

    #[must_use]
    fn to_raw(&self) -> raw::IbcSudoChange {
        raw::IbcSudoChange {
            new_address: Some(self.new_address.to_raw()),
        }
    }

    /// Convert from a reference to a raw, unchecked protobuf [`raw::IbcSudoChangeAction`].
    ///
    /// # Errors
    ///
    /// Returns an error if the raw action's `new_address` did not have the expected
    /// length or if the field was not set.
    fn try_from_raw_ref(proto: &Self::Raw) -> Result<Self, IbcSudoChangeError> {
        let raw::IbcSudoChange {
            new_address,
        } = proto;
        let Some(new_address) = new_address else {
            return Err(IbcSudoChangeError::field_not_set("new_address"));
        };
        let new_address =
            Address::try_from_raw_ref(new_address).map_err(IbcSudoChangeError::address)?;
        Ok(Self {
            new_address,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct IbcSudoChangeError(IbcSudoChangeErrorKind);

impl IbcSudoChangeError {
    fn field_not_set(field: &'static str) -> Self {
        Self(IbcSudoChangeErrorKind::FieldNotSet(field))
    }

    fn address(source: AddressError) -> Self {
        Self(IbcSudoChangeErrorKind::Address {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum IbcSudoChangeErrorKind {
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
    // the height (on the counterparty chain) at which this transfer expires.
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
            return_address.ok_or(Ics20WithdrawalError::field_not_set("return_address"))?,
        )
        .map_err(Ics20WithdrawalError::return_address)?;

        let timeout_height = timeout_height
            .ok_or(Ics20WithdrawalError::field_not_set("timeout_height"))?
            .into();
        let bridge_address = bridge_address
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
        let return_address = return_address
            .as_ref()
            .ok_or_else(|| Ics20WithdrawalError::field_not_set("return_address"))
            .and_then(|return_address| {
                Address::try_from_raw_ref(return_address)
                    .map_err(Ics20WithdrawalError::return_address)
            })?;

        let timeout_height = timeout_height
            .ok_or(Ics20WithdrawalError::field_not_set("timeout_height"))?
            .into();
        let bridge_address = bridge_address
            .as_ref()
            .map(Address::try_from_raw_ref)
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

#[derive(Debug, Clone)]
pub enum IbcRelayerChange {
    Addition(Address),
    Removal(Address),
}

impl Protobuf for IbcRelayerChange {
    type Error = IbcRelayerChangeError;
    type Raw = raw::IbcRelayerChange;

    #[must_use]
    fn to_raw(&self) -> raw::IbcRelayerChange {
        match self {
            IbcRelayerChange::Addition(address) => raw::IbcRelayerChange {
                value: Some(raw::ibc_relayer_change::Value::Addition(address.to_raw())),
            },
            IbcRelayerChange::Removal(address) => raw::IbcRelayerChange {
                value: Some(raw::ibc_relayer_change::Value::Removal(address.to_raw())),
            },
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::IbcRelayerChangeAction`].
    ///
    /// # Errors
    ///
    /// - if the `address` field is invalid
    fn try_from_raw_ref(raw: &raw::IbcRelayerChange) -> Result<Self, IbcRelayerChangeError> {
        match raw {
            raw::IbcRelayerChange {
                value: Some(raw::ibc_relayer_change::Value::Addition(address)),
            } => {
                let address =
                    Address::try_from_raw_ref(address).map_err(IbcRelayerChangeError::address)?;
                Ok(IbcRelayerChange::Addition(address))
            }
            raw::IbcRelayerChange {
                value: Some(raw::ibc_relayer_change::Value::Removal(address)),
            } => {
                let address =
                    Address::try_from_raw_ref(address).map_err(IbcRelayerChangeError::address)?;
                Ok(IbcRelayerChange::Removal(address))
            }
            _ => Err(IbcRelayerChangeError::missing_address()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct IbcRelayerChangeError(IbcRelayerChangeErrorKind);

impl IbcRelayerChangeError {
    #[must_use]
    fn address(source: AddressError) -> Self {
        Self(IbcRelayerChangeErrorKind::Address {
            source,
        })
    }

    #[must_use]
    fn missing_address() -> Self {
        Self(IbcRelayerChangeErrorKind::MissingAddress)
    }
}

#[derive(Debug, thiserror::Error)]
enum IbcRelayerChangeErrorKind {
    #[error("the `address` was invalid")]
    Address { source: AddressError },
    #[error("the `address` was not set")]
    MissingAddress,
}

#[derive(Debug, Clone)]
pub enum FeeAssetChange {
    Addition(asset::Denom),
    Removal(asset::Denom),
}

impl Protobuf for FeeAssetChange {
    type Error = FeeAssetChangeError;
    type Raw = raw::FeeAssetChange;

    #[must_use]
    fn to_raw(&self) -> raw::FeeAssetChange {
        match self {
            FeeAssetChange::Addition(asset) => raw::FeeAssetChange {
                value: Some(raw::fee_asset_change::Value::Addition(asset.to_string())),
            },
            FeeAssetChange::Removal(asset) => raw::FeeAssetChange {
                value: Some(raw::fee_asset_change::Value::Removal(asset.to_string())),
            },
        }
    }

    /// Convert from a reference to a raw, unchecked protobuf [`raw::FeeAssetChangeAction`].
    ///
    /// # Errors
    ///
    /// - if the `asset` field is invalid
    fn try_from_raw_ref(raw: &raw::FeeAssetChange) -> Result<Self, FeeAssetChangeError> {
        match raw {
            raw::FeeAssetChange {
                value: Some(raw::fee_asset_change::Value::Addition(asset)),
            } => {
                let asset = asset.parse().map_err(FeeAssetChangeError::invalid_asset)?;
                Ok(FeeAssetChange::Addition(asset))
            }
            raw::FeeAssetChange {
                value: Some(raw::fee_asset_change::Value::Removal(asset)),
            } => {
                let asset = asset.parse().map_err(FeeAssetChangeError::invalid_asset)?;
                Ok(FeeAssetChange::Removal(asset))
            }
            _ => Err(FeeAssetChangeError::missing_asset()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct FeeAssetChangeError(FeeAssetChangeErrorKind);

impl FeeAssetChangeError {
    #[must_use]
    fn invalid_asset(err: asset::ParseDenomError) -> Self {
        Self(FeeAssetChangeErrorKind::InvalidAsset(err))
    }

    #[must_use]
    fn missing_asset() -> Self {
        Self(FeeAssetChangeErrorKind::MissingAsset)
    }
}

#[derive(Debug, thiserror::Error)]
enum FeeAssetChangeErrorKind {
    #[error("the `asset` field was invalid")]
    InvalidAsset(#[source] asset::ParseDenomError),
    #[error("the `asset` field was not set")]
    MissingAsset,
}

#[derive(Debug, Clone)]
pub struct InitBridgeAccount {
    // the rollup ID to register for the sender of this action
    pub rollup_id: RollupId,
    // the asset accepted by the bridge account
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

impl Protobuf for InitBridgeAccount {
    type Error = InitBridgeAccountError;
    type Raw = raw::InitBridgeAccount;

    #[must_use]
    fn into_raw(self) -> raw::InitBridgeAccount {
        raw::InitBridgeAccount {
            rollup_id: Some(self.rollup_id.to_raw()),
            asset: self.asset.to_string(),
            fee_asset: self.fee_asset.to_string(),
            sudo_address: self.sudo_address.map(Address::into_raw),
            withdrawer_address: self.withdrawer_address.map(Address::into_raw),
        }
    }

    #[must_use]
    fn to_raw(&self) -> raw::InitBridgeAccount {
        raw::InitBridgeAccount {
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
    fn try_from_raw(proto: raw::InitBridgeAccount) -> Result<Self, InitBridgeAccountError> {
        let Some(rollup_id) = proto.rollup_id else {
            return Err(InitBridgeAccountError::field_not_set("rollup_id"));
        };
        let rollup_id =
            RollupId::try_from_raw(rollup_id).map_err(InitBridgeAccountError::invalid_rollup_id)?;
        let asset = proto
            .asset
            .parse()
            .map_err(InitBridgeAccountError::invalid_asset)?;
        let fee_asset = proto
            .fee_asset
            .parse()
            .map_err(InitBridgeAccountError::invalid_fee_asset)?;
        let sudo_address = proto
            .sudo_address
            .map(Address::try_from_raw)
            .transpose()
            .map_err(InitBridgeAccountError::invalid_sudo_address)?;
        let withdrawer_address = proto
            .withdrawer_address
            .map(Address::try_from_raw)
            .transpose()
            .map_err(InitBridgeAccountError::invalid_withdrawer_address)?;

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
    fn try_from_raw_ref(proto: &Self::Raw) -> Result<Self, InitBridgeAccountError> {
        Self::try_from_raw(proto.clone())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct InitBridgeAccountError(InitBridgeAccountErrorKind);

impl InitBridgeAccountError {
    #[must_use]
    fn field_not_set(field: &'static str) -> Self {
        Self(InitBridgeAccountErrorKind::FieldNotSet(field))
    }

    #[must_use]
    fn invalid_rollup_id(err: IncorrectRollupIdLength) -> Self {
        Self(InitBridgeAccountErrorKind::InvalidRollupId(err))
    }

    #[must_use]
    fn invalid_asset(err: asset::ParseDenomError) -> Self {
        Self(InitBridgeAccountErrorKind::InvalidAsset(err))
    }

    #[must_use]
    fn invalid_fee_asset(err: asset::ParseDenomError) -> Self {
        Self(InitBridgeAccountErrorKind::InvalidFeeAsset(err))
    }

    #[must_use]
    fn invalid_sudo_address(err: AddressError) -> Self {
        Self(InitBridgeAccountErrorKind::InvalidSudoAddress(err))
    }

    #[must_use]
    fn invalid_withdrawer_address(err: AddressError) -> Self {
        Self(InitBridgeAccountErrorKind::InvalidWithdrawerAddress(err))
    }
}

#[derive(Debug, thiserror::Error)]
enum InitBridgeAccountErrorKind {
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

#[derive(Debug, Clone)]
pub struct BridgeLock {
    pub to: Address,
    pub amount: u128,
    // asset to be transferred.
    pub asset: asset::Denom,
    // asset to use for fee payment.
    pub fee_asset: asset::Denom,
    // the address on the destination chain to send the transfer to.
    pub destination_chain_address: String,
}

impl Protobuf for BridgeLock {
    type Error = BridgeLockError;
    type Raw = raw::BridgeLock;

    #[must_use]
    fn into_raw(self) -> raw::BridgeLock {
        raw::BridgeLock {
            to: Some(self.to.to_raw()),
            amount: Some(self.amount.into()),
            asset: self.asset.to_string(),
            fee_asset: self.fee_asset.to_string(),
            destination_chain_address: self.destination_chain_address,
        }
    }

    #[must_use]
    fn to_raw(&self) -> raw::BridgeLock {
        raw::BridgeLock {
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
    fn try_from_raw(proto: raw::BridgeLock) -> Result<Self, BridgeLockError> {
        let Some(to) = proto.to else {
            return Err(BridgeLockError::field_not_set("to"));
        };
        let to = Address::try_from_raw(to).map_err(BridgeLockError::address)?;
        let amount = proto.amount.ok_or(BridgeLockError::missing_amount())?;
        let asset = proto
            .asset
            .parse()
            .map_err(BridgeLockError::invalid_asset)?;
        let fee_asset = proto
            .fee_asset
            .parse()
            .map_err(BridgeLockError::invalid_fee_asset)?;
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
    fn try_from_raw_ref(proto: &raw::BridgeLock) -> Result<Self, BridgeLockError> {
        Self::try_from_raw(proto.clone())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct BridgeLockError(BridgeLockErrorKind);

impl BridgeLockError {
    #[must_use]
    fn field_not_set(field: &'static str) -> Self {
        Self(BridgeLockErrorKind::FieldNotSet(field))
    }

    #[must_use]
    fn address(source: AddressError) -> Self {
        Self(BridgeLockErrorKind::Address {
            source,
        })
    }

    #[must_use]
    fn missing_amount() -> Self {
        Self(BridgeLockErrorKind::MissingAmount)
    }

    #[must_use]
    fn invalid_asset(err: asset::ParseDenomError) -> Self {
        Self(BridgeLockErrorKind::InvalidAsset(err))
    }

    #[must_use]
    fn invalid_fee_asset(err: asset::ParseDenomError) -> Self {
        Self(BridgeLockErrorKind::InvalidFeeAsset(err))
    }
}

#[derive(Debug, thiserror::Error)]
enum BridgeLockErrorKind {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeUnlock {
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

impl Protobuf for BridgeUnlock {
    type Error = BridgeUnlockError;
    type Raw = raw::BridgeUnlock;

    #[must_use]
    fn into_raw(self) -> raw::BridgeUnlock {
        raw::BridgeUnlock {
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
    fn to_raw(&self) -> raw::BridgeUnlock {
        raw::BridgeUnlock {
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
    fn try_from_raw(proto: raw::BridgeUnlock) -> Result<Self, Self::Error> {
        let raw::BridgeUnlock {
            to,
            amount,
            fee_asset,
            memo,
            bridge_address,
            rollup_block_number,
            rollup_withdrawal_event_id,
        } = proto;
        let to = to
            .ok_or_else(|| BridgeUnlockError::field_not_set("to"))
            .and_then(|to| Address::try_from_raw(to).map_err(BridgeUnlockError::address))?;
        let amount = amount.ok_or_else(|| BridgeUnlockError::field_not_set("amount"))?;
        let fee_asset = fee_asset.parse().map_err(BridgeUnlockError::fee_asset)?;

        let bridge_address = bridge_address
            .ok_or_else(|| BridgeUnlockError::field_not_set("bridge_address"))
            .and_then(|to| Address::try_from_raw(to).map_err(BridgeUnlockError::bridge_address))?;
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
    fn try_from_raw_ref(proto: &raw::BridgeUnlock) -> Result<Self, BridgeUnlockError> {
        Self::try_from_raw(proto.clone())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct BridgeUnlockError(BridgeUnlockErrorKind);

impl BridgeUnlockError {
    #[must_use]
    fn field_not_set(field: &'static str) -> Self {
        Self(BridgeUnlockErrorKind::FieldNotSet(field))
    }

    #[must_use]
    fn address(source: AddressError) -> Self {
        Self(BridgeUnlockErrorKind::Address {
            source,
        })
    }

    #[must_use]
    fn fee_asset(source: asset::ParseDenomError) -> Self {
        Self(BridgeUnlockErrorKind::FeeAsset {
            source,
        })
    }

    #[must_use]
    fn bridge_address(source: AddressError) -> Self {
        Self(BridgeUnlockErrorKind::BridgeAddress {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum BridgeUnlockErrorKind {
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("the `to` field was invalid")]
    Address { source: AddressError },
    #[error("the `fee_asset` field was invalid")]
    FeeAsset { source: asset::ParseDenomError },
    #[error("the `bridge_address` field was invalid")]
    BridgeAddress { source: AddressError },
}

#[derive(Debug, Clone)]
pub struct BridgeSudoChange {
    pub bridge_address: Address,
    pub new_sudo_address: Option<Address>,
    pub new_withdrawer_address: Option<Address>,
    pub fee_asset: asset::Denom,
}

impl Protobuf for BridgeSudoChange {
    type Error = BridgeSudoChangeError;
    type Raw = raw::BridgeSudoChange;

    #[must_use]
    fn into_raw(self) -> raw::BridgeSudoChange {
        raw::BridgeSudoChange {
            bridge_address: Some(self.bridge_address.to_raw()),
            new_sudo_address: self.new_sudo_address.map(Address::into_raw),
            new_withdrawer_address: self.new_withdrawer_address.map(Address::into_raw),
            fee_asset: self.fee_asset.to_string(),
        }
    }

    #[must_use]
    fn to_raw(&self) -> raw::BridgeSudoChange {
        raw::BridgeSudoChange {
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
    fn try_from_raw(proto: raw::BridgeSudoChange) -> Result<Self, BridgeSudoChangeError> {
        let Some(bridge_address) = proto.bridge_address else {
            return Err(BridgeSudoChangeError::field_not_set("bridge_address"));
        };
        let bridge_address = Address::try_from_raw(bridge_address)
            .map_err(BridgeSudoChangeError::invalid_bridge_address)?;
        let new_sudo_address = proto
            .new_sudo_address
            .map(Address::try_from_raw)
            .transpose()
            .map_err(BridgeSudoChangeError::invalid_new_sudo_address)?;
        let new_withdrawer_address = proto
            .new_withdrawer_address
            .map(Address::try_from_raw)
            .transpose()
            .map_err(BridgeSudoChangeError::invalid_new_withdrawer_address)?;
        let fee_asset = proto
            .fee_asset
            .parse()
            .map_err(BridgeSudoChangeError::invalid_fee_asset)?;

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
    fn try_from_raw_ref(proto: &raw::BridgeSudoChange) -> Result<Self, BridgeSudoChangeError> {
        Self::try_from_raw(proto.clone())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct BridgeSudoChangeError(BridgeSudoChangeErrorKind);

impl BridgeSudoChangeError {
    #[must_use]
    fn field_not_set(field: &'static str) -> Self {
        Self(BridgeSudoChangeErrorKind::FieldNotSet(field))
    }

    #[must_use]
    fn invalid_bridge_address(err: AddressError) -> Self {
        Self(BridgeSudoChangeErrorKind::InvalidBridgeAddress(err))
    }

    #[must_use]
    fn invalid_new_sudo_address(err: AddressError) -> Self {
        Self(BridgeSudoChangeErrorKind::InvalidNewSudoAddress(err))
    }

    #[must_use]
    fn invalid_new_withdrawer_address(err: AddressError) -> Self {
        Self(BridgeSudoChangeErrorKind::InvalidNewWithdrawerAddress(err))
    }

    #[must_use]
    fn invalid_fee_asset(err: asset::ParseDenomError) -> Self {
        Self(BridgeSudoChangeErrorKind::InvalidFeeAsset(err))
    }
}

#[derive(Debug, thiserror::Error)]
enum BridgeSudoChangeErrorKind {
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
pub struct BridgeTransfer {
    pub to: Address,
    pub amount: u128,
    // asset to use for fee payment.
    pub fee_asset: asset::Denom,
    // the address on the destination chain to send the transfer to.
    pub destination_chain_address: String,
    // the address of the bridge account to transfer from.
    pub bridge_address: Address,
    // The block number of the rollup block containing the withdrawal event.
    pub rollup_block_number: u64,
    // The identifier of the withdrawal event in the rollup block.
    pub rollup_withdrawal_event_id: String,
}

impl Protobuf for BridgeTransfer {
    type Error = BridgeTransferError;
    type Raw = raw::BridgeTransfer;

    #[must_use]
    fn into_raw(self) -> raw::BridgeTransfer {
        raw::BridgeTransfer {
            to: Some(self.to.into_raw()),
            amount: Some(self.amount.into()),
            fee_asset: self.fee_asset.to_string(),
            bridge_address: Some(self.bridge_address.into_raw()),
            destination_chain_address: self.destination_chain_address,
            rollup_block_number: self.rollup_block_number,
            rollup_withdrawal_event_id: self.rollup_withdrawal_event_id,
        }
    }

    #[must_use]
    fn to_raw(&self) -> raw::BridgeTransfer {
        raw::BridgeTransfer {
            to: Some(self.to.to_raw()),
            amount: Some(self.amount.into()),
            fee_asset: self.fee_asset.to_string(),
            bridge_address: Some(self.bridge_address.to_raw()),
            destination_chain_address: self.destination_chain_address.clone(),
            rollup_block_number: self.rollup_block_number,
            rollup_withdrawal_event_id: self.rollup_withdrawal_event_id.clone(),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::BridgeTransferAction`].
    ///
    /// # Errors
    ///
    /// - if the `to` field is not set
    /// - if the `to` field is invalid
    /// - if the `amount` field is invalid
    /// - if the `fee_asset` field is invalid
    /// - if the `from` field is invalid
    /// - if `destination_chain_address` is not set
    fn try_from_raw(proto: raw::BridgeTransfer) -> Result<Self, BridgeTransferError> {
        let raw::BridgeTransfer {
            to,
            amount,
            fee_asset,
            bridge_address,
            destination_chain_address,
            rollup_block_number,
            rollup_withdrawal_event_id,
        } = proto;
        let to = to
            .ok_or_else(|| BridgeTransferError::field_not_set("to"))
            .and_then(|to| Address::try_from_raw(to).map_err(BridgeTransferError::address))?;
        let amount = amount.ok_or_else(|| BridgeTransferError::field_not_set("amount"))?;
        let fee_asset = fee_asset.parse().map_err(BridgeTransferError::fee_asset)?;
        if destination_chain_address.is_empty() {
            return Err(BridgeTransferError::field_not_set(
                "destination_chain_address",
            ));
        }

        let bridge_address = bridge_address
            .ok_or_else(|| BridgeTransferError::field_not_set("bridge_address"))
            .and_then(|to| {
                Address::try_from_raw(to).map_err(BridgeTransferError::bridge_address)
            })?;
        Ok(Self {
            to,
            amount: amount.into(),
            fee_asset,
            bridge_address,
            destination_chain_address,
            rollup_block_number,
            rollup_withdrawal_event_id,
        })
    }

    /// Convert from a reference to a raw, unchecked protobuf [`raw::BridgeTransferAction`].
    /// # Errors
    /// - if the `to` field is not set
    /// - if the `to` field is invalid
    /// - if the `amount` field is invalid
    /// - if the `fee_asset` field is invalid
    /// - if the `from` field is invalid
    fn try_from_raw_ref(proto: &raw::BridgeTransfer) -> Result<Self, BridgeTransferError> {
        Self::try_from_raw(proto.clone())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct BridgeTransferError(BridgeTransferErrorKind);

impl BridgeTransferError {
    #[must_use]
    fn field_not_set(field: &'static str) -> Self {
        Self(BridgeTransferErrorKind::FieldNotSet(field))
    }

    #[must_use]
    fn address(source: AddressError) -> Self {
        Self(BridgeTransferErrorKind::Address {
            source,
        })
    }

    #[must_use]
    fn fee_asset(source: asset::ParseDenomError) -> Self {
        Self(BridgeTransferErrorKind::FeeAsset {
            source,
        })
    }

    #[must_use]
    fn bridge_address(source: AddressError) -> Self {
        Self(BridgeTransferErrorKind::BridgeAddress {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum BridgeTransferErrorKind {
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("the `to` field was invalid")]
    Address { source: AddressError },
    #[error("the `fee_asset` field was invalid")]
    FeeAsset { source: asset::ParseDenomError },
    #[error("the `bridge_address` field was invalid")]
    BridgeAddress { source: AddressError },
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct FeeChangeError(FeeChangeErrorKind);

impl FeeChangeError {
    fn field_unset(name: &'static str) -> Self {
        Self(FeeChangeErrorKind::FieldUnset {
            name,
        })
    }
}

impl From<FeeComponentError> for FeeChangeError {
    fn from(source: FeeComponentError) -> Self {
        Self(FeeChangeErrorKind::FeeComponent {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed to validate on-wire type `{}`", raw::FeeChange::full_name())]
enum FeeChangeErrorKind {
    FeeComponent {
        // NOTE: the name of the fee change variant is not specified because it is included in
        // the source FeeComponentError.
        #[from]
        source: FeeComponentError,
    },
    #[error("field `{name}` was not set")]
    FieldUnset { name: &'static str },
}

#[derive(Debug, Clone)]
pub enum FeeChange {
    Transfer(FeeComponents<Transfer>),
    RollupDataSubmission(FeeComponents<RollupDataSubmission>),
    Ics20Withdrawal(FeeComponents<Ics20Withdrawal>),
    InitBridgeAccount(FeeComponents<InitBridgeAccount>),
    BridgeLock(FeeComponents<BridgeLock>),
    BridgeUnlock(FeeComponents<BridgeUnlock>),
    BridgeSudoChange(FeeComponents<BridgeSudoChange>),
    IbcRelay(FeeComponents<IbcRelay>),
    ValidatorUpdate(FeeComponents<ValidatorUpdate>),
    FeeAssetChange(FeeComponents<FeeAssetChange>),
    FeeChange(FeeComponents<FeeChange>),
    IbcRelayerChange(FeeComponents<IbcRelayerChange>),
    SudoAddressChange(FeeComponents<SudoAddressChange>),
    IbcSudoChange(FeeComponents<IbcSudoChange>),
    BridgeTransfer(FeeComponents<BridgeTransfer>),
    RecoverIbcClient(FeeComponents<RecoverIbcClient>),
    CurrencyPairsChange(FeeComponents<CurrencyPairsChange>),
    MarketsChange(FeeComponents<MarketsChange>),
}

impl Protobuf for FeeChange {
    type Error = FeeChangeError;
    type Raw = raw::FeeChange;

    #[must_use]
    fn to_raw(&self) -> raw::FeeChange {
        raw::FeeChange {
            fee_components: Some(match &self {
                Self::Transfer(fee_change) => {
                    raw::fee_change::FeeComponents::Transfer(fee_change.to_raw())
                }
                Self::RollupDataSubmission(fee_change) => {
                    raw::fee_change::FeeComponents::RollupDataSubmission(fee_change.to_raw())
                }
                Self::Ics20Withdrawal(fee_change) => {
                    raw::fee_change::FeeComponents::Ics20Withdrawal(fee_change.to_raw())
                }
                Self::InitBridgeAccount(fee_change) => {
                    raw::fee_change::FeeComponents::InitBridgeAccount(fee_change.to_raw())
                }
                Self::BridgeLock(fee_change) => {
                    raw::fee_change::FeeComponents::BridgeLock(fee_change.to_raw())
                }
                Self::BridgeUnlock(fee_change) => {
                    raw::fee_change::FeeComponents::BridgeUnlock(fee_change.to_raw())
                }
                Self::BridgeSudoChange(fee_change) => {
                    raw::fee_change::FeeComponents::BridgeSudoChange(fee_change.to_raw())
                }
                Self::IbcRelay(fee_change) => {
                    raw::fee_change::FeeComponents::IbcRelay(fee_change.to_raw())
                }
                Self::ValidatorUpdate(fee_change) => {
                    raw::fee_change::FeeComponents::ValidatorUpdate(fee_change.to_raw())
                }
                Self::FeeAssetChange(fee_change) => {
                    raw::fee_change::FeeComponents::FeeAssetChange(fee_change.to_raw())
                }
                Self::FeeChange(fee_change) => {
                    raw::fee_change::FeeComponents::FeeChange(fee_change.to_raw())
                }
                Self::IbcRelayerChange(fee_change) => {
                    raw::fee_change::FeeComponents::IbcRelayerChange(fee_change.to_raw())
                }
                Self::SudoAddressChange(fee_change) => {
                    raw::fee_change::FeeComponents::SudoAddressChange(fee_change.to_raw())
                }
                Self::IbcSudoChange(fee_change) => {
                    raw::fee_change::FeeComponents::IbcSudoChange(fee_change.to_raw())
                }
                Self::BridgeTransfer(fee_change) => {
                    raw::fee_change::FeeComponents::BridgeTransfer(fee_change.to_raw())
                }
                Self::RecoverIbcClient(fee_change) => {
                    raw::fee_change::FeeComponents::RecoverIbcClient(fee_change.to_raw())
                }
                Self::CurrencyPairsChange(fee_change) => {
                    raw::fee_change::FeeComponents::CurrencyPairsChange(fee_change.to_raw())
                }
                Self::MarketsChange(fee_change) => {
                    raw::fee_change::FeeComponents::MarketsChange(fee_change.to_raw())
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
    fn try_from_raw_ref(proto: &raw::FeeChange) -> Result<Self, Self::Error> {
        Ok(match &proto.fee_components {
            Some(raw::fee_change::FeeComponents::Transfer(fee_change)) => {
                Self::Transfer(FeeComponents::<Transfer>::try_from_raw_ref(fee_change)?)
            }
            Some(raw::fee_change::FeeComponents::RollupDataSubmission(fee_change)) => {
                Self::RollupDataSubmission(FeeComponents::<RollupDataSubmission>::try_from_raw_ref(
                    fee_change,
                )?)
            }
            Some(raw::fee_change::FeeComponents::Ics20Withdrawal(fee_change)) => {
                Self::Ics20Withdrawal(FeeComponents::<Ics20Withdrawal>::try_from_raw_ref(
                    fee_change,
                )?)
            }
            Some(raw::fee_change::FeeComponents::InitBridgeAccount(fee_change)) => {
                Self::InitBridgeAccount(FeeComponents::<InitBridgeAccount>::try_from_raw_ref(
                    fee_change,
                )?)
            }
            Some(raw::fee_change::FeeComponents::BridgeLock(fee_change)) => {
                Self::BridgeLock(FeeComponents::<BridgeLock>::try_from_raw_ref(fee_change)?)
            }
            Some(raw::fee_change::FeeComponents::BridgeUnlock(fee_change)) => {
                Self::BridgeUnlock(FeeComponents::<BridgeUnlock>::try_from_raw_ref(fee_change)?)
            }
            Some(raw::fee_change::FeeComponents::BridgeSudoChange(fee_change)) => {
                Self::BridgeSudoChange(FeeComponents::<BridgeSudoChange>::try_from_raw_ref(
                    fee_change,
                )?)
            }
            Some(raw::fee_change::FeeComponents::IbcRelay(fee_change)) => {
                Self::IbcRelay(FeeComponents::<IbcRelay>::try_from_raw_ref(fee_change)?)
            }
            Some(raw::fee_change::FeeComponents::ValidatorUpdate(fee_change)) => {
                Self::ValidatorUpdate(FeeComponents::<ValidatorUpdate>::try_from_raw_ref(
                    fee_change,
                )?)
            }
            Some(raw::fee_change::FeeComponents::FeeAssetChange(fee_change)) => {
                Self::FeeAssetChange(FeeComponents::<FeeAssetChange>::try_from_raw_ref(
                    fee_change,
                )?)
            }
            Some(raw::fee_change::FeeComponents::FeeChange(fee_change)) => {
                Self::FeeChange(FeeComponents::<FeeChange>::try_from_raw_ref(fee_change)?)
            }
            Some(raw::fee_change::FeeComponents::IbcRelayerChange(fee_change)) => {
                Self::IbcRelayerChange(FeeComponents::<IbcRelayerChange>::try_from_raw_ref(
                    fee_change,
                )?)
            }
            Some(raw::fee_change::FeeComponents::SudoAddressChange(fee_change)) => {
                Self::SudoAddressChange(FeeComponents::<SudoAddressChange>::try_from_raw_ref(
                    fee_change,
                )?)
            }
            Some(raw::fee_change::FeeComponents::IbcSudoChange(fee_change)) => Self::IbcSudoChange(
                FeeComponents::<IbcSudoChange>::try_from_raw_ref(fee_change)?,
            ),
            Some(raw::fee_change::FeeComponents::BridgeTransfer(fee_change)) => {
                Self::BridgeTransfer(FeeComponents::<BridgeTransfer>::try_from_raw_ref(
                    fee_change,
                )?)
            }
            Some(raw::fee_change::FeeComponents::RecoverIbcClient(fee_change)) => {
                Self::RecoverIbcClient(FeeComponents::<RecoverIbcClient>::try_from_raw_ref(
                    fee_change,
                )?)
            }
            Some(raw::fee_change::FeeComponents::CurrencyPairsChange(fee_change)) => {
                Self::CurrencyPairsChange(FeeComponents::<CurrencyPairsChange>::try_from_raw_ref(
                    fee_change,
                )?)
            }
            Some(raw::fee_change::FeeComponents::MarketsChange(fee_change)) => Self::MarketsChange(
                FeeComponents::<MarketsChange>::try_from_raw_ref(fee_change)?,
            ),
            None => return Err(FeeChangeError::field_unset("fee_components")),
        })
    }
}

impl From<FeeComponents<Transfer>> for FeeChange {
    fn from(fee: FeeComponents<Transfer>) -> Self {
        FeeChange::Transfer(fee)
    }
}

impl From<FeeComponents<RollupDataSubmission>> for FeeChange {
    fn from(fee: FeeComponents<RollupDataSubmission>) -> Self {
        FeeChange::RollupDataSubmission(fee)
    }
}

impl From<FeeComponents<Ics20Withdrawal>> for FeeChange {
    fn from(fee: FeeComponents<Ics20Withdrawal>) -> Self {
        FeeChange::Ics20Withdrawal(fee)
    }
}

impl From<FeeComponents<InitBridgeAccount>> for FeeChange {
    fn from(fee: FeeComponents<InitBridgeAccount>) -> Self {
        FeeChange::InitBridgeAccount(fee)
    }
}

impl From<FeeComponents<BridgeLock>> for FeeChange {
    fn from(fee: FeeComponents<BridgeLock>) -> Self {
        FeeChange::BridgeLock(fee)
    }
}

impl From<FeeComponents<BridgeUnlock>> for FeeChange {
    fn from(fee: FeeComponents<BridgeUnlock>) -> Self {
        FeeChange::BridgeUnlock(fee)
    }
}

impl From<FeeComponents<BridgeSudoChange>> for FeeChange {
    fn from(fee: FeeComponents<BridgeSudoChange>) -> Self {
        FeeChange::BridgeSudoChange(fee)
    }
}

impl From<FeeComponents<IbcRelay>> for FeeChange {
    fn from(fee: FeeComponents<IbcRelay>) -> Self {
        FeeChange::IbcRelay(fee)
    }
}

impl From<FeeComponents<ValidatorUpdate>> for FeeChange {
    fn from(fee: FeeComponents<ValidatorUpdate>) -> Self {
        FeeChange::ValidatorUpdate(fee)
    }
}

impl From<FeeComponents<FeeAssetChange>> for FeeChange {
    fn from(fee: FeeComponents<FeeAssetChange>) -> Self {
        FeeChange::FeeAssetChange(fee)
    }
}

impl From<FeeComponents<FeeChange>> for FeeChange {
    fn from(fee: FeeComponents<FeeChange>) -> Self {
        FeeChange::FeeChange(fee)
    }
}

impl From<FeeComponents<IbcRelayerChange>> for FeeChange {
    fn from(fee: FeeComponents<IbcRelayerChange>) -> Self {
        FeeChange::IbcRelayerChange(fee)
    }
}

impl From<FeeComponents<SudoAddressChange>> for FeeChange {
    fn from(fee: FeeComponents<SudoAddressChange>) -> Self {
        FeeChange::SudoAddressChange(fee)
    }
}

impl From<FeeComponents<IbcSudoChange>> for FeeChange {
    fn from(fee: FeeComponents<IbcSudoChange>) -> Self {
        FeeChange::IbcSudoChange(fee)
    }
}

impl From<FeeComponents<BridgeTransfer>> for FeeChange {
    fn from(fee: FeeComponents<BridgeTransfer>) -> Self {
        FeeChange::BridgeTransfer(fee)
    }
}

impl From<FeeComponents<RecoverIbcClient>> for FeeChange {
    fn from(fee: FeeComponents<RecoverIbcClient>) -> Self {
        FeeChange::RecoverIbcClient(fee)
    }
}

impl From<FeeComponents<CurrencyPairsChange>> for FeeChange {
    fn from(fee: FeeComponents<CurrencyPairsChange>) -> Self {
        FeeChange::CurrencyPairsChange(fee)
    }
}

impl From<FeeComponents<MarketsChange>> for FeeChange {
    fn from(fee: FeeComponents<MarketsChange>) -> Self {
        FeeChange::MarketsChange(fee)
    }
}

#[derive(Debug, Clone)]
pub struct RecoverIbcClient {
    pub client_id: ibc_types::core::client::ClientId,
    pub replacement_client_id: ibc_types::core::client::ClientId,
}

impl Protobuf for RecoverIbcClient {
    type Error = RecoverIbcClientError;
    type Raw = raw::RecoverIbcClient;

    #[must_use]
    fn into_raw(self) -> raw::RecoverIbcClient {
        raw::RecoverIbcClient {
            client_id: self.client_id.to_string(),
            replacement_client_id: self.replacement_client_id.to_string(),
        }
    }

    #[must_use]
    fn to_raw(&self) -> raw::RecoverIbcClient {
        raw::RecoverIbcClient {
            client_id: self.client_id.clone().to_string(),
            replacement_client_id: self.replacement_client_id.clone().to_string(),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::RecoverIbcClientAction`].
    ///
    /// # Errors
    ///
    /// - if the `client_id` field is not set
    /// - if the `replacement_client_id` field is not set
    fn try_from_raw(proto: raw::RecoverIbcClient) -> Result<Self, RecoverIbcClientError> {
        let client_id = proto.client_id.parse().map_err(|_| {
            RecoverIbcClientError(RecoverIbcClientErrorKind::InvalidSubjectClientId)
        })?;
        let replacement_client_id = proto.replacement_client_id.parse().map_err(|_| {
            RecoverIbcClientError(RecoverIbcClientErrorKind::InvalidSubstituteClientId)
        })?;
        Ok(Self {
            client_id,
            replacement_client_id,
        })
    }

    /// Convert from a reference to a raw, unchecked protobuf [`raw::RecoverIbcClientAction`].
    ///
    /// # Errors
    ///
    /// - if the `client_id` field is not set
    /// - if the `replacement_client_id` field is not set
    fn try_from_raw_ref(proto: &Self::Raw) -> Result<Self, RecoverIbcClientError> {
        Self::try_from_raw(proto.clone())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct RecoverIbcClientError(RecoverIbcClientErrorKind);

#[derive(Debug, thiserror::Error)]
enum RecoverIbcClientErrorKind {
    #[error("the `client_id` field was invalid")]
    InvalidSubjectClientId,
    #[error("the `replacement_client_id` field was invalid")]
    InvalidSubstituteClientId,
}

#[derive(Debug, Clone)]
pub enum CurrencyPairsChange {
    Addition(Vec<CurrencyPair>),
    Removal(Vec<CurrencyPair>),
}

impl Protobuf for CurrencyPairsChange {
    type Error = CurrencyPairsChangeError;
    type Raw = raw::CurrencyPairsChange;

    #[must_use]
    fn into_raw(self) -> Self::Raw {
        let raw = match self {
            CurrencyPairsChange::Addition(pairs) => {
                raw::currency_pairs_change::Value::Addition(raw::CurrencyPairs {
                    pairs: pairs.into_iter().map(CurrencyPair::into_raw).collect(),
                })
            }
            CurrencyPairsChange::Removal(pairs) => {
                raw::currency_pairs_change::Value::Removal(raw::CurrencyPairs {
                    pairs: pairs.into_iter().map(CurrencyPair::into_raw).collect(),
                })
            }
        };
        Self::Raw {
            value: Some(raw),
        }
    }

    #[must_use]
    fn to_raw(&self) -> Self::Raw {
        self.clone().into_raw()
    }

    /// Convert from a raw, unchecked protobuf [`raw::CurrencyPairsChange`].
    ///
    /// # Errors
    ///
    /// - if the raw value is `None`
    /// - if any of the `pairs` field is invalid
    fn try_from_raw(raw: raw::CurrencyPairsChange) -> Result<Self, Self::Error> {
        match raw.value {
            Some(raw::currency_pairs_change::Value::Addition(raw::CurrencyPairs {
                pairs,
            })) => {
                let pairs = pairs
                    .into_iter()
                    .map(CurrencyPair::try_from_raw)
                    .collect::<Result<_, _>>()
                    .map_err(Self::Error::invalid_currency_pair)?;
                Ok(Self::Addition(pairs))
            }
            Some(raw::currency_pairs_change::Value::Removal(raw::CurrencyPairs {
                pairs,
            })) => {
                let pairs = pairs
                    .into_iter()
                    .map(CurrencyPair::try_from_raw)
                    .collect::<Result<_, _>>()
                    .map_err(Self::Error::invalid_currency_pair)?;
                Ok(Self::Removal(pairs))
            }
            None => Err(Self::Error::unset()),
        }
    }

    /// Convert from a reference to a raw, unchecked protobuf [`raw::PriceFeed`].
    ///
    /// # Errors
    ///
    /// - if the raw value is `None`
    /// - if any of the `pairs` field is invalid
    fn try_from_raw_ref(raw: &raw::CurrencyPairsChange) -> Result<Self, Self::Error> {
        Self::try_from_raw(raw.clone())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct CurrencyPairsChangeError(CurrencyPairsChangeErrorKind);

impl CurrencyPairsChangeError {
    #[must_use]
    fn unset() -> Self {
        Self(CurrencyPairsChangeErrorKind::Unset)
    }

    #[must_use]
    fn invalid_currency_pair(err: CurrencyPairError) -> Self {
        Self(CurrencyPairsChangeErrorKind::InvalidCurrencyPair(err))
    }
}

#[derive(Debug, thiserror::Error)]
enum CurrencyPairsChangeErrorKind {
    #[error("required action value was not set")]
    Unset,
    #[error("a currency pair was invalid")]
    InvalidCurrencyPair(#[from] CurrencyPairError),
}

/// Takes a list of markets and either creates, removes or updates them depending on its variant.
/// - **Creation:** Creates the markets in the market map. If no market map is found, one will be
///   created. If any of the markets to create already exist, this action will err.
/// - **Removal:** Removes the markets from the market map. If a market is not found in the map, it
///   will be ignored.
/// - **Update:** Updates the markets in the market map, matching based on `Ticker.currency_pair`.
///   If no market map is found, or any market is missing a counterpart in the map, this action will
///   err.
#[derive(Debug, Clone)]
pub enum MarketsChange {
    Creation(Vec<Market>),
    Removal(Vec<Market>),
    Update(Vec<Market>),
}

impl Protobuf for MarketsChange {
    type Error = MarketsChangeError;
    type Raw = raw::MarketsChange;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        match &raw.action {
            Some(raw::markets_change::Action::Creation(markets)) => Ok(Self::Creation(
                markets
                    .markets
                    .iter()
                    .map(|market| Market::try_from_raw(market.clone()))
                    .collect::<Result<_, _>>()
                    .map_err(MarketsChangeError::invalid_market)?,
            )),
            Some(raw::markets_change::Action::Removal(markets)) => Ok(Self::Removal(
                markets
                    .markets
                    .iter()
                    .map(|market| Market::try_from_raw(market.clone()))
                    .collect::<Result<_, _>>()
                    .map_err(MarketsChangeError::invalid_market)?,
            )),
            Some(raw::markets_change::Action::Update(markets)) => Ok(Self::Update(
                markets
                    .markets
                    .iter()
                    .map(|market| Market::try_from_raw(market.clone()))
                    .collect::<Result<_, _>>()
                    .map_err(MarketsChangeError::invalid_market)?,
            )),
            None => Err(MarketsChangeError::missing_markets()),
        }
    }

    fn to_raw(&self) -> Self::Raw {
        let action = match self {
            Self::Creation(markets) => raw::markets_change::Action::Creation(raw::Markets {
                markets: markets
                    .iter()
                    .map(|market| Market::into_raw(market.clone()))
                    .collect(),
            }),
            Self::Removal(markets) => raw::markets_change::Action::Removal(raw::Markets {
                markets: markets
                    .iter()
                    .map(|market| Market::into_raw(market.clone()))
                    .collect(),
            }),
            Self::Update(markets) => raw::markets_change::Action::Update(raw::Markets {
                markets: markets
                    .iter()
                    .map(|market| Market::into_raw(market.clone()))
                    .collect(),
            }),
        };
        Self::Raw {
            action: Some(action),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct MarketsChangeError(MarketsChangeErrorKind);

impl MarketsChangeError {
    #[must_use]
    pub fn invalid_market(err: MarketError) -> Self {
        Self(MarketsChangeErrorKind::InvalidMarket(err))
    }

    #[must_use]
    pub fn missing_markets() -> Self {
        Self(MarketsChangeErrorKind::MissingMarkets)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MarketsChangeErrorKind {
    #[error("invalid market in market list")]
    InvalidMarket(#[from] MarketError),
    #[error("change market action contained no markets to change")]
    MissingMarkets,
}
