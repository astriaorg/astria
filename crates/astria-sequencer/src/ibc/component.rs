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
use tendermint::{
    abci::{
        self,
    },
    account::Id,
    block::Header,
    Hash,
};
use tracing::{
    instrument,
    Level,
};

use crate::{
    component::{
        Component,
        PrepareStateInfo,
    },
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
        prepare_state_info: &PrepareStateInfo,
    ) -> Result<()> {
        // Only fields used currently are: `app_hash`, `chain_id`, `height`, `next_validators_hash`,
        // and `time`
        let begin_block: abci::request::BeginBlock = abci::request::BeginBlock {
            hash: Hash::default(),
            byzantine_validators: prepare_state_info.byzantine_validators.clone(),
            header: Header {
                app_hash: prepare_state_info.app_hash.clone(),
                chain_id: prepare_state_info.chain_id.clone(),
                consensus_hash: Hash::default(),
                data_hash: Some(Hash::default()),
                evidence_hash: Some(Hash::default()),
                height: prepare_state_info.height,
                last_block_id: None,
                last_commit_hash: Some(Hash::default()),
                last_results_hash: Some(Hash::default()),
                next_validators_hash: prepare_state_info.next_validators_hash,
                proposer_address: Id::new([0; 20]),
                time: prepare_state_info.time,
                validators_hash: Hash::default(),
                version: tendermint::block::header::Version {
                    app: 0,
                    block: 0,
                },
            },
            last_commit_info: tendermint::abci::types::CommitInfo {
                round: 0u16.into(),
                votes: vec![],
            },
        };
        Ibc::begin_block::<AstriaHost, S>(state, &begin_block).await;
        Ok(())
    }

    #[instrument(name = "IbcComponent::end_block", skip_all, er(level = Level::WARN))]
    async fn end_block<S: StateWriteExt + 'static>(
        state: &mut Arc<S>,
        height: tendermint::block::Height,
    ) -> Result<()> {
        Ibc::end_block(
            state,
            &abci::request::EndBlock {
                height: height.into(),
            },
        )
        .await;
        Ok(())
    }
}
