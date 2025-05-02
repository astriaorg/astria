use std::{
    fmt::{
        self,
        Display,
        Formatter,
    },
    future::Future,
    pin::Pin,
    task::{
        Context,
        Poll,
    },
    time::Duration,
};

use astria_core::{
    primitive::v1::Address,
    protocol::account::v1::AssetBalance,
};
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use sequencer_client::SequencerClientExt as _;
use tokio::{
    task::{
        JoinError,
        JoinHandle,
    },
    time::{
        interval,
        timeout,
    },
};
use tokio_util::sync::CancellationToken;
use tracing::{
    instrument,
    warn,
};

use crate::{
    config::{
        Asset,
        Config,
        SequencerAccountsToMonitor,
    },
    metrics::Metrics,
};

pub struct AccountMonitor {
    shutdown_token: CancellationToken,
    task: Option<JoinHandle<eyre::Result<()>>>,
}

impl AccountMonitor {
    /// Spawns the Account Monitor service.
    ///
    /// # Errors
    /// Returns an error if the Auctioneer cannot be initialized.
    pub fn spawn(cfg: Config, metrics: &'static Metrics) -> eyre::Result<Self> {
        let shutdown_token = CancellationToken::new();
        let inner = Inner::new(cfg, metrics)?;
        let task = tokio::spawn(inner.run());

        Ok(Self {
            shutdown_token,
            task: Some(task),
        })
    }

    /// Shuts down Account Monitor.
    ///
    /// # Errors
    /// Returns an error if an error occured during shutdown.
    pub async fn shutdown(self) -> eyre::Result<()> {
        self.shutdown_token.cancel();
        self.await
    }
}

impl Future for AccountMonitor {
    type Output = eyre::Result<()>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        use futures::future::FutureExt as _;

        let task = self
            .task
            .as_mut()
            .expect("auctioneer must not be polled after shutdown");
        task.poll_unpin(ctx).map(flatten_join_result)
    }
}

struct Inner {
    shutdown_token: CancellationToken,
    sequencer_abci_client: sequencer_client::HttpClient,
    sequencer_accounts: SequencerAccountsToMonitor,
    sequencer_asset: Asset,
    metrics: &'static Metrics,
    interval: Duration,
}

impl Inner {
    /// Instantiates a new `Service`.
    ///
    /// # Errors
    ///
    /// - If the provided `sequencer_abci_endpoint` string cannot be contructed to a cometbft http
    ///   client.
    #[instrument(skip_all, err)]
    fn new(cfg: Config, metrics: &'static Metrics) -> eyre::Result<Self> {
        let Config {
            sequencer_abci_endpoint,
            sequencer_asset,
            sequencer_accounts,
            ..
        } = cfg;

        let shutdown_token = CancellationToken::new();
        let sequencer_cometbft_client =
            sequencer_client::HttpClient::new(&*sequencer_abci_endpoint).wrap_err_with(|| {
                format!("failed to create sequencer client for url `{sequencer_abci_endpoint}`")
            })?;

        let interval = Duration::from_millis(cfg.query_interval_ms);
        Ok(Self {
            shutdown_token,
            sequencer_abci_client: sequencer_cometbft_client,
            sequencer_accounts,
            sequencer_asset,
            metrics,
            interval,
        })
    }

    /// Run the query loop, polling the sequencer for accounts information.
    ///
    /// # Errors
    /// An error is returned if bridge last transaction height is not found.
    async fn run(self) -> eyre::Result<()> {
        let mut poll_timer = interval(self.interval);
        poll_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                biased;
                () = self.shutdown_token.cancelled() => {
                    return Ok(());
                }
                _ = poll_timer.tick() => {
                    fetch_all_info(
                        self.metrics,
                        &self.sequencer_abci_client,
                        &self.sequencer_accounts,
                        &self.sequencer_asset,
                    );
                }
            }
        }
    }
}

