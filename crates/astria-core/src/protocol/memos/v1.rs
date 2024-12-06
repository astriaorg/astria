pub use crate::generated::astria::protocol::memos::v1::{
    Ics20TransferDeposit,
    Ics20WithdrawalFromRollup,
};

#[cfg(all(feature = "serde", test))]
mod tests {
    use super::*;

    #[test]
    fn ics20_withdrawal_from_rollup_memo_snapshot() {
        let memo = Ics20WithdrawalFromRollup {
            rollup_block_number: 1,
            rollup_return_address: "a-rollup-defined-address".to_string(),
            rollup_withdrawal_event_id: "a-rollup-defined-hash".to_string(),
            memo: "hello".to_string(),
        };

        insta::assert_json_snapshot!("ics20_withdrawal_from_rollup_memo", memo);
    }

    #[test]
    fn ics20_transfer_deposit_memo_snapshot() {
        let memo = Ics20TransferDeposit {
            rollup_deposit_address: "an-address-on-the-rollup".to_string(),
        };

        insta::assert_json_snapshot!("ics20_transfer_deposit_memo", memo);
    }
}
