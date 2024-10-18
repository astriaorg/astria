use bytes::Bytes;
use prost::{
    Message as _,
    Name as _,
};

use crate::{
    crypto::{
        self,
        Signature,
        SigningKey,
        VerificationKey,
    },
    generated::protocol::transaction::v1 as raw,
    primitive::v1::{
        TransactionId,
        ADDRESS_LEN,
    },
    Protobuf as _,
};

pub mod action;
use action::group::Actions;
pub use action::{
    group::{
        Error,
        Group,
    },
    Action,
};

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct TransactionError(TransactionErrorKind);

impl TransactionError {
    fn signature(inner: crypto::Error) -> Self {
        Self(TransactionErrorKind::Signature(inner))
    }

    fn body(inner: TransactionBodyError) -> Self {
        Self(TransactionErrorKind::TransactionBody(inner))
    }

    fn verification(inner: crypto::Error) -> Self {
        Self(TransactionErrorKind::Verification(inner))
    }

    fn verification_key(inner: crypto::Error) -> Self {
        Self(TransactionErrorKind::VerificationKey(inner))
    }

    fn unset_body() -> Self {
        Self(TransactionErrorKind::UnsetBody)
    }
}

#[derive(Debug, thiserror::Error)]
enum TransactionErrorKind {
    #[error("`body` field not set")]
    UnsetBody,
    #[error("`signature` field invalid")]
    Signature(#[source] crypto::Error),
    #[error("`body` field invalid")]
    TransactionBody(#[source] TransactionBodyError),
    #[error("`public_key` field invalid")]
    VerificationKey(#[source] crypto::Error),
    #[error("transaction could not be verified given the signature and verification key")]
    Verification(crypto::Error),
}

/// An Astria transaction.
///
/// [`Transaction`] contains an [`Body`] together
/// with its signature and public key.
#[derive(Clone, Debug)]
pub struct Transaction {
    signature: Signature,
    verification_key: VerificationKey,
    body: TransactionBody,
    body_bytes: bytes::Bytes,
}

impl Transaction {
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
    pub fn into_raw(self) -> raw::Transaction {
        let Self {
            signature,
            verification_key,
            body_bytes: transaction_bytes,
            ..
        } = self;
        raw::Transaction {
            signature: Bytes::copy_from_slice(&signature.to_bytes()),
            public_key: Bytes::copy_from_slice(&verification_key.to_bytes()),
            body: Some(pbjson_types::Any {
                type_url: raw::TransactionBody::type_url(),
                value: transaction_bytes,
            }),
        }
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::Transaction {
        let Self {
            signature,
            verification_key,
            body_bytes: transaction_bytes,
            ..
        } = self;
        raw::Transaction {
            signature: Bytes::copy_from_slice(&signature.to_bytes()),
            public_key: Bytes::copy_from_slice(&verification_key.to_bytes()),
            body: Some(pbjson_types::Any {
                type_url: raw::TransactionBody::type_url(),
                value: transaction_bytes.clone(),
            }),
        }
    }

    /// Attempt to convert from a raw, unchecked protobuf [`raw::Transaction`].
    ///
    /// # Errors
    ///
    /// Will return an error if signature or verification key cannot be reconstructed from the bytes
    /// contained in the raw input, if the transaction field was empty (meaning it was mapped to
    /// `None`), if the inner transaction could not be verified given the key and signature, or
    /// if the native [`Body`] could not be created from the inner raw
    /// [`raw::Body`].
    pub fn try_from_raw(proto: raw::Transaction) -> Result<Self, TransactionError> {
        let raw::Transaction {
            signature,
            public_key,
            body,
        } = proto;
        let signature = Signature::try_from(&*signature).map_err(TransactionError::signature)?;
        let verification_key =
            VerificationKey::try_from(&*public_key).map_err(TransactionError::verification_key)?;
        let Some(body) = body else {
            return Err(TransactionError::unset_body());
        };
        let bytes = body.value.clone();
        verification_key
            .verify(&signature, &bytes)
            .map_err(TransactionError::verification)?;
        let transaction = TransactionBody::try_from_any(body).map_err(TransactionError::body)?;
        Ok(Self {
            signature,
            verification_key,
            body: transaction,
            body_bytes: bytes,
        })
    }

    #[must_use]
    pub fn into_unsigned(self) -> TransactionBody {
        self.body
    }

    #[must_use]
    pub fn actions(&self) -> &[Action] {
        self.body.actions.actions()
    }

    #[must_use]
    pub fn group(&self) -> Group {
        self.body.actions.group()
    }

    #[must_use]
    pub fn is_bundleable_sudo_action_group(&self) -> bool {
        self.body.actions.group().is_bundleable_sudo()
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
    pub fn unsigned_transaction(&self) -> &TransactionBody {
        &self.body
    }

    pub fn chain_id(&self) -> &str {
        self.body.chain_id()
    }

    #[must_use]
    pub fn nonce(&self) -> u32 {
        self.body.nonce()
    }
}

#[derive(Clone, Debug)]
pub struct TransactionBody {
    actions: Actions,
    params: TransactionParams,
}

impl TransactionBody {
    #[must_use]
    pub fn builder() -> TransactionBodyBuilder {
        TransactionBodyBuilder::new()
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
    pub fn sign(self, signing_key: &SigningKey) -> Transaction {
        let bytes = self.to_raw().encode_to_vec();
        let signature = signing_key.sign(&bytes);
        let verification_key = signing_key.verification_key();
        Transaction {
            signature,
            verification_key,
            body: self,
            body_bytes: bytes.into(),
        }
    }

    pub fn into_raw(self) -> raw::TransactionBody {
        let Self {
            actions,
            params,
        } = self;
        let actions = actions
            .into_actions()
            .into_iter()
            .map(Action::into_raw)
            .collect();
        raw::TransactionBody {
            actions,
            params: Some(params.into_raw()),
        }
    }

    #[must_use]
    pub fn into_any(self) -> pbjson_types::Any {
        let raw = self.into_raw();
        pbjson_types::Any {
            type_url: raw::TransactionBody::type_url(),
            value: raw.encode_to_vec().into(),
        }
    }

    pub fn to_raw(&self) -> raw::TransactionBody {
        let Self {
            actions,
            params,
        } = self;
        let actions = actions.actions().iter().map(Action::to_raw).collect();
        let params = params.clone().into_raw();
        raw::TransactionBody {
            actions,
            params: Some(params),
        }
    }

    #[must_use]
    pub fn to_any(&self) -> pbjson_types::Any {
        self.clone().into_any()
    }

    /// Attempt to convert from a raw, unchecked protobuf [`raw::Body`].
    ///
    /// # Errors
    ///
    /// Returns an error if one of the inner raw actions could not be converted to a native
    /// [`Action`].
    pub fn try_from_raw(proto: raw::TransactionBody) -> Result<Self, TransactionBodyError> {
        let raw::TransactionBody {
            actions,
            params,
        } = proto;
        let Some(params) = params else {
            return Err(TransactionBodyError::unset_params());
        };
        let params = TransactionParams::from_raw(params);
        let actions: Vec<_> = actions
            .into_iter()
            .map(Action::try_from_raw)
            .collect::<Result<_, _>>()
            .map_err(TransactionBodyError::action)?;

        TransactionBody::builder()
            .actions(actions)
            .chain_id(params.chain_id)
            .nonce(params.nonce)
            .try_build()
            .map_err(TransactionBodyError::group)
    }

    /// Attempt to convert from a protobuf [`pbjson_types::Any`].
    ///
    /// # Errors
    ///
    /// - if the type URL is not the expected type URL
    /// - if the bytes in the [`Any`] do not decode to an [`Body`]
    pub fn try_from_any(any: pbjson_types::Any) -> Result<Self, TransactionBodyError> {
        if any.type_url != raw::TransactionBody::type_url() {
            return Err(TransactionBodyError::invalid_type_url(any.type_url));
        }

        let raw =
            raw::TransactionBody::decode(any.value).map_err(TransactionBodyError::decode_any)?;
        Self::try_from_raw(raw)
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct TransactionBodyError(TransactionBodyErrorKind);

impl TransactionBodyError {
    fn action(inner: action::Error) -> Self {
        Self(TransactionBodyErrorKind::Action(inner))
    }

    fn unset_params() -> Self {
        Self(TransactionBodyErrorKind::UnsetParams())
    }

    fn invalid_type_url(got: String) -> Self {
        Self(TransactionBodyErrorKind::InvalidTypeUrl {
            got,
        })
    }

    fn decode_any(inner: prost::DecodeError) -> Self {
        Self(TransactionBodyErrorKind::DecodeAny(inner))
    }

    fn group(inner: action::group::Error) -> Self {
        Self(TransactionBodyErrorKind::Group(inner))
    }
}

#[derive(Debug, thiserror::Error)]
enum TransactionBodyErrorKind {
    #[error("`actions` field is invalid")]
    Action(#[source] action::Error),
    #[error("`params` field is unset")]
    UnsetParams(),
    #[error(
        "encountered invalid type URL when converting from `google.protobuf.Any`; got `{got}`, \
         expected `{}`",
        raw::TransactionBody::type_url()
    )]
    InvalidTypeUrl { got: String },
    #[error(
        "failed to decode `google.protobuf.Any` to `{}`",
        raw::TransactionBody::type_url()
    )]
    DecodeAny(#[source] prost::DecodeError),
    #[error("`actions` field does not form a valid group of actions")]
    Group(#[source] action::group::Error),
}

#[derive(Default)]
pub struct TransactionBodyBuilder {
    nonce: u32,
    chain_id: String,
    actions: Vec<Action>,
}

impl TransactionBodyBuilder {
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
    pub fn chain_id<T: Into<String>>(self, chain_id: T) -> TransactionBodyBuilder {
        TransactionBodyBuilder {
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

    /// Constructs a [`Body`] from the configured builder.
    ///
    /// # Errors
    /// Returns an error if the actions do not make a valid [`action::Group`].
    ///
    /// Returns an error if the set chain ID does not contain a chain name that can be turned into
    /// a bech32 human readable prefix (everything before the first dash i.e. `<name>-<rest>`).
    pub fn try_build(self) -> Result<TransactionBody, action::group::Error> {
        let Self {
            nonce,
            chain_id,
            actions,
        } = self;
        let actions = Actions::try_from_list_of_actions(actions)?;
        Ok(TransactionBody {
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

    /// Convert from a raw protobuf [`raw::Body`].
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        primitive::v1::{
            asset,
            Address,
        },
        protocol::transaction::v1::action::Transfer,
    };
    const ASTRIA_ADDRESS_PREFIX: &str = "astria";

    fn asset() -> asset::Denom {
        "nria".parse().unwrap()
    }

    #[test]
    fn transaction_id() {
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

        let transfer = Transfer {
            to: Address::builder()
                .array([0; 20])
                .prefix(ASTRIA_ADDRESS_PREFIX)
                .try_build()
                .unwrap(),
            amount: 0,
            asset: asset(),
            fee_asset: asset(),
        };

        let body = TransactionBody::builder()
            .actions(vec![transfer.into()])
            .chain_id("test-1".to_string())
            .nonce(1)
            .try_build()
            .unwrap();

        let tx = Transaction {
            signature,
            verification_key,
            body: body.clone(),
            body_bytes: body.to_raw().encode_to_vec().into(),
        };

        insta::assert_json_snapshot!(tx.id().to_raw());
    }

    #[test]
    fn signed_transaction_verification_roundtrip() {
        let signing_key = SigningKey::from([
            213, 191, 74, 63, 204, 231, 23, 176, 56, 139, 204, 39, 73, 235, 193, 72, 173, 153, 105,
            178, 63, 69, 238, 27, 96, 95, 213, 135, 120, 87, 106, 196,
        ]);

        let transfer = Transfer {
            to: Address::builder()
                .array([0; 20])
                .prefix(ASTRIA_ADDRESS_PREFIX)
                .try_build()
                .unwrap(),
            amount: 0,
            asset: asset(),
            fee_asset: asset(),
        };

        let body = TransactionBody::builder()
            .actions(vec![transfer.into()])
            .chain_id("test-1".to_string())
            .nonce(1)
            .try_build()
            .unwrap();

        let signed_tx = body.sign(&signing_key);
        let raw = signed_tx.to_raw();

        // `try_from_raw` verifies the signature
        Transaction::try_from_raw(raw).unwrap();
    }
}
