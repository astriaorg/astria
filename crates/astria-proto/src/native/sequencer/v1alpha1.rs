use std::{
    error::Error,
    fmt::Display,
};

use ed25519_consensus::{
    Signature,
    SigningKey,
    VerificationKey,
};
use indexmap::IndexMap;
use penumbra_ibc::IbcRelay;
use sha2::{
    Digest as _,
    Sha256,
};
use tracing::info;

pub use super::asset;
use crate::{
    generated::sequencer::v1alpha1 as raw,
    native::{
        sequencer::v1alpha1::asset::IncorrectAssetIdLength,
        Protobuf,
    },
};

pub const ADDRESS_LEN: usize = 20;
pub const ROLLUP_ID_LEN: usize = 32;

#[derive(Debug)]
pub struct SignedTransactionError {
    kind: SignedTransactionErrorKind,
}

impl SignedTransactionError {
    fn signature(inner: ed25519_consensus::Error) -> Self {
        Self {
            kind: SignedTransactionErrorKind::Signature(inner),
        }
    }

    fn transaction(inner: UnsignedTransactionError) -> Self {
        Self {
            kind: SignedTransactionErrorKind::Transaction(inner),
        }
    }

    fn verification(inner: ed25519_consensus::Error) -> Self {
        Self {
            kind: SignedTransactionErrorKind::Verification(inner),
        }
    }

    fn verification_key(inner: ed25519_consensus::Error) -> Self {
        Self {
            kind: SignedTransactionErrorKind::VerificationKey(inner),
        }
    }

    fn unset_transaction() -> Self {
        Self {
            kind: SignedTransactionErrorKind::UnsetTransaction,
        }
    }
}

impl Display for SignedTransactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match &self.kind {
            SignedTransactionErrorKind::UnsetTransaction => {
                "`transaction` field of raw protobuf message was not set"
            }
            SignedTransactionErrorKind::Signature(_) => {
                "could not reconstruct an ed25519 signature from the bytes contained in the \
                 `signature` field of the raw protobuf message"
            }
            SignedTransactionErrorKind::Transaction(_) => {
                "the decoded raw unsigned protobuf transaction could not be converted to a native \
                 astria transaction"
            }
            SignedTransactionErrorKind::VerificationKey(_) => {
                "could not reconstruct an ed25519 verification key from the bytes contained in the \
                 `public_key` field of the raw protobuf message"
            }
            SignedTransactionErrorKind::Verification(_) => {
                "the encoded bytes of the raw unsigned protobuf transaction could not be verified"
            }
        };
        f.pad(msg)
    }
}

impl Error for SignedTransactionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            SignedTransactionErrorKind::UnsetTransaction => None,
            SignedTransactionErrorKind::Signature(e)
            | SignedTransactionErrorKind::VerificationKey(e)
            | SignedTransactionErrorKind::Verification(e) => Some(e),
            SignedTransactionErrorKind::Transaction(e) => Some(e),
        }
    }
}

#[derive(Debug)]
enum SignedTransactionErrorKind {
    UnsetTransaction,
    Signature(ed25519_consensus::Error),
    Transaction(UnsignedTransactionError),
    VerificationKey(ed25519_consensus::Error),
    Verification(ed25519_consensus::Error),
}

/// A signed transaction.
///
/// [`SignedTransaction`] contains an [`UnsignedTransaction`] together
/// with its signature and public key.
#[derive(Clone, Debug)]
pub struct SignedTransaction {
    signature: Signature,
    verification_key: VerificationKey,
    transaction: UnsignedTransaction,
}

impl SignedTransaction {
    #[must_use]
    pub fn into_raw(self) -> raw::SignedTransaction {
        let Self {
            signature,
            verification_key,
            transaction,
        } = self;
        raw::SignedTransaction {
            signature: signature.to_bytes().to_vec(),
            public_key: verification_key.to_bytes().to_vec(),
            transaction: Some(transaction.into_raw()),
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::SignedTransaction {
        let Self {
            signature,
            verification_key,
            transaction,
        } = self;
        raw::SignedTransaction {
            signature: signature.to_bytes().to_vec(),
            public_key: verification_key.to_bytes().to_vec(),
            transaction: Some(transaction.to_raw()),
        }
    }

    /// Attempt to convert from a raw, unchecked protobuf [`raw::SignedTransaction`].
    ///
    /// # Errors
    ///
    /// Will return an error if signature or verification key cannot be reconstructed from the bytes
    /// contained in the raw input, if the transaction field was empty (mmeaning it was mapped to
    /// `None`), if the inner transaction could not be verified given the key and signature, or
    /// if the native [`UnsignedTransaction`] could not be created from the inner raw
    /// [`raw::UnsignedTransaction`].
    pub fn try_from_raw(proto: raw::SignedTransaction) -> Result<Self, SignedTransactionError> {
        use crate::Message as _;
        let raw::SignedTransaction {
            signature,
            public_key,
            transaction,
        } = proto;
        let signature =
            Signature::try_from(&*signature).map_err(SignedTransactionError::signature)?;
        let verification_key = VerificationKey::try_from(&*public_key)
            .map_err(SignedTransactionError::verification_key)?;
        let Some(transaction) = transaction else {
            return Err(SignedTransactionError::unset_transaction());
        };
        let bytes = transaction.encode_to_vec();
        verification_key
            .verify(&signature, &bytes)
            .map_err(SignedTransactionError::verification)?;
        let transaction = UnsignedTransaction::try_from_raw(transaction)
            .map_err(SignedTransactionError::transaction)?;
        Ok(Self {
            signature,
            verification_key,
            transaction,
        })
    }

    #[must_use]
    pub fn into_parts(self) -> (Signature, VerificationKey, UnsignedTransaction) {
        let Self {
            signature,
            verification_key,
            transaction,
        } = self;
        (signature, verification_key, transaction)
    }

    #[must_use]
    pub fn actions(&self) -> &[Action] {
        &self.transaction.actions
    }

    #[must_use]
    pub fn signature(&self) -> Signature {
        self.signature
    }

    #[must_use]
    pub fn verification_key(&self) -> VerificationKey {
        self.verification_key
    }

    #[must_use]
    pub fn unsigned_transaction(&self) -> &UnsignedTransaction {
        &self.transaction
    }
}

#[derive(Clone, Debug)]
pub struct UnsignedTransaction {
    pub nonce: u32,
    pub actions: Vec<Action>,
    /// asset to use for fee payment.
    pub fee_asset_id: asset::Id,
}

impl UnsignedTransaction {
    #[must_use]
    pub fn into_signed(self, signing_key: &SigningKey) -> SignedTransaction {
        use crate::Message as _;
        let bytes = self.to_raw().encode_to_vec();
        let signature = signing_key.sign(&bytes);
        let verification_key = signing_key.verification_key();
        SignedTransaction {
            signature,
            verification_key,
            transaction: self,
        }
    }

    pub fn into_raw(self) -> raw::UnsignedTransaction {
        let Self {
            nonce,
            actions,
            fee_asset_id,
        } = self;
        let actions = actions.into_iter().map(Action::into_raw).collect();
        raw::UnsignedTransaction {
            nonce,
            actions,
            fee_asset_id: fee_asset_id.as_bytes().to_vec(),
        }
    }

    pub fn to_raw(&self) -> raw::UnsignedTransaction {
        let Self {
            nonce,
            actions,
            fee_asset_id,
        } = self;
        let actions = actions.iter().map(Action::to_raw).collect();
        raw::UnsignedTransaction {
            nonce: *nonce,
            actions,
            fee_asset_id: fee_asset_id.as_bytes().to_vec(),
        }
    }

    /// Attempt to convert from a raw, unchecked protobuf [`raw::UnsignedTransaction`].
    ///
    /// # Errors
    ///
    /// Returns an error if one of the inner raw actions could not be converted to a native
    /// [`Action`].
    pub fn try_from_raw(proto: raw::UnsignedTransaction) -> Result<Self, UnsignedTransactionError> {
        let raw::UnsignedTransaction {
            nonce,
            actions,
            fee_asset_id,
        } = proto;
        let n_raw_actions = actions.len();
        let actions: Vec<_> = actions
            .into_iter()
            .map(Action::try_from_raw)
            .collect::<Result<_, _>>()
            .map_err(UnsignedTransactionError::action)?;
        if actions.len() != n_raw_actions {
            info!(
                actions.raw = n_raw_actions,
                actions.converted = actions.len(),
                "ignored unset raw protobuf actions",
            );
        }

        let fee_asset_id = asset::Id::try_from_slice(&fee_asset_id)
            .map_err(UnsignedTransactionError::fee_asset_id)?;

        Ok(Self {
            nonce,
            actions,
            fee_asset_id,
        })
    }
}

#[derive(Debug)]
pub struct UnsignedTransactionError {
    kind: UnsignedTransactionErrorKind,
}

impl UnsignedTransactionError {
    fn action(inner: ActionError) -> Self {
        Self {
            kind: UnsignedTransactionErrorKind::Action(inner),
        }
    }

    fn fee_asset_id(inner: IncorrectAssetIdLength) -> Self {
        Self {
            kind: UnsignedTransactionErrorKind::FeeAsset(inner),
        }
    }
}

impl Display for UnsignedTransactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad("constructing unsigned tx failed")
    }
}

