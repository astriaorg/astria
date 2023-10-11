use async_trait::async_trait;
use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use deadpool::managed::{
    self,
    Pool,
};
use sequencer_client::WebSocketClient;
use tokio::{
    select,
    sync::{
        mpsc,
        oneshot,
    },
    task::JoinHandle,
};
use tracing::warn;

type ClientRx = mpsc::UnboundedReceiver<oneshot::Sender<Result<WebSocketClient, Error>>>;
type ClientTx = mpsc::UnboundedSender<oneshot::Sender<Result<WebSocketClient, Error>>>;

pub(super) async fn start_pool(url: &str) -> eyre::Result<Pool<ClientProvider>> {
    let client_provider = ClientProvider::new(url)
        .await
        .wrap_err("failed initializing sequencer client provider")?;
    Pool::builder(client_provider)
        .max_size(50)
        .build()
        .wrap_err("failed to create sequencer client pool")
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("the client provider failed to reconnect and is permanently closed")]
    Failed,
    #[error("the channel over which to receive a client was closed unexpectedly")]
    ClientChannelDropped,
}

pub(crate) struct ClientProvider {
    client_tx: ClientTx,
    _provider_loop: JoinHandle<()>,
}

impl ClientProvider {
    pub(crate) async fn new(url: &str) -> eyre::Result<Self> {
        use futures::{
            future::FusedFuture as _,
            FutureExt as _,
        };
        let url = url.to_string();
        let (client, driver) = WebSocketClient::new(&*url)
            .await
            .wrap_err("failed constructing a cometbft websocket client to read off sequencer")?;
        let (client_tx, mut client_rx): (ClientTx, ClientRx) = mpsc::unbounded_channel();
        let _provider_loop = tokio::spawn(async move {
            let mut client = Some(client);
            let mut driver_fut = driver.run().boxed().fuse();
            let mut reconnect = futures::future::Fuse::terminated();
            let mut pending_requests: Vec<oneshot::Sender<Result<WebSocketClient, Error>>> =
                Vec::new();
            loop {
                select!(
                    _ = &mut driver_fut => {
                        warn!("websocket driver failed, attempting to reconnect");
                        client = None;
                        let url = url.clone();
                        reconnect = async move { WebSocketClient::new(&*url).await }.boxed().fuse();
                    }

                    res = &mut reconnect, if !reconnect.is_terminated() => {
                        match res {
                            Ok((new_client, driver)) => {
                                driver_fut = driver.run().boxed().fuse();
                                for tx in pending_requests.drain(..) {
                                    let _ = tx.send(Ok(new_client.clone()));
                                }
                                client = Some(new_client);
                            }
                            Err(e) => {
                                warn!(error.message = %e, error.cause = ?e, "failed to reestablish websocket connection; exiting");
                                break;
                            }
                        }
                    }

                    Some(tx) = client_rx.recv() => {
                        // immediately return a client if available
                        if let Some(client) = client.clone() {
                            let _ = tx.send(Ok(client));
                        // or schedule to return them once available
                        } else {
                            pending_requests.push(tx);
                        }
                    }
                )
            }
        });

        Ok(Self {
            client_tx,
            _provider_loop,
        })
    }

    async fn get(&self) -> Result<WebSocketClient, Error> {
        let (tx, rx) = oneshot::channel();
        if self.client_tx.send(tx).is_err() {
            return Err(Error::Failed);
        }
        rx.await.map_err(|_| Error::ClientChannelDropped)?
    }
}

#[async_trait]
impl managed::Manager for ClientProvider {
    type Error = Error;
    type Type = WebSocketClient;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        self.get().await
    }

    async fn recycle(
        &self,
        _obj: &mut Self::Type,
        _: &managed::Metrics,
    ) -> managed::RecycleResult<Self::Error> {
        Ok(())
    }
}
