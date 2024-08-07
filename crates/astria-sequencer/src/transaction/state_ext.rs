use astria_core::{
    primitive::v1::ADDRESS_LEN,
    protocol::transaction::v1alpha1::SignedTransaction,
};
use cnidarium::{
    StateRead,
    StateWrite,
};

fn current_source() -> &'static str {
    "transaction/current_source"
}

#[derive(Clone)]
pub(crate) struct TransactionContext {
    pub(crate) address_bytes: [u8; ADDRESS_LEN],
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
}

pub(crate) trait StateReadExt: StateRead {
    fn get_current_source(&self) -> Option<TransactionContext> {
        self.object_get(current_source())
    }
}

impl<T: ?Sized + StateRead> StateReadExt for T {}
impl<T: StateWrite> StateWriteExt for T {}
