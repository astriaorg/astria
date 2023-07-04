use anyhow::{
    bail,
    Context as _,
    Result,
};
use astria_proto::sequencer::v1::{
    unsigned_transaction::Value::AccountsTransaction as ProtoAccountsTransaction,
    SignedTransaction as ProtoSignedTransaction,
};
use async_trait::async_trait;
use penumbra_storage::{
    StateRead,
    StateWrite,
};
use prost::Message as _;
use tracing::instrument;

use crate::{
    accounts::{
        transaction::Transaction as AccountsTransaction,
        types::Address,
    },
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
pub(crate) struct Signed {
    pub(crate) signature: Signature,
    pub(crate) public_key: VerificationKey,
    pub(crate) transaction: Unsigned,
}

impl Signed {
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

        let Some(value) = proto_transaction.value else {
            bail!("unsigned transaction value missing")
        };

        let transaction = match value {
            ProtoAccountsTransaction(tx) => {
                Unsigned::AccountsTransaction(AccountsTransaction::try_from_proto(&tx)?)
            }
        };
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
    pub(crate) fn try_from_slice(slice: &[u8]) -> Result<Self> {
        let proto = ProtoSignedTransaction::decode_length_delimited(slice)
            .context("failed to decode slice to proto signed transaction")?;
        Self::try_from_proto(proto)
    }
}

#[async_trait]
impl ActionHandler for Signed {
    #[instrument]
    fn check_stateless(&self) -> Result<()> {
        self.verify_signature()?;
        match &self.transaction {
            Unsigned::AccountsTransaction(_) => Ok(()),
        }
    }

    #[instrument(skip(state))]
    async fn check_stateful<S: StateRead + 'static>(&self, state: &S) -> Result<()> {
        match &self.transaction {
            Unsigned::AccountsTransaction(tx) => {
                tx.check_stateful(state, &self.signer_address()).await
            }
        }
    }

    #[instrument(skip(state))]
    async fn execute<S: StateWrite>(&self, state: &mut S) -> Result<()> {
        match &self.transaction {
            Unsigned::AccountsTransaction(tx) => tx.execute(state, &self.signer_address()).await,
        }
    }
}
