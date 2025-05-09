//! A minimal mocked geth jsonrpc server for black box testing.
//!
//! At the moment, this mock supports the `eth_subscribe`
//! RPC with the parameters `["newPendingTransaction", true]` to allow
//! subscribing to full new pending transactions using the
//! [`ethers::providers::Middleware::subscribe_full_pendings_txs`] high
//! level abstraction. The mock also supports the `txpool_content` RPC to allow for observing the
//! entire contents of the mocked tx pool. Notably, the mock does **not** update the tx pool when
//! new transactions are pushed. The intended use of the mock tx pool is to be checked only upon
//! startup of the client.
//!
//! The intended use for the mock is to:
//!
//! 1. spawn a server with [`Geth::spawn`];
//! 2. establish a websocket connection using its local socket address at [`MockGet::local_addr`],
//!    for example with [`Provider::<Ws>::connect`];
//! 3. call the `txpool_content` RPC using [`Middleware::txpool_content`] to obtain currently
//!    pending transactions
//! 4. subscribe to its mocked `eth_subscribe` JSONRPC using
//!    [`Middleware::subscribe_full_pending_txs`];
//! 5. push new transactions into the server using [`Geth::push_tx`], which will subsequently be
//!    sent to all subscribers and can be observed by the client.
//!
//! # Examples
//!
//! ```
//! # tokio_test::block_on( async {
//! use std::{
//!     collections::BTreeMap,
//!     time::Duration,
//! };
//!
//! use astria_test_utils::mock::Geth;
//! use ethers::{
//!     providers::{
//!         Middleware as _,
//!         Provider,
//!         StreamExt as _,
//!         Ws,
//!     },
//!     types::{
//!         Transaction,
//!         H160,
//!     },
//! };
//!
//! println!("connecting!!");
//! let mut pending_txs = BTreeMap::new();
//! let mut account_txs = BTreeMap::new();
//! account_txs.insert("0".to_string(), Transaction::default());
//! pending_txs.insert(H160::zero(), account_txs);
//! let mock_geth = Geth::spawn_with_pending_txs(pending_txs.clone()).await;
//! let server_addr = mock_geth.local_addr();
//!
//! let geth_client = Provider::<Ws>::connect(format!("ws://{server_addr}"))
//!     .await
//!     .expect("client should be able to connect to local ws server");
//!
//! let txpool_content = geth_client.txpool_content().await.unwrap();
//! assert_eq!(txpool_content.pending, pending_txs);
//!
//! tokio::spawn(async move {
//!     loop {
//! #       // FIXME: remove the sleep. at the moment this doc tests only passes
//! #       //        when explicitly sleeping. Why?
//!         tokio::time::sleep(Duration::from_secs(1)).await;
//!         let r = mock_geth.push_tx(Transaction::default().into());
//!     }
//! });
//!
//! let mut new_txs = geth_client
//!     .subscribe_full_pending_txs()
//!     .await
//!     .unwrap()
//!     .take(3);
//! while let Some(new_tx) = new_txs.next().await {
//!     assert_eq!(new_tx, Transaction::default());
//! }
//! # });
//! ```

use std::{
    collections::BTreeMap,
    net::SocketAddr,
};

#[expect(
    clippy::module_name_repetitions,
    reason = "naming is helpful for clarity here"
)]
pub use __rpc_traits::GethServer;
use ethers::types::{
    Transaction,
    TxpoolContent,
    H160,
};
use jsonrpsee::{
    core::{
        async_trait,
        SubscriptionResult,
    },
    server::IdProvider,
    types::{
        ErrorObjectOwned,
        SubscriptionId,
    },
    PendingSubscriptionSink,
};
use tokio::sync::broadcast::{
    channel,
    error::SendError,
    Sender,
};
use tracing::instrument;

#[derive(Debug)]
pub struct RandomU256IdProvider;

impl IdProvider for RandomU256IdProvider {
    fn next_id(&self) -> SubscriptionId<'static> {
        use ethers::types::U256;
        use impl_serde::serialize::to_hex;
        use rand::RngCore as _;

        let mut rng = rand::thread_rng();
        let mut raw_u256 = [0u8; 32];
        rng.fill_bytes(&mut raw_u256);
        // Just in case, convert to u256 and back to big endian because parity's u256
        // implementation does some extra complex transformations.
        let u256 = U256::from(raw_u256);
        let mut byte_repr = [0u8; 32];
        u256.to_big_endian(&mut byte_repr);
        let u256_ser = to_hex(&byte_repr, true);
        SubscriptionId::from(u256_ser)
    }
}

mod __rpc_traits {
    use ethers::types::TxpoolContent;
    use jsonrpsee::{
        core::SubscriptionResult,
        proc_macros::rpc,
        types::ErrorObjectOwned,
    };
    // The mockserver has to be able to handle an `eth_subscribe` RPC with parameters
    // `"newPendingTransactions"` and `true`
    #[rpc(server)]
    pub trait Geth {
        #[subscription(name = "eth_subscribe", item = Transaction, unsubscribe = "eth_unsubscribe")]
        async fn eth_subscribe(&self, target: String, full_txs: Option<bool>)
            -> SubscriptionResult;

