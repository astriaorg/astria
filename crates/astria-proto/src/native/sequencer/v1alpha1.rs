use std::{
    error::Error,
    fmt::Display,
};

use ed25519_consensus::{
    Signature,
    VerificationKey,
};
use tracing::info;

use crate::generated::sequencer::v1alpha1 as raw;

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
            SignedTransactionErrorKind::Signature(e) => Some(e),
            SignedTransactionErrorKind::Transaction(e) => Some(e),
            SignedTransactionErrorKind::VerificationKey(e) => Some(e),
            SignedTransactionErrorKind::Verification(e) => Some(e),
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
/// with its signature and public_key.
#[derive(Clone, Debug)]
pub struct SignedTransaction {
    signature: Signature,
    verification_key: VerificationKey,
    transaction: UnsignedTransaction,
}

impl SignedTransaction {
    pub fn into_proto(self) -> raw::SignedTransaction {
        let Self {
            signature,
            verification_key,
            transaction,
        } = self;
        raw::SignedTransaction {
            signature: signature.to_bytes().to_vec(),
            public_key: verification_key.to_bytes().to_vec(),
            transaction: Some(transaction.into_proto()),
        }
    }

    pub fn try_from_proto(proto: raw::SignedTransaction) -> Result<Self, SignedTransactionError> {
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
        let transaction = UnsignedTransaction::try_from_proto(transaction)
            .map_err(SignedTransactionError::transaction)?;
        Ok(Self {
            signature,
            verification_key,
            transaction,
        })
    }

    pub fn into_parts(self) -> (Signature, VerificationKey, UnsignedTransaction) {
        let Self {
            signature,
            verification_key,
            transaction,
        } = self;
        (signature, verification_key, transaction)
    }

    pub fn actions(&self) -> &[Action] {
        &self.transaction.actions
    }

    pub fn signature(&self) -> Signature {
        self.signature
    }

    pub fn verification_key(&self) -> VerificationKey {
        self.verification_key
    }

    pub fn unsigned_transaction(&self) -> &UnsignedTransaction {
        &self.transaction
    }
}

#[derive(Clone, Debug)]
pub struct UnsignedTransaction {
    pub nonce: u32,
    pub actions: Vec<Action>,
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
        }
    }
}

#[derive(Debug)]
enum UnsignedTransactionErrorKind {
    Action(ActionError),
}

impl UnsignedTransaction {
    pub fn into_proto(self) -> raw::UnsignedTransaction {
        let Self {
            nonce,
            actions,
        } = self;
        let actions = actions.into_iter().map(Action::into_proto).collect();
        raw::UnsignedTransaction {
            nonce,
            actions,
        }
    }

    pub fn try_from_proto(
        proto: raw::UnsignedTransaction,
    ) -> Result<Self, UnsignedTransactionError> {
        let raw::UnsignedTransaction {
            nonce,
            actions,
        } = proto;
        let n_raw_actions = actions.len();
        let actions: Vec<_> = actions
            .into_iter()
            .filter_map(|raw_act| {
                // Silently drop unset actions.
                let converted = Action::try_from_proto(raw_act);
                if let Err(e) = &converted {
                    if e.is_unset() {
                        return None;
                    }
                }
                Some(converted)
            })
            .collect::<Result<_, _>>()
            .map_err(UnsignedTransactionError::action)?;
        if actions.len() != n_raw_actions {
            info!(
                actions.raw = n_raw_actions,
                actions.converted = actions.len(),
                "ignored unset raw protobuf actions",
            );
        }
        Ok(Self {
            nonce,
            actions,
        })
    }
}

#[derive(Clone, Debug)]
pub enum Action {
    Sequence(SequenceAction),
    Transfer(TransferAction),
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

    fn is_unset(&self) -> bool {
        matches!(self.kind, ActionErrorKind::Unset)
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
}

impl Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match &self.kind {
            ActionErrorKind::Unset => "oneof value was not set",
            ActionErrorKind::Transfer(_) => "raw transfer action was not valid",
            ActionErrorKind::Sequence(_) => "raw sequence action was not valid",
        };
        f.pad(msg)
    }
}

impl Error for ActionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            ActionErrorKind::Unset => None,
            ActionErrorKind::Transfer(e) => Some(e),
            ActionErrorKind::Sequence(e) => Some(e),
        }
    }
}

#[derive(Debug)]
enum ActionErrorKind {
    Unset,
    Transfer(TransferActionError),
    Sequence(SequenceActionError),
}

impl Action {
    pub fn into_proto(self) -> raw::Action {
        use raw::action::Value;
        let kind = match self {
            Action::Sequence(act) => Value::SequenceAction(act.into_proto()),
            Action::Transfer(act) => Value::TransferAction(act.into_proto()),
        };
        raw::Action {
            value: Some(kind),
        }
    }

