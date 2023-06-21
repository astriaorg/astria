use anyhow::{
    anyhow,
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
    UnsignedTransaction as ProtoUnsignedTransaction,
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
        PublicKey,
        Signature,
        Verifier,
    },
    hash,
    secondary::transaction::Transaction as SecondaryTransaction,
    transaction::{
        ActionHandler,
        TransactionHash,
        UnsignedTransaction,
    },
};

#[derive(Debug, Clone)]
pub struct SignedTransaction {
    pub signature: Signature,
    pub public_key: PublicKey,
    pub transaction: UnsignedTransaction,
}

impl SignedTransaction {
    /// Verifies the transaction signature.
    /// The transaction signature message is the hash of the transaction.
    ///
    /// # Errors
    ///
    /// - If the signature is invalid
    pub fn verify_signature(&self) -> Result<()> {
        self.public_key
            .verify(&self.transaction.hash(), &self.signature)
            .context("failed to verify transaction signature")
    }

    /// Returns the address which signed the transaction.
    ///
    /// # Errors
    ///
    /// - If the public key cannot be converted into an address
    pub fn signer_address(&self) -> Result<Address> {
        Address::try_from(&self.public_key)
    }

    #[must_use]
    pub fn to_vec(&self) -> Vec<u8> {
        let tx = match &self.transaction {
            UnsignedTransaction::AccountsTransaction(tx) => ProtoUnsignedTransaction {
                value: Some(ProtoAccountsTransaction(tx.to_proto())),
            },
            UnsignedTransaction::SecondaryTransaction(tx) => ProtoUnsignedTransaction {
                value: Some(ProtoSecondaryTransaction(tx.to_proto())),
            },
        };

        let proto = ProtoSignedTransaction {
            transaction: Some(tx),
            signature: self.signature.to_bytes().to_vec(),
            public_key: self.public_key.to_bytes().to_vec(),
        };

        proto.encode_length_delimited_to_vec()
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
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self> {
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
                UnsignedTransaction::AccountsTransaction(AccountsTransaction::from_proto(&tx)?)
            }
            ProtoSecondaryTransaction(tx) => {
                UnsignedTransaction::SecondaryTransaction(SecondaryTransaction::from_proto(&tx)?)
            }
        };
        let signed_tx = SignedTransaction {
            transaction,
            signature: Signature::from_bytes(&proto_tx.signature)?,
            public_key: PublicKey::from_bytes(&proto_tx.public_key)?,
        };
        Ok(signed_tx)
    }

    /// Returns the sha256 hash of the encoded transaction.
    ///
    /// # Errors
    ///
    /// - If the hash cannot be converted into 32 bytes
    pub fn hash(&self) -> Result<TransactionHash> {
        hash(&self.to_vec())
            .try_into()
            .map_err(|_| anyhow!("failed to turn hash into 32 bytes"))
    }
}

#[async_trait]
impl ActionHandler for SignedTransaction {
    #[instrument]
    fn check_stateless(&self) -> Result<()> {
        self.verify_signature()?;
        match &self.transaction {
            UnsignedTransaction::AccountsTransaction(tx) => tx.check_stateless(),
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