impl Error for UnsignedTransactionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            UnsignedTransactionErrorKind::Action(e) => Some(e),
            UnsignedTransactionErrorKind::FeeAsset(e) => Some(e),
        }
    }
}

#[derive(Debug)]
enum UnsignedTransactionErrorKind {
    Action(ActionError),
    FeeAsset(IncorrectAssetIdLength),
}

#[derive(Clone, Debug)]
pub enum Action {
    Sequence(SequenceAction),
    Transfer(TransferAction),
    ValidatorUpdate(tendermint::validator::Update),
    SudoAddressChange(SudoAddressChangeAction),
    Mint(MintAction),
    Ibc(IbcRelay),
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

#[derive(Debug)]
pub struct ActionError {
    kind: ActionErrorKind,
}

impl ActionError {
    fn unset() -> Self {
        Self {
            kind: ActionErrorKind::Unset,
        }
    }

    fn sequence(inner: SequenceActionError) -> Self {
        Self {
            kind: ActionErrorKind::Sequence(inner),
        }
    }

    fn transfer(inner: TransferActionError) -> Self {
        Self {
            kind: ActionErrorKind::Transfer(inner),
        }
    }

    fn validator_update(inner: tendermint::error::Error) -> Self {
        Self {
            kind: ActionErrorKind::ValidatorUpdate(inner),
        }
    }

    fn sudo_address_change(inner: SudoAddressChangeActionError) -> Self {
        Self {
            kind: ActionErrorKind::SudoAddressChange(inner),
        }
    }

    fn mint(inner: MintActionError) -> Self {
        Self {
            kind: ActionErrorKind::Mint(inner),
        }
    }

    fn ibc(inner: Box<dyn Error + Send + Sync>) -> Self {
        Self {
            kind: ActionErrorKind::Ibc(inner),
        }
    }
}

impl Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match &self.kind {
            ActionErrorKind::Unset => "oneof value was not set",
            ActionErrorKind::Sequence(_) => "raw sequence action was not valid",
            ActionErrorKind::Transfer(_) => "raw transfer action was not valid",
            ActionErrorKind::ValidatorUpdate(_) => "raw validator update action was not valid",
            ActionErrorKind::SudoAddressChange(_) => "raw sudo address change action was not valid",
            ActionErrorKind::Mint(_) => "raw mint action was not valid",
            ActionErrorKind::Ibc(_) => "raw ibc action was not valid",
        };
        f.pad(msg)
    }
}

impl Error for ActionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            ActionErrorKind::Unset => None,
            ActionErrorKind::Sequence(e) => Some(e),
            ActionErrorKind::Transfer(e) => Some(e),
            ActionErrorKind::ValidatorUpdate(e) => Some(e),
            ActionErrorKind::SudoAddressChange(e) => Some(e),
            ActionErrorKind::Mint(e) => Some(e),
            ActionErrorKind::Ibc(e) => Some(e.as_ref()),
        }
    }
}

#[derive(Debug)]
enum ActionErrorKind {
    Unset,
    Sequence(SequenceActionError),
    Transfer(TransferActionError),
    ValidatorUpdate(tendermint::error::Error),
    SudoAddressChange(SudoAddressChangeActionError),
    Mint(MintActionError),
    Ibc(Box<dyn Error + Send + Sync>),
}

#[derive(Debug)]
pub struct SequenceActionError {
    kind: SequenceActionErrorKind,
}

impl SequenceActionError {
    fn rollup_id(inner: IncorrectRollupIdLength) -> Self {
        Self {
            kind: SequenceActionErrorKind::RollupId(inner),
        }
    }
}

impl Display for SequenceActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            SequenceActionErrorKind::RollupId(_) => {
                f.pad("`rollup_id` field did not contain a valid rollup ID")
            }
        }
    }
}

impl Error for SequenceActionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            SequenceActionErrorKind::RollupId(e) => Some(e),
        }
    }
}

#[derive(Debug)]
enum SequenceActionErrorKind {
    RollupId(IncorrectRollupIdLength),
}

#[derive(Debug)]
pub struct IncorrectRollupIdLength {
    received: usize,
}

impl Display for IncorrectRollupIdLength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "expected 32 bytes, got {}", self.received)
    }
}

impl Error for IncorrectRollupIdLength {}

#[derive(Clone, Debug)]
pub struct SequenceAction {
    pub rollup_id: RollupId,
    pub data: Vec<u8>,
}

impl SequenceAction {
    #[must_use]
    pub fn into_raw(self) -> raw::SequenceAction {
        let Self {
            rollup_id,
            data,
        } = self;
        raw::SequenceAction {
            rollup_id: rollup_id.to_vec(),
            data,
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::SequenceAction {
        let Self {
            rollup_id,
            data,
        } = self;
        raw::SequenceAction {
            rollup_id: rollup_id.to_vec(),
            data: data.clone(),
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
        } = proto;
        let rollup_id =
            RollupId::try_from_slice(&rollup_id).map_err(SequenceActionError::rollup_id)?;
        Ok(Self {
            rollup_id,
            data,
        })
    }
}

#[derive(Clone, Debug)]
pub struct TransferAction {
    pub to: Address,
    pub amount: u128,
    // asset to be transferred.
    pub asset_id: asset::Id,
}

impl TransferAction {
    #[must_use]
    pub fn into_raw(self) -> raw::TransferAction {
        let Self {
            to,
            amount,
            asset_id,
        } = self;
        raw::TransferAction {
            to: to.to_vec(),
            amount: Some(amount.into()),
            asset_id: asset_id.as_bytes().to_vec(),
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::TransferAction {
        let Self {
            to,
            amount,
            asset_id,
        } = self;
        raw::TransferAction {
            to: to.to_vec(),
            amount: Some((*amount).into()),
            asset_id: asset_id.as_bytes().to_vec(),
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
        } = proto;
        let to = Address::try_from_slice(&to).map_err(TransferActionError::address)?;
        let amount = amount.map_or(0, Into::into);
        let asset_id =
            asset::Id::try_from_slice(&asset_id).map_err(TransferActionError::asset_id)?;

        Ok(Self {
            to,
            amount,
            asset_id,
        })
    }
}

#[derive(Debug)]
pub struct TransferActionError {
    kind: TransferActionErrorKind,
}

impl TransferActionError {
    fn address(inner: IncorrectAddressLength) -> Self {
        Self {
            kind: TransferActionErrorKind::Address(inner),
        }
    }

    fn asset_id(inner: IncorrectAssetIdLength) -> Self {
        Self {
            kind: TransferActionErrorKind::Asset(inner),
        }
    }
}

impl Display for TransferActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            TransferActionErrorKind::Address(_) => {
                f.pad("`to` field did not contain a valid address")
            }
            TransferActionErrorKind::Asset(_) => {
                f.pad("`asset_id` field did not contain a valid asset ID")
            }
        }
    }
}

impl Error for TransferActionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            TransferActionErrorKind::Address(e) => Some(e),
            TransferActionErrorKind::Asset(e) => Some(e),
        }
    }
}

#[derive(Debug)]
enum TransferActionErrorKind {
    Address(IncorrectAddressLength),
    Asset(IncorrectAssetIdLength),
}

#[derive(Clone, Debug)]
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

#[derive(Debug)]
pub struct SudoAddressChangeActionError {
    kind: SudoAddressChangeActionErrorKind,
}

impl SudoAddressChangeActionError {
    fn address(inner: IncorrectAddressLength) -> Self {
        Self {
            kind: SudoAddressChangeActionErrorKind::Address(inner),
        }
    }
}

impl Display for SudoAddressChangeActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            SudoAddressChangeActionErrorKind::Address(_) => {
                f.pad("`new_address` field did not contain a valid address")
            }
        }
    }
}

impl Error for SudoAddressChangeActionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            SudoAddressChangeActionErrorKind::Address(e) => Some(e),
        }
    }
}

