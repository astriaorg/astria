use std::{
    fmt::{
        self,
        Display,
        Formatter,
    },
    time::Duration,
};

use astria_core::{
    primitive::v1::{
        asset::Denom,
        Address,
    },
    protocol::account::v1::AssetBalance,
};
use astria_eyre::eyre::{
    self,
    OptionExt,
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
};

use crate::{
    config::{
        Account,
        Config,
    },
    metrics::Metrics,
};

pub struct AccountMonitor {
    shutdown_token: ShutdownHandle,
    sequencer_abci_client: sequencer_client::HttpClient,
    sequencer_accounts: Vec<Account>,
    sequencer_asset: Denom,
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
    pub fn new(cfg: Config, metrics: &'static Metrics) -> eyre::Result<Self> {
        let shutdown_handle = ShutdownHandle::new();

        let Config {
            sequencer_abci_endpoint,
            sequencer_asset,
            sequencer_accounts,
            ..
        } = cfg;

        let sequencer_cometbft_client =
            sequencer_client::HttpClient::new(&*sequencer_abci_endpoint)
                .wrap_err("failed constructing cometbft http client")?;

        let sequencer_asset = sequencer_asset
            .parse()
            .wrap_err("msg: failed to parse asset")?;

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
                self.sequencer_accounts.clone(),
                self.sequencer_asset.clone(),
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
    accounts: Vec<Account>,
    denom: Denom,
    pull_interval: Duration,
) {
    let mut poll_timer = interval(pull_interval);

    loop {
        poll_timer.tick().await;

        fetch_all_info(metrics, client, accounts.clone(), &denom);
    }
}

fn fetch_all_info(
    metrics: &'static Metrics,
    client: &sequencer_client::HttpClient,
    accounts: Vec<Account>,
    denom: &Denom,
) {
    for account in accounts {
        let client = client.clone();
        let denom = denom.clone();
        let address = account.address;
        let _handle =
            tokio::spawn(async move { fetch_account_info(metrics, &client, address, denom).await });
    }
}

#[instrument(skip_all, fields(%address), ret(Display))]
async fn fetch_account_info(
    metrics: &'static Metrics,
    client: &sequencer_client::HttpClient,
    address: Address,
    denom: Denom,
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
            get_latest_balance(client, address, denom)
        )
    );

    let account_nonce = match nonce {
        Ok(Ok(nonce)) => {
            metrics.set_account_nonce(address.to_string().as_str(), nonce);
            QueryResponse::Value(nonce)
        }
        Ok(Err(_)) => {
            println!("Failed to get nonce");
            metrics.increment_nonce_fetch_failure_count();
            QueryResponse::Error
        }
        Err(_) => {
            println!("Nonce query timed out");
            metrics.increment_nonce_fetch_failure_count();
            QueryResponse::Timeout
        }
    };

    let account_balance = match balance {
        Ok(Ok(balance)) => {
            metrics.set_account_balance(address.to_string().as_str(), balance.balance);
            QueryResponse::Value(balance.balance)
        }
        Ok(Err(_)) => {
            println!("Failed to get balance");
            metrics.increment_balance_fetch_failure_count();
            QueryResponse::Error
        }
        Err(_) => {
            println!("Balance query timed out");
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
            QueryResponse::Value(value) => write!(f, "{value}"),
            QueryResponse::Error => write!(f, "Error"),
            QueryResponse::Timeout => write!(f, "Timeout"),
        }
    }
}

// Implement Display for AccountInfo
impl Display for AccountInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AccountInfo {{ nonce: {}, balance: {} }}",
            self.nonce, self.balance
        )
    }
}

async fn get_latest_balance(
    client: &sequencer_client::HttpClient,
    account: Address,
    asset: Denom,
) -> eyre::Result<AssetBalance> {
    let balances = client.get_latest_balance(account).await?;
    balances
        .balances
        .into_iter()
        .find(|b| b.denom == asset)
        .ok_or_eyre("failed to find asset balance")
}

async fn get_latest_nonce(
    client: &sequencer_client::HttpClient,
    account: Address,
) -> eyre::Result<u32> {
    let nonce = client.get_latest_nonce(account).await?;
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
