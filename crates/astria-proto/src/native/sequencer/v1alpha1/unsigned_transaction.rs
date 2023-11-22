use std::{
    error::Error,
    fmt::Display,
};

use ed25519_consensus::SigningKey;
use tracing::info;

use super::{
    asset,
    Action,
    ActionError,
    SignedTransaction,
};
use crate::{
    generated::sequencer::v1alpha1 as raw,
    native::sequencer::v1alpha1::asset::IncorrectAssetIdLength,
};

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
