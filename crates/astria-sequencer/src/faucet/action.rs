use std::time::SystemTime;

use anyhow::{
    ensure,
    Context as _,
    Result,
};
use astria_proto::native::sequencer::v1alpha1::Address;
use tracing::instrument;

use super::state_ext::AccountInfo;
use crate::{
    accounts,
    accounts::types::Balance,
    faucet,
    transaction::action_handler::ActionHandler,
};

pub(crate) const FAUCET_LIMIT_PER_DAY: Balance = Balance(1_000);

/// Represents a request for funds from the faucet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    to: Address,
    amount: Balance,
}

#[async_trait::async_trait]
impl ActionHandler for Request {
    async fn check_stateful<S: faucet::state_ext::StateReadExt + 'static>(
        &self,
        state: &S,
        _: Address,
    ) -> Result<()> {
        // check that `to` hasn't exceeded their daily limit
        let info = state
            .get_account_info(self.to)
            .await
            .context("failed getting `to` account info")?;
        if SystemTime::now() > u64_to_system_time(info.reset_time) {
            ensure!(
                FAUCET_LIMIT_PER_DAY >= self.amount,
                "request exceeds permitted amount"
            );
        } else {
            ensure!(
                info.amount_remaining >= self.amount,
                "request exceeds permitted amount"
            );
        }
        Ok(())
    }

    #[instrument(
        skip_all,
        fields(
            to = self.to.to_string(),
            amount = self.amount.into_inner(),
        )
    )]
    async fn execute<
        S: accounts::state_ext::StateReadExt
            + accounts::state_ext::StateWriteExt
            + faucet::state_ext::StateWriteExt
            + faucet::state_ext::StateReadExt,
    >(
        &self,
        state: &mut S,
        _: Address,
    ) -> Result<()> {
        let to_balance = state
            .get_account_balance(self.to)
            .await
            .context("failed getting `to` account balance")?;
        state
            .put_account_balance(self.to, to_balance + self.amount)
            .context("failed updating `to` account balance")?;

        let info = state
            .get_account_info(self.to)
            .await
            .context("failed getting `to` account info")?;

        if SystemTime::now() > u64_to_system_time(info.reset_time) {
            // we're past the reset time, so reset the amount remaining
            let reset_time = calculate_reset_time()?;
            state
                .put_account_info(
                    self.to,
                    AccountInfo {
                        amount_remaining: FAUCET_LIMIT_PER_DAY - self.amount,
                        reset_time,
                    },
                )
                .context("failed updating `to` account info")?;
        } else {
            // we're still within the same day, so just subtract the amount
            state
                .put_account_info(
                    self.to,
                    AccountInfo {
                        amount_remaining: info.amount_remaining - self.amount,
                        reset_time: info.reset_time,
                    },
                )
                .context("failed updating `to` account info")?;
        }

        Ok(())
    }
}

// converts a u64 unix timestamp to a SystemTime
fn u64_to_system_time(seconds: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(seconds)
}

// calculates the unix timestamp of the next midnight
fn calculate_reset_time() -> Result<u64> {
    let now_in_seconds = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();
    let seconds_since_last_midnight = now_in_seconds % (24 * 60 * 60);
    let seconds_until_next_midnight = (24 * 60 * 60) - seconds_since_last_midnight;
    Ok(now_in_seconds + seconds_until_next_midnight)
}
