use std::{
    str::FromStr,
    time::Duration,
};

use astria_core::{
    primitive::v1::{
        Address,
        asset::Denom,
    },
    protocol::account::v1::AssetBalance,
};
use astria_eyre::eyre::{
    self,
    OptionExt,
    WrapErr as _,
};
use sequencer_client::{
    Client,
    SequencerClientExt as _,
    tendermint::{
        Hash,
        hash::Algorithm,
    },
};
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{
    debug,
    info,
    instrument,
};

use crate::{
    config::Config,
    metrics::Metrics,
};

pub struct AccountMonitor {
    shutdown_token: ShutdownHandle,
    sequencer_abci_client: sequencer_client::HttpClient,
    accounts: Vec<Address>,
    bridge_accounts: Vec<Address>,
    sequencer_asset: Denom,
    metrics: &'static Metrics,
    interval: Duration,
    // fields omitted
}

impl AccountMonitor {
    pub fn new(cfg: Config, metrics: &'static Metrics) -> eyre::Result<Self> {
        let shutdown_handle = ShutdownHandle::new();

        let accounts = cfg.parse_accounts()?;
        let bridge_accounts = cfg.parse_bridge_accounts()?;

        let Config {
            sequencer_abci_endpoint,
            sequencer_asset,
            ..
        } = cfg;

        let sequencer_cometbft_client =
            sequencer_client::HttpClient::new(&*sequencer_abci_endpoint)
                .wrap_err("failed constructing cometbft http client")?;

        let sequencer_asset = Denom::from_str(sequencer_asset.as_str())
            .map_err(|e| eyre::eyre!("failed to parse asset: {e}"))?;

        let interval = Duration::from_millis(cfg.block_time_ms);
        Ok(Self {
            shutdown_token: shutdown_handle,
            sequencer_abci_client: sequencer_cometbft_client,
            accounts,
            bridge_accounts,
            sequencer_asset,
            metrics,
            interval,
        })
    }

    pub async fn run_until_stopped(&self) -> eyre::Result<()> {
        info!("starting account monitor");

        let mut poll_timer = interval(self.interval);

        loop {
            // Check if shutdown signal has been received
            if self.shutdown_token.token.is_cancelled() {
                info!("received shutdown signal");
                break;
            }

            // Wait for the next poll interval
            poll_timer.tick().await;

            for account in self.bridge_accounts.iter() {
                let account = account.clone();
                let last_tx_hash = self
                    .sequencer_abci_client
                    .get_bridge_account_last_transaction_hash(account)
                    .await
                    .wrap_err("failed to get last transaction hash")?
                    .tx_hash;

                if let Some(tx_hash) = last_tx_hash {
                    let last_tx_height = self
                        .sequencer_abci_client
                        .tx(Hash::from_bytes(Algorithm::Sha256, &tx_hash)?, false)
                        .await
                        .wrap_err("failed query bridge tx by hash")?
                        .height;

                    self.metrics.set_bridge_last_transaction_height(
                        account.to_string().as_str(),
                        last_tx_height.into(),
                    );
                }
            }

            // Process regular accounts
            for account in self.accounts.iter() {
                let account = account.clone();

                // Get latest nonce
                self.metrics.increment_nonce_fetch_count();
                match get_latest_nonce(&self.sequencer_abci_client, account).await {
                    Ok(nonce) => {
                        self.metrics
                            .set_account_nonce(account.to_string().as_str(), nonce);
                    }
                    Err(e) => {
                        debug!("failed to get latest nonce: {e}");
                        self.metrics.increment_nonce_fetch_failure_count();
                    }
                };

                // Get latest balance
                self.metrics.increment_balance_fetch_count();
                match get_latest_balance(
                    &self.sequencer_abci_client,
                    account,
                    self.sequencer_asset.clone(),
                )
                .await
                {
                    Ok(balance) => {
                        self.metrics
                            .set_account_balance(account.to_string().as_str(), balance.balance);
                    }
                    Err(e) => {
                        debug!("failed to get latest balance: {e}");
                        self.metrics.increment_balance_fetch_failure_count();
                    }
                };
            }
        }

        Ok(())
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
