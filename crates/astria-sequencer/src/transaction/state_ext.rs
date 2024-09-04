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

fn current_source() -> &'static str {
    "transaction/current_source"
}

#[derive(Clone, Copy)]
pub(crate) struct TransactionContext {
    pub(crate) address_bytes: [u8; ADDRESS_LEN],
    pub(crate) transaction_id: TransactionId,
    pub(crate) position_in_source_transaction: Option<u64>,
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
            position_in_source_transaction: Some(0),
        }
    }
}

pub(crate) trait StateWriteExt: StateWrite {
    fn put_transaction_context(
        &mut self,
        transaction: impl Into<TransactionContext>,
    ) -> TransactionContext {
        let context: TransactionContext = transaction.into();
        self.object_put(current_source(), context);
        context
    }

    fn delete_current_source(&mut self) {
        self.object_delete(current_source());
    }

    #[instrument(skip_all)]
    fn set_position_in_source_transaction(&mut self, val: u64) -> Result<()> {
        let mut context = self
            .get_current_source()
            .context("failed to get current source")?;
        context.position_in_source_transaction = Some(val);
        self.object_put(current_source(), context);
        Ok(())
    }
}

pub(crate) trait StateReadExt: StateRead {
    fn get_current_source(&self) -> Option<TransactionContext> {
        self.object_get(current_source())
    }
}

impl<T: ?Sized + StateRead> StateReadExt for T {}
impl<T: StateWrite> StateWriteExt for T {}
