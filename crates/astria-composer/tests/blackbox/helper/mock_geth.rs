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

use ethers::types::Transaction;
use jsonrpsee::{
    core::{
        async_trait,
        SubscriptionResult,
    },
    proc_macros::rpc,
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

#[derive(Debug)]
pub struct RandomU256IdProvider;

impl IdProvider for RandomU256IdProvider {
    fn next_id(&self) -> SubscriptionId<'static> {
        use ethers::types::U256;
        use impl_serde::serialize::to_hex;
        use rand::RngCore as _;

        let mut rng = rand::thread_rng();
        let mut raw_u256 = [0u8; 32];
        (&mut rng).fill_bytes(&mut raw_u256);
        // Just in case, convert to u256 and back to big endian because parity's u256
        // implementation does some extra complex transformations.
        let u256 = U256::from(raw_u256);
        let mut byte_repr = [0u8; 32];
        u256.to_big_endian(&mut byte_repr);
        let u256_ser = to_hex(&byte_repr, true);
        SubscriptionId::from(u256_ser)
    }
}

// The mockserver has to be able to handle an `eth_subscribe` RPC with parameters
// `"newPendingTransactions"` and `true`
#[rpc(server)]
pub trait Geth {
    #[subscription(name = "eth_subscribe", item = Transaction, unsubscribe = "eth_unsubscribe")]
    async fn eth_subscribe(&self, target: String, full_txs: Option<bool>) -> SubscriptionResult;

    #[method(name = "net_version")]
    async fn net_version(&self) -> Result<String, ErrorObjectOwned>;
}

pub struct GethImpl {
    new_tx_sender: Sender<Transaction>,
}

#[async_trait]
impl GethServer for GethImpl {
    async fn eth_subscribe(
        &self,
        pending: PendingSubscriptionSink,
        subscription_target: String,
        full_txs: Option<bool>,
    ) -> SubscriptionResult {
        assert_eq!(
            ("newPendingTransactions", Some(true)),
            (&*subscription_target, full_txs),
            "the mocked geth server only supports the `eth_subscribe` RPC with
            parameters [\"newPendingTransaction\", true]",
        );
        use jsonrpsee::server::SubscriptionMessage;
        let sink = pending.accept().await?;
        let mut rx = self.new_tx_sender.subscribe();
        loop {
            tokio::select!(
                biased;
                () = sink.closed() => break,
                Ok(new_tx) = rx.recv() => sink.send(
                    SubscriptionMessage::from_json(&new_tx)?
                ).await?,
            )
        }
        Ok(())
    }

    async fn net_version(&self) -> Result<String, ErrorObjectOwned> {
        Ok("mock_geth".into())
    }
}

pub struct MockGeth {
    /// The local address to which the mocked jsonrpc server is bound.
    local_addr: SocketAddr,
    /// A channel over which new transactions can be inserted into the mocked
    /// server so that they are forwarded to a client that subscribed to new
    /// pending transactions over websocket.
    new_tx_sender: Sender<Transaction>,
    _server_task_handle: tokio::task::JoinHandle<()>,
}

impl MockGeth {
    pub async fn spawn() -> Self {
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
        let (new_tx_sender, _) = channel(256);
        let mock_geth_impl = GethImpl {
            new_tx_sender: new_tx_sender.clone(),
        };
        let handle = server.start(mock_geth_impl.into_rpc());
        let _server_task_handle = tokio::spawn(handle.stopped());
        Self {
            local_addr,
            new_tx_sender,
            _server_task_handle,
        }
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn push_tx(&self, tx: Transaction) -> Result<usize, SendError<Transaction>> {
        self.new_tx_sender.send(tx)
    }
}
