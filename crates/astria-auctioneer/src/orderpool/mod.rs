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

use std::{
    future::Future,
    sync::Arc,
};

use alloy_consensus::Transaction as _;
use alloy_eips::Encodable2718 as _;
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
    OptionExt as _,
    WrapErr as _,
};
use futures::FutureExt as _;
use tokio::{
    select,
    sync::{
        oneshot,
        watch,
    },
    task::JoinHandle,
};
use tokio_util::{
    sync::CancellationToken,
    task::JoinMap,
};
use tracing::{
    field::display,
    info,
    instrument,
    warn,
    Level,
    Span,
};
use uuid::Uuid;

pub(crate) mod channel;
mod in_memory;
pub(crate) mod rpc;

pub(crate) use channel::{
    ForCancellation,
    ForOrder,
    Request,
    Response,
    Sender,
};
pub(crate) use in_memory::{
    InsertedOrReplaced,
    RemovedOrNotFound,
};

use crate::{
    auctioneer::{
        auction,
        Bidpipe,
    },
    bundle::{
        Bundle,
        Transaction,
    },
    OptionalExt as _,
};

#[derive(Debug, Clone)]
pub(crate) enum Order {
    New(Arc<crate::bundle::Bundle>),
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
    uuid: Uuid,
    timestamp: jiff::Timestamp,
}

impl Cancellation {
    pub(crate) fn new(uuid: Uuid) -> Self {
        Self {
            uuid,
            timestamp: jiff::Timestamp::now(),
        }
    }
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
                        auction_id_to_submitted_bundle: JoinMap::new(),
                        bundle_storage: in_memory::Storage::new(),
                        eth_client,
                        requests: rx,
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
    bundle_storage: in_memory::Storage,
    /// A map of all actively running simulations for given auctions.
    /// This does not expect a return value and only exists to report panics.
    auction_id_to_simulations: JoinMap<crate::auctioneer::auction::Id, ()>,
    /// A map of the bundle that ended up being submitted for a given auction.
    auction_id_to_submitted_bundle: JoinMap<
        crate::auctioneer::auction::Id,
        Result<(Uuid, jiff::Timestamp), tokio::sync::oneshot::error::RecvError>,
    >,
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
                self.handle_auction_changed(auction_change)?;
            }

            request = self.requests.recv() => {
                self.handle_request(request)?;
            }

            Some((auction_id, bundle_submitted)) = self.auction_id_to_submitted_bundle.join_next() => {
                let bundle_submitted =  bundle_submitted
                    .expect(
                        "should not panic because the task is just a oneshot rx; \
                        if it does then something is very wrong in tokio::sync::oneshot"
                    );
                self.handle_bundle_submitted(
                    auction_id,
                    bundle_submitted
                );
            }
        );
        Ok(())
    }

    fn handle_auction_changed(
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

    fn handle_bundle_submitted(
        &self,
        auction_id: auction::Id,
        res: Result<(Uuid, jiff::Timestamp), oneshot::error::RecvError>,
    ) {
        let _ = self.handle_bundle_submitted_impl(auction_id, res);
    }

    #[instrument(
        name = "handle_submitted_bundle",
        skip_all,
        fields(
            %auction_id,
            uuid = tracing::field::Empty,
            timestamp = tracing::field::Empty,
        ),
        err(level = Level::INFO, Display),
    )]
    fn handle_bundle_submitted_impl(
        &self,
        auction_id: auction::Id,
        res: Result<(Uuid, jiff::Timestamp), oneshot::error::RecvError>,
    ) -> eyre::Result<()> {
        let (uuid, timestamp) = res.wrap_err(
            "sender was dropped before receiving a winning bundle; this might be because there \
             was no winner to submit",
        )?;
        {
            let span = Span::current();
            span.record("uuid", display(uuid));
            span.record("timestamp", display(timestamp));
        }
        match self.bundle_storage.remove(uuid, timestamp) {
            in_memory::RemovedOrNotFound::Removed(bundle) => info!(
                bundle_hash = %bundle.hash(),
                "removed bundle from storage",
            ),
            in_memory::RemovedOrNotFound::NotFound(_) => {
                info!("could not find bundle with given UUID")
            }
            in_memory::RemovedOrNotFound::Aborted {
                in_storage_bundle, ..
            } => info!(
                    in_storage_timestamp = %in_storage_bundle.timestamp(),
                    "did not remove bundle because timestamp was older than stored"),
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

    /// Removes an bundle from the pool, if found.
    ///
    /// This method only removes a bundle identified by `cancellation.uuid`, if
    /// `bundle.timestamp` is older than `cancellation.timestamp`.
    ///
    /// Additionally, care is taken that the inverse indices are also only updated
    /// if the `bundle.timestamp` matches the timestamp stored in that index.
    #[instrument(skip_all, fields(%uuid, %timestamp))]
    fn process_order_cancellation(
        &self,
        Cancellation {
            uuid,
            timestamp,
        }: Cancellation,
        to_requester: tokio::sync::oneshot::Sender<Response>,
    ) {
        let response = ForCancellation {
            uuid,
            timestamp,
            action: self.bundle_storage.remove(uuid, timestamp),
        }
        .into();
        if let Err(_error) = to_requester.send(response) {
            // XXX: the error is just the sent item; it does not contain any useful info so we
            // ignore it.
            warn!("could not send response to requester because channel was already dropped");
        }
    }

    /// Inserts a new bundle into the pool or replaces a previous bundle at `bundle.uuid`.
    #[instrument(skip_all, fields(
        uuid = %bundle.uuid(),
        bundle_hash = %bundle.hash(),
    ))]
    fn process_new_order(
        &self,
        bundle: Arc<Bundle>,
        to_requester: tokio::sync::oneshot::Sender<Response>,
    ) {
        let response = ForOrder {
            uuid: *bundle.uuid(),
            action: self.bundle_storage.insert_or_replace(bundle),
        }
        .into();

        if let Err(_error) = to_requester.send(response) {
            // XXX: the error is just the sent item; it does not contain any useful info so we
            // ignore it.
            warn!("could not send response to requester because channel was already dropped");
        }
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
        for (uuid, bundle) in self.bundle_storage.pin().iter() {
            // TODO: only simulate those bundles that are valid at
            // bid_pipe.optimistic_block_number. This would also be
            // a good place to evict them (or martk them for eviction?).
            simulations.spawn(
                (*uuid, *bundle.timestamp()),
                simulate_and_estimate_bid(self.eth_client.clone(), bundle.clone()),
            );
        }

        let (notify_submitted_tx, notify_submitted_rx) = tokio::sync::oneshot::channel();
        self.auction_id_to_submitted_bundle
            .spawn(bid_pipe.auction_id, notify_submitted_rx);
        self.auction_id_to_simulations.spawn(
            bid_pipe.auction_id,
            SimulationsForAuction {
                bid_pipe,
                simulations,
                notify_orderpool: Some(notify_submitted_tx),
            }
            .run(),
        )
    }
}

