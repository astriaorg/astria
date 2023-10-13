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
use tracing::instrument::Instrumented;

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
    _provider_loop: Instrumented<JoinHandle<()>>,
}

impl ClientProvider {
    const RECONNECTION_ATTEMPTS: u32 = 1024;

    pub(crate) async fn new(url: &str) -> eyre::Result<Self> {
        use std::time::Duration;

        use futures::{
            future::FusedFuture as _,
            FutureExt as _,
        };
        use tracing::{
            info,
            info_span,
            warn,
            Instrument as _,
        };
        let (client_tx, mut client_rx): (ClientTx, ClientRx) = mpsc::unbounded_channel();

        info!(
            max_attempts = Self::RECONNECTION_ATTEMPTS,
            strategy = "exponential backoff",
            "connecting to sequencer websocket"
        );
        let retry_config = tryhard::RetryFutureConfig::new(Self::RECONNECTION_ATTEMPTS)
            .exponential_backoff(Duration::from_secs(5))
            .max_delay(Duration::from_secs(60))
            .on_retry(
                |attempt,
                 next_delay: Option<Duration>,
                 error: &sequencer_client::tendermint_rpc::Error| {
                    let error = error.clone();
                    let wait_duration = next_delay
                        .map(humantime::format_duration)
                        .map(tracing::field::display);
                    async move {
                        warn!(
                            attempt,
                            wait_duration,
                            error.message = %error,
                            error.cause = ?error,
                            "attempt to connect to sequencer websocket failed; retrying after backoff",
                        );
                    }
                },
            );

        let url_ = url.to_string();
        let _provider_loop = tokio::spawn(async move {
            let mut client = None;
            let mut driver_fut = futures::future::Fuse::terminated();
            let mut reconnect = tryhard::retry_fn(|| {
                let url = url_.clone();
                async move { WebSocketClient::new(&*url).await }
            })
            .with_config(retry_config)
            .boxed()
            .fuse();

            let mut pending_requests: Vec<oneshot::Sender<Result<WebSocketClient, Error>>> =
                Vec::new();

            loop {
                select!(
                    _ = &mut driver_fut, if !driver_fut.is_terminated() => {
                        warn!("websocket driver failed, attempting to reconnect");
                        client = None;
                        reconnect = tryhard::retry_fn(|| {
                            let url = url_.clone();
                            async move {
                                WebSocketClient::new(&*url).await
                            }}).with_config(retry_config).boxed().fuse();
                    }

                    res = &mut reconnect, if !reconnect.is_terminated() => {
                        match res {
                            Ok((new_client, driver)) => {
                                info!("established a new websocket connection; handing out clients to all pending requests");
                                driver_fut = driver.run().boxed().fuse();
                                for tx in pending_requests.drain(..) {
                                    let _ = tx.send(Ok(new_client.clone()));
                                }
                                client = Some(new_client);
                            }
                            Err(e) => {
                                warn!(
                                    error.message = %e,
                                    error.cause = ?e,
                                    attempts = Self::RECONNECTION_ATTEMPTS,
                                    "repeatedly failed to re-establish websocket connection; giving up",
                                );
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
        }).instrument(info_span!("client provider loop", url));

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
        Err(deadpool::managed::RecycleError::StaticMessage(
            "client automatically invalidated",
        ))
    }
}
