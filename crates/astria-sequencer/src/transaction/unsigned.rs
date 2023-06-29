use anyhow::{
    ensure,
    Context,
    Result,
};
use astria_proto::sequencer::v1::UnsignedTransaction as ProtoUnsignedTransaction;
use prost::Message as _;
use serde::{
    Deserialize,
    Serialize,
};
use tracing::instrument;

use crate::{
    accounts::{
        state_ext::{
            StateReadExt,
            StateWriteExt,
        },
        types::{
            Address,
            Nonce,
        },
    },
    crypto::SigningKey,
    hash,
    transaction::{
        action::Action,
        signed::Transaction as SignedTransaction,
        ActionHandler,
    },
};

/// Represents an unsigned sequencer chain transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Transaction {
    pub(crate) nonce: Nonce,
    pub(crate) actions: Vec<Action>,
}

impl Transaction {
    /// Attempts to encode the unsigned transaction into bytes.
    #[must_use]
    pub(crate) fn to_vec(&self) -> Vec<u8> {
        self.to_proto().encode_length_delimited_to_vec()
    }

    pub(crate) fn to_proto(&self) -> ProtoUnsignedTransaction {
        let mut proto = ProtoUnsignedTransaction {
            nonce: self.nonce.into(),
            actions: Vec::with_capacity(self.actions.len()),
        };
        for action in &self.actions {
            proto.actions.push(action.to_proto());
        }
        proto
    }

    pub(crate) fn try_from_proto(proto: &ProtoUnsignedTransaction) -> Result<Self> {
        let mut actions = Vec::with_capacity(proto.actions.len());
        for action in &proto.actions {
            actions.push(Action::try_from_proto(action)?);
        }
        Ok(Self {
            nonce: proto.nonce.into(),
            actions,
        })
    }

    /// Signs the transaction with the given keypair.
    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn sign(self, secret_key: &SigningKey) -> SignedTransaction {
        let signature = secret_key.sign(&self.hash());
        SignedTransaction {
            transaction: self,
            signature,
            public_key: secret_key.verification_key(),
        }
    }

    pub(crate) fn hash(&self) -> Vec<u8> {
        hash(&self.to_vec())
    }
}

#[async_trait::async_trait]
impl ActionHandler for Transaction {
    fn check_stateless(&self) -> Result<()> {
        for action in &self.actions {
            match action {
                Action::AccountsAction(_) | Action::SecondaryAction(_) => {}
            }
        }
        Ok(())
    }

    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: &Address,
    ) -> Result<()> {
        let curr_nonce = state.get_account_nonce(from).await?;
        ensure!(curr_nonce < self.nonce, "invalid nonce");
        Ok(())
    }

    #[instrument(
        skip_all,
        fields(
            nonce = self.nonce.into_inner(),
        )
    )]
    async fn execute<S: StateWriteExt>(&self, state: &mut S, from: &Address) -> Result<()> {
        // TODO: make a new StateDelta so this is atomic / can be rolled back in case of error

        let from_nonce = state
            .get_account_nonce(from)
            .await
            .context("failed getting `from` nonce")?;
        state
            .put_account_nonce(from, from_nonce + Nonce::from(1))
            .context("failed updating `from` nonce")?;

        for action in &self.actions {
            match action {
                Action::AccountsAction(tx) => {
                    tx.execute(state, from).await?;
                }
                Action::SecondaryAction(tx) => {
                    tx.execute(state, from).await?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use anyhow::{
        Context as _,
        Result,
    };
    use rand::rngs::OsRng;

    use super::*;
    use crate::accounts::{
        transaction::Transfer,
        types::{
            Address,
            Balance,
            Nonce,
            ADDRESS_LEN,
        },
    };

    const BOB_ADDRESS: &str = "34fec43c7fcab9aef3b3cf8aba855e41ee69ca3a";

    /// attempts to decode the given hex string into an address.
    fn address_from_hex_string(s: &str) -> Address {
        let bytes = hex::decode(s).unwrap();
        let arr: [u8; ADDRESS_LEN] = bytes.try_into().unwrap();
        Address::from_array(arr)
    }

    impl Transaction {
        /// Attempts to decode an unsigned transaction from the given bytes.
        ///
        /// # Errors
        ///
        /// - If the bytes cannot be decoded into the prost-generated `UnsignedTransaction` type
        /// - If the value is missing
        /// - If the value is not a valid transaction type (ie. does not correspond to any
        ///   component)
        fn try_from_slice(bytes: &[u8]) -> Result<Self> {
            let proto = ProtoUnsignedTransaction::decode_length_delimited(bytes)
                .context("failed to decode unsigned transaction")?;
            Self::try_from_proto(&proto)
        }
    }

    #[test]
    fn test_unsigned_transaction() {
        let tx = Transaction {
            nonce: Nonce::from(1),
            actions: vec![Action::AccountsAction(Transfer::new(
                address_from_hex_string(BOB_ADDRESS),
                Balance::from(333_333),
            ))],
        };
        let bytes = tx.to_vec();
        let tx2 = Transaction::try_from_slice(&bytes).unwrap();
        assert_eq!(tx, tx2);
        println!("0x{}", hex::encode(bytes));

        let secret_key: SigningKey = SigningKey::new(OsRng);
        let signed = tx.sign(&secret_key);
        signed.verify_signature().unwrap();
    }
}