#[derive(Debug)]
enum SudoAddressChangeActionErrorKind {
    Address(IncorrectAddressLength),
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
#[derive(Debug)]
pub struct MintActionError {
    kind: MintActionErrorKind,
}

impl MintActionError {
    fn address(inner: IncorrectAddressLength) -> Self {
        Self {
            kind: MintActionErrorKind::Address(inner),
        }
    }
}

impl Display for MintActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            MintActionErrorKind::Address(_) => f.pad("`to` field did not contain a valid address"),
        }
    }
}

impl Error for MintActionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            MintActionErrorKind::Address(e) => Some(e),
        }
    }
}

#[derive(Debug)]
enum MintActionErrorKind {
    Address(IncorrectAddressLength),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Address(pub [u8; ADDRESS_LEN]);

impl Address {
    #[must_use]
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// Construct a sequencer address from a [`ed25519_consensus::VerificationKey`].
    ///
    /// The first 20 bytes of the sha256 hash of the verification key is the address.
    #[must_use]
    // Silence the clippy lint because the function body asserts that the panic
    // cannot happen.
    #[allow(clippy::missing_panics_doc)]
    pub fn from_verification_key(public_key: ed25519_consensus::VerificationKey) -> Self {
        /// this ensures that `ADDRESS_LEN` is never accidentally changed to a value
        /// that would violate this assumption.
        #[allow(clippy::assertions_on_constants)]
        const _: () = assert!(ADDRESS_LEN <= 32);
        let bytes: [u8; 32] = Sha256::digest(public_key).into();
        Self::try_from_slice(&bytes[..ADDRESS_LEN])
            .expect("can convert 32 byte hash to 20 byte array")
    }

    /// Convert a byte slice to an address.
    ///
    /// # Errors
    ///
    /// Returns an error if the account buffer was not 20 bytes long.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, IncorrectAddressLength> {
        let inner = <[u8; ADDRESS_LEN]>::try_from(bytes).map_err(|_| IncorrectAddressLength {
            received: bytes.len(),
        })?;
        Ok(Self::from_array(inner))
    }

    #[must_use]
    pub fn from_array(array: [u8; ADDRESS_LEN]) -> Self {
        Self(array)
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; ADDRESS_LEN]> for Address {
    fn from(inner: [u8; ADDRESS_LEN]) -> Self {
        Self(inner)
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct RollupId {
    #[cfg_attr(feature = "serde", serde(with = "hex::serde"))]
    inner: [u8; 32],
}

impl RollupId {
    /// Creates a new rollup ID from a 32 byte array.
    ///
    /// Use this if you already have a 32 byte array. Prefer
    /// [`RollupId::from_unhashed_bytes`] if you have a clear text
    /// name what you want to use to identify your rollup.
    ///
    /// # Examples
    /// ```
    /// use astria_proto::native::sequencer::v1alpha1::RollupId;
    /// let bytes = [42u8; 32];
    /// let rollup_id = RollupId::new(bytes);
    /// assert_eq!(bytes, rollup_id.get());
    /// ```
    #[must_use]
    pub fn new(inner: [u8; ROLLUP_ID_LEN]) -> Self {
        Self {
            inner,
        }
    }

    /// Returns the 32 bytes array representing the rollup ID.
    ///
    /// # Examples
    /// ```
    /// use astria_proto::native::sequencer::v1alpha1::RollupId;
    /// let bytes = [42u8; 32];
    /// let rollup_id = RollupId::new(bytes);
    /// assert_eq!(bytes, rollup_id.get());
    /// ```
    #[must_use]
    pub const fn get(self) -> [u8; 32] {
        self.inner
    }

    /// Creates a new rollup ID by applying Sha256 to `bytes`.
    ///
    /// Examples
    /// ```
    /// use astria_proto::native::sequencer::v1alpha1::RollupId;
    /// use sha2::{
    ///     Digest,
    ///     Sha256,
    /// };
    /// let name = "MyRollup-1";
    /// let hashed = Sha256::digest(name);
    /// let rollup_id = RollupId::from_unhashed_bytes(name);
    /// assert_eq!(rollup_id, RollupId::new(hashed.into()));
    /// ```
    #[must_use]
    pub fn from_unhashed_bytes<T: AsRef<[u8]>>(bytes: T) -> Self {
        Self {
            inner: Sha256::digest(bytes).into(),
        }
    }

    /// Allocates a vector from the fixed size array holding the rollup ID.
    ///
    /// # Examples
    /// ```
    /// use astria_proto::native::sequencer::v1alpha1::RollupId;
    /// let rollup_id = RollupId::new([42u8; 32]);
    /// assert_eq!(vec![42u8; 32], rollup_id.to_vec());
    /// ```
    #[must_use]
    pub fn to_vec(self) -> Vec<u8> {
        self.inner.to_vec()
    }

    /// Convert a byte slice to a rollup ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the byte slice was not 32 bytes long.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, IncorrectRollupIdLength> {
        let inner =
            <[u8; ROLLUP_ID_LEN]>::try_from(bytes).map_err(|_| IncorrectRollupIdLength {
                received: bytes.len(),
            })?;
        Ok(Self::new(inner))
    }

    /// Converts a byte vector to a rollup ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the byte slice was not 32 bytes long.
    pub fn try_from_vec(bytes: Vec<u8>) -> Result<Self, IncorrectRollupIdLength> {
        let inner =
            <[u8; ROLLUP_ID_LEN]>::try_from(bytes).map_err(|bytes| IncorrectRollupIdLength {
                received: bytes.len(),
            })?;
        Ok(Self::new(inner))
    }
}

impl AsRef<[u8]> for RollupId {
    fn as_ref(&self) -> &[u8] {
        &self.inner
    }
}

impl From<[u8; ROLLUP_ID_LEN]> for RollupId {
    fn from(inner: [u8; ROLLUP_ID_LEN]) -> Self {
        Self {
            inner,
        }
    }
}

impl Display for RollupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.inner {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl raw::BalanceResponse {
    /// Converts an astria native [`BalanceResponse`] to a
    /// protobuf [`raw::BalanceResponse`].
    #[must_use]
    pub fn from_native(native: BalanceResponse) -> Self {
        let BalanceResponse {
            height,
            balance,
        } = native;
        Self {
            height,
            balance: Some(balance.into()),
        }
    }

    /// Converts a protobuf [`raw::BalanceResponse`] to an astria
    /// native [`BalanceResponse`].
    #[must_use]
    pub fn into_native(self) -> BalanceResponse {
        BalanceResponse::from_raw(&self)
    }

    /// Converts a protobuf [`raw::BalanceResponse`] to an astria
    /// native [`BalanceResponse`] by allocating a new [`v1alpha::BalanceResponse`].
    #[must_use]
    pub fn to_native(&self) -> BalanceResponse {
        self.clone().into_native()
    }
}

/// The sequencer response to a balance request for a given account at a given height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BalanceResponse {
    pub height: u64,
    pub balance: u128,
}

impl BalanceResponse {
    /// Converts a protobuf [`raw::BalanceResponse`] to an astria
    /// native [`BalanceResponse`].
    pub fn from_raw(proto: &raw::BalanceResponse) -> Self {
        let raw::BalanceResponse {
            height,
            balance,
        } = *proto;
        Self {
            height,
            balance: balance.map_or(0, Into::into),
        }
    }

    /// Converts an astria native [`BalanceResponse`] to a
    /// protobuf [`raw::BalanceResponse`].
    #[must_use]
    pub fn into_raw(self) -> raw::BalanceResponse {
        raw::BalanceResponse::from_native(self)
    }
}

impl raw::NonceResponse {
    /// Converts a protobuf [`raw::NonceResponse`] to a native
    /// astria `NonceResponse`.
    #[must_use]
    pub fn from_native(native: NonceResponse) -> Self {
        let NonceResponse {
            height,
            nonce,
        } = native;
        Self {
            height,
            nonce,
        }
    }

    /// Converts a protobuf [`raw::NonceResponse`] to an astria
    /// native [`NonceResponse`].
    #[must_use]
    pub fn into_native(self) -> NonceResponse {
        NonceResponse::from_raw(&self)
    }

    /// Converts a protobuf [`raw::NonceResponse`] to an astria
    /// native [`NonceResponse`] by allocating a new [`v1alpha::NonceResponse`].
    #[must_use]
    pub fn to_native(&self) -> NonceResponse {
        self.clone().into_native()
    }
}

/// The sequencer response to a nonce request for a given account at a given height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NonceResponse {
    pub height: u64,
    pub nonce: u32,
}

impl NonceResponse {
    /// Converts a protobuf [`raw::NonceResponse`] to an astria
    /// native [`NonceResponse`].
    #[must_use]
    pub fn from_raw(proto: &raw::NonceResponse) -> Self {
        let raw::NonceResponse {
            height,
            nonce,
        } = *proto;
        Self {
            height,
            nonce,
        }
    }