struct SimulationsForAuction {
    bid_pipe: Bidpipe,
    simulations: JoinMap<(Uuid, jiff::Timestamp), Result<ProcessedSimulation, SimulationError>>,
    notify_orderpool: Option<tokio::sync::oneshot::Sender<(Uuid, jiff::Timestamp)>>,
}

impl SimulationsForAuction {
    async fn run(mut self) {
        let mut uuid_to_notify = JoinMap::new();
        loop {
            select!(
                biased;

                Some(((uuid, timestamp), fut_out)) = uuid_to_notify.join_next(),
                if self.notify_orderpool.is_some()
                => {
                    // XXX: Abort and detach all to forever inactivate this select-arm
                    uuid_to_notify.abort_all();
                    uuid_to_notify.detach_all();
                    fut_out.expect("the task spawned here is just awaiting a tokio Notify; this should never panic and if it does something very bad is going on");

                    if let Err((uuid, timestamp)) =
                        self
                        .notify_orderpool
                        .take()
                        .expect("in a select arm that asserts that the field is set")
                        .send((uuid, timestamp))
                    {
                        warn!(%uuid, %timestamp, "tried notifying orderpool of the bundle submitted by the auction, but the channel was already closed");
                    }
                }


                Some(((uuid, timestamp), sim_res)) = self.simulations.join_next() =>
                {
                    // TODO: report the time it took for the simulation to respond.
                    // we can probably do this by instrumenting the RPC future.
                    let sim_result = match sim_res {
                        Ok(Ok(success)) => success,
                        Ok(Err(_error)) => unimplemented!("report simulation error return value"),
                        Err(_error) => unimplemented!("report panicked simulation"),
                    };
                    // TODO: at this point there needs to be some kind of feedback mechanism with the
                    // actual orderpool to evict bundles that failed simulation (taking into consideration
                    // the reverted_txs and dropped_txs fields) so that bad bundles don't stay in the
                    // orderpool indefinitely.

                    // TODO: probably report a failure explicitly so that the logs can be grepped for those
                    // auctions that returned after an auction was cancelled/ended.
                    match self.bid_pipe.send(
                        sim_result.total_fee,
                        sim_result
                            .transactions_considered
                            .into_iter()
                            .map(|tx| tx.inner.encoded_2718())
                            .map(bytes::Bytes::from)
                            .collect(),
                    ) {
                        Ok(notify) => {
                            if self.notify_orderpool.is_some() {
                                uuid_to_notify.spawn((uuid, timestamp), async move { notify.notified().await });
                            }
                        }
                        Err(_error) => todo!("report that the simulation is already done,"),
                    };
                }
            )
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum SimulationError {
    #[error(
        "one block was supplied to eth_simulateV1 for simulation, but its response contained no \
         simulated blocks"
    )]
    NoSimulatedBlocks,
    #[error("requested full transactions be returned by eth_simulateV1, but got `{actual}`")]
    NotFullTransactions { actual: &'static str },
    #[error(
        "eth_simulateV1 returned `{transactions}` number of transactions but `{sim_results}` \
         simulation results"
    )]
    NumberOfSimulationResultsDoesNotMatchTransactions {
        transactions: usize,
        sim_results: usize,
    },
    #[error("the eth_simulateV1 RPC failed")]
    Rpc(#[from] alloy::transports::RpcError<alloy::transports::TransportErrorKind>),
    #[error(
        "one block was supplied to eth_simualteV1 for simulation, but its response contained \
         `{actual}`"
    )]
    TooManySimulatedBlocks { actual: usize },
    #[error(
        "the transaction with hash `{tx_hash}` failed with error code `{}` and message `{}`",
        call.error.as_ref().map(|err| &err.code).display_or("<not set>"),
        call.error.as_ref().map(|err| &err.message).display_or("<not set>"),
    )]
    TransactionReverted { call: SimCallResult, tx_hash: B256 },
}

