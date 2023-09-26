// TODO: update these comments to reflect the actual code for the mock sequencer

//! A minimal mocked geth jsonrpc server for black box testing.
//!
//! At the moment, this mock only supports the `eth_subscribe`
//! RPC with the parameters `["newPendingTransaction", true]` to allow
//! subscribing to full new pending transactions using the
//! [`ethers::providers::Middleware::subscribe_full_pendings_txs`] high
//! level abstraction.
//!
//! The intended use for the mock is to:
//!
//! 1. spawn a server with [`MockGeth::spawn`];
//! 2. establish a websocket connection using its local socket address at [`MockGet::local_addr`],
//!    for example with [`Provider::<Ws>::connect`];
//! 3. subscribe to its mocked `eth_subscribe` JSONRPC using
//!    [`Middleware::subscribe_full_pending_txs`];
//! 4. push new transactions into the server using [`MockGeth::push_tx`], which will subsequently be
//!    sent to all suscribers and can be observed by the client.
//!
//! # Examples
//!
//! ```
//! # tokio_test::block_on( async {
//! use std::time::Duration;
//!
//! use ethers::{
//!     providers::{Middleware as _, Provider, StreamExt as _, Ws},
//!     types::Transaction,
//! };
//! use jsonrpsee_ethers::MockGeth;
//!
//! println!("connecting!!");
//! let mock_geth = MockGeth::spawn().await;
//! let server_addr = mock_geth.local_addr();
//!
//! tokio::spawn(async move {
//!     loop {
//! #       // FIXME: remove the sleep. at the moment this doc tests only passes when
//! #       //        when expclitly sleeping. Why?
//!         tokio::time::sleep(Duration::from_secs(1)).await;
//!         let r = mock_geth.push_tx(Transaction::default());
//!     }
//! })
//!
//! let geth_client = Provider::<Ws>::connect(format!("ws://{server_addr}"))
//!     .await
//!     .expect("client should be able to conenct to local ws server");
//! let mut new_txs = geth_client.subscribe_full_pending_txs()
//!     .await
//!     .unwrap()
//!     .take(3);
//! while let Some(new_tx) = new_txs.next().await {
//!     assert_eq!(new_tx, Transaction::default());
//! }
//! # });
//! ```

use std::net::SocketAddr;

use jsonrpsee::{
    core::{
        async_trait,
        RpcResult,
        SubscriptionResult,
    },
    proc_macros::rpc,
    server::{
        IdProvider,
        IntoSubscriptionCloseResponse,
        PendingSubscriptionSink,
        ServerBuilder,
        SubscriptionCloseResponse,
        SubscriptionMessage,
    },
    types::{
        ErrorObjectOwned,
        SubscriptionId,
    },
    ws_client::*,
    PendingSubscriptionSink,
};
use tendermint_rpc::{
    client::sync::unbounded,
    query::Query,
    utils::uuid_str,
    Error,
    Subscription,
};

#[rpc(server)]
pub trait Sequencer {
    #[subscription(name = "subscribe", item = String)]
    async fn subscribe(&self, query: Query) -> SubscriptionResult;
}

pub struct SequencerImpl {}

#[async_trait]
impl SequencerServer for SequencerImpl {
    async fn subscribe(
        &self,
        pending: PendingSubscriptionSink,
        query: Query,
    ) -> Result<Subscription, Error> {
        let id = uuid_str();
        let (_subs_tx, subs_rx) = unbounded();
        Ok(Subscription::new(id, query, subs_rx))
    }
}

pub struct MockSequencer {
    /// The local address to which the mocked jsonrpc server is bound.
    local_addr: SocketAddr,
    _server_task_handle: tokio::task::JoinHandle<()>,
}

impl MockSequencer {
    /// Spawns a new mocked geth server.
    /// # Panics
    /// Panics if the server fails to start.
    pub async fn spawn() -> Self {
        use jsonrpsee::server::Server;
        let server = Server::builder()
            .ws_only()
            .build("127.0.0.1:0")
            .await
            .expect("should be able to start a jsonrpsee server bound to a 0 port");
        let local_addr = server
            .local_addr()
            .expect("server should have a local addr");
        let mock_sequencer_impl = SequencerImpl {};
        let handle = server.start(mock_sequencer_impl.into_rpc());
        let server_task_handle = tokio::spawn(handle.stopped());
        Self {
            local_addr,
            _server_task_handle: server_task_handle,
        }
    }

    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}
