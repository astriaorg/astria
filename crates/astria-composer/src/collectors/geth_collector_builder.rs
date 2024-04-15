use astria_core::sequencer::v1::RollupId;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::{
    collectors::geth::Status,
    executor,
};

pub(crate) struct GethCollectorBuilder {
    pub(crate) chain_name: String,
    pub(crate) url: String,
    pub(crate) executor_handle: executor::Handle,
    pub(crate) shutdown_token: CancellationToken,
}

impl GethCollectorBuilder {
    pub(crate) fn build(self) -> super::Geth {
        let Self {
            chain_name,
            url,
            executor_handle,
            shutdown_token,
        } = self;
        let (status, _) = watch::channel(Status::new());
        let rollup_id = RollupId::from_unhashed_bytes(&chain_name);
        info!(
            rollup_name = %chain_name,
            rollup_id = %rollup_id,
            "created new geth collector for rollup",
        );
        super::Geth {
            rollup_id,
            chain_name,
            executor_handle,
            status,
            url,
            shutdown_token,
        }
    }
}