        #[method(name = "txpool_content")]
        async fn txpool_content(&self) -> Result<TxpoolContent, ErrorObjectOwned>;

        #[method(name = "net_version")]
        async fn net_version(&self) -> Result<String, ErrorObjectOwned>;
    }
}

#[derive(Clone, Debug)]
pub enum SubscriptionCommand {
    Abort,
    Send(Transaction),
}

impl From<Transaction> for SubscriptionCommand {
    fn from(transaction: Transaction) -> Self {
        Self::Send(transaction)
    }
}

#[expect(
    clippy::module_name_repetitions,
    reason = "naming is helpful for clarity here"
)]
pub struct GethImpl {
    starting_tx_pool: TxpoolContent,
    command: Sender<SubscriptionCommand>,
}

#[async_trait]
impl GethServer for GethImpl {
    async fn eth_subscribe(
        &self,
        pending: PendingSubscriptionSink,
        subscription_target: String,
        full_txs: Option<bool>,
    ) -> SubscriptionResult {
        use jsonrpsee::server::SubscriptionMessage;
        tracing::debug!("received eth_subscription request");

        assert_eq!(
            ("newPendingTransactions", Some(true)),
            (&*subscription_target, full_txs),
            "the mocked geth server only supports the `eth_subscribe` RPC with
            parameters [\"newPendingTransaction\", true]",
        );
        let sink = pending.accept().await?;
        let mut rx = self.command.subscribe();
        loop {
            tokio::select!(
                biased;
                () = sink.closed() => break Err("subscription closed by client".into()),
                Ok(cmd) = rx.recv() => {
                    match cmd {
                        SubscriptionCommand::Abort => {
                            tracing::debug!("abort command received; exiting eth_subscription");
                            break Err("mock received abort command".into());
                        }
                        SubscriptionCommand::Send(tx) => {
                            let () = sink.send(SubscriptionMessage::from_json(&tx)?).await?;
                        }
                    }
                }
            );
        }
    }

    #[instrument(skip_all, level = "trace")]
    async fn txpool_content(&self) -> Result<TxpoolContent, ErrorObjectOwned> {
        Ok(self.starting_tx_pool.clone())
    }

    async fn net_version(&self) -> Result<String, ErrorObjectOwned> {
        Ok("mock_geth".into())
    }
}

/// A mocked geth server for subscribing to new transactions.
///
/// Allows for explicitly pushing transactions to subscribed clients.
pub struct Geth {
    /// The local address to which the mocked jsonrpc server is bound.
    local_addr: SocketAddr,
    /// A channel over which new transactions can be inserted into the mocked
    /// server so that they are forwarded to a client that subscribed to new
    /// pending transactions over websocket.
    command: Sender<SubscriptionCommand>,
    _server_task_handle: tokio::task::JoinHandle<()>,
}

impl Geth {
    /// Spawns a new mocked geth server.
    ///
    /// # Panics
    ///
    /// Panics if the server fails to start.
    pub async fn spawn() -> Self {
        Self::spawn_with_pending_txs(BTreeMap::default()).await
    }

    /// Spawns a new mocked geth server with the given pending transactions in the tx pool.
    ///
    /// # Panics
    ///
    /// Panics if the server fails to start.
    pub async fn spawn_with_pending_txs(
        pending_txs: BTreeMap<H160, BTreeMap<String, Transaction>>,
    ) -> Self {
        use jsonrpsee::server::Server;
        let server = Server::builder()
            .ws_only()
            .set_id_provider(RandomU256IdProvider)
            .build("127.0.0.1:0")
            .await
            .expect("should be able to start a jsonrpsee server bound to a 0 port");
        let local_addr = server
            .local_addr()
            .expect("server should have a local addr");
        let (command, _) = channel(256);
        let mock_geth_impl = GethImpl {
            starting_tx_pool: TxpoolContent {
                pending: pending_txs,
                queued: BTreeMap::default(),
            },
            command: command.clone(),
        };
        let handle = server.start(mock_geth_impl.into_rpc());
        let server_task_handle = tokio::spawn(handle.stopped());
        Self {
            local_addr,
            command,
            _server_task_handle: server_task_handle,
        }
    }

    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Sends an Abort command to all subscription tasks, causing them to exit and close the
    /// subscriptions.
    ///
    /// # Errors
    ///
    /// Returns the same error as tokio's [`Sender::send`].
    pub fn cancel_subscriptions(&self) -> Result<usize, SendError<SubscriptionCommand>> {
        self.command.send(SubscriptionCommand::Abort)
    }

    /// Push a new transaction into the mocked geth server.
    ///
    /// If composer is subscribed to the mocked geth server using its
    /// `eth_subscribe` JSONRPC, the transaction will be immediately
    /// forwarded to it.
    ///
    /// # Errors
    ///
    /// Returns the same error as tokio's [`Sender::send`].
    pub fn push_tx(&self, tx: Transaction) -> Result<usize, SendError<SubscriptionCommand>> {
        self.command.send(tx.into())
    }
}
