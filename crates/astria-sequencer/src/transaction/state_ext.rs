use anyhow::{
    Context as _,
    Result,
};
use astria_core::{
    primitive::v1::ADDRESS_LEN,
    protocol::transaction::v1alpha1::SignedTransaction,
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::instrument;

fn deposit_index_storage_key() -> &'static str {
    "transaction/deposit_index"
}

fn current_source() -> &'static str {
    "transaction/current_source"
}

#[derive(Clone)]
pub(crate) struct TransactionContext {
    pub(crate) address_bytes: [u8; ADDRESS_LEN],
    pub(crate) transaction_hash: String,
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
            transaction_hash: hex::encode(value.sha256_of_proto_encoding()),
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
    fn put_transaction_deposit_index(&mut self, index: u32) {
        self.nonverifiable_put_raw(
            deposit_index_storage_key().as_bytes().to_vec(),
            borsh::to_vec(&index).expect("serialize deposit index"),
        );
    }

    #[instrument(skip_all)]
    fn clear_transaction_deposit_index(&mut self) {
        self.nonverifiable_delete(deposit_index_storage_key().as_bytes().to_vec());
    }
}

pub(crate) trait StateReadExt: StateRead {
    fn get_current_source(&self) -> Option<TransactionContext> {
        self.object_get(current_source())
    }

    #[instrument(skip_all)]
    async fn get_transaction_deposit_index(&self) -> Result<Option<u32>> {
        let Some(bytes) = self
            .nonverifiable_get_raw(deposit_index_storage_key().as_bytes())
            .await
            .context("failed reading raw deposit index from state")?
        else {
            return Ok(None);
        };

        let index = borsh::from_slice(&bytes).context("failed to deserialize index bytes")?;
        Ok(Some(index))
    }
}

impl<T: ?Sized + StateRead> StateReadExt for T {}
impl<T: StateWrite> StateWriteExt for T {}
