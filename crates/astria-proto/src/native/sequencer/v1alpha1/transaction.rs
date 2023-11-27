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

use super::{
    asset,
    Action,
    ActionError,
};
use crate::{
    generated::sequencer::v1alpha1 as raw,
    native::sequencer::v1alpha1::asset::IncorrectAssetIdLength,
};

#[allow(clippy::module_name_repetitions)]
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

#[allow(clippy::module_name_repetitions)]
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

/// A signed transaction.
///
/// [`SignedTransaction`] contains an [`UnsignedTransaction`] together
/// with its signature and public key.
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug)]
pub struct SignedTransaction {
    pub(super) signature: Signature,
    pub(super) verification_key: VerificationKey,
    pub(super) transaction: UnsignedTransaction,
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

#[allow(clippy::module_name_repetitions)]
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