impl SimulationError {
    fn not_full_transactions<T>(
        returned_transactions: &alloy_rpc_types_eth::BlockTransactions<T>,
    ) -> Self {
        Self::NotFullTransactions {
            actual: match returned_transactions {
                alloy_rpc_types_eth::BlockTransactions::Full(_) => "full",
                alloy_rpc_types_eth::BlockTransactions::Hashes(_) => "hashes",
                alloy_rpc_types_eth::BlockTransactions::Uncle => "uncle,",
            },
        }
    }

    fn transaction_reverted(call: SimCallResult, tx_hash: B256) -> Self {
        Self::TransactionReverted {
            call,
            tx_hash,
        }
    }
}

/// The validated response of `eth_simulateV1`.
struct AssertedSimulationResponse {
    calls: Vec<SimCallResult>,
    header: alloy_rpc_types_eth::Header,
    transactions: Vec<alloy_rpc_types_eth::Transaction>,
}

impl AssertedSimulationResponse {
    fn try_from_rpc_response(
        mut resp: Vec<SimulatedBlock<alloy_rpc_types_eth::Block>>,
    ) -> Result<Self, SimulationError> {
        let simulated_block = match resp.as_slice() {
            [_] => resp
                .pop()
                .expect("in a match arm that asserts there being exactly one element"),
            [] => return Err(SimulationError::NoSimulatedBlocks),
            [xs @ ..] => {
                return Err(SimulationError::TooManySimulatedBlocks {
                    actual: xs.len(),
                })
            }
        };

        let alloy_rpc_types_eth::BlockTransactions::Full(transactions) =
            simulated_block.inner.transactions
        else {
            return Err(SimulationError::not_full_transactions(
                &simulated_block.inner.transactions,
            ));
        };
        if transactions.len() != simulated_block.calls.len() {
            return Err(
                SimulationError::NumberOfSimulationResultsDoesNotMatchTransactions {
                    transactions: transactions.len(),
                    sim_results: simulated_block.calls.len(),
                },
            );
        }
        Ok(AssertedSimulationResponse {
            calls: simulated_block.calls,
            header: simulated_block.inner.header,
            transactions,
        })
    }

