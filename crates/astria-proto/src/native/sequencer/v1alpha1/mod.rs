pub mod action;
pub mod asset;
pub mod celestia;
pub mod merkle_proof;
pub mod mint_action;
pub mod rollup_id;
pub mod sequence_action;
pub mod sequencer_block;
pub mod sudo_address_change_action;
pub mod transaction;
pub mod transfer_action;

use std::{
    error::Error,
    fmt::Display,
};

pub use action::{
    Action,
    ActionError,
};
pub use asset::{
    Denom,
    Id,
    IncorrectAssetIdLength,
};
pub use celestia::{
    CelestiaRollupBlob,
    CelestiaRollupBlobError,
    CelestiaSequencerBlob,
    CelestiaSequencerBlobError,
    UncheckedCelestiaRollupBlob,
    UncheckedCelestiaSequencerBlob,
};
use indexmap::IndexMap;
pub use mint_action::{
    MintAction,
    MintActionError,
};
pub use rollup_id::{
    IncorrectRollupIdLength,
    RollupId,
    ROLLUP_ID_LEN,
};
pub use sequence_action::{
    SequenceAction,
    SequenceActionError,
};
pub use sequencer_block::{
    SequencerBlock,
    SequencerBlockError,
};
use sha2::{
    Digest as _,
    Sha256,
};
pub use sudo_address_change_action::{
    SudoAddressChangeAction,
    SudoAddressChangeActionError,
};
pub use transaction::{
    SignedTransaction,
    SignedTransactionError,
    UnsignedTransaction,
    UnsignedTransactionError,
};
pub use transfer_action::{
    TransferAction,
    TransferActionError,
};

use crate::generated::sequencer::v1alpha1 as raw;

pub const ADDRESS_LEN: usize = 20;

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
        for byte in self.0 {
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

pub(crate) fn are_rollup_ids_included<'a, TRollupIds: 'a>(
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
