use anyhow::{
    anyhow,
    bail,
    Context as _,
    Result,
};
use astria_proto::sequencer::v1::SignedTransaction as ProtoSignedTransaction;
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
    pub fn verify_signature(&self) -> Result<()> {
        self.public_key
            .verify(&self.transaction.hash()?, &self.signature)
            .context("failed to verify transaction signature")
    }

    pub fn from_address(&self) -> Result<Address> {
        Address::try_from(&self.public_key)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let proto = ProtoSignedTransaction {
            transaction: Some(match &self.transaction {
                UnsignedTransaction::AccountsTransaction(tx) => {
                    astria_proto::sequencer::v1::UnsignedTransaction {
                        value: Some(astria_proto::sequencer::v1::unsigned_transaction::Value::AccountsTransaction (
                            tx.to_proto()
                        )),
                    }
                }
            }),
            signature: self.signature.to_bytes().to_vec(),
            public_key: self.public_key.to_bytes().to_vec(),
        };

        let bytes = proto.encode_length_delimited_to_vec();
        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let proto_tx: ProtoSignedTransaction =
            ProtoSignedTransaction::decode_length_delimited(bytes)?;
        let Some(proto_transaction) = proto_tx.transaction else {
            bail!("transaction is missing");
        };

        let Some(value) = proto_transaction.value else {
            bail!("unsigned transaction value missing")
        };

        let transaction = match value {
            astria_proto::sequencer::v1::unsigned_transaction::Value::AccountsTransaction(tx) => {
                UnsignedTransaction::AccountsTransaction(AccountsTransaction::from_proto(&tx)?)
            }
        };
        let signed_tx = SignedTransaction {
            transaction,
            signature: Signature::from_bytes(&proto_tx.signature)?,
            public_key: PublicKey::from_bytes(&proto_tx.public_key)?,
        };
        Ok(signed_tx)
    }

    pub fn hash(&self) -> Result<TransactionHash> {
        hash(&self.to_bytes()?)
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
        }
    }

    #[instrument(skip(state))]
    async fn check_stateful<S: StateRead + 'static>(&self, state: &S) -> Result<()> {
        match &self.transaction {
            UnsignedTransaction::AccountsTransaction(tx) => {
                tx.check_stateful(state, &self.from_address()?).await
            }
        }
    }

    #[instrument(skip(state))]
    async fn execute<S: StateWrite>(&self, state: &mut S) -> Result<()> {
        match &self.transaction {
            UnsignedTransaction::AccountsTransaction(tx) => {
                tx.execute(state, &self.from_address()?).await
            }
        }
    }
}
