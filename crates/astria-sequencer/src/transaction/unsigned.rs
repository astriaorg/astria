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
        ActionHandler,
        Signed,
    },
};

/// Represents an unsigned sequencer chain transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unsigned {
    pub(crate) nonce: Nonce,
    pub(crate) actions: Vec<Action>,
}

impl Unsigned {
    /// Creates a new unsigned transaction with the given nonce and actions.
    #[must_use]
    pub fn new_with_actions(nonce: Nonce, actions: Vec<Action>) -> Self {
        Self {
            nonce,
            actions,
        }
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

    /// Signs the transaction with the given signing key.
    #[must_use]
    pub fn into_signed(self, secret_key: &SigningKey) -> Signed {
        let signature = secret_key.sign(&self.hash());
        Signed {
            transaction: self,
            signature,
            public_key: secret_key.verification_key(),
        }
    }

    /// Returns the sha256 hash of the protobuf-encoded transaction.
    pub(crate) fn hash(&self) -> Vec<u8> {
        hash(&self.to_proto().encode_length_delimited_to_vec())
    }
}

#[async_trait::async_trait]
impl ActionHandler for Unsigned {
    fn check_stateless(&self) -> Result<()> {
        for action in &self.actions {
            match action {
                Action::TransferAction(tx) => tx
                    .check_stateless()
                    .context("stateless check failed for TransferAction")?,
                Action::SequenceAction(tx) => tx
                    .check_stateless()
                    .context("stateless check failed for SequenceAction")?,
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

        // do we need to make a StateDelta here so we can check the actions on the successive state?
        for action in &self.actions {
            match action {
                Action::TransferAction(tx) => tx
                    .check_stateful(state, from)
                    .await
                    .context("stateful check failed for TransferAction")?,
                Action::SequenceAction(tx) => tx
                    .check_stateful(state, from)
                    .await
                    .context("stateful check failed for SequenceAction")?,
            }
        }

        Ok(())
    }

    #[instrument(
        skip_all,
        fields(
            nonce = self.nonce.into_inner(),
            from = from.to_string(),
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
                Action::TransferAction(tx) => {
                    tx.execute(state, from)
                        .await
                        .context("execution failed for TransferAction")?;
                }
                Action::SequenceAction(tx) => {
                    tx.execute(state, from)
                        .await
                        .context("execution failed for SequenceAction")?;
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
        types::{
            Address,
            Balance,
            Nonce,
            ADDRESS_LEN,
        },
        Transfer,
    };

    const BOB_ADDRESS: &str = "34fec43c7fcab9aef3b3cf8aba855e41ee69ca3a";

    /// attempts to decode the given hex string into an address.
    fn address_from_hex_string(s: &str) -> Address {
        let bytes = hex::decode(s).unwrap();
        let arr: [u8; ADDRESS_LEN] = bytes.try_into().unwrap();
        Address(arr)
    }

    impl Unsigned {
        /// Converts the encoded protobuf bytes into the corresponding `Transaction` type.
        ///
        /// # Errors
        ///
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
        let tx = Unsigned {
            nonce: Nonce::from(1),
            actions: vec![Action::TransferAction(Transfer::new(
                address_from_hex_string(BOB_ADDRESS),
                Balance::from(333_333),
            ))],
        };
        let bytes = tx.to_proto().encode_length_delimited_to_vec();
        let tx2 = Unsigned::try_from_slice(&bytes).unwrap();
        assert_eq!(tx, tx2);
        println!("0x{}", hex::encode(bytes));

        let secret_key: SigningKey = SigningKey::new(OsRng);
        let signed = tx.into_signed(&secret_key);
        let bytes = signed.to_proto().encode_length_delimited_to_vec();
        Signed::try_from_slice(bytes.as_slice()).unwrap();
    }
}
