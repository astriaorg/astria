use std::sync::Arc;

use anyhow::{
    Context,
    Result,
};
use astria_core::protocol::genesis::v1alpha1::GenesisAppState;
use penumbra_ibc::{
    component::Ibc,
    genesis::Content,
};
use tendermint::abci::request::{
    BeginBlock,
    EndBlock,
};
use tracing::instrument;

use crate::{
    component::Component,
    ibc::{
        host_interface::AstriaHost,
        state_ext::StateWriteExt,
    },
};

#[derive(Default)]
pub(crate) struct IbcComponent;

#[async_trait::async_trait]
impl Component for IbcComponent {
    type AppState = GenesisAppState;

    #[instrument(name = "IbcComponent::init_chain", skip_all)]
    async fn init_chain<S: StateWriteExt>(mut state: S, app_state: &Self::AppState) -> Result<()> {
        Ibc::init_chain(
            &mut state,
            Some(&Content {
                ibc_params: app_state.ibc_parameters().clone(),
            }),
        )
        .await;

        state
            .put_ibc_sudo_address(*app_state.ibc_sudo_address())
            .context("failed to set IBC sudo key")?;

        for address in app_state.ibc_relayer_addresses() {
            // No need to add context as this method already reports sufficient context on error.
            state.put_ibc_relayer_address(address)?;
        }

        state
            .put_ics20_withdrawal_base_fee(app_state.fees().ics20_withdrawal_base_fee)
            .context("failed to put ics20 withdrawal base fee")?;
        Ok(())
    }

    #[instrument(name = "IbcComponent::begin_block", skip_all)]
    async fn begin_block<S: StateWriteExt + 'static>(
        state: &mut Arc<S>,
        begin_block: &BeginBlock,
    ) -> Result<()> {
        Ibc::begin_block::<AstriaHost, S>(state, begin_block).await;
        Ok(())
    }

    #[instrument(name = "IbcComponent::end_block", skip_all)]
    async fn end_block<S: StateWriteExt + 'static>(
        state: &mut Arc<S>,
        end_block: &EndBlock,
    ) -> Result<()> {
        Ibc::end_block(state, end_block).await;
        Ok(())
    }
}
