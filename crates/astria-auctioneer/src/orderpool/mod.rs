//! The orderpool receives and executes order requests.
//!
//! The orderpool is a long lived task that accepts two kinds of messages:
//!
//! 1. a request to insert or cancel an order in the pool (an order being a collection of
//!    transactions, a.k.a. a bundle);
//! 2. a bid pipeline, which contains the parent hash of a new optimistically executed block as well
//!    as the channel to a new auction
//!
//! Order requests can be:
//!
//! + new orders, which are essentially collections of transactions ("bundles" in eth parlance);
//! + replacement orders, which are new orders but also provide the UUID of an existing order,
//!   overwriting it;
//! + cancellations, which provide the UUID of an existing order and cause the orderpool to delete
//!   it.

use std::future::Future;

use alloy_primitives::B256;
use alloy_provider::{
    DynProvider,
    Provider as _,
};
use alloy_rpc_types_eth::simulate::{
    SimCallResult,
    SimulatedBlock,
};
use astria_eyre::eyre::{
    self,
    OptionExt,
    WrapErr as _,
};
use channel::{
    Request,
    Response,
};
use futures::FutureExt as _;
use tokio::{
    select,
    sync::watch,
    task::JoinHandle,
};
use tokio_util::{
    sync::CancellationToken,
    task::JoinMap,
};
use tracing::instrument;
use uuid::Uuid;

pub(crate) mod channel;

pub(crate) use channel::Sender;

use crate::{
    auctioneer::Bidpipe,
    bundle::{
        alloy_to_bytes_bytes,
        Bundle,
        Transaction,
    },
};

#[derive(Debug, Clone)]
pub(crate) enum Order {
    New(crate::bundle::Bundle),
    Cancel(Cancellation),
}

/// A request for cancelling a live order.
// TODO: what other information is necessary?
// Information the rbuilder uses:
// + replacement_nonce
// + first_seen_at
// + signing_address (the bundle signer)
//
// they construct a "sequence number" from the replacement
// nonce or first_seen_at (if the nonce is absent) to act
// as a tie breaker.
//
// they also use a field `signing_address` to construct
// a key (sigining_address, uuid) as the unique identifier
// of the order to cancel.
//
// Not using a signing address seems to be a bad idea, because
// it allows attackers to just submit a UUID to cancel arbitrary
// orders (i.e. we'd rely on obscurity).
//
// Supplying the signing address does not really eliminate this
// issue either.
//
// Would a fix be to just sign the UUID then? But in that case
// we should also have a signing address for the initial set of
// transactions that made up the order we now want to cancel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Cancellation {
    /// The UUID idenfiying the order.
    pub(crate) uuid: Uuid,
}

pub(crate) struct Orderpool {
    cancellation_token: CancellationToken,
    requests: channel::Sender,
    task: JoinHandle<Option<eyre::Result<()>>>,
}

impl Orderpool {
    /// Spawns the orderpool on the tokio runtime.
    pub(crate) fn spawn(
        cancellation_token: CancellationToken,
        active_auction: watch::Receiver<Option<crate::auctioneer::Bidpipe>>,
        eth_url: String,
    ) -> Self {
        let (tx, rx) = channel::new();
        let task = tokio::spawn(
            cancellation_token
                .clone()
                .run_until_cancelled_owned(async move {
                    let eth_client = alloy_provider::ProviderBuilder::new()
                        .connect(&eth_url)
                        .await
                        .wrap_err_with(|| {
                            format!("failed to connect to eth endpoint at `{eth_url}`")
                        })?
                        .erased();
                    Inner {
                        active_auction,
                        auction_id_to_simulations: JoinMap::new(),
                        eth_client,
                        requests: rx,
                        uuid_to_bundle: papaya::HashMap::new(),
                        bundle_hash_to_uuid: papaya::HashMap::new(),
                    }
                    .run()
                    .await
                }),
        );
        Self {
            cancellation_token,
            requests: tx,
            task,
        }
    }

    pub(crate) fn cancel(&self) -> () {
        self.cancellation_token.cancel()
    }

    pub(crate) fn sender(&self) -> &Sender {
        &self.requests
    }
}

