use anyhow::{
    bail,
    Context as _,
    Result,
};
use astria_proto::sequencer::v1::{
    unsigned_transaction::Value::{
        AccountsTransaction as ProtoAccountsTransaction,
        SecondaryTransaction as ProtoSecondaryTransaction,
    },
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
    secondary::transaction::Transaction as SecondaryTransaction,
    transaction::{
        unsigned::Transaction as UnsignedTransaction,
        ActionHandler,
    },
};

/// Represents a transaction signed by a user.
/// It contains the signature and the public key of the user,
/// as well as the transaction itself.
#[derive(Debug, Clone)]
pub(crate) struct Transaction {
    pub(crate) signature: Signature,
    pub(crate) public_key: VerificationKey,
    pub(crate) transaction: UnsignedTransaction,
}

impl Transaction {
    /// Verifies the transaction signature.
    /// The transaction signature message is the hash of the transaction.
    ///
    /// # Errors
    ///
    /// - If the signature is invalid
    pub(crate) fn verify_signature(&self) -> Result<()> {
        self.public_key
            .verify(&self.signature, &self.transaction.hash())
            .context("failed to verify transaction signature")
    }

    /// Returns the address which signed the transaction.
    ///
    /// # Errors
    ///
    /// - If the public key cannot be converted into an address
    pub(crate) fn signer_address(&self) -> Result<Address> {
        Address::try_from(&self.public_key)
    }

    /// Attempts to decode a signed transaction from the given bytes.
    ///
    /// # Errors
    ///
    /// - If the bytes cannot be decoded into the prost-generated `SignedTransaction` type
    /// - If the transaction value is missing
    /// - If the transaction value is not a valid transaction type (ie. does not correspond to any
    ///   component)
    /// - If the signature cannot be decoded
    /// - If the public key cannot be decoded
    pub(crate) fn try_from_slice(bytes: &[u8]) -> Result<Self> {
        let proto_tx: ProtoSignedTransaction =
            ProtoSignedTransaction::decode_length_delimited(bytes)?;
        let Some(proto_transaction) = proto_tx.transaction else {
            bail!("transaction is missing");
        };

        let Some(value) = proto_transaction.value else {
            bail!("unsigned transaction value missing")
        };

        let transaction = match value {
            ProtoAccountsTransaction(tx) => {
                UnsignedTransaction::AccountsTransaction(AccountsTransaction::try_from_proto(&tx)?)
            }
            ProtoSecondaryTransaction(tx) => {
                UnsignedTransaction::SecondaryTransaction(SecondaryTransaction::from_proto(&tx)?)
            }
        };
        let signed_tx = Transaction {
            transaction,
            signature: Signature::try_from(proto_tx.signature.as_slice())?,
            public_key: VerificationKey::try_from(proto_tx.public_key.as_slice())?,
        };
        Ok(signed_tx)
    }
}

#[async_trait]
impl ActionHandler for Transaction {
    #[instrument]
    fn check_stateless(&self) -> Result<()> {
        self.verify_signature()?;
        match &self.transaction {
            UnsignedTransaction::AccountsTransaction(_) => Ok(()),
            UnsignedTransaction::SecondaryTransaction(tx) => tx.check_stateless(),
        }
    }

    #[instrument(skip(state))]
    async fn check_stateful<S: StateRead + 'static>(&self, state: &S) -> Result<()> {
        match &self.transaction {
            UnsignedTransaction::AccountsTransaction(tx) => {
                tx.check_stateful(state, &self.signer_address()?).await
            }
            UnsignedTransaction::SecondaryTransaction(tx) => {
                tx.check_stateful(state, &self.signer_address()?).await
            }
        }
    }

    #[instrument(skip(state))]
    async fn execute<S: StateWrite>(&self, state: &mut S) -> Result<()> {
        match &self.transaction {
            UnsignedTransaction::AccountsTransaction(tx) => {
                tx.execute(state, &self.signer_address()?).await
            }
            UnsignedTransaction::SecondaryTransaction(tx) => {
                tx.execute(state, &self.signer_address()?).await
            }
        }
    }
}
