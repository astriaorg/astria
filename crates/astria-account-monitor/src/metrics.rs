use std::collections::HashMap;

use telemetry::{
    metric_names,
    metrics::{
        Counter,
        Error,
        Gauge,
        RegisteringBuilder,
    },
};
use tracing::error;

pub struct Metrics {
    nonce_fetch_count: Counter,
    nonce_fetch_failure_count: Counter,
    balance_fetch_count: Counter,
    balance_fetch_failure_count: Counter,
    account_nonce: HashMap<String, Gauge>,
    account_balance: HashMap<String, Gauge>,
}

impl Metrics {
    pub(crate) fn increment_nonce_fetch_count(&self) {
        self.nonce_fetch_count.increment(1);
    }

    pub(crate) fn increment_nonce_fetch_failure_count(&self) {
        self.nonce_fetch_failure_count.increment(1);
    }

    pub fn set_account_nonce(&self, account: &str, nonce: u32) {
        if let Some(gauge) = self.account_nonce.get(account) {
            gauge.set(nonce);
        } else {
            error!("no gauge found for account nonce: {}", account);
        }
    }

    pub fn set_account_balance(&self, account: &str, balance: u128) {
        if let Some(gauge) = self.account_balance.get(account) {
            gauge.set(balance);
        } else {
            error!("no gauge found for account balance: {}", account);
        }
    }

    pub fn increment_balance_fetch_count(&self) {
        self.balance_fetch_count.increment(1);
    }

    pub fn increment_balance_fetch_failure_count(&self) {
        self.balance_fetch_failure_count.increment(1);
    }
}

impl telemetry::Metrics for Metrics {
    type Config = crate::Config;

    fn register(builder: &mut RegisteringBuilder, config: &Self::Config) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let nonce_fetch_count = builder
            .new_counter_factory(
                NONCE_FETCH_COUNT,
                "The number of times we have attempted to fetch the nonce",
            )?
            .register()?;

        let nonce_fetch_failure_count = builder
            .new_counter_factory(
                NONCE_FETCH_FAILURE_COUNT,
                "The number of times we have failed to fetch the nonce",
            )?
            .register()?;

        let balance_fetch_count = builder
            .new_counter_factory(
                BALANCE_FETCH_COUNT,
                "The number of times we have attempted to fetch the balance",
            )?
            .register()?;

        let balance_fetch_failure_count = builder
            .new_counter_factory(
                BALANCE_FETCH_FAILURE_COUNT,
                "The number of times we have failed to fetch the balance",
            )?
            .register()?;

        let mut account_nonce = HashMap::new();
        let mut account_balance = HashMap::new();
        let mut nonce_factory =
            builder.new_gauge_factory(ACCOUNT_NONCE, "The current nonce for a specific account")?;

        for account in &config.sequencer_accounts {
            let nonce_gauge =
                nonce_factory.register_with_labels(&[(ACCOUNT_LABEL, account.to_string())])?;
            account_nonce.insert(account.to_string().clone(), nonce_gauge);
        }

        let mut balance_factory = builder.new_gauge_factory(
            ACCOUNT_BALANCE,
            "The current balance for a specific account",
        )?;

        for account in &config.sequencer_accounts {
            let balance_gauge =
                balance_factory.register_with_labels(&[(ACCOUNT_LABEL, account.to_string())])?;
            account_balance.insert(account.to_string().clone(), balance_gauge);
        }

        Ok(Self {
            nonce_fetch_count,
            nonce_fetch_failure_count,
            balance_fetch_count,
            balance_fetch_failure_count,
            account_nonce,
            account_balance,
        })
    }
}

metric_names!(pub const METRICS_NAMES:
    NONCE_FETCH_COUNT,
    NONCE_FETCH_FAILURE_COUNT,
    BALANCE_FETCH_COUNT,
    BALANCE_FETCH_FAILURE_COUNT,
    CURRENT_NONCE,
    ACCOUNT_NONCE,
    ACCOUNT_LABEL,
    ACCOUNT_BALANCE,
);