impl Future for Orderpool {
    type Output = eyre::Result<()>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let res = match std::task::ready!(self.task.poll_unpin(cx)) {
            Ok(Some(Ok(())) | None) => Ok(()),
            Ok(Some(err @ Err(_))) => err.wrap_err("orderpool exited with an error"),
            Err(err) => Err(err).wrap_err("orderpool panicked"),
        };
        std::task::Poll::Ready(res)
    }
}

struct Inner {
    active_auction: watch::Receiver<Option<crate::auctioneer::Bidpipe>>,
    eth_client: DynProvider,
    requests: channel::Receiver,
    /// The collection of currently active bundles.
    uuid_to_bundle: papaya::HashMap<Uuid, crate::bundle::Bundle>,
    /// A reverse index mapping a bundle's hash to its UUID
    bundle_hash_to_uuid: papaya::HashMap<B256, Uuid>,
    /// A map of all actively running auctions. This does not expect a return
    /// value and only exists to report panics.
    auction_id_to_simulations: JoinMap<crate::auctioneer::auction::Id, ()>,
}

impl Inner {
    async fn run(mut self) -> eyre::Result<()> {
        loop {
            self.on_event().await?;
        }
    }

    async fn on_event(&mut self) -> eyre::Result<()> {
        select!(
            biased;

            auction_change = self.active_auction.changed() => {
                self.handle_auction_change(auction_change)?;
            }

            request = self.requests.recv() => {
                self.handle_request(request)?;
            }
        );
        Ok(())
    }

    fn handle_auction_change(
        &mut self,
        auction_changed: Result<(), tokio::sync::watch::error::RecvError>,
    ) -> eyre::Result<()> {
        auction_changed.wrap_err(
            "all senders of auction changes are dead; the orderpool can no longer receive \
             notifications of new auctions; exiting",
        )?;
        let new_auction = self.active_auction.borrow_and_update().clone();
        match new_auction {
            Some(bid_pipe) => {
                self.start_simulations_for_auction(bid_pipe);
            }
            None => {
                self.cancel_active_simulations();
            }
        }
        Ok(())
    }

    fn handle_request(&self, request: Option<Request>) -> eyre::Result<()> {
        let request = request.ok_or_eyre(
            "all senders of orderpool requests are dead; the orderpool can no longer receive \
             requests; exiting",
        )?;
        match request.order {
            Order::New(bundle) => self.process_new_order(bundle, request.to_requester),
            Order::Cancel(cancellation) => {
                self.process_order_cancellation(cancellation, request.to_requester)
            }
        }
        Ok(())
    }

    #[instrument(skip_all, fields(%cancellation.uuid))]
    fn process_order_cancellation(
        &self,
        cancellation: Cancellation,
        to_requester: tokio::sync::oneshot::Sender<Response>,
    ) {
        // XXX: this is potentially racy. We should timestamp the receipt of the cancellation
        // and only remove if the cancellation is newer than the bundle in the map. It's
        // still not perfect, but slightly better.
        let _ = self.uuid_to_bundle.pin().remove(&cancellation.uuid);
        // TODO: report if the requester (usually a jsonrpc request) went away; could point to some
        // lower level issues.
        let _ = to_requester.send(Response);
    }

