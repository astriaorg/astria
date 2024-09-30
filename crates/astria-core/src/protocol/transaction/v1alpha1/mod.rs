use action_group::Actions;
use bytes::Bytes;
use prost::{
    Message as _,
    Name as _,
};

use super::raw;
use crate::{
    crypto::{
        self,
        Signature,
        SigningKey,
        VerificationKey,
    },
    primitive::v1::{
        asset,
        TransactionId,
        ADDRESS_LEN,
    },
    Protobuf as _,
};

pub mod action;
pub mod action_group;
pub use action::Action;

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct SignedTransactionError(SignedTransactionErrorKind);

impl SignedTransactionError {
    fn signature(inner: crypto::Error) -> Self {
        Self(SignedTransactionErrorKind::Signature(inner))
    }

    fn transaction(inner: UnsignedTransactionError) -> Self {
        Self(SignedTransactionErrorKind::Transaction(inner))
    }

    fn verification(inner: crypto::Error) -> Self {
        Self(SignedTransactionErrorKind::Verification(inner))
    }

    fn verification_key(inner: crypto::Error) -> Self {
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
    Signature(#[source] crypto::Error),
    #[error("`transaction` field invalid")]
    Transaction(#[source] UnsignedTransactionError),
    #[error("`public_key` field invalid")]
    VerificationKey(#[source] crypto::Error),
    #[error("transaction could not be verified given the signature and verification key")]
    Verification(crypto::Error),
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
    transaction_bytes: bytes::Bytes,
}

impl SignedTransaction {
    pub fn address_bytes(&self) -> &[u8; ADDRESS_LEN] {
        self.verification_key.address_bytes()
    }

    /// Returns the transaction ID, containing the transaction hash.
    ///
    /// The transaction hash is calculated by protobuf-encoding the transaction
    /// and hashing the resulting bytes with sha256.
    #[must_use]
    pub fn id(&self) -> TransactionId {
        use sha2::{
            Digest as _,
            Sha256,
        };
        let bytes = self.to_raw().encode_to_vec();
        TransactionId::new(Sha256::digest(bytes).into())
    }

    #[must_use]
    pub fn into_raw(self) -> raw::SignedTransaction {
        let Self {
            signature,
            verification_key,
            transaction_bytes,
            ..
        } = self;
        raw::SignedTransaction {
            signature: Bytes::copy_from_slice(&signature.to_bytes()),
            public_key: Bytes::copy_from_slice(&verification_key.to_bytes()),
            transaction: Some(pbjson_types::Any {
                type_url: raw::UnsignedTransaction::type_url(),
                value: transaction_bytes,
            }),
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::SignedTransaction {
        let Self {
            signature,
            verification_key,
            transaction_bytes,
            ..
        } = self;
        raw::SignedTransaction {
            signature: Bytes::copy_from_slice(&signature.to_bytes()),
            public_key: Bytes::copy_from_slice(&verification_key.to_bytes()),
            transaction: Some(pbjson_types::Any {
                type_url: raw::UnsignedTransaction::type_url(),
                value: transaction_bytes.clone(),
            }),
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
        let bytes = transaction.value.clone();
        verification_key
            .verify(&signature, &bytes)
            .map_err(SignedTransactionError::verification)?;
        let transaction = UnsignedTransaction::try_from_any(transaction)
            .map_err(SignedTransactionError::transaction)?;
        Ok(Self {
            signature,
            verification_key,
            transaction,
            transaction_bytes: bytes,
        })
    }

    #[must_use]
    pub fn into_unsigned(self) -> UnsignedTransaction {
        self.transaction
    }

    #[must_use]
    pub fn actions(&self) -> &[Action] {
        self.transaction.actions.actions()
    }

    #[must_use]
    pub fn is_bundleable_sudo_action_group(&self) -> bool {
        if let Some(group) = self.transaction.actions.group() {
            group.is_bundleable_sudo()
        } else {
            false
        }
    }

    #[must_use]
    pub fn signature(&self) -> Signature {
        self.signature
    }

    #[must_use]
    pub fn verification_key(&self) -> &VerificationKey {
        &self.verification_key
    }

    #[must_use]
    pub fn unsigned_transaction(&self) -> &UnsignedTransaction {
        &self.transaction
    }

    pub fn chain_id(&self) -> &str {
        self.transaction.chain_id()
    }

    #[must_use]
    pub fn nonce(&self) -> u32 {
        self.transaction.nonce()
    }
}

#[derive(Clone, Debug)]
pub struct UnsignedTransaction {
    actions: Actions,
    params: TransactionParams,
}

impl UnsignedTransaction {
    #[must_use]
    pub fn builder() -> UnsignedTransactionBuilder {
        UnsignedTransactionBuilder::new()
    }

    #[must_use]
    pub fn into_actions(self) -> Vec<Action> {
        self.actions.into_actions()
    }

    #[must_use]
    pub fn actions(&self) -> &[Action] {
        self.actions.actions()
    }

    #[must_use]
    pub fn nonce(&self) -> u32 {
        self.params.nonce
    }

    #[must_use]
    pub fn chain_id(&self) -> &str {
        &self.params.chain_id
    }

    #[must_use]
    pub fn into_signed(self, signing_key: &SigningKey) -> SignedTransaction {
        let bytes = self.to_raw().encode_to_vec();
        let signature = signing_key.sign(&bytes);
        let verification_key = signing_key.verification_key();
        SignedTransaction {
            signature,
            verification_key,
            transaction: self,
            transaction_bytes: bytes.into(),
        }
    }

    pub fn into_raw(self) -> raw::UnsignedTransaction {
        let Self {
            actions,
            params,
        } = self;
        let actions = actions
            .into_actions()
            .into_iter()
            .map(Action::into_raw)
            .collect();
        raw::UnsignedTransaction {
            actions,
            params: Some(params.into_raw()),
        }
    }

    #[must_use]
    pub fn into_any(self) -> pbjson_types::Any {
        let raw = self.into_raw();
        pbjson_types::Any {
            type_url: raw::UnsignedTransaction::type_url(),
            value: raw.encode_to_vec().into(),
        }
    }

    pub fn to_raw(&self) -> raw::UnsignedTransaction {
        let Self {
            actions,
            params,
        } = self;
        let actions = actions.actions().iter().map(Action::to_raw).collect();
        let params = params.clone().into_raw();
        raw::UnsignedTransaction {
            actions,
            params: Some(params),
        }
    }

    #[must_use]
    pub fn to_any(&self) -> pbjson_types::Any {
        self.clone().into_any()
    }

    /// Attempt to convert from a raw, unchecked protobuf [`raw::UnsignedTransaction`].
    ///
    /// # Errors
    ///
    /// Returns an error if one of the inner raw actions could not be converted to a native
    /// [`Action`].
    pub fn try_from_raw(proto: raw::UnsignedTransaction) -> Result<Self, UnsignedTransactionError> {
        let raw::UnsignedTransaction {
            actions,
            params,
        } = proto;
        let Some(params) = params else {
            return Err(UnsignedTransactionError::unset_params());
        };
        let params = TransactionParams::from_raw(params);
        let actions: Vec<_> = actions
            .into_iter()
            .map(Action::try_from_raw)
            .collect::<Result<_, _>>()
            .map_err(UnsignedTransactionError::action)?;

        UnsignedTransaction::builder()
            .actions(actions)
            .chain_id(params.chain_id)
            .nonce(params.nonce)
            .try_build()
            .map_err(UnsignedTransactionError::action_group)
    }

    /// Attempt to convert from a protobuf [`pbjson_types::Any`].
    ///
    /// # Errors
    ///
    /// - if the type URL is not the expected type URL
    /// - if the bytes in the [`Any`] do not decode to an [`UnsignedTransaction`]
    pub fn try_from_any(any: pbjson_types::Any) -> Result<Self, UnsignedTransactionError> {
        if any.type_url != raw::UnsignedTransaction::type_url() {
            return Err(UnsignedTransactionError::invalid_type_url(any.type_url));
        }

        let raw = raw::UnsignedTransaction::decode(any.value)
            .map_err(UnsignedTransactionError::decode_any)?;
        Self::try_from_raw(raw)
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct UnsignedTransactionError(UnsignedTransactionErrorKind);

impl UnsignedTransactionError {
    fn action(inner: action::ActionError) -> Self {
        Self(UnsignedTransactionErrorKind::Action(inner))
    }

    fn unset_params() -> Self {
        Self(UnsignedTransactionErrorKind::UnsetParams())
    }

    fn invalid_type_url(got: String) -> Self {
        Self(UnsignedTransactionErrorKind::InvalidTypeUrl {
            got,
        })
    }

    fn decode_any(inner: prost::DecodeError) -> Self {
        Self(UnsignedTransactionErrorKind::DecodeAny(inner))
    }

    fn action_group(inner: action_group::Error) -> Self {
        Self(UnsignedTransactionErrorKind::ActionGroup(inner))
    }
}

#[derive(Debug, thiserror::Error)]
enum UnsignedTransactionErrorKind {
    #[error("`actions` field is invalid")]
    Action(#[source] action::ActionError),
    #[error("`params` field is unset")]
    UnsetParams(),
    #[error(
        "encountered invalid type URL when converting from `google.protobuf.Any`; got `{got}`, \
         expected `{}`",
        raw::UnsignedTransaction::type_url()
    )]
    InvalidTypeUrl { got: String },
    #[error(
        "failed to decode `google.protobuf.Any` to `{}`",
        raw::UnsignedTransaction::type_url()
    )]
    DecodeAny(#[source] prost::DecodeError),
    #[error("`actions` field does not form a valid group of actions")]
    ActionGroup(#[source] action_group::Error),
}

#[derive(Default)]
pub struct UnsignedTransactionBuilder {
    nonce: u32,
    chain_id: String,
    actions: Vec<Action>,
}

impl UnsignedTransactionBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn actions(self, actions: Vec<Action>) -> Self {
        Self {
            actions,
            ..self
        }
    }

    #[must_use]
    pub fn chain_id<T: Into<String>>(self, chain_id: T) -> UnsignedTransactionBuilder {
        UnsignedTransactionBuilder {
            chain_id: chain_id.into(),
            nonce: self.nonce,
            actions: self.actions,
        }
    }

    #[must_use]
    pub fn nonce(self, nonce: u32) -> Self {
        Self {
            nonce,
            ..self
        }
    }

    /// Constructs a [`UnsignedTransaction`] from the configured builder.
    ///
    /// # Errors
    /// Returns an error if the actions do not make a valid `ActionGroup`.
    ///
    /// Returns an error if the set chain ID does not contain a chain name that can be turned into
    /// a bech32 human readable prefix (everything before the first dash i.e. `<name>-<rest>`).
    pub fn try_build(self) -> Result<UnsignedTransaction, action_group::Error> {
        let Self {
            nonce,
            chain_id,
            actions,
        } = self;
        let actions = Actions::try_from_list_of_actions(actions)?;
        Ok(UnsignedTransaction {
            actions,
            params: TransactionParams {
                nonce,
                chain_id,
            },
        })
    }
}

#[derive(Clone, Debug)]
pub struct TransactionParams {
    nonce: u32,
    chain_id: String,
}

impl TransactionParams {
    #[must_use]
    pub fn into_raw(self) -> raw::TransactionParams {
        let Self {
            nonce,
            chain_id,
            ..
        } = self;
        raw::TransactionParams {
            nonce,
            chain_id,
        }
    }

    /// Convert from a raw protobuf [`raw::UnsignedTransaction`].
    #[must_use]
    pub fn from_raw(proto: raw::TransactionParams) -> Self {
        let raw::TransactionParams {
            nonce,
            chain_id,
        } = proto;

        Self {
            nonce,
            chain_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransactionFeeResponse {
    pub height: u64,
    pub fees: Vec<(asset::Denom, u128)>,
}

impl TransactionFeeResponse {
    #[must_use]
    pub fn into_raw(self) -> raw::TransactionFeeResponse {
        raw::TransactionFeeResponse {
            height: self.height,
            fees: self
                .fees
                .into_iter()
                .map(|(asset, fee)| raw::TransactionFee {
                    asset: asset.to_string(),
                    fee: Some(fee.into()),
                })
                .collect(),
        }
    }

    /// Attempt to convert from a raw protobuf [`raw::TransactionFeeResponse`].
    ///
    /// # Errors
    ///
    /// - if the asset ID could not be converted from bytes
    /// - if the fee was unset
    pub fn try_from_raw(
        proto: raw::TransactionFeeResponse,
    ) -> Result<Self, TransactionFeeResponseError> {
        let raw::TransactionFeeResponse {
            height,
            fees,
        } = proto;
        let fees = fees
            .into_iter()
            .map(
                |raw::TransactionFee {
                     asset,
                     fee,
                 }| {
                    let asset = asset.parse().map_err(TransactionFeeResponseError::asset)?;
                    let fee = fee.ok_or(TransactionFeeResponseError::unset_fee())?;
                    Ok((asset, fee.into()))
                },
            )
            .collect::<Result<_, _>>()?;
        Ok(Self {
            height,
            fees,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct TransactionFeeResponseError(TransactionFeeResponseErrorKind);

impl TransactionFeeResponseError {
    fn unset_fee() -> Self {
        Self(TransactionFeeResponseErrorKind::UnsetFee)
    }

    fn asset(inner: asset::ParseDenomError) -> Self {
        Self(TransactionFeeResponseErrorKind::Asset(inner))
    }
}

#[derive(Debug, thiserror::Error)]
enum TransactionFeeResponseErrorKind {
    #[error("`fee` field is unset")]
    UnsetFee,
    #[error("failed to parse asset denom in the `assets` field")]
    Asset(#[source] asset::ParseDenomError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        primitive::v1::Address,
        protocol::transaction::v1alpha1::action::TransferAction,
    };
    const ASTRIA_ADDRESS_PREFIX: &str = "astria";

    fn asset() -> asset::Denom {
        "nria".parse().unwrap()
    }

    #[test]
    fn signed_transaction_hash() {
        let verification_key = VerificationKey::try_from([
            213, 191, 74, 63, 204, 231, 23, 176, 56, 139, 204, 39, 73, 235, 193, 72, 173, 153, 105,
            178, 63, 69, 238, 27, 96, 95, 213, 135, 120, 87, 106, 196,
        ])
        .unwrap();
        let signature = Signature::from([
            227, 85, 139, 137, 185, 81, 103, 226, 85, 208, 68, 190, 196, 105, 191, 191, 37, 227,
            167, 21, 69, 165, 229, 163, 187, 104, 165, 40, 92, 8, 113, 67, 166, 194, 232, 156, 232,
            117, 134, 105, 2, 90, 151, 35, 241, 136, 200, 46, 222, 37, 124, 219, 195, 20, 195, 24,
            227, 96, 127, 152, 22, 47, 146, 10,
        ]);

        let transfer = TransferAction {
            to: Address::builder()
                .array([0; 20])
                .prefix(ASTRIA_ADDRESS_PREFIX)
                .try_build()
                .unwrap(),
            amount: 0,
            asset: asset(),
            fee_asset: asset(),
        };

        let unsigned = UnsignedTransaction::builder()
            .actions(vec![transfer.into()])
            .chain_id("test-1".to_string())
            .nonce(1)
            .try_build()
            .unwrap();

        let tx = SignedTransaction {
            signature,
            verification_key,
            transaction: unsigned.clone(),
            transaction_bytes: unsigned.to_raw().encode_to_vec().into(),
        };

        insta::assert_json_snapshot!(tx.id().to_raw());
    }

    #[test]
    fn signed_transaction_verification_roundtrip() {
        let signing_key = SigningKey::from([
            213, 191, 74, 63, 204, 231, 23, 176, 56, 139, 204, 39, 73, 235, 193, 72, 173, 153, 105,
            178, 63, 69, 238, 27, 96, 95, 213, 135, 120, 87, 106, 196,
        ]);

        let transfer = TransferAction {
            to: Address::builder()
                .array([0; 20])
                .prefix(ASTRIA_ADDRESS_PREFIX)
                .try_build()
                .unwrap(),
            amount: 0,
            asset: asset(),
            fee_asset: asset(),
        };

        let unsigned_tx = UnsignedTransaction::builder()
            .actions(vec![transfer.into()])
            .chain_id("test-1".to_string())
            .nonce(1)
            .try_build()
            .unwrap();

        let signed_tx = unsigned_tx.into_signed(&signing_key);
        let raw = signed_tx.to_raw();

        // `try_from_raw` verifies the signature
        SignedTransaction::try_from_raw(raw).unwrap();
    }
}
