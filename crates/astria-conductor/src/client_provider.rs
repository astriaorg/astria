use async_trait::async_trait;
use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use deadpool::managed;
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

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("the client provider failed to reconnect and is permanently closed")]
    Failed,
    #[error("the client provider is currently trying to reconnect")]
    Reconnecting,
    #[error(
        "the channel over which the client provider was expected to return a client was dropped \
         unexpectedly"
    )]
    ClientChannelDroped,
}

pub(crate) struct ClientProvider {
    client_tx: ClientTx,
    _driver: JoinHandle<()>,
}

impl ClientProvider {
    pub(crate) async fn new(url: &str) -> eyre::Result<Self> {
        let url = url.to_string();
        let (client, driver) = WebSocketClient::new(&*url)
            .await
            .wrap_err("failed constructing a cometbft websocket client to read off sequencer")?;
        let (client_tx, mut client_rx): (ClientTx, ClientRx) = mpsc::unbounded_channel();
        let _driver = tokio::spawn(async move {
            let mut client = Some(client);
            let mut driver_fut = Some(Box::pin(driver.run()));
            let mut reconnect = None;
            loop {
                select!(
                    _ = async { driver_fut.as_mut().unwrap().await }, if driver_fut.is_some() => {
                        warn!("websocket driver failed, attempting to reconnect");
                        client = None;
                        driver_fut = None;
                        let url = url.clone();
                        reconnect = Some(tokio::spawn(async move { WebSocketClient::new(&*url).await } ));
                    }

                    res = async { reconnect.as_mut().unwrap().await }, if reconnect.is_some() => {
                        reconnect = None;
                        match res {
                            Ok(Ok((new_client, driver))) => {
                                client = Some(new_client);
                                driver_fut = Some(Box::pin(driver.run()));
                            }
                            Ok(Err(e)) => {
                                warn!(error.message = %e, error.cause = ?e, "failed to reestablish websocket connection; exiting");
                                break;
                            }
                            Err(e) => {
                                warn!(error.message = %e, error.cause = ?e, "task trying to reestablish websocket failed; exiting");
                                break;
                            }
                        }
                    }

                    Some(tx) = client_rx.recv() => {
                        let _ = if let Some(client) = client.clone() {{}
                            tx.send(Ok(client.clone()))
                        } else {
                            tx.send(Err(Error::Reconnecting))
                        };
                    }
                )
            }
        });

        Ok(Self {
            client_tx,
            _driver,
        })
    }

    async fn get(&self) -> Result<WebSocketClient, Error> {
        let (tx, rx) = oneshot::channel();
        if self.client_tx.send(tx).is_err() {
            return Err(Error::Failed);
        }
        rx.await.map_err(|_| Error::ClientChannelDroped)?
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
