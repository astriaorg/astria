use anyhow::{
    bail,
    Context as _,
    Result,
};
use astria_proto::sequencer::v1::SignedTransaction as ProtoSignedTransaction;
use penumbra_storage::{
    StateRead,
    StateWrite,
};
use prost::Message as _;
use tracing::instrument;

use crate::{
    accounts::types::Address,
    crypto::{
        Signature,
        VerificationKey,
    },
    transaction::{
        ActionHandler,
        Unsigned,
    },
};

/// Represents a transaction signed by a user.
/// It contains the signature and the public key of the user,
/// as well as the transaction itself.
///
/// Invariant: this type can only be constructed with a valid signature.
#[derive(Debug, Clone)]
pub struct Signed {
    pub(crate) signature: Signature,
    pub(crate) public_key: VerificationKey,
    pub(crate) transaction: Unsigned,
}

impl Signed {
    #[must_use]
    pub(crate) fn to_proto(&self) -> ProtoSignedTransaction {
        ProtoSignedTransaction {
            transaction: Some(self.transaction.to_proto()),
            signature: self.signature.to_bytes().to_vec(),
            public_key: self.public_key.to_bytes().to_vec(),
        }
    }

    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_proto().encode_to_vec()
    }

    #[must_use]
    pub fn transaction(&self) -> &Unsigned {
        &self.transaction
    }

    /// Verifies the transaction signature.
    /// The transaction signature message is the hash of the transaction.
    ///
    /// # Errors
    ///
    /// - If the signature is invalid
    fn verify_signature(&self) -> Result<()> {
        self.public_key
            .verify(&self.signature, &self.transaction.hash())
            .context("failed to verify transaction signature")
    }

    /// Returns the address which signed the transaction.
    ///
    /// # Errors
    ///
    /// - If the public key cannot be converted into an address
    pub(crate) fn signer_address(&self) -> Address {
        Address::from_verification_key(&self.public_key)
    }

    /// Converts the protobuf signed transaction into a `SignedTransaction`.
    ///
    /// # Errors
    ///
    /// - If the transaction value is missing
    /// - If the transaction value is not a valid transaction type (ie. does not correspond to any
    ///   component)
    /// - If the signature cannot be decoded
    /// - If the public key cannot be decoded
    /// - If the signature is invalid
    pub(crate) fn try_from_proto(proto: ProtoSignedTransaction) -> Result<Self> {
        let Some(proto_transaction) = proto.transaction else {
            bail!("transaction is missing");
        };
        let transaction = Unsigned::try_from_proto(&proto_transaction)
            .context("failed to convert proto to unsigned transaction")?;

        let signed_tx = Signed {
            transaction,
            signature: Signature::try_from(proto.signature.as_slice())?,
            public_key: VerificationKey::try_from(proto.public_key.as_slice())?,
        };
        signed_tx.verify_signature()?;

        Ok(signed_tx)
    }

    /// Attempts to convert a slice into a `SignedTransaction`, where the slice
    /// is an encoded protobuf signed transaction.
    ///
    /// # Errors
    ///
    /// - If the slice cannot be decoded into a protobuf signed transaction
    /// - If the protobuf signed transaction cannot be converted into a `SignedTransaction`
    pub fn try_from_slice(slice: &[u8]) -> Result<Self> {
        let proto = ProtoSignedTransaction::decode(slice)
            .context("failed to decode slice to proto signed transaction")?;
        Self::try_from_proto(proto)
    }
}

impl Signed {
    #[instrument]
    pub(crate) fn check_stateless(&self) -> Result<()> {
        self.verify_signature()?;
        self.transaction
            .check_stateless()
            .context("stateless check failed")?;
        Ok(())
    }

    #[instrument(skip(state))]
    pub(crate) async fn check_stateful<S: StateRead + 'static>(&self, state: &S) -> Result<()> {
        self.transaction
            .check_stateful(state, &self.signer_address())
            .await
    }

    #[instrument(skip(state))]
    pub(crate) async fn execute<S: StateWrite>(&self, state: &mut S) -> Result<()> {
        self.transaction
            .execute(state, &self.signer_address())
            .await
    }
}
