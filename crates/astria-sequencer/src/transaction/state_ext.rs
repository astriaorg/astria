use anyhow::{
    Context as _,
    Result,
};
use astria_core::{
    primitive::v1::{
        TransactionId,
        ADDRESS_LEN,
    },
    protocol::transaction::v1alpha1::SignedTransaction,
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::instrument;

fn index_of_action_storage_key() -> &'static str {
    "transaction/index_of_action"
}

fn current_source() -> &'static str {
    "transaction/current_source"
}

#[derive(Clone)]
pub(crate) struct TransactionContext {
    pub(crate) address_bytes: [u8; ADDRESS_LEN],
    pub(crate) transaction_id: TransactionId,
}

impl TransactionContext {
    pub(crate) fn address_bytes(&self) -> [u8; ADDRESS_LEN] {
        self.address_bytes
    }
}

impl From<&SignedTransaction> for TransactionContext {
    fn from(value: &SignedTransaction) -> Self {
        Self {
            address_bytes: value.address_bytes(),
            transaction_id: value.id(),
        }
    }
}

pub(crate) trait StateWriteExt: StateWrite {
    fn put_current_source(&mut self, transaction: impl Into<TransactionContext>) {
        let context: TransactionContext = transaction.into();
        self.object_put(current_source(), context);
    }

    fn delete_current_source(&mut self) {
        self.object_delete(current_source());
    }

    #[instrument(skip_all)]
    fn put_transaction_index_of_action(&mut self, index: u32) {
        self.nonverifiable_put_raw(
            index_of_action_storage_key().as_bytes().to_vec(),
            borsh::to_vec(&index).expect("serialize index of action"),
        );
    }

    #[instrument(skip_all)]
    async fn increment_transaction_index_of_action(&mut self) -> Result<()> {
        let index = self
            .get_transaction_index_of_action()
            .await?
            .expect("index of action should be `Some`");
        let index = index.checked_add(1).expect("increment index of action");
        self.nonverifiable_put_raw(
            index_of_action_storage_key().as_bytes().to_vec(),
            borsh::to_vec(&index).expect("serialize index of action"),
        );
        Ok(())
    }

    #[instrument(skip_all)]
    fn clear_transaction_index_of_action(&mut self) {
        self.nonverifiable_delete(index_of_action_storage_key().as_bytes().to_vec());
    }
}

pub(crate) trait StateReadExt: StateRead {
    fn get_current_source(&self) -> Option<TransactionContext> {
        self.object_get(current_source())
    }

    #[instrument(skip_all)]
    async fn get_transaction_index_of_action(&self) -> Result<Option<u32>> {
        let Some(bytes) = self
            .nonverifiable_get_raw(index_of_action_storage_key().as_bytes())
            .await
            .context("failed reading raw index of action from state")?
        else {
            return Ok(None);
        };

        let index = borsh::from_slice(&bytes).context("failed to deserialize index bytes")?;
        Ok(Some(index))
    }
}

impl<T: ?Sized + StateRead> StateReadExt for T {}
impl<T: StateWrite> StateWriteExt for T {}
