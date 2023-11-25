use std::{
    error::Error,
    fmt::Display,
};

use ed25519_consensus::{
    Signature,
    SigningKey,
    VerificationKey,
};
use tracing::info;

pub use super::asset;
use crate::{
    generated::sequencer::v1alpha1 as raw,
    native::sequencer::v1alpha1::asset::IncorrectAssetIdLength,
};

pub const ADDRESS_LEN: usize = 20;
pub const CHAIN_ID_LEN: usize = 32;

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
}

#[derive(Debug)]
pub struct SequenceActionError {
    kind: SequenceActionErrorKind,
}

impl SequenceActionError {
    fn chain_id(inner: IncorrectChainIdLength) -> Self {
        Self {
            kind: SequenceActionErrorKind::ChainId(inner),
        }
    }
}
impl Display for SequenceActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            SequenceActionErrorKind::ChainId(_) => {
                f.pad("`chain_id` field did not contain a valid chain ID")
            }
        }
    }
}

impl Error for SequenceActionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            SequenceActionErrorKind::ChainId(e) => Some(e),
        }
    }
}

#[derive(Debug)]
enum SequenceActionErrorKind {
    ChainId(IncorrectChainIdLength),
}

#[derive(Debug)]
pub struct IncorrectChainIdLength {
    received: usize,
}

impl Display for IncorrectChainIdLength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "expected 32 bytes, got {}", self.received)
    }
}

impl Error for IncorrectChainIdLength {}

#[derive(Clone, Debug)]
pub struct SequenceAction {
    pub chain_id: ChainId,
    pub data: Vec<u8>,
}

impl SequenceAction {
    #[must_use]
    pub fn into_raw(self) -> raw::SequenceAction {
        let Self {
            chain_id,
            data,
        } = self;
        raw::SequenceAction {
            chain_id: chain_id.to_vec(),
            data,
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::SequenceAction {
        let Self {
            chain_id,
            data,
        } = self;
        raw::SequenceAction {
            chain_id: chain_id.to_vec(),
            data: data.clone(),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::SequenceAction`].
    ///
    /// # Errors
    /// Returns an error if the `proto.chain_id` field was not 32 bytes.
    pub fn try_from_raw(proto: raw::SequenceAction) -> Result<Self, SequenceActionError> {
        let raw::SequenceAction {
            chain_id,
            data,
        } = proto;
        let chain_id = ChainId::try_from_slice(&chain_id).map_err(SequenceActionError::chain_id)?;
        Ok(Self {
            chain_id,
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
        use sha2::{
            Digest as _,
            Sha256,
        };
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ChainId {
    #[cfg_attr(feature = "serde", serde(with = "hex::serde"))]
    inner: [u8; 32],
}

impl ChainId {
    /// Creates a new `ChainId` from a 32 byte array.
    ///
    /// Use this if you already have a 32 byte array. Prefer
    /// [`ChainId::with_unhashed_bytes`] if you have a clear text
    /// name what you want to use to identify your rollup.
    ///
    /// # Examples
    /// ```
    /// use astria_proto::native::sequencer::v1alpha1::ChainId;
    /// let bytes = [42u8; 32];
    /// let chain_id = ChainId::new(bytes);
    /// assert_eq!(bytes, chain_id.get());
    /// ```
    #[must_use]
    pub fn new(inner: [u8; CHAIN_ID_LEN]) -> Self {
        Self {
            inner,
        }
    }

    /// Returns the 32 bytes array representing the chain ID.
    ///
    /// # Examples
    /// ```
    /// use astria_proto::native::sequencer::v1alpha1::ChainId;
    /// let bytes = [42u8; 32];
    /// let chain_id = ChainId::new(bytes);
    /// assert_eq!(bytes, chain_id.get());
    /// ```
    #[must_use]
    pub fn get(self) -> [u8; 32] {
        self.inner
    }

    /// Creates a new `ChainId` by applying Sha256 to `bytes`.
    ///
    /// Examples
    /// ```
    /// use astria_proto::native::sequencer::v1alpha1::ChainId;
    /// use sha2::{
    ///     Digest,
    ///     Sha256,
    /// };
    /// let name = "MyRollup-1";
    /// let hashed = Sha256::digest(name);
    /// let chain_id = ChainId::with_unhashed_bytes(name);
    /// assert_eq!(chain_id, ChainId::new(hashed.into()));
    /// ```
    #[must_use]
    pub fn with_unhashed_bytes<T: AsRef<[u8]>>(bytes: T) -> Self {
        use sha2::{
            Digest as _,
            Sha256,
        };
        Self {
            inner: Sha256::digest(bytes).into(),
        }
    }

    /// Allocates a vector from the fixed size array holding the chain ID.
    ///
    /// # Examples
    /// ```
    /// use astria_proto::native::sequencer::v1alpha1::ChainId;
    /// let chain_id = ChainId::new([42u8; 32]);
    /// assert_eq!(vec![42u8; 32], chain_id.to_vec());
    /// ```
    #[must_use]
    pub fn to_vec(&self) -> Vec<u8> {
        self.inner.to_vec()
    }

    /// Convert a byte slice to a chain ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the byte slice was not 32 bytes long.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, IncorrectChainIdLength> {
        let inner = <[u8; CHAIN_ID_LEN]>::try_from(bytes).map_err(|_| IncorrectChainIdLength {
            received: bytes.len(),
        })?;
        Ok(Self::new(inner))
    }
}

impl AsRef<[u8]> for ChainId {
    fn as_ref(&self) -> &[u8] {
        &self.inner
    }
}

impl From<[u8; CHAIN_ID_LEN]> for ChainId {
    fn from(inner: [u8; CHAIN_ID_LEN]) -> Self {
        Self {
            inner,
        }
    }
}

impl Display for ChainId {
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
