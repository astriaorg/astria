use std::collections::HashMap;

use astria_core::{
    generated::sequencerblock::v1::sequencer_service_client::SequencerServiceClient,
    primitive::v1::{
        asset,
        RollupId,
    },
};
use astria_eyre::eyre::{
    self,
    OptionExt as _,
    WrapErr as _,
};
use tokio_util::{
    sync::CancellationToken,
    task::JoinMap,
};
use tonic::transport::Endpoint;
use tracing::{
    info,
    instrument,
};

use super::{
    Bundle,
    Handle,
    Id,
    SequencerKey,
};
use crate::flatten_result;

pub(crate) struct Builder {
    pub(crate) metrics: &'static crate::Metrics,
    pub(crate) shutdown_token: CancellationToken,

    /// The gRPC endpoint for the sequencer service used by auctions.
    pub(crate) sequencer_grpc_endpoint: String,
    /// The ABCI endpoint for the sequencer service used by auctions.
    pub(crate) sequencer_abci_endpoint: String,
    /// The amount of time to run the auction timer for.
    pub(crate) latency_margin: std::time::Duration,
    /// The private key used to sign sequencer transactions.
    pub(crate) sequencer_private_key_path: String,
    /// The prefix for the address used to sign sequencer transactions
    pub(crate) sequencer_address_prefix: String,
    /// The denomination of the fee asset used in the sequencer transactions
    pub(crate) fee_asset_denomination: asset::Denom,
    /// The chain ID for sequencer transactions
    pub(crate) sequencer_chain_id: String,
    /// The rollup ID for the `RollupDataSubmission`s with auction results
    pub(crate) rollup_id: String,
}

impl Builder {
    pub(crate) fn build(self) -> eyre::Result<Manager> {
        let Self {
            metrics,
            shutdown_token,
            sequencer_grpc_endpoint,
            sequencer_abci_endpoint,
            latency_margin,
            fee_asset_denomination,
            rollup_id,
            sequencer_private_key_path,
            sequencer_address_prefix,
            sequencer_chain_id,
        } = self;

        let sequencer_key = SequencerKey::builder()
            .path(sequencer_private_key_path)
            .prefix(sequencer_address_prefix)
            .try_build()
            .wrap_err("failed to load sequencer private key")?;
        info!(address = %sequencer_key.address(), "loaded sequencer signer");

        let sequencer_grpc_uri: tonic::transport::Uri = sequencer_grpc_endpoint
            .parse()
            .wrap_err("failed to parse sequencer grpc endpoint as URI")?;
        let sequencer_grpc_client =
            SequencerServiceClient::new(Endpoint::from(sequencer_grpc_uri).connect_lazy());

        let sequencer_abci_client =
            sequencer_client::HttpClient::new(sequencer_abci_endpoint.as_str())
                .wrap_err("failed constructing sequencer abci client")?;

        let rollup_id = RollupId::from_unhashed_bytes(&rollup_id);

        Ok(Manager {
            metrics,
            shutdown_token,
            sequencer_grpc_client,
            sequencer_abci_client,
            latency_margin,
            running_auctions: JoinMap::new(),
            auction_handles: HashMap::new(),
            sequencer_key,
            fee_asset_denomination,
            sequencer_chain_id,
            rollup_id,
        })
    }
}

pub(crate) struct Manager {
    metrics: &'static crate::Metrics,
    shutdown_token: CancellationToken,
    sequencer_grpc_client: SequencerServiceClient<tonic::transport::Channel>,
    sequencer_abci_client: sequencer_client::HttpClient,
    latency_margin: std::time::Duration,
    running_auctions: JoinMap<Id, eyre::Result<()>>,
    auction_handles: HashMap<Id, Handle>,
    // TODO: hold the bundle stream here?
    sequencer_key: SequencerKey,
    fee_asset_denomination: asset::Denom,
    sequencer_chain_id: String,
    rollup_id: RollupId,
}

impl Manager {
    #[instrument(skip(self))]
    pub(crate) fn new_auction(&mut self, auction_id: Id) {
        let (handle, auction) = super::Builder {
            metrics: self.metrics,
            shutdown_token: self.shutdown_token.child_token(),
            sequencer_grpc_client: self.sequencer_grpc_client.clone(),
            sequencer_abci_client: self.sequencer_abci_client.clone(),
            latency_margin: self.latency_margin,
            auction_id,
            sequencer_key: self.sequencer_key.clone(),
            fee_asset_denomination: self.fee_asset_denomination.clone(),
            sequencer_chain_id: self.sequencer_chain_id.clone(),
            rollup_id: self.rollup_id,
        }
        .build();

        // spawn and save handle
        self.running_auctions.spawn(auction_id, auction.run());
        self.auction_handles.insert(auction_id, handle);
    }

    pub(crate) fn abort_auction(&mut self, auction_id: Id) -> eyre::Result<()> {
        // TODO: this should return an option in case the auction returned before being aborted
        let handle = self
            .auction_handles
            .get_mut(&auction_id)
            .ok_or_eyre("unable to get handle for the given auction")?;

        handle.abort().expect("should only abort once per auction");
        Ok(())
    }

    #[instrument(skip(self))]
    pub(crate) fn start_timer(&mut self, auction_id: Id) -> eyre::Result<()> {
        let handle = self
            .auction_handles
            .get_mut(&auction_id)
            .ok_or_eyre("unable to get handle for the given auction")?;

        handle
            .start_timer()
            .expect("should only start timer once per auction");

        Ok(())
    }

    #[instrument(skip(self))]
    pub(crate) fn start_processing_bids(&mut self, auction_id: Id) -> eyre::Result<()> {
        let handle = self
            .auction_handles
            .get_mut(&auction_id)
            .ok_or_eyre("unable to get handle for the given auction")?;

        handle
            .start_processing_bids()
            .expect("should only start processing bids once per auction");
        Ok(())
    }

    pub(crate) fn try_send_bundle(&mut self, auction_id: Id, bundle: Bundle) -> eyre::Result<()> {
        self.auction_handles
            .get_mut(&auction_id)
            .ok_or_eyre("unable to get handle for the given auction")?
            .try_send_bundle(bundle)
            .wrap_err("failed to add bundle to auction")
    }

    pub(crate) async fn join_next(&mut self) -> Option<(Id, eyre::Result<()>)> {
        if let Some((auction_id, result)) = self.running_auctions.join_next().await {
            // TODO: get rid of this expect?
            self.auction_handles
                .remove(&auction_id)
                .expect("unable to get handle for the given auction");

            Some((auction_id, flatten_result(result)))
        } else {
            None
        }
    }
}