    /// Converts an astria native [`NonceResponse`] to a
    /// protobuf [`raw::NonceResponse`].
    #[must_use]
    pub fn into_raw(self) -> raw::NonceResponse {
        raw::NonceResponse::from_native(self)
    }
}

/// Indicates that the protobuf response contained an array field that was not 20 bytes long.
#[derive(Debug)]
pub struct IncorrectAddressLength {
    received: usize,
}

impl Display for IncorrectAddressLength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "expected 20 bytes, got {}", self.received)
    }
}

impl Error for IncorrectAddressLength {}

impl Protobuf for merkle::Proof {
    type Error = merkle::audit::InvalidProof;
    type Raw = raw::Proof;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        // XXX: Implementing this by cloning is ok because `audit_path`
        //      has to be cloned always due to `UncheckedProof`'s constructor.
        Self::try_from_raw(raw.clone())
    }

    fn try_from_raw(raw: Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            audit_path,
            leaf_index,
            tree_size,
        } = raw;
        let leaf_index = leaf_index.try_into().expect(
            "running on a machine with at least 64 bit pointer width and can convert from u64 to \
             usize",
        );
        let tree_size = tree_size.try_into().expect(
            "running on a machine with at least 64 bit pointer width and can convert from u64 to \
             usize",
        );
        Self::unchecked()
            .audit_path(audit_path)
            .leaf_index(leaf_index)
            .tree_size(tree_size)
            .try_into_proof()
    }

    fn to_raw(&self) -> Self::Raw {
        // XXX: Implementing in terms of clone is ok because the fields would need to be cloned
        // anyway.
        self.clone().into_raw()
    }

    fn into_raw(self) -> Self::Raw {
        let merkle::audit::UncheckedProof {
            audit_path,
            leaf_index,
            tree_size,
        } = self.into_unchecked();
        Self::Raw {
            audit_path,
            leaf_index: leaf_index.try_into().expect(
                "running on a machine with at most 64 bit pointer width and can convert from \
                 usize to u64",
            ),
            tree_size: tree_size.try_into().expect(
                "running on a machine with at most 64 bit pointer width and can convert from \
                 usize to u64",
            ),
        }
    }
}

#[derive(Debug)]
pub enum RollupTransactionsError {
    RollupId(IncorrectRollupIdLength),
}

/// The opaque transactions belonging to a rollup identified by its rollup ID.
#[derive(Clone)]
pub struct RollupTransactions {
    /// The 32 bytes identifying a rollup. Usually the sha256 hash of a plain rollup name.
    id: RollupId,
    /// The serialized opaque bytes of the rollup transactions.
    transactions: Vec<Vec<u8>>,
}

impl RollupTransactions {
    /// Returns the [`RollupId`] identifying the rollup these transactions belong to.
    #[must_use]
    pub fn id(&self) -> RollupId {
        self.id
    }

    /// Returns the opaque transactions bytes.
    #[must_use]
    pub fn transactions(&self) -> &[Vec<u8>] {
        &self.transactions
    }

    /// Transforms these rollup transactions into their raw representation, which can in turn be
    /// encoded as protobuf.
    #[must_use]
    pub fn into_raw(self) -> raw::RollupTransactions {
        let Self {
            id,
            transactions,
        } = self;
        raw::RollupTransactions {
            id: id.get().to_vec(),
            transactions,
        }
    }

    /// Attempts to transform the rollup transactions from their raw representation.
    ///
    /// # Errors
    /// Returns an error if the rollup ID bytes could not be turned into a [`RollupId`].
    pub fn try_from_raw(raw: raw::RollupTransactions) -> Result<Self, RollupTransactionsError> {
        let raw::RollupTransactions {
            id,
            transactions,
        } = raw;
        let id = RollupId::try_from_slice(&id).map_err(RollupTransactionsError::RollupId)?;
        Ok(Self {
            id,
            transactions,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SequencerBlockError {
    #[error(
        "the CometBFT block.header.data_hash does not match the Merkle Tree Hash derived from \
         block.data"
    )]
    CometBftDataHashDoesNotMatchReconstructed,
    #[error("hashing the CometBFT block.header returned an empty hash which is not permitted")]
    CometBftBlockHashIsNone,
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("failed creating a native cometbft Header from the raw protobuf header")]
    Header(#[source] tendermint::Error),
    #[error(
        "failed parsing a raw protobuf rollup transaction because it contained an invalid rollup \
         ID"
    )]
    ParseRollupTransactions(#[source] IncorrectRollupIdLength),
    #[error("failed constructing a transaction proof from the raw protobuf transaction proof")]
    TransactionProofInvalid(#[source] merkle::audit::InvalidProof),
    #[error("failed constructing a rollup ID proof from the raw protobuf rollup ID proof")]
    IdProofInvalid(#[source] merkle::audit::InvalidProof),
    #[error(
        "the cometbft block.data field was too short and did not contain the rollup transaction \
         root"
    )]
    NoRollupTransactionsRoot,
    #[error(
        "the rollup transaction root in the cometbft block.data field was expected to be 32 bytes \
         long, but was actually `{0}`"
    )]
    IncorrectRollupTransactionsRootLength(usize),
    #[error("the cometbft block.data field was too short and did not contain the rollup ID root")]
    NoRollupIdsRoot,
    #[error(
        "the rollup ID root in the cometbft block.data field was expected to be 32 bytes long, \
         but was actually `{0}`"
    )]
    IncorrectRollupIdsRootLength(usize),
    #[error(
        "the Merkle Tree Hash derived from the rollup transactions recorded in the raw protobuf \
         sequencer block could not be verified against their proof and the block's data hash"
    )]
    RollupTransactionsNotInSequencerBlock,
    #[error(
        "the Merkle Tree Hash derived from the rollup IDs recorded in the raw protobuf sequencer \
         block could not be verified against their proof and the block's data hash"
    )]
    RollupIdsNotInSequencerBlock,
    #[error(
        "failed decoding an entry in the cometbft block.data field as a protobuf signed astria \
         transaction"
    )]
    SignedTransactionProtobufDecode(#[source] prost::DecodeError),
    #[error(
        "failed converting a raw protobuf signed transaction decoded from the cometbft block.data
        field to a native astria signed transaction"
    )]
    RawSignedTransactionConversion(#[source] SignedTransactionError),
    #[error(
        "the root derived from the rollup transactions in the cometbft block.data field did not \
         match the root stored in the same block.data field"
    )]
    RollupTransactionsRootDoesNotMatchReconstructed,
    #[error(
        "the root derived from the rollup IDs in the cometbft block.data field did not match the \
         root stored in the same block.data field"
    )]
    RollupIdsRootDoesNotMatchReconstructed,
}

/// A shadow of [`SequencerBlock`] with full public access to its fields.
///
/// This type does not guarantee any invariants and is mainly useful to get
/// access the sequencer block's internal types.
#[derive(Clone, Debug)]
pub struct UncheckedSequencerBlock {
    /// The original `CometBFT` header that was the input to this sequencer block.
    pub header: tendermint::block::header::Header,
    /// The collection of rollup transactions that were included in this block.
    pub rollup_transactions: IndexMap<RollupId, Vec<Vec<u8>>>,
    // The proof that the rollup transactions are included in the `CometBFT` block this
    // sequencer block is derived form. This proof together with
    // `Sha256(MTH(rollup_transactions))` must match `header.data_hash`.
    // `MTH(rollup_transactions)` is the Merkle Tree Hash derived from the
    // rollup transactions.
    pub rollup_transactions_proof: merkle::Proof,
    // The proof that the rollup IDs listed in `rollup_transactions` are included
    // in the `CometBFT` block this sequencer block is derived form. This proof together
    // with `Sha256(MTH(rollup_ids))` must match `header.data_hash`.
    // `MTH(rollup_ids)` is the Merkle Tree Hash derived from the rollup IDs listed in
    // the rollup transactions.
    pub rollup_ids_proof: merkle::Proof,
}

