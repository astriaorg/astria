use astria_core::{
    crypto::ADDRESS_LENGTH,
    primitive::v1::Address,
    protocol::{
        memos::v1::Ics20WithdrawalFromRollup,
        transaction::v1::action::Ics20Withdrawal,
    },
};
use ibc_types::core::client::Height;

use crate::test_utils::astria_address;

fn new_rollup_withdrawal() -> Ics20WithdrawalFromRollup {
    Ics20WithdrawalFromRollup {
        rollup_block_number: 2,
        rollup_withdrawal_event_id: "event-1".to_string(),
        rollup_return_address: "abc".to_string(),
        memo: "a memo".to_string(),
    }
}

/// A builder for an [`Ics20Withdrawal`].
///
/// By default, the following values are used:
///   * `amount`: 1
///   * `return_address`: `astria_address(&[1; ADDRESS_LENGTH])`
///   * `timeout_time`: 100,000,000,000
///   * `bridge_address`: `None`
///   * `rollup_withdrawal`: `None`
pub(crate) struct Ics20WithdrawalBuilder {
    amount: u128,
    return_address: Address,
    timeout_time: u64,
    bridge_address: Option<Address>,
    rollup_withdrawal: Option<Ics20WithdrawalFromRollup>,
}

impl Ics20WithdrawalBuilder {
    pub(crate) fn new() -> Self {
        Self {
            amount: 1,
            return_address: astria_address(&[1; ADDRESS_LENGTH]),
            timeout_time: 100_000_000_000,
            bridge_address: None,
            rollup_withdrawal: None,
        }
    }

    pub(crate) fn with_amount(mut self, amount: u128) -> Self {
        self.amount = amount;
        self
    }

    pub(crate) fn with_return_address(mut self, return_address: Address) -> Self {
        self.return_address = return_address;
        self
    }

    pub(crate) fn with_timeout_time(mut self, timeout_time: u64) -> Self {
        self.timeout_time = timeout_time;
        self
    }

    pub(crate) fn with_bridge_address(mut self, bridge_address: Address) -> Self {
        self.bridge_address = Some(bridge_address);
        self
    }

    pub(crate) fn with_default_rollup_withdrawal(mut self) -> Self {
        self.rollup_withdrawal = Some(new_rollup_withdrawal());
        self
    }

    pub(crate) fn with_rollup_return_address<T: Into<String>>(
        mut self,
        rollup_return_address: T,
    ) -> Self {
        if self.rollup_withdrawal.is_none() {
            self.rollup_withdrawal = Some(new_rollup_withdrawal());
        }
        self.rollup_withdrawal
            .as_mut()
            .unwrap()
            .rollup_return_address = rollup_return_address.into();
        self
    }

    pub(crate) fn with_rollup_withdrawal_event_id<T: Into<String>>(
        mut self,
        rollup_withdrawal_event_id: T,
    ) -> Self {
        if self.rollup_withdrawal.is_none() {
            self.rollup_withdrawal = Some(new_rollup_withdrawal());
        }
        self.rollup_withdrawal
            .as_mut()
            .unwrap()
            .rollup_withdrawal_event_id = rollup_withdrawal_event_id.into();
        self
    }

    pub(crate) fn with_rollup_block_number(mut self, rollup_block_number: u64) -> Self {
        if self.rollup_withdrawal.is_none() {
            self.rollup_withdrawal = Some(new_rollup_withdrawal());
        }
        self.rollup_withdrawal.as_mut().unwrap().rollup_block_number = rollup_block_number;
        self
    }

    pub(crate) fn build(self) -> Ics20Withdrawal {
        let Self {
            amount,
            return_address,
            timeout_time,
            bridge_address,
            rollup_withdrawal,
        } = self;
        let memo = rollup_withdrawal
            .map(|rollup_withdrawal| {
                assert!(
                    bridge_address.is_some(),
                    "setting rollup withdrawal fields has no effect if bridge address is not set"
                );
                serde_json::to_string(&rollup_withdrawal).unwrap()
            })
            .unwrap_or_default();
        Ics20Withdrawal {
            amount,
            denom: "test".parse().unwrap(),
            destination_chain_address: "test-chain".to_string(),
            return_address,
            timeout_height: Height::new(10, 1).unwrap(),
            timeout_time,
            source_channel: "channel-0".to_string().parse().unwrap(),
            fee_asset: "test".parse().unwrap(),
            memo,
            bridge_address,
            use_compat_address: false,
        }
    }
}
