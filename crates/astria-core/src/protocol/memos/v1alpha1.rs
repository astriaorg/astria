pub use raw::Ics20TransferDeposit;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    generated::protocol::memos::v1alpha1 as raw,
    Protobuf,
};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Ics20WithdrawalFromRollup {
    pub rollup_block_number: u64,
    pub rollup_return_address: String,
    pub rollup_withdrawal_event_id: String,
    pub memo: String,
}

impl Protobuf for Ics20WithdrawalFromRollup {
    type Error = prost::DecodeError;
    type Raw = raw::Ics20WithdrawalFromRollup;

    fn try_from_raw_ref(proto: &raw::Ics20WithdrawalFromRollup) -> Result<Self, Self::Error> {
        Self::try_from_raw(proto.clone())
    }

    fn try_from_raw(raw: raw::Ics20WithdrawalFromRollup) -> Result<Self, Self::Error> {
        Ok(Self {
            rollup_block_number: raw.rollup_block_number,
            rollup_return_address: raw.rollup_return_address,
            rollup_withdrawal_event_id: raw.rollup_withdrawal_event_id,
            memo: raw.memo,
        })
    }

    #[must_use]
    fn to_raw(&self) -> raw::Ics20WithdrawalFromRollup {
        raw::Ics20WithdrawalFromRollup {
            rollup_block_number: self.rollup_block_number,
            rollup_return_address: self.rollup_return_address.clone(),
            rollup_withdrawal_event_id: self.rollup_withdrawal_event_id.clone(),
            memo: self.memo.clone(),
        }
    }

    #[must_use]
    fn into_raw(self) -> raw::Ics20WithdrawalFromRollup {
        raw::Ics20WithdrawalFromRollup {
            rollup_block_number: self.rollup_block_number,
            rollup_return_address: self.rollup_return_address,
            rollup_withdrawal_event_id: self.rollup_withdrawal_event_id,
            memo: self.memo,
        }
    }
}

#[cfg(all(feature = "serde", test))]
mod test {
    use super::*;

    #[test]
    fn ics20_withdrawal_from_rollup_memo_snapshot() {
        let memo = Ics20WithdrawalFromRollup {
            rollup_block_number: 1,
            rollup_return_address: "a-rollup-defined-address".to_string(),
            rollup_withdrawal_event_id: "a-rollup-defined-hash".to_string(),
            memo: "hello".to_string(),
        };

        insta::assert_json_snapshot!(memo);
    }

    #[test]
    fn ics20_transfer_deposit_memo_snapshot() {
        let memo = Ics20TransferDeposit {
            rollup_deposit_address: "an-address-on-the-rollup".to_string(),
        };

        insta::assert_json_snapshot!(memo);
    }
}