    #[instrument(skip_all, fields(
        uuid = %bundle.uuid(),
        bundle_hash = %bundle.hash(),
    ))]
    fn process_new_order(
        &self,
        bundle: Bundle,
        to_requester: tokio::sync::oneshot::Sender<Response>,
    ) {
        let uuid_to_bundle = self.uuid_to_bundle.pin();
        let bundle_hash_to_uuid = self.bundle_hash_to_uuid.pin();

        // insert and update the reverse indices atomically
        //
        // XXX: this assumes that by updating the uuid_to_bundle map atomically,
        // this also provides guarantees about the inverse index having
        // atomic guarantees (in this case, that the reverse index is only updated
        // if and only if the actual map is also updated).
        //
        // TODO: is this actually the case? Can we be sure? We definitely must
        // avoid altering the inverse index in any other way.
        uuid_to_bundle.update(*bundle.uuid(), move |_| {
            let _ = bundle_hash_to_uuid.insert(*bundle.hash(), *bundle.uuid());
            bundle.clone()
        });

        // TODO: report if the requester (usually a jsonrpc request) went away; could point to some
        // lower level issues.
        let _ = to_requester.send(Response);
    }

    #[instrument(skip_all)]
    fn cancel_active_simulations(&self) {
        // TODO: what should cancellting active simulations entail? If they are not
        // done yet, they should be put on a timer and killed.
    }

    #[instrument(skip_all, fields(
        auction_id = %bid_pipe.auction_id,
        optimistic_block_hash = %bid_pipe.optimistic_block_hash,
        optimistic_block_number = %bid_pipe.optimistic_block_number,
    ))]
    fn start_simulations_for_auction(&mut self, bid_pipe: Bidpipe) {
        // TODO: assert that no simulations for bid_pipe.auction_id exist.
        let mut simulations = JoinMap::new();
        for (uuid, bundle) in self.uuid_to_bundle.pin().iter() {
            // TODO: only simulate those bundles that are valid at
            // bid_pipe.optimistic_block_number. This would also be
            // a good place to evict them (or martk them for eviction?).
            simulations.spawn(
                *uuid,
                simulate_and_estimate_bid(self.eth_client.clone(), bundle.clone()),
            );
        }

        self.auction_id_to_simulations.spawn(
            bid_pipe.auction_id,
            SimulationsForAuction {
                bid_pipe,
                simulations,
            }
            .run(),
        )
    }
}

struct SimulationsForAuction {
    bid_pipe: Bidpipe,
    simulations: JoinMap<Uuid, eyre::Result<(Bundle, u64)>>,
}

impl SimulationsForAuction {
    async fn run(mut self) {
        while let Some((_uuid, res)) = self.simulations.join_next().await {
            // TODO: report the time it took for the simulation to respond.
            // we can probably do this by instrumenting the RPC future.
            let (bundle, fee) = match res {
                Ok(Ok(success)) => success,
                Ok(Err(_error)) => unimplemented!("report simulation error return value"),
                Err(_error) => unimplemented!("report panicked simulation"),
            };
            // TODO: at this point there needs to be some kind of feedback mechanism with the
            // actual orderpool to evict bundles that failed simulation (taking into consideration
            // the reverted_txs and dropped_txs fields) so that bad bundles don't stay in the
            // orderpool indefinitely.

            // TODO: probably report this explicitly so that the logs can be grepped for those
            // auctions that returned after an auction was cancelled/ended.
            let _ = self.bid_pipe.send(
                fee,
                bundle
                    .into_raw_txs()
                    .into_iter()
                    .map(alloy_to_bytes_bytes)
                    .collect(),
            );
        }
    }
}

async fn simulate_and_estimate_bid(
    client: DynProvider,
    bundle: Bundle,
) -> eyre::Result<(Bundle, u64)> {
    use alloy_rpc_types_eth::simulate::{
        SimBlock,
        SimulatePayload,
    };

    // XXX: Setting everything to `true`. Seems wasteful. Probably not necessary.
    //
    // XXX: the eth_simulateV1 api allows for sending multiple block state calls at
    // once. I believe this does not trigger multiple simulations, just a single one
    // with each block being simulated after the other. This might be wrong though and
    // is worth double checking. Either way - we probably want to send a single bundle
    // always because we need to get each order's/bundle's simulation results ASAP.
    let payload = SimulatePayload {
        block_state_calls: vec![SimBlock {
            block_overrides: None,
            // XXX: Override the gas collector?
            state_overrides: None,
            calls: bundle
                .txs()
                .iter()
                .map(Transaction::to_transaction_request)
                .collect(),
        }],
        trace_transfers: true,
        validation: true,
        return_full_transactions: true,
    };
    // TODO: this should only contain one simulated block. We should probably
    // assert this, but later.
    let simulated_blocks = client
        .simulate(&payload)
        .await
        .wrap_err("simulation failed")?;
    Ok((bundle, gas_from_simulated_blocks(simulated_blocks)))
}

/// Calcaultes the total gas used from the eth_simulateV1 results.
///
/// This just adds up the `gas_used` field and is likely completely wrong.
/// It just acts as a placeholder for now.
fn gas_from_simulated_blocks(blocks: Vec<SimulatedBlock>) -> u64 {
    blocks
        .into_iter()
        .map(|block| block.calls.into_iter().map(|calls| calls.gas_used))
        .flatten()
        .fold(0u64, |acc, gas| acc.saturating_add(gas))
}
