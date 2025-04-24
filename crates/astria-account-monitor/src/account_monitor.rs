use std::{
    fmt::{
        self,
        Display,
        Formatter,
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
use tokio::time::{
    interval,
    timeout,
};
use tokio_util::sync::CancellationToken;
use tracing::{
    info,
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
    shutdown_token: ShutdownHandle,
    sequencer_abci_client: sequencer_client::HttpClient,
    sequencer_accounts: SequencerAccountsToMonitor,
    sequencer_asset: Asset,
    metrics: &'static Metrics,
    interval: Duration,
}

impl AccountMonitor {
    /// Instantiates a new `Service`.
    ///
    /// # Errors
    ///
    /// - If the provided `sequencer_abci_endpoint` string cannot be contructed to a cometbft http
    ///   client.
    /// - If the provided `sequencer_asset` string cannot be parsed to a valid asset.
    /// - If the provided `sequencer_accounts` string cannot be parsed to a valid address.
    /// - If the provided `sequencer_bridge_accounts` string cannot be parsed to a valid address.
    #[instrument(skip_all, err)]
    pub fn new(cfg: Config, metrics: &'static Metrics) -> eyre::Result<Self> {
        let shutdown_handle = ShutdownHandle::new();

        let Config {
            sequencer_abci_endpoint,
            sequencer_asset,
            sequencer_accounts,
            ..
        } = cfg;

        let sequencer_cometbft_client =
            sequencer_client::HttpClient::new(&*sequencer_abci_endpoint).wrap_err_with(|| {
                format!("failed to create sequencer client for url {sequencer_abci_endpoint}")
            })?;

        let interval = Duration::from_millis(cfg.query_interval_ms);
        Ok(Self {
            shutdown_token: shutdown_handle,
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
    pub async fn run(&self) -> eyre::Result<()> {
        let Some(_res) = self
            .shutdown_token
            .token
            .run_until_cancelled(run_loop(
                self.metrics,
                &self.sequencer_abci_client,
                &self.sequencer_accounts,
                &self.sequencer_asset,
                self.interval,
            ))
            .await
        else {
            return Ok(());
        };

        Ok(())
    }
}

async fn run_loop(
    metrics: &'static Metrics,
    client: &sequencer_client::HttpClient,
    accounts: &SequencerAccountsToMonitor,
    asset: &Asset,
    pull_interval: Duration,
) {
    let mut poll_timer = interval(pull_interval);
    poll_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        poll_timer.tick().await;

        fetch_all_info(metrics, client, accounts, asset);
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
        let _handle =
            tokio::spawn(async move { fetch_account_info(metrics, &client, address, asset).await });
    }
}

/// Note: The return value of this function exists only to be emitted as part of instrumentation.
#[instrument(skip_all, fields(%address, %asset), ret(Display))]
async fn fetch_account_info(
    metrics: &'static Metrics,
    client: &sequencer_client::HttpClient,
    address: Address,
    asset: Asset,
) -> AccountInfo {
    metrics.increment_nonce_fetch_count();
    metrics.increment_balance_fetch_count();
    let (nonce, balance) = tokio::join!(
        timeout(
            Duration::from_millis(1000),
            get_latest_nonce(client, address)
        ),
        timeout(
            Duration::from_millis(1000),
            get_latest_balance(client, address, asset)
        )
    );

    let account_nonce = match nonce {
        Ok(Ok(nonce)) => {
            metrics.set_account_nonce(&address.into(), nonce);
            QueryResponse::Value(nonce)
        }
        Ok(Err(err)) => {
            warn!(%err, "failed to get nonce");
            metrics.increment_nonce_fetch_failure_count();
            QueryResponse::Error
        }
        Err(err) => {
            warn!(%err, "nonce query timed out");
            metrics.increment_nonce_fetch_failure_count();
            QueryResponse::Timeout
        }
    };

    let account_balance = match balance {
        Ok(Ok(balance)) => {
            metrics.set_account_balance(&address.into(), balance.balance);
            QueryResponse::Value(balance.balance)
        }
        Ok(Err(err)) => {
            warn!(%err, "failed to get balance");
            metrics.increment_balance_fetch_failure_count();
            QueryResponse::Error
        }
        Err(err) => {
            warn!(%err, "balance query timed out");
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
pub enum QueryResponse<T> {
    Value(T),
    Error,
    Timeout,
}

#[derive(Debug, Clone)]
pub struct AccountInfo {
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

/// A handle for instructing the [`Service`] to shut down.
///
/// It is returned along with its related `Service` from [`Service::new`].  The
/// `Service` will begin to shut down as soon as [`ShutdownHandle::shutdown`] is called or
/// when the `ShutdownHandle` is dropped.
pub struct ShutdownHandle {
    token: CancellationToken,
}

impl ShutdownHandle {
    #[must_use]
    fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }

    /// Returns a clone of the wrapped cancellation token.
    #[must_use]
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }

    /// Consumes `self` and cancels the wrapped cancellation token.
    pub fn shutdown(self) {
        self.token.cancel();
    }
}

impl Drop for ShutdownHandle {
    #[instrument(skip_all)]
    fn drop(&mut self) {
        if !self.token.is_cancelled() {
            info!("shutdown handle dropped, issuing shutdown to all services");
        }
        self.token.cancel();
    }
}
