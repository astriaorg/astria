#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    derive(serde::Deserialize),
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
pub struct UnlockMemo {
    pub block_number: u64,
    #[cfg_attr(
        feature = "serde",
        serde(
            serialize_with = "crate::serde::base64_serialize",
            deserialize_with = "crate::serde::base64_deserialize_array"
        )
    )]
    pub transaction_hash: [u8; 32],
}

/// Memo format for a ICS20 withdrawal from the rollup which is sent to
/// an external IBC-enabled chain.
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    derive(serde::Deserialize),
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
pub struct Ics20WithdrawalFromRollupMemo {
    pub memo: String,
    pub block_number: u64,
    pub rollup_return_address: String,
    #[cfg_attr(
        feature = "serde",
        serde(
            serialize_with = "crate::serde::base64_serialize",
            deserialize_with = "crate::serde::base64_deserialize_array"
        )
    )]
    pub transaction_hash: [u8; 32],
}

/// Memo format for a ICS20 transfer to Astria which is sent to a
/// bridge account, which will then be deposited into the rollup.
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    derive(serde::Deserialize),
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
pub struct Ics20TransferDepositMemo {
    /// the destination address for the deposit on the rollup
    pub rollup_address: String,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bridge_unlock_memo_snapshot() {
        let memo = UnlockMemo {
            block_number: 42,
            transaction_hash: [88; 32],
        };

        insta::assert_json_snapshot!(memo);
    }

    #[test]
    fn ics20_withdrawal_from_rollup_memo_snapshot() {
        let memo = Ics20WithdrawalFromRollupMemo {
            memo: "hello".to_string(),
            block_number: 1,
            rollup_return_address: "rollup-defined".to_string(),
            transaction_hash: [88; 32],
        };

        insta::assert_json_snapshot!(memo);
    }

    #[test]
    fn ics20_transfer_deposit_memo_snapshot() {
        let memo = Ics20TransferDepositMemo {
            rollup_address: "some_rollup_address".to_string(),
        };

        insta::assert_json_snapshot!(memo);
    }
}
