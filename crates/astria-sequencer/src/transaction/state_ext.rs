use cnidarium::{
    StateRead,
    StateWrite,
};

fn transaction_context() -> &'static str {
    "transaction/context"
}

/// Extension trait to write transaction context to the ephemeral store.
pub(crate) trait StateWriteExt: StateWrite {
    fn put_transaction_context(&mut self, context: super::Context) {
        self.object_put(transaction_context(), context);
    }

    fn delete_current_transaction_context(&mut self) {
        self.object_delete(transaction_context());
    }
}

pub(crate) trait StateReadExt: StateRead {
    fn get_transaction_context(&self) -> Option<super::Context> {
        self.object_get(transaction_context())
    }
}

impl<T: ?Sized + StateRead> StateReadExt for T {}
impl<T: StateWrite> StateWriteExt for T {}