fn flatten_join_result<T>(res: Result<eyre::Result<T>, JoinError>) -> eyre::Result<T> {
    match res {
        Ok(Ok(val)) => Ok(val),
        Ok(Err(err)) => Err(err).wrap_err("task returned with error"),
        Err(err) => Err(err).wrap_err("task panicked"),
    }
}

fn fetch_all_info(
    metrics: &'static Metrics,
    client: &sequencer_client::HttpClient,
    accounts: &SequencerAccountsToMonitor,
    asset: &Asset,
) {
    for account in accounts {
        let client = client.clone();
        let asset = asset.clone();
        let address = account.address;
        tokio::spawn(fetch_account_info(metrics, client, address, asset));
    }
}

/// Note: The return value of this function exists only to be emitted as part of instrumentation.
#[instrument(skip_all, fields(%address, %asset), ret(Display))]
async fn fetch_account_info(
    metrics: &'static Metrics,
    client: sequencer_client::HttpClient,
    address: Address,
    asset: Asset,
) -> AccountInfo {
    metrics.increment_nonce_fetch_count();
    metrics.increment_balance_fetch_count();
    let (nonce, balance) = tokio::join!(
        timeout(
            Duration::from_millis(1000),
            get_latest_nonce(&client, address)
        ),
        timeout(
            Duration::from_millis(1000),
            get_latest_balance(&client, address, asset)
        )
    );

    let account_nonce = match nonce {
        Ok(Ok(nonce)) => {
            metrics.set_account_nonce(&address.into(), nonce);
            QueryResponse::Value(nonce)
        }
        Ok(Err(error)) => {
            warn!(%error, "failed to get nonce");
            metrics.increment_nonce_fetch_failure_count();
            QueryResponse::Error
        }
        Err(error) => {
            warn!(%error, "nonce query timed out");
            metrics.increment_nonce_fetch_failure_count();
            QueryResponse::Timeout
        }
    };

    let account_balance = match balance {
        Ok(Ok(balance)) => {
            metrics.set_account_balance(&address.into(), balance.balance);
            QueryResponse::Value(balance.balance)
        }
        Ok(Err(error)) => {
            warn!(%error, "failed to get balance");
            metrics.increment_balance_fetch_failure_count();
            QueryResponse::Error
        }
        Err(error) => {
            warn!(%error, "balance query timed out");
            metrics.increment_balance_fetch_failure_count();
            QueryResponse::Timeout
        }
    };

    AccountInfo {
        nonce: account_nonce,
        balance: account_balance,
    }
}

#[derive(Debug, Clone)]
enum QueryResponse<T> {
    Value(T),
    Error,
    Timeout,
}

#[derive(Debug, Clone)]
struct AccountInfo {
    nonce: QueryResponse<u32>,
    balance: QueryResponse<u128>,
}

impl<T: Display> Display for QueryResponse<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            QueryResponse::Value(value) => value.fmt(f),
            QueryResponse::Error => f.write_str("<error>"),
            QueryResponse::Timeout => f.write_str("<timed out>"),
        }
    }
}

// Implement Display for AccountInfo
impl Display for AccountInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "nonce = {}, balance = {}", self.nonce, self.balance)
    }
}

async fn get_latest_balance(
    client: &sequencer_client::HttpClient,
    account: Address,
    asset: Asset,
) -> eyre::Result<AssetBalance> {
    let balances = client
        .get_latest_balance(account)
        .await
        .wrap_err("failed to fetch the balance")?;
    balances
        .balances
        .into_iter()
        .find(|b| b.denom == asset.asset)
        .ok_or_else(|| eyre::eyre!("response did not contain target asset `{asset}`"))
}

async fn get_latest_nonce(
    client: &sequencer_client::HttpClient,
    account: Address,
) -> eyre::Result<u32> {
    let nonce = client
        .get_latest_nonce(account)
        .await
        .wrap_err("failed to fetch the nonce")?;
    Ok(nonce.nonce)
}
