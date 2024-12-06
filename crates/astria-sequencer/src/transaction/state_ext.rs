use astria_core::{
    primitive::v1::{
        TransactionId,
        ADDRESS_LEN,
    },
    protocol::transaction::v1::Transaction,
};
use cnidarium::{
    StateRead,
    StateWrite,
};

fn transaction_context() -> &'static str {
    "transaction/context"
}

#[derive(Clone, Copy)]
pub(crate) struct TransactionContext {
    pub(crate) address_bytes: [u8; ADDRESS_LEN],
    pub(crate) transaction_id: TransactionId,
    pub(crate) position_in_transaction: u64,
}

impl TransactionContext {
    pub(crate) fn address_bytes(self) -> [u8; ADDRESS_LEN] {
        self.address_bytes
    }
}

impl From<&Transaction> for TransactionContext {
    fn from(value: &Transaction) -> Self {
        Self {
            address_bytes: *value.address_bytes(),
            transaction_id: value.id(),
            position_in_transaction: 0,
        }
    }
}

pub(crate) trait StateWriteExt: StateWrite {
    fn put_transaction_context(
        &mut self,
        transaction: impl Into<TransactionContext>,
    ) -> TransactionContext {
        let context: TransactionContext = transaction.into();
        self.object_put(transaction_context(), context);
        context
    }

    fn delete_current_transaction_context(&mut self) {
        self.object_delete(transaction_context());
    }
}

pub(crate) trait StateReadExt: StateRead {
    fn get_transaction_context(&self) -> Option<TransactionContext> {
        self.object_get(transaction_context())
    }
}

impl<T: ?Sized + StateRead> StateReadExt for T {}
impl<T: StateWrite> StateWriteExt for T {}
