use std::time::SystemTime;

use anyhow::{
    ensure,
    Context as _,
    Result,
};
use astria_proto::{
    generated::sequencer::v1alpha1::FaucetAction as ProtoFaucetAction,
    native::sequencer::v1alpha1::Address,
};
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

impl Request {
    pub(crate) fn to_proto(&self) -> ProtoFaucetAction {
        ProtoFaucetAction {
            to: self.to.0.to_vec(),
            amount: Some(self.amount.as_proto()),
        }
    }

    pub(crate) fn try_from_proto(proto: &ProtoFaucetAction) -> Result<Self> {
        Ok(Self {
            to: Address::try_from_slice(&proto.to)
                .context("failed to convert proto address to native Address")?,
            amount: Balance::from_proto(
                *proto
                    .amount
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("missing amount"))?,
            ),
        })
    }
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

        let now = SystemTime::now();
        if now > u64_to_system_time(info.reset_time) {
            // we're past the reset time, so reset the amount remaining
            let reset_time = calculate_reset_time(now)?;
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

// calculates the unix timestamp of the next midnight (UTC time) after the given time
fn calculate_reset_time(time: SystemTime) -> Result<u64> {
    const SECONDS_PER_DAY: u64 = 24 * 60 * 60;

    let now_in_seconds = time.duration_since(SystemTime::UNIX_EPOCH)?.as_secs();
    let seconds_since_last_midnight = now_in_seconds % SECONDS_PER_DAY;
    let seconds_until_next_midnight = SECONDS_PER_DAY - seconds_since_last_midnight;
    Ok(now_in_seconds + seconds_until_next_midnight)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::faucet::state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    };

    const TEST_ADDRESS: Address = Address([99u8; 20]);

    #[test]
    fn calculate_reset_time_ensure_midnight_in_future() {
        let now = SystemTime::now();
        let now_in_seconds = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let time = calculate_reset_time(now).unwrap();
        assert!(time > now_in_seconds);
        assert!(now_in_seconds + (24 * 60 * 60) > time);
    }

    #[tokio::test]
    async fn request_check_stateful_ok() {
        let storage = penumbra_storage::TempStorage::new()
            .await
            .expect("failed to create temp storage backing chain state");
        let snapshot = storage.latest_snapshot();
        let request = Request {
            to: TEST_ADDRESS,
            amount: Balance(1),
        };

        request
            .check_stateful(&snapshot, TEST_ADDRESS)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn request_execute_ok() {
        let storage = penumbra_storage::TempStorage::new()
            .await
            .expect("failed to create temp storage backing chain state");
        let snapshot = storage.latest_snapshot();
        let mut state_delta = penumbra_storage::StateDelta::new(snapshot);

        let request = Request {
            to: TEST_ADDRESS,
            amount: Balance(1),
        };

        let expected_reset_time = calculate_reset_time(SystemTime::now()).unwrap();
        request
            .execute(&mut state_delta, TEST_ADDRESS)
            .await
            .unwrap();
        storage.commit(state_delta).await.unwrap();
        let snapshot = storage.latest_snapshot();

        let info = snapshot.get_account_info(TEST_ADDRESS).await.unwrap();
        assert_eq!(info.amount_remaining, FAUCET_LIMIT_PER_DAY - Balance(1));
        assert_eq!(info.reset_time, expected_reset_time);
    }

    #[tokio::test]
    async fn request_check_stateful_err() {
        let storage = penumbra_storage::TempStorage::new()
            .await
            .expect("failed to create temp storage backing chain state");
        let snapshot = storage.latest_snapshot();
        let request = Request {
            to: TEST_ADDRESS,
            amount: super::FAUCET_LIMIT_PER_DAY + Balance(1),
        };
        let err = request
            .check_stateful(&snapshot, TEST_ADDRESS)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("request exceeds permitted amount"));
    }

    #[tokio::test]
    async fn request_check_stateful_reset_time_past_ok() {
        let storage = penumbra_storage::TempStorage::new()
            .await
            .expect("failed to create temp storage backing chain state");
        let snapshot = storage.latest_snapshot();
        let mut state_delta = penumbra_storage::StateDelta::new(snapshot);

        // store an account info with a reset time in the past
        let info = AccountInfo {
            amount_remaining: Balance(0),
            reset_time: 100,
        };
        state_delta.put_account_info(TEST_ADDRESS, info).unwrap();
        storage.commit(state_delta).await.unwrap();

        // make a request, which should succeed
        let snapshot = storage.latest_snapshot();
        let request = Request {
            to: TEST_ADDRESS,
            amount: super::FAUCET_LIMIT_PER_DAY,
        };

        request
            .check_stateful(&snapshot, TEST_ADDRESS)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn request_check_stateful_reset_time_past_err() {
        let storage = penumbra_storage::TempStorage::new()
            .await
            .expect("failed to create temp storage backing chain state");
        let snapshot = storage.latest_snapshot();
        let mut state_delta = penumbra_storage::StateDelta::new(snapshot);

        // store an account info with a reset time in the past
        let info = AccountInfo {
            amount_remaining: Balance(0),
            reset_time: 100,
        };
        state_delta.put_account_info(TEST_ADDRESS, info).unwrap();
        storage.commit(state_delta).await.unwrap();

        // make a request, which should fail because the requested amount is too high
        let snapshot = storage.latest_snapshot();
        let request = Request {
            to: TEST_ADDRESS,
            amount: super::FAUCET_LIMIT_PER_DAY + 1,
        };

        let err = request
            .check_stateful(&snapshot, TEST_ADDRESS)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("request exceeds permitted amount"));
    }
}
