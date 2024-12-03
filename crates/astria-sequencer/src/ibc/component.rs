use std::sync::Arc;

use astria_core::protocol::genesis::v1::GenesisAppState;
use astria_eyre::eyre::{
    Result,
    WrapErr as _,
};
use penumbra_ibc::{
    component::Ibc,
    genesis::Content,
};
use tendermint::abci::request::{
    BeginBlock,
    EndBlock,
};
use tracing::{
    instrument,
    Level,
};

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

    #[instrument(name = "IbcComponent::init_chain", skip_all, err)]
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
            .wrap_err("failed to set IBC sudo key")?;

        for address in app_state.ibc_relayer_addresses() {
            state
                .put_ibc_relayer_address(address)
                .wrap_err("failed to write IBC relayer address")?;
        }

        Ok(())
    }

    #[instrument(name = "IbcComponent::begin_block", skip_all, err(level = Level::WARN))]
    async fn begin_block<S: StateWriteExt + 'static>(
        state: &mut Arc<S>,
        begin_block: &BeginBlock,
    ) -> Result<()> {
        Ibc::begin_block::<AstriaHost, S>(state, begin_block).await;
        Ok(())
    }

    #[instrument(name = "IbcComponent::end_block", skip_all, er(level = Level::WARN))]
    async fn end_block<S: StateWriteExt + 'static>(
        state: &mut Arc<S>,
        end_block: &EndBlock,
    ) -> Result<()> {
        Ibc::end_block(state, end_block).await;
        Ok(())
    }
}