    pub fn try_from_proto(proto: raw::Action) -> Result<Self, ActionError> {
        use raw::action::Value;
        let raw::Action {
            value,
        } = proto;
        let Some(action) = value else {
            return Err(ActionError::unset());
        };
        let action = match action {
            Value::SequenceAction(act) => {
                Self::Sequence(SequenceAction::try_from_proto(act).map_err(ActionError::sequence)?)
            }
            Value::TransferAction(act) => {
                Self::Transfer(TransferAction::try_from_proto(act).map_err(ActionError::transfer)?)
            }
        };
        Ok(action)
    }

    pub fn as_sequence(&self) -> Option<&SequenceAction> {
        let Self::Sequence(sequence_action) = self else { return None };
        Some(sequence_action)
    }

    pub fn as_transfer(&self) -> Option<&TransferAction> {
        let Self::Transfer(transfer_action) = self else { return None };
        Some(transfer_action)
    }
}

#[derive(Clone, Debug)]
pub struct SequenceAction {
    pub chain_id: [u8; 32],
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct SequenceActionError {
    kind: SequenceActionErrorKind,
}

impl Display for SequenceActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            SequenceActionErrorKind::ChainIdLen {
                received,
            } => write!(
                f,
                "expected a chain ID of {CHAIN_ID_LEN} bytes, but received {received}",
            ),
        }
    }
}

impl Error for SequenceActionError {}

impl SequenceActionError {
    fn chain_id_len<T: AsRef<[u8]>>(bytes: T) -> Self {
        fn chain_id_len(bytes: &[u8]) -> SequenceActionError {
            SequenceActionError {
                kind: SequenceActionErrorKind::ChainIdLen {
                    received: bytes.len(),
                },
            }
        }
        chain_id_len(bytes.as_ref())
    }
}

#[derive(Debug)]
enum SequenceActionErrorKind {
    ChainIdLen { received: usize },
}

impl SequenceAction {
    pub fn into_proto(self) -> raw::SequenceAction {
        let Self {
            chain_id,
            data,
        } = self;
        raw::SequenceAction {
            chain_id: chain_id.to_vec(),
            data,
        }
    }

    pub fn try_from_proto(proto: raw::SequenceAction) -> Result<Self, SequenceActionError> {
        let raw::SequenceAction {
            chain_id,
            data,
        } = proto;
        let chain_id = chain_id
            .try_into()
            .map_err(SequenceActionError::chain_id_len)?;
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
}

impl Display for TransferActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            TransferActionErrorKind::Address(_) => {
                f.pad("`to` field did not contain a valid address")
            }
        }
    }
}

impl Error for TransferActionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            TransferActionErrorKind::Address(e) => Some(e),
        }
    }
}

#[derive(Debug)]
enum TransferActionErrorKind {
    Address(IncorrectAddressLength),
}

impl TransferAction {
    pub fn into_proto(self) -> raw::TransferAction {
        let Self {
            to,
            amount,
        } = self;
        raw::TransferAction {
            to: to.to_vec(),
            amount: Some(amount.into()),
        }
    }

    pub fn try_from_proto(proto: raw::TransferAction) -> Result<Self, TransferActionError> {
        let raw::TransferAction {
            to,
            amount,
        } = proto;
        let to = Address::try_from_slice(&to).map_err(TransferActionError::address)?;
        let amount = amount.map_or(0, Into::into);
        Ok(Self {
            to,
            amount,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Address(pub [u8; ADDRESS_LEN]);

impl Address {
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// Construct a sequencer address from a [`ed25519_consensus::VerificationKey`].
    ///
    /// The first 20 bytes of the sha256 hash of the verification key is the address.
    #[must_use]
    pub fn from_verification_key(public_key: ed25519_consensus::VerificationKey) -> Self {
        use sha2::Digest as _;
        /// this ensures that `ADDRESS_LEN` is never accidentally changed to a value
        /// that would violate this assumption.
        #[allow(clippy::assertions_on_constants)]
        const _: () = assert!(ADDRESS_LEN <= 32);
        let mut hasher = sha2::Sha256::new();
        hasher.update(public_key);
        let bytes: [u8; 32] = hasher.finalize().into();
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
        use hex::ToHex as _;
        f.write_str(&self.encode_hex::<String>())
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
        BalanceResponse::from_proto(&self)
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
    pub fn from_proto(proto: &raw::BalanceResponse) -> Self {
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
    pub fn into_proto(self) -> raw::BalanceResponse {
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
        NonceResponse::from_proto(&self)
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
    pub fn from_proto(proto: &raw::NonceResponse) -> Self {
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
    pub fn into_proto(self) -> raw::NonceResponse {
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
        let actual = expected.into_proto().into_native();
        assert_eq!(expected, actual);
    }

    #[test]
    fn nonce_roundtrip_is_correct() {
        let expected = NonceResponse {
            height: 42,
            nonce: 42,
        };
        let actual = expected.into_proto().into_native();
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
