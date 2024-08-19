use std::sync::Arc;

use astria_core::generated::composer::v1alpha1::{
    builder_bundle_relay_service_server::BuilderBundleRelayService,
    ToBBundleRequest,
    ToBBundleResponse,
};
use astria_eyre::eyre;

use crate::{
    executor,
    metrics::Metrics,
};

pub(crate) struct Relay {
    executor: executor::Handle,
    metrics: &'static Metrics,
}

impl Relay {
    pub(crate) fn new(executor: executor::Handle, metrics: &'static Metrics) -> Self {
        Self {
            executor,
            metrics,
        }
    }
}

#[async_trait::async_trait]
impl BuilderBundleRelayService for Relay {
    async fn get_bundle(
        self: Arc<Self>,
        request: tonic::Request<ToBBundleRequest>,
    ) -> eyre::Result<tonic::Response<ToBBundleResponse>, tonic::Status> {
        todo!()
    }
}