/// `SequencerBlock` is constructed from a tendermint/cometbft block by
/// converting its opaque `data` bytes into sequencer specific types.
#[derive(Clone, Debug, PartialEq)]
pub struct SequencerBlock {
    /// The result of hashing `header`. Guaranteed to not be `None` as compared to
    /// the cometbft/tendermint-rs return type.
    block_hash: [u8; 32],
    /// The original `CometBFT` header that was the input to this sequencer block.
    header: tendermint::block::header::Header,
    /// The collection of rollup transactions that were included in this block.
    rollup_transactions: IndexMap<RollupId, Vec<Vec<u8>>>,
    // The proof that the rollup transactions are included in the `CometBFT` block this
    // sequencer block is derived form. This proof together with
    // `Sha256(MTH(rollup_transactions))` must match `header.data_hash`.
    // `MTH(rollup_transactions)` is the Merkle Tree Hash derived from the
    // rollup transactions.
    rollup_transactions_proof: merkle::Proof,
    // The proof that the rollup IDs listed in `rollup_transactions` are included
    // in the `CometBFT` block this sequencer block is derived form. This proof together
    // with `Sha256(MTH(rollup_ids))` must match `header.data_hash`.
    // `MTH(rollup_ids)` is the Merkle Tree Hash derived from the rollup IDs listed in
    // the rollup transactions.
    rollup_ids_proof: merkle::Proof,
}

impl SequencerBlock {
    /// Returns the hash of the `CometBFT` block this sequencer block is derived from.
    ///
    /// This is done by hashing the `CometBFT` header stored in this block.
    #[must_use]
    pub fn block_hash(&self) -> [u8; 32] {
        self.block_hash
    }

    #[must_use]
    pub fn header(&self) -> &tendermint::block::header::Header {
        &self.header
    }

    #[must_use]
    pub fn rollup_transactions(&self) -> &IndexMap<RollupId, Vec<Vec<u8>>> {
        &self.rollup_transactions
    }

    #[must_use]
    pub fn into_raw(self) -> raw::SequencerBlock {
        fn tuple_to_rollup_txs(
            (rollup_id, transactions): (RollupId, Vec<Vec<u8>>),
        ) -> raw::RollupTransactions {
            raw::RollupTransactions {
                id: rollup_id.to_vec(),
                transactions,
            }
        }

        let Self {
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
            ..
        } = self;
        raw::SequencerBlock {
            header: Some(header.into()),
            rollup_transactions: rollup_transactions
                .into_iter()
                .map(tuple_to_rollup_txs)
                .collect(),
            rollup_transactions_proof: Some(rollup_transactions_proof.into_raw()),
            rollup_ids_proof: Some(rollup_ids_proof.into_raw()),
        }
    }

    #[must_use]
    pub fn into_unchecked(self) -> UncheckedSequencerBlock {
        let Self {
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
            ..
        } = self;
        UncheckedSequencerBlock {
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
        }
    }

    /// Turn the sequencer block into a [`CelestiaSequencerBlob`] and its associated list of
    /// [`CelestiaRollupBlob`]s.
    #[must_use]
    #[allow(clippy::missing_panics_doc)] // the proofs are guaranteed to exist; revisit if refactoring
    pub fn to_celestia_blobs(&self) -> (CelestiaSequencerBlob, Vec<CelestiaRollupBlob>) {
        let SequencerBlock {
            block_hash,
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
        } = self;

        let tree = derive_merkle_tree_from_rollup_txs(rollup_transactions);

        let head = CelestiaSequencerBlob {
            block_hash: *block_hash,
            header: header.clone(),
            rollup_ids: rollup_transactions.keys().copied().collect(),
            rollup_transactions_root: tree.root(),
            rollup_transactions_proof: rollup_transactions_proof.clone(),
            rollup_ids_proof: rollup_ids_proof.clone(),
        };

        let mut tail = Vec::with_capacity(self.rollup_transactions.len());
        for (i, (rollup_id, transactions)) in self.rollup_transactions.iter().enumerate() {
            let proof = tree
                .construct_proof(i)
                .expect("the proof must exist because the tree was derived with the same leaf");
            tail.push(CelestiaRollupBlob {
                sequencer_block_hash: self.block_hash(),
                rollup_id: *rollup_id,
                transactions: transactions.clone(),
                proof,
            });
        }
        (head, tail)
    }

    /// Converts from a [`tendermint::Block`].
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    #[allow(clippy::missing_panics_doc)] // the panic sources are checked before hand; revisit if refactoring
    pub fn try_from_cometbft(block: tendermint::Block) -> Result<Self, SequencerBlockError> {
        use prost::Message as _;

        let tendermint::Block {
            header,
            data,
            ..
        } = block;

        let Some(tendermint::Hash::Sha256(data_hash)) = header.data_hash else {
            // header.data_hash is Option<Hash> and Hash itself has
            // variants Sha256([u8; 32]) or None.
            return Err(SequencerBlockError::FieldNotSet("header.data_hash"));
        };

        let tendermint::Hash::Sha256(block_hash) = header.hash() else {
            return Err(SequencerBlockError::CometBftBlockHashIsNone);
        };

        let tree = merkle_tree_from_data(&data);
        if tree.root() != data_hash {
            return Err(SequencerBlockError::CometBftDataHashDoesNotMatchReconstructed);
        }

        let mut data_list = data.into_iter();
        let rollup_transactions_root: [u8; 32] = data_list
            .next()
            .ok_or(SequencerBlockError::NoRollupTransactionsRoot)?
            .try_into()
            .map_err(|e: Vec<_>| {
                SequencerBlockError::IncorrectRollupTransactionsRootLength(e.len())
            })?;

        let rollup_ids_root: [u8; 32] = data_list
            .next()
            .ok_or(SequencerBlockError::NoRollupIdsRoot)?
            .try_into()
            .map_err(|e: Vec<_>| SequencerBlockError::IncorrectRollupIdsRootLength(e.len()))?;

        let mut rollup_transactions = IndexMap::new();
        for elem in data_list {
            let raw_tx = raw::SignedTransaction::decode(&*elem)
                .map_err(SequencerBlockError::SignedTransactionProtobufDecode)?;
            let signed_tx = SignedTransaction::try_from_raw(raw_tx)
                .map_err(SequencerBlockError::RawSignedTransactionConversion)?;
            for action in signed_tx.transaction.actions {
                if let Action::Sequence(SequenceAction {
                    rollup_id,
                    data,
                }) = action
                {
                    let elem = rollup_transactions.entry(rollup_id).or_insert(vec![]);
                    elem.push(data);
                }
            }
        }
        rollup_transactions.sort_unstable_keys();

        if rollup_transactions_root
            != derive_merkle_tree_from_rollup_txs(&rollup_transactions).root()
        {
            return Err(SequencerBlockError::RollupTransactionsRootDoesNotMatchReconstructed);
        }

        // ensure the rollup IDs commitment matches the one calculated from the rollup data
        if rollup_ids_root != merkle::Tree::from_leaves(rollup_transactions.keys()).root() {
            return Err(SequencerBlockError::RollupIdsRootDoesNotMatchReconstructed);
        }

        // action tree root is always the first tx in a block
        let rollup_transactions_proof = tree.construct_proof(0).expect(
            "the tree has at least one leaf; if this line is reached and `construct_proof` \
             returns None it means that the short circuiting checks above it have been removed",
        );

        let rollup_ids_proof = tree.construct_proof(1).expect(
            "the tree has at least two leaves; if this line is reached and `construct_proof` \
             returns None it means that the short circuiting checks above it have been removed",
        );

        Ok(Self {
            block_hash,
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
        })
    }