    fn len(&self) -> usize {
        debug_assert_eq!(
            self.transactions.len(),
            self.calls.len(),
            "invariant; asserted simulation response constructed with try_from_rpc_response must
            have the same number of calls as well as returned transactions"
        );
        self.transactions.len()
    }

    fn into_iter(self) -> impl Iterator<Item = (alloy_rpc_types_eth::Transaction, SimCallResult)> {
        self.transactions.into_iter().zip(self.calls)
    }
}

/// The simulated transactions that did not revert and were not dropped and their
/// summed total gas used.
// NOTE: This is using the transactions returned by eth_simulateV1 for now, not the
// transactions passed in via the order. These should be equivalent, but we are relying
// on geth not to do anything untoward.
//
// TODO: It might be an optimization to use the raw transactions from the order/bundle so that
// there is no need to re-encode these transactions.
#[allow(dead_code)]
struct ProcessedSimulation {
    total_fee: u128,
    transactions_considered: Vec<alloy_rpc_types_eth::Transaction>,
    source_bundle: Arc<Bundle>,
}

async fn simulate_and_estimate_bid(
    client: DynProvider,
    bundle: Arc<Bundle>,
) -> Result<ProcessedSimulation, SimulationError> {
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

    // TODO: wrap this in a timeout.
    let sim_response =
        AssertedSimulationResponse::try_from_rpc_response(client.simulate(&payload).await?)?;

    let base_fee_per_gas = sim_response.header.base_fee_per_gas;
    let mut transactions_considered = Vec::with_capacity(sim_response.len());
    let mut total_fee = 0u128;
    // NOTE: This is just iterating the simulation response without any extra verification of
    // whether this matches the bundles we have passed in. Should we also do that or just trust
    // it?
    for (tx, call) in sim_response.into_iter() {
        // TODO: We consider a simulated transaction with false call.status to be "reverted". Is
        //       this correct?
        //
        // TODO: for comparison, rbuilder [1] is checking the execution of each single transaction
        // directly, looking at both `receipt.success` and the error path at the same time.
        // It's not clear in how far we can do that with the `eth_simulateV1` API, so we
        // just consider `call.status`.
        //
        // [1]: https://github.com/flashbots/rbuilder/blob/e67726d5e285183d0f9cc4e850e787740304bf4c/crates/rbuilder/src/building/order_commit.rs#L684-L719
        //
        // TODO: is looking at call.status enough? What about the error log in call.error?
        if !call.status {
            // TODO: The conditions under which failed/reverted transactions are dropped from bid
            // submission are completely clear. We will follow Rbuilder, which performs these
            // branches (1 before 2):
            //
            // 1. if a transactions is in `dropping_tx_hashes`: rollback execution, go to the next
            //    transactions.
            // 2. if a transaction is *not* in `reverting_tx_hashes`: fail bundle building by
            //    returning a TransactionReverted error.
            //
            // This is confusing in as far as titanbuilder and beaver builder have the following to
            // say:
            //
            // On `dropping_tx_hashes`:
            // Titan> A list of tx hashes that are allowed to be discarded, but may not revert
            // Beaver> A list of transaction hashes contained in the bundle, that can be allowed to
            // be removed from your bundle if it's deemed useful (but not revert).
            //
            // On `reverting_tx_hashes`:
            // Titan> A list of tx hashes that are allowed to revert or be discarded
            // Beaver> A list of transaction hashes contained in the bundle, that can be allowed to
            // revert, or be removed from your bundle if it's deemed useful.
            //
            //
            if bundle.is_dropping(tx.inner.tx_hash()) {
                continue;
            }
            if !bundle.is_reverting(tx.inner.tx_hash()) {
                return Err(SimulationError::transaction_reverted(
                    call,
                    *tx.inner.tx_hash(),
                ));
            }

            total_fee = total_fee.saturating_add(
                u128::from(call.gas_used).saturating_mul(tx.effective_gas_price(base_fee_per_gas)),
            );

            transactions_considered.push(tx);
        }
    }
    Ok(ProcessedSimulation {
        source_bundle: bundle,
        total_fee,
        transactions_considered,
    })
}
