use std::time::Duration;

use async_trait::async_trait;
use deadpool::managed::{
    self,
    Pool,
};
use eyre::{
    self,
    WrapErr as _,
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
use tracing::{
    instrument::Instrumented,
    warn,
};
use tryhard::{
    backoff_strategies::ExponentialBackoff,
    OnRetry,
    RetryFutureConfig,
};

type ClientRx = mpsc::UnboundedReceiver<oneshot::Sender<Result<WebSocketClient, Error>>>;
type ClientTx = mpsc::UnboundedSender<oneshot::Sender<Result<WebSocketClient, Error>>>;

pub(super) fn start_pool(url: &str) -> eyre::Result<Pool<ClientProvider>> {
    let client_provider = ClientProvider::new(url);
    Pool::builder(client_provider)
        .max_size(50)
        .build()
        .wrap_err("failed to create sequencer client pool")
}

#[derive(Clone, Debug, thiserror::Error)]
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

fn make_retry_config(
    attempts: u32,
) -> RetryFutureConfig<
    ExponentialBackoff,
    impl Copy + OnRetry<sequencer_client::tendermint_rpc::Error>,
> {
    RetryFutureConfig::new(attempts)
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
                    let error = &error as &(dyn std::error::Error + 'static);
                    warn!(
                        attempt,
                        wait_duration,
                        error,
                        "attempt to connect to sequencer websocket failed; retrying after backoff",
                    );
                }
            },
        )
}

impl ClientProvider {
    const RECONNECTION_ATTEMPTS: u32 = 1024;

    pub(crate) fn new(url: &str) -> Self {
        use futures::{
            future::FusedFuture as _,
            FutureExt as _,
        };
        use tracing::{
            info,
            info_span,
            Instrument as _,
        };
        let (client_tx, mut client_rx): (ClientTx, ClientRx) = mpsc::unbounded_channel();

        info!(
            max_attempts = Self::RECONNECTION_ATTEMPTS,
            strategy = "exponential backoff",
            "connecting to sequencer websocket"
        );
        let retry_config = make_retry_config(Self::RECONNECTION_ATTEMPTS);

        let url_ = url.to_string();
        let provider_loop = tokio::spawn(async move {
            let mut client = None;
            let mut driver_task = futures::future::Fuse::terminated();
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
                    res = &mut driver_task, if !driver_task.is_terminated() => {
                        let (reason, err) = match res {
                            Ok(Ok(())) => ("received exit command", None),
                            Ok(Err(e)) => ("error", Some(eyre::Report::new(e).wrap_err("driver task exited with error"))),
                            Err(e) => ("panic", Some(eyre::Report::new(e).wrap_err("driver task failed"))),
                        };
                        let error: Option<&(dyn std::error::Error + 'static)> = err.as_ref().map(AsRef::as_ref);
                        warn!(
                            error,
                            reason,
                            "websocket driver exited, attempting to reconnect");
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
                                driver_task = tokio::spawn(driver.run()).fuse();
                                for tx in pending_requests.drain(..) {
                                    let _ = tx.send(Ok(new_client.clone()));
                                }
                                client = Some(new_client);
                            }
                            Err(e) => {
                                let error = &e as &(dyn std::error::Error + 'static);
                                warn!(
                                    error,
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
                );
            }
        }).instrument(info_span!("client provider loop", url));

        Self {
            client_tx,
            _provider_loop: provider_loop,
        }
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