    /// Converts from the raw decoded protobuf representatin of this type.
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    pub fn try_from_raw(raw: raw::SequencerBlock) -> Result<Self, SequencerBlockError> {
        fn rollup_txs_to_tuple(
            raw: raw::RollupTransactions,
        ) -> Result<(RollupId, Vec<Vec<u8>>), IncorrectRollupIdLength> {
            let id = RollupId::try_from_slice(&raw.id)?;
            Ok((id, raw.transactions))
        }

        let raw::SequencerBlock {
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
        } = raw;

        let rollup_transactions_proof = 'proof: {
            let Some(rollup_transactions_proof) = rollup_transactions_proof else {
                break 'proof Err(SequencerBlockError::FieldNotSet(
                    "rollup_transactions_proof",
                ));
            };
            merkle::Proof::try_from_raw(rollup_transactions_proof)
                .map_err(SequencerBlockError::TransactionProofInvalid)
        }?;
        let rollup_ids_proof = 'proof: {
            let Some(rollup_ids_proof) = rollup_ids_proof else {
                break 'proof Err(SequencerBlockError::FieldNotSet("rollup_ids_proof"));
            };
            merkle::Proof::try_from_raw(rollup_ids_proof)
                .map_err(SequencerBlockError::IdProofInvalid)
        }?;
        let header = 'header: {
            let Some(header) = header else {
                break 'header Err(SequencerBlockError::FieldNotSet("header"));
            };
            tendermint::block::Header::try_from(header).map_err(SequencerBlockError::Header)
        }?;
        let tendermint::Hash::Sha256(block_hash) = header.hash() else {
            return Err(SequencerBlockError::CometBftBlockHashIsNone);
        };

        // header.data_hash is Option<Hash> and Hash itself has
        // variants Sha256([u8; 32]) or None.
        let Some(tendermint::Hash::Sha256(data_hash)) = header.data_hash else {
            return Err(SequencerBlockError::FieldNotSet("header.data_hash"));
        };

        let rollup_transactions = rollup_transactions
            .into_iter()
            .map(rollup_txs_to_tuple)
            .collect::<Result<_, _>>()
            .map_err(SequencerBlockError::ParseRollupTransactions)?;

        if !are_rollup_txs_included(&rollup_transactions, &rollup_transactions_proof, data_hash) {
            return Err(SequencerBlockError::RollupTransactionsNotInSequencerBlock);
        }
        if !are_rollup_ids_included(
            rollup_transactions.keys().copied(),
            &rollup_ids_proof,
            data_hash,
        ) {
            return Err(SequencerBlockError::RollupIdsNotInSequencerBlock);
        }

        Ok(Self {
            block_hash,
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed constructing a celestia rollup blob")]
pub struct CelestiaRollupBlobError {
    #[source]
    kind: CelestiaRollupBlobErrorKind,
}

impl CelestiaRollupBlobError {
    fn field_not_set(field: &'static str) -> Self {
        Self {
            kind: CelestiaRollupBlobErrorKind::FieldNotSet {
                field,
            },
        }
    }

    fn rollup_id(source: IncorrectRollupIdLength) -> Self {
        Self {
            kind: CelestiaRollupBlobErrorKind::RollupId {
                source,
            },
        }
    }

    fn proof(source: <merkle::Proof as Protobuf>::Error) -> Self {
        Self {
            kind: CelestiaRollupBlobErrorKind::Proof {
                source,
            },
        }
    }

    fn sequencer_block_hash(actual_len: usize) -> Self {
        Self {
            kind: CelestiaRollupBlobErrorKind::SequencerBlockHash(actual_len),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum CelestiaRollupBlobErrorKind {
    #[error("the expected field in the raw source type was not set: `{field}`")]
    FieldNotSet { field: &'static str },
    #[error("failed converting the provided bytes to Rollup ID")]
    RollupId { source: IncorrectRollupIdLength },
    #[error("failed constructing a Merkle Hash Tree Proof from the provided raw protobf type")]
    Proof {
        source: <merkle::Proof as Protobuf>::Error,
    },
    #[error(
        "the provided bytes were too short for a sequencer block hash. Expected: 32 bytes, \
         provided: {0}"
    )]
    SequencerBlockHash(usize),
}

/// A shadow of [`CelestiaRollupBlob`] with public access to all its fields.
///
/// At the moment there are no invariants upheld by [`CelestiaRollupBlob`] so
/// they can be converted directly into one another. This can change in the future.
pub struct UncheckedCelestiaRollupBlob {
    /// The hash of the sequencer block. Must be 32 bytes.
    pub sequencer_block_hash: [u8; 32],
    /// The 32 bytes identifying the rollup this blob belongs to. Matches
    /// `astria.sequencer.v1alpha1.RollupTransactions.rollup_id`
    pub rollup_id: RollupId,
    /// A list of opaque bytes that are serialized rollup transactions.
    pub transactions: Vec<Vec<u8>>,
    /// The proof that these rollup transactions are included in sequencer block.
    pub proof: merkle::Proof,
}

impl UncheckedCelestiaRollupBlob {
    #[must_use]
    pub fn into_celestia_rollup_blob(self) -> CelestiaRollupBlob {
        CelestiaRollupBlob::from_unchecked(self)
    }
}

#[derive(Clone, Debug)]
pub struct CelestiaRollupBlob {
    /// The hash of the sequencer block. Must be 32 bytes.
    sequencer_block_hash: [u8; 32],
    /// The 32 bytes identifying the rollup this blob belongs to. Matches
    /// `astria.sequencer.v1alpha1.RollupTransactions.rollup_id`
    rollup_id: RollupId,
    /// A list of opaque bytes that are serialized rollup transactions.
    transactions: Vec<Vec<u8>>,
    /// The proof that these rollup transactions are included in sequencer block.
    proof: merkle::Proof,
}

impl CelestiaRollupBlob {
    #[must_use]
    pub fn proof(&self) -> &merkle::Proof {
        &self.proof
    }

    #[must_use]
    pub fn transactions(&self) -> &[Vec<u8>] {
        &self.transactions
    }

    #[must_use]
    pub fn rollup_id(&self) -> RollupId {
        self.rollup_id
    }

    #[must_use]
    pub fn sequencer_block_hash(&self) -> [u8; 32] {
        self.sequencer_block_hash
    }

    /// Converts from the unchecked representation of this type (its shadow).
    ///
    /// This type does not uphold any extra invariants so there are no extra checks necessary.
    #[must_use]
    pub fn from_unchecked(unchecked: UncheckedCelestiaRollupBlob) -> Self {
        let UncheckedCelestiaRollupBlob {
            sequencer_block_hash,
            rollup_id,
            transactions,
            proof,
        } = unchecked;
        Self {
            sequencer_block_hash,
            rollup_id,
            transactions,
            proof,
        }
    }

    /// Converts to the unchecked representation of this type (its shadow).
    ///
    /// Useful to get public access to the type's fields.
    #[must_use]
    pub fn into_unchecked(self) -> UncheckedCelestiaRollupBlob {
        let Self {
            sequencer_block_hash,
            rollup_id,
            transactions,
            proof,
        } = self;
        UncheckedCelestiaRollupBlob {
            sequencer_block_hash,
            rollup_id,
            transactions,
            proof,
        }
    }

    /// Converts to the raw decoded protobuf representation of this type.
    ///
    /// Useful for then encoding it as protobuf.
    #[must_use]
    pub fn into_raw(self) -> raw::CelestiaRollupBlob {
        let Self {
            sequencer_block_hash,
            rollup_id,
            transactions,
            proof,
        } = self;
        raw::CelestiaRollupBlob {
            sequencer_block_hash: sequencer_block_hash.to_vec(),
            rollup_id: rollup_id.to_vec(),
            transactions,
            proof: Some(proof.into_raw()),
        }
    }

    /// Converts from the raw decoded protobuf representation of this type.
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    pub fn try_from_raw(raw: raw::CelestiaRollupBlob) -> Result<Self, CelestiaRollupBlobError> {
        let raw::CelestiaRollupBlob {
            sequencer_block_hash,
            rollup_id,
            transactions,
            proof,
        } = raw;
        let rollup_id =
            RollupId::try_from_vec(rollup_id).map_err(CelestiaRollupBlobError::rollup_id)?;
        let sequencer_block_hash = sequencer_block_hash
            .try_into()
            .map_err(|bytes: Vec<u8>| CelestiaRollupBlobError::sequencer_block_hash(bytes.len()))?;
        let proof = 'proof: {
            let Some(proof) = proof else {
                break 'proof Err(CelestiaRollupBlobError::field_not_set("proof"));
            };
            merkle::Proof::try_from_raw(proof).map_err(CelestiaRollupBlobError::proof)
        }?;
        Ok(Self {
            sequencer_block_hash,
            rollup_id,
            transactions,
            proof,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed constructing a celestia sequencer blob")]
pub struct CelestiaSequencerBlobError {
    #[source]
    kind: CelestiaSequencerBlobErrorKind,
}

impl CelestiaSequencerBlobError {
    fn empty_cometbft_block_hash() -> Self {
        Self {
            kind: CelestiaSequencerBlobErrorKind::EmptyCometBftBlockHash,
        }
    }

    fn cometbft_header(source: tendermint::Error) -> Self {
        Self {
            kind: CelestiaSequencerBlobErrorKind::CometBftHeader {
                source,
            },
        }
    }

    fn field_not_set(field: &'static str) -> Self {
        Self {
            kind: CelestiaSequencerBlobErrorKind::FieldNotSet(field),
        }
    }

    fn rollup_ids(source: IncorrectRollupIdLength) -> Self {
        Self {
            kind: CelestiaSequencerBlobErrorKind::RollupIds {
                source,
            },
        }
    }

    fn rollup_transactions_root(actual_len: usize) -> Self {
        Self {
            kind: CelestiaSequencerBlobErrorKind::RollupTransactionsRoot(actual_len),
        }
    }

    fn rollup_transactions_proof(source: <merkle::Proof as Protobuf>::Error) -> Self {
        Self {
            kind: CelestiaSequencerBlobErrorKind::RollupTransactionsProof {
                source,
            },
        }
    }

    fn rollup_ids_proof(source: <merkle::Proof as Protobuf>::Error) -> Self {
        Self {
            kind: CelestiaSequencerBlobErrorKind::RollupIdsProof {
                source,
            },
        }
    }

    fn rollup_transactions_not_in_cometbft_block() -> Self {
        Self {
            kind: CelestiaSequencerBlobErrorKind::RollupTransactiosnNotInCometBftBlock,
        }
    }

    fn rollup_ids_not_in_cometbft_block() -> Self {
        Self {
            kind: CelestiaSequencerBlobErrorKind::RollupIdsNotInCometBftBlock,
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum CelestiaSequencerBlobErrorKind {
    #[error("the hash derived from the cometbft header was empty where it should be 32 bytes")]
    EmptyCometBftBlockHash,
    #[error("failed constructing the cometbft header from its raw source value")]
    CometBftHeader { source: tendermint::Error },
    #[error("the field of the raw source value was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("one of the rollup IDs in the raw source value was invalid")]
    RollupIds { source: IncorrectRollupIdLength },
    #[error(
        "the provided bytes were too short for a rollup transactions Merkle Tree Hash; expected: \
         32 bytes, actual: {0} bytes"
    )]
    RollupTransactionsRoot(usize),
    #[error(
        "failed constructing a Merkle Hash Tree Proof for the rollup transactions from the raw \
         raw source type"
    )]
    RollupTransactionsProof {
        source: <merkle::Proof as Protobuf>::Error,
    },
    #[error(
        "failed constructing a Merkle Hash Tree Proof for the rollup IDs from the raw raw source \
         type"
    )]
    RollupIdsProof {
        source: <merkle::Proof as Protobuf>::Error,
    },
    #[error(
        "the Merkle Tree Hash of the rollup transactions was not a leaf in the sequencer block \
         data"
    )]
    RollupTransactiosnNotInCometBftBlock,
    #[error("the Merkle Tree Hash of the rollup IDs was not a leaf in the sequencer block data")]
    RollupIdsNotInCometBftBlock,
}

/// A shadow of [`CelestiaSequencerBlob`] with public access to its fields.
///
/// This type does not guarantee any invariants and is mainly useful to get
/// access the sequencer block's internal types.
#[derive(Clone, Debug)]
pub struct UncheckedCelestiaSequencerBlob {
    /// The original `CometBFT` header that is the input to this blob's original sequencer block.
    /// Corresponds to `astria.sequencer.v1alpha.SequencerBlock.header`.
    pub header: tendermint::block::header::Header,
    /// The rollup rollup IDs for which `CelestiaRollupBlob`s were submitted to celestia.
    /// Corresponds to the `astria.sequencer.v1alpha1.RollupTransactions.id` field
    /// and is extracted from `astria.sequencer.v1alpha.SequencerBlock.rollup_transactions`.
    pub rollup_ids: Vec<RollupId>,
    /// The Merkle Tree Hash of the rollup transactions. Corresponds to
    /// `MHT(astria.sequencer.v1alpha.SequencerBlock.rollup_transactions)`, the Merkle
    /// Tree Hash deriveed from the rollup transactions.
    /// Always 32 bytes.
    pub rollup_transactions_root: [u8; 32],
    /// The proof that the rollup transactions are included in sequencer block.
    /// Corresponds to `astria.sequencer.v1alpha.SequencerBlock.rollup_transactions_proof`.
    pub rollup_transactions_proof: merkle::Proof,
    /// The proof that this sequencer blob includes all rollup IDs of the original sequencer
    /// block it was derived from. This proof together with `Sha256(MHT(rollup_ids))` (Sha256
    /// applied to the Merkle Tree Hash of the rollup ID sequence) must be equal to
    /// `header.data_hash` which itself must match
    /// `astria.sequencer.v1alpha.SequencerBlock.header.data_hash`. This field corresponds to
    /// `astria.sequencer.v1alpha.SequencerBlock.rollup_ids_proof`.
    pub rollup_ids_proof: merkle::Proof,
}

impl UncheckedCelestiaSequencerBlob {
    /// Converts this unchecked blob into its checked [`CelestiaSequencerBlob`] representation.
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    pub fn try_into_celestia_sequencer_blob(
        self,
    ) -> Result<CelestiaSequencerBlob, CelestiaSequencerBlobError> {
        CelestiaSequencerBlob::try_from_unchecked(self)
    }

    /// Converts from the raw decoded protobuf representation of this type.
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    pub fn try_from_raw(
        raw: raw::CelestiaSequencerBlob,
    ) -> Result<Self, CelestiaSequencerBlobError> {
        let raw::CelestiaSequencerBlob {
            header,
            rollup_ids,
            rollup_transactions_root,
            rollup_transactions_proof,
            rollup_ids_proof,
        } = raw;
        let header = 'cometbft_header: {
            let Some(header) = header else {
                break 'cometbft_header Err(CelestiaSequencerBlobError::field_not_set("header"));
            };
            tendermint::block::Header::try_from(header)
                .map_err(CelestiaSequencerBlobError::cometbft_header)
        }?;
        let rollup_ids: Vec<_> = rollup_ids
            .into_iter()
            .map(RollupId::try_from_vec)
            .collect::<Result<_, _>>()
            .map_err(CelestiaSequencerBlobError::rollup_ids)?;

        let rollup_transactions_root =
            rollup_transactions_root
                .try_into()
                .map_err(|bytes: Vec<_>| {
                    CelestiaSequencerBlobError::rollup_transactions_root(bytes.len())
                })?;

        let rollup_transactions_proof = 'transactions_proof: {
            let Some(rollup_transactions_proof) = rollup_transactions_proof else {
                break 'transactions_proof Err(CelestiaSequencerBlobError::field_not_set(
                    "rollup_transactions_root",
                ));
            };
            merkle::Proof::try_from_raw(rollup_transactions_proof)
                .map_err(CelestiaSequencerBlobError::rollup_transactions_proof)
        }?;

        let rollup_ids_proof = 'ids_proof: {
            let Some(rollup_ids_proof) = rollup_ids_proof else {
                break 'ids_proof Err(CelestiaSequencerBlobError::field_not_set(
                    "rollup_ids_proof",
                ));
            };
            merkle::Proof::try_from_raw(rollup_ids_proof)
                .map_err(CelestiaSequencerBlobError::rollup_ids_proof)
        }?;

        Ok(Self {
            header,
            rollup_ids,
            rollup_transactions_root,
            rollup_transactions_proof,
            rollup_ids_proof,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CelestiaSequencerBlob {
    /// The block hash obtained from hashing `.header`.
    block_hash: [u8; 32],
    /// The original `CometBFT` header that is the input to this blob's original sequencer block.
    /// Corresponds to `astria.sequencer.v1alpha.SequencerBlock.header`.
    header: tendermint::block::header::Header,
    /// The rollup IDs for which `CelestiaRollupBlob`s were submitted to celestia.
    /// Corresponds to the `astria.sequencer.v1alpha1.RollupTransactions.id` field
    /// and is extracted from `astria.sequencer.v1alpha.SequencerBlock.rollup_transactions`.
    rollup_ids: Vec<RollupId>,
    /// The Merkle Tree Hash of the rollup transactions. Corresponds to
    /// `MHT(astria.sequencer.v1alpha.SequencerBlock.rollup_transactions)`, the Merkle
    /// Tree Hash deriveed from the rollup transactions.
    /// Always 32 bytes.
    rollup_transactions_root: [u8; 32],
    /// The proof that the rollup transactions are included in sequencer block.
    /// Corresponds to `astria.sequencer.v1alpha.SequencerBlock.rollup_transactions_proof`.
    rollup_transactions_proof: merkle::Proof,
    /// The proof that this sequencer blob includes all rollup IDs of the original sequencer
    /// block it was derived from. This proof together with `Sha256(MHT(rollup_ids))` (Sha256
    /// applied to the Merkle Tree Hash of the rollup ID sequence) must be equal to
    /// `header.data_hash` which itself must match
    /// `astria.sequencer.v1alpha.SequencerBlock.header.data_hash`. This field corresponds to
    /// `astria.sequencer.v1alpha.SequencerBlock.rollup_ids_proof`.
    rollup_ids_proof: merkle::Proof,
}

impl CelestiaSequencerBlob {
    /// Returns the block hash of the tendermint header stored in this blob.
    #[must_use]
    pub fn block_hash(&self) -> [u8; 32] {
        self.block_hash
    }

    /// Returns the sequencer's `CometBFT` chain ID.
    #[must_use]
    pub fn cometbft_chain_id(&self) -> &tendermint::chain::Id {
        &self.header.chain_id
    }

    /// Returns the `CometBFT` height stored in the header of the [`SequencerBlock`] this blob was
    /// derived from.
    #[must_use]
    pub fn height(&self) -> tendermint::block::Height {
        self.header.height
    }

    /// Returns the `CometBFT` header of the [`SequencerBlock`] this blob was derived from.
    #[must_use]
    pub fn header(&self) -> &tendermint::block::Header {
        &self.header
    }

    /// Returns the Merkle Tree Hash constructed from the rollup transactions of the original
    /// [`SequencerBlock`] this blob was derived from.
    #[must_use]
    pub fn rollup_transactions_root(&self) -> [u8; 32] {
        self.rollup_transactions_root
    }

    /// Converts into the unchecked representation fo this type.
    #[must_use]
    pub fn into_unchecked(self) -> UncheckedCelestiaSequencerBlob {
        let Self {
            header,
            rollup_ids,
            rollup_transactions_root,
            rollup_transactions_proof,
            rollup_ids_proof,
            ..
        } = self;
        UncheckedCelestiaSequencerBlob {
            header,
            rollup_ids,
            rollup_transactions_root,
            rollup_transactions_proof,
            rollup_ids_proof,
        }
    }

    /// Converts from the unchecked representation of this type.
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    pub fn try_from_unchecked(
        unchecked: UncheckedCelestiaSequencerBlob,
    ) -> Result<Self, CelestiaSequencerBlobError> {
        let UncheckedCelestiaSequencerBlob {
            header,
            rollup_ids,
            rollup_transactions_root,
            rollup_transactions_proof,
            rollup_ids_proof,
        } = unchecked;
        let tendermint::Hash::Sha256(block_hash) = header.hash() else {
            return Err(CelestiaSequencerBlobError::empty_cometbft_block_hash());
        };
        // header.data_hash is Option<Hash> and Hash itself has
        // variants Sha256([u8; 32]) or None.
        let Some(tendermint::Hash::Sha256(data_hash)) = header.data_hash else {
            return Err(CelestiaSequencerBlobError::field_not_set(
                "header.data_hash",
            ));
        };

        if !rollup_transactions_proof.verify(&Sha256::digest(rollup_transactions_root), data_hash) {
            return Err(CelestiaSequencerBlobError::rollup_transactions_not_in_cometbft_block());
        }

        if !are_rollup_ids_included(rollup_ids.iter().copied(), &rollup_ids_proof, data_hash) {
            return Err(CelestiaSequencerBlobError::rollup_ids_not_in_cometbft_block());
        }

        Ok(Self {
            block_hash,
            header,
            rollup_ids,
            rollup_transactions_root,
            rollup_transactions_proof,
            rollup_ids_proof,
        })
    }

    /// Converts into the raw decoded protobuf representation of this type.
    pub fn into_raw(self) -> raw::CelestiaSequencerBlob {
        let Self {
            header,
            rollup_ids,
            rollup_transactions_root,
            rollup_transactions_proof,
            rollup_ids_proof,
            ..
        } = self;
        raw::CelestiaSequencerBlob {
            header: Some(header.into()),
            rollup_ids: rollup_ids.into_iter().map(RollupId::to_vec).collect(),
            rollup_transactions_root: rollup_transactions_root.to_vec(),
            rollup_transactions_proof: Some(rollup_transactions_proof.into_raw()),
            rollup_ids_proof: Some(rollup_ids_proof.into_raw()),
        }
    }

    /// Converts from the raw decoded protobuf representation of this type.
    ///
    /// # Errors
    /// TODO(https://github.com/astriaorg/astria/issues/612)
    pub fn try_from_raw(
        raw: raw::CelestiaSequencerBlob,
    ) -> Result<Self, CelestiaSequencerBlobError> {
        UncheckedCelestiaSequencerBlob::try_from_raw(raw)
            .and_then(UncheckedCelestiaSequencerBlob::try_into_celestia_sequencer_blob)
    }
}

fn are_rollup_ids_included<'a, TRollupIds: 'a>(
    ids: TRollupIds,
    proof: &merkle::Proof,
    data_hash: [u8; 32],
) -> bool
where
    TRollupIds: IntoIterator<Item = RollupId>,
{
    let tree = merkle::Tree::from_leaves(ids);
    let hash_of_root = Sha256::digest(tree.root());
    proof.verify(&hash_of_root, data_hash)
}

fn are_rollup_txs_included(
    rollup_txs: &IndexMap<RollupId, Vec<Vec<u8>>>,
    rollup_proof: &merkle::Proof,
    data_hash: [u8; 32],
) -> bool {
    let rollup_tree = derive_merkle_tree_from_rollup_txs(rollup_txs);
    let hash_of_rollup_root = Sha256::digest(rollup_tree.root());
    rollup_proof.verify(&hash_of_rollup_root, data_hash)
}

/// Derive a [`merkle::Tree`] from an iterable.
///
/// It is the responsbility if the caller to ensure that the iterable is
/// deterministic. Prefer types like `Vec`, `BTreeMap` or `IndexMap` over
/// `HashMap`.
pub fn derive_merkle_tree_from_rollup_txs<'a, T: 'a>(rollup_ids_to_txs: T) -> merkle::Tree
where
    T: IntoIterator<Item = (&'a RollupId, &'a Vec<Vec<u8>>)>,
{
    let mut tree = merkle::Tree::new();
    for (rollup_id, txs) in rollup_ids_to_txs {
        let root = merkle::Tree::from_leaves(txs).root();
        tree.build_leaf().write(rollup_id.as_ref()).write(&root);
    }
    tree
}

// TODO: This can all be done in-place once https://github.com/rust-lang/rust/issues/80552 is stabilized.
pub fn group_sequence_actions_in_signed_transaction_transactions_by_rollup_id(
    signed_transactions: &[SignedTransaction],
) -> IndexMap<RollupId, Vec<Vec<u8>>> {
    let mut map = IndexMap::new();
    for action in signed_transactions
        .iter()
        .flat_map(SignedTransaction::actions)
    {
        if let Some(action) = action.as_sequence() {
            let txs_for_rollup: &mut Vec<Vec<u8>> = map.entry(action.rollup_id).or_insert(vec![]);
            txs_for_rollup.push(action.data.clone());
        }
    }
    map.sort_unstable_keys();
    map
}

/// Constructs a `[merkle::Tree]` from an iterator yielding byte slices.
///
/// This hashes each item before pushing it into the Merkle Tree, which
/// effectively causes a double hashing. The leaf hash of an item `d_i`
/// is then `MTH(d_i) = SHA256(0x00 || SHA256(d_i))`.
fn merkle_tree_from_data<I, B>(iter: I) -> merkle::Tree
where
    I: IntoIterator<Item = B>,
    B: AsRef<[u8]>,
{
    merkle::Tree::from_leaves(iter.into_iter().map(|item| Sha256::digest(&item)))
}

#[cfg(test)]
mod tests {
    use super::{
        Address,
        BalanceResponse,
        IncorrectAddressLength,
        NonceResponse,
    };

    #[test]
    fn balance_roundtrip_is_correct() {
        let expected = BalanceResponse {
            height: 42,
            balance: 42,
        };
        let actual = expected.into_raw().into_native();
        assert_eq!(expected, actual);
    }

    #[test]
    fn nonce_roundtrip_is_correct() {
        let expected = NonceResponse {
            height: 42,
            nonce: 42,
        };
        let actual = expected.into_raw().into_native();
        assert_eq!(expected, actual);
    }

    #[test]
    fn account_of_20_bytes_is_converted_correctly() {
        let expected = Address([42; 20]);
        let account_vec = expected.0.to_vec();
        let actual = Address::try_from_slice(&account_vec).unwrap();
        assert_eq!(expected, actual);
    }

    #[track_caller]
    fn account_conversion_check(bad_account: &[u8]) {
        let error = Address::try_from_slice(bad_account);
        assert!(
            matches!(error, Err(IncorrectAddressLength { .. })),
            "converting form incorrect sized account succeeded where it should have failed"
        );
    }

    #[test]
    fn account_of_incorrect_length_gives_error() {
        account_conversion_check(&[42; 0]);
        account_conversion_check(&[42; 19]);
        account_conversion_check(&[42; 21]);
        account_conversion_check(&[42; 100]);
    }
}
