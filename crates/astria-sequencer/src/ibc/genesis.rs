use astria_core::protocol::genesis::v1::GenesisAppState;
use astria_eyre::eyre::{
    Result,
    WrapErr as _,
};
use penumbra_ibc::{
    component::Ibc,
    genesis::Content,
};
use tracing::instrument;

use crate::{
    genesis::Genesis,
    ibc::state_ext::StateWriteExt,
};

#[derive(Default)]
pub(crate) struct IbcComponent;

#[async_trait::async_trait]
impl Genesis for IbcComponent {
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
            .wrap_err("failed to set IBC sudo key")?;

        for address in app_state.ibc_relayer_addresses() {
            state
                .put_ibc_relayer_address(address)
                .wrap_err("failed to write IBC relayer address")?;
        }

        Ok(())
    }
}
