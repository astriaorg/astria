use astria_core::protocol::transaction::v1::action::IbcRelay;
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        self,
        ensure,
        WrapErr as _,
    },
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use penumbra_ibc::IbcRelayWithHandlers;

use crate::{
    app::ActionHandler,
    ibc::{
        host_interface::AstriaHost,
        StateReadExt as _,
    },
};

mod msg_handler;
use msg_handler::Ics20Transfer;

#[async_trait::async_trait]
impl ActionHandler for IbcRelay {
    async fn check_stateless(&self) -> astria_eyre::eyre::Result<()> {
        let action = self.clone().with_handler::<Ics20Transfer, AstriaHost>();
        action
            .check_stateless(())
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("stateless check failed for IbcAction")?;
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(
        &self,
        state: S,
        context: crate::transaction::Context,
    ) -> astria_eyre::eyre::Result<()> {
        ensure!(
            state
                .is_ibc_relayer(&context.address_bytes)
                .await
                .wrap_err("failed to check if address is IBC relayer")?,
            "only IBC sudo address can execute IBC actions"
        );
        wrap_foreign_check_and_execute(self, state, context)
            .check_and_execute()
            .await
            .wrap_err("failed executing ibc action")?;
        Ok(())
    }
}

fn wrap_foreign_check_and_execute<S: StateWrite>(
    action: &IbcRelay,
    mut state: S,
    context: crate::transaction::Context,
) -> Guard<S> {
    state.put_transaction_context(context);
    Guard {
        state: Some(state),
        action: action.clone().with_handler::<Ics20Transfer, AstriaHost>(),
    }
}

struct Guard<S: StateWrite> {
    action: IbcRelayWithHandlers<Ics20Transfer, AstriaHost>,
    state: Option<S>,
}

impl<S: StateWrite> Guard<S> {
    async fn check_and_execute(mut self) -> eyre::Result<()> {
        self.action
            .check_and_execute(
                self.state.as_mut().expect(
                    "the state must be present on check_and_execute; it's only nulled on drop",
                ),
            )
            .await
            .map_err(anyhow_to_eyre)?;
        Ok(())
    }
}

impl<S: StateWrite> Drop for Guard<S> {
    fn drop(&mut self) {
        self.state
            .take()
            .expect("drop must not be called twice on the guard")
            .delete_current_transaction_context()
    }
}

const TRANSACTION_CONTEXT: &str = "ibc/transaction_context";

trait StateWriteExt: StateWrite {
    fn put_transaction_context(&mut self, context: crate::transaction::Context) {
        self.object_put(TRANSACTION_CONTEXT, context);
    }

    fn delete_current_transaction_context(&mut self) {
        self.object_delete(TRANSACTION_CONTEXT);
    }
}

trait StateReadExt: StateRead {
    fn get_transaction_context(&self) -> Option<crate::transaction::Context> {
        self.object_get(TRANSACTION_CONTEXT)
    }
}

impl<T: ?Sized + StateRead> StateReadExt for T {}
impl<T: StateWrite> StateWriteExt for T {}
