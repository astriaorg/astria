use ed25519_consensus::{
    Signature,
    SigningKey,
    VerificationKey,
};
use prost::Message as _;
use sha2::Sha256;

use super::raw;

pub mod action;
pub use action::Action;

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct SignedTransactionError(SignedTransactionErrorKind);

impl SignedTransactionError {
    fn signature(inner: ed25519_consensus::Error) -> Self {
        Self(SignedTransactionErrorKind::Signature(inner))
    }

    fn transaction(inner: UnsignedTransactionError) -> Self {
        Self(SignedTransactionErrorKind::Transaction(inner))
    }

    fn verification(inner: ed25519_consensus::Error) -> Self {
        Self(SignedTransactionErrorKind::Verification(inner))
    }

    fn verification_key(inner: ed25519_consensus::Error) -> Self {
        Self(SignedTransactionErrorKind::VerificationKey(inner))
    }

    fn unset_transaction() -> Self {
        Self(SignedTransactionErrorKind::UnsetTransaction)
    }
}

#[derive(Debug, thiserror::Error)]
enum SignedTransactionErrorKind {
    #[error("`transaction` field not set")]
    UnsetTransaction,
    #[error("`signature` field invalid")]
    Signature(#[source] ed25519_consensus::Error),
    #[error("`transaction` field invalid")]
    Transaction(#[source] UnsignedTransactionError),
    #[error("`public_key` field invalid")]
    VerificationKey(#[source] ed25519_consensus::Error),
    #[error("transaction could not be verified given the signature and verification key")]
    Verification(ed25519_consensus::Error),
}

/// A signed transaction.
///
/// [`SignedTransaction`] contains an [`UnsignedTransaction`] together
/// with its signature and public key.
#[derive(Clone, Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct SignedTransaction {
    signature: Signature,
    verification_key: VerificationKey,
    transaction: UnsignedTransaction,
}

impl SignedTransaction {
    /// Returns the transaction hash.
    ///
    /// The transaction hash is calculated by protobuf-encoding the transaction
    /// and hashing the resulting bytes with sha256.
    #[must_use]
    pub fn hash(&self) -> [u8; 32] {
        use sha2::Digest as _;
        let bytes = self.to_raw().encode_to_vec();
        Sha256::digest(bytes).into()
    }

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
    /// contained in the raw input, if the transaction field was empty (meaning it was mapped to
    /// `None`), if the inner transaction could not be verified given the key and signature, or
    /// if the native [`UnsignedTransaction`] could not be created from the inner raw
    /// [`raw::UnsignedTransaction`].
    pub fn try_from_raw(proto: raw::SignedTransaction) -> Result<Self, SignedTransactionError> {
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
    pub fn into_unsigned(self) -> UnsignedTransaction {
        self.transaction
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
#[allow(clippy::module_name_repetitions)]
pub struct UnsignedTransaction {
    pub nonce: u32,
    pub actions: Vec<Action>,
    /// asset to use for fee payment.
    pub fee_asset_id: super::asset::Id,
}

impl UnsignedTransaction {
    #[must_use]
    pub fn into_signed(self, signing_key: &SigningKey) -> SignedTransaction {
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
        let actions: Vec<_> = actions
            .into_iter()
            .map(Action::try_from_raw)
            .collect::<Result<_, _>>()
            .map_err(UnsignedTransactionError::action)?;
        let fee_asset_id = super::asset::Id::try_from_slice(&fee_asset_id)
            .map_err(UnsignedTransactionError::fee_asset_id)?;

        Ok(Self {
            nonce,
            actions,
            fee_asset_id,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct UnsignedTransactionError(UnsignedTransactionErrorKind);

impl UnsignedTransactionError {
    fn action(inner: action::ActionError) -> Self {
        Self(UnsignedTransactionErrorKind::Action(inner))
    }

    fn fee_asset_id(inner: super::IncorrectAssetIdLength) -> Self {
        Self(UnsignedTransactionErrorKind::FeeAsset(inner))
    }
}

#[derive(Debug, thiserror::Error)]
enum UnsignedTransactionErrorKind {
    #[error("`actions` field is invalid")]
    Action(#[source] action::ActionError),
    #[error("`fee_asset_id` field is invalid")]
    FeeAsset(#[source] super::IncorrectAssetIdLength),
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::sequencer::v1alpha1::{
        asset::{
            default_native_asset_id,
            Id,
        },
        transaction::action::{
            Action::Transfer,
            TransferAction,
        },
        Address,
    };

    #[test]
    fn signed_transaction_hash() {
        let verification_key = VerificationKey::try_from([
            213, 191, 74, 63, 204, 231, 23, 176, 56, 139, 204, 39, 73, 235, 193, 72, 173, 153, 105,
            178, 63, 69, 238, 27, 96, 95, 213, 135, 120, 87, 106, 196,
        ])
        .unwrap();
        let signature = Signature::try_from([
            227, 85, 139, 137, 185, 81, 103, 226, 85, 208, 68, 190, 196, 105, 191, 191, 37, 227,
            167, 21, 69, 165, 229, 163, 187, 104, 165, 40, 92, 8, 113, 67, 166, 194, 232, 156, 232,
            117, 134, 105, 2, 90, 151, 35, 241, 136, 200, 46, 222, 37, 124, 219, 195, 20, 195, 24,
            227, 96, 127, 152, 22, 47, 146, 10,
        ])
        .unwrap();
        let expected_hash: [u8; 32] = [
            83, 12, 185, 239, 139, 236, 107, 26, 116, 237, 242, 144, 144, 37, 188, 82, 115, 228,
            59, 229, 164, 251, 142, 181, 82, 188, 68, 99, 165, 61, 83, 135,
        ];

        let transfer = TransferAction {
            to: Address::from([0; 20]),
            amount: 0,
            asset_id: default_native_asset_id(),
        };

        let unsigned = UnsignedTransaction {
            nonce: 0,
            actions: vec![Transfer(transfer)],
            fee_asset_id: default_native_asset_id(),
        };

        let tx = SignedTransaction {
            signature,
            verification_key,
            transaction: unsigned,
        };

        assert_eq!(tx.hash(), expected_hash);
    }
}
