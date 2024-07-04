use ethers::types::{
    TxHash,
    U64,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::primitive::v1::Address;

/// Memo format for a native bridge unlock from the rollup which is sent to a sequencer-native
/// address.
#[derive(Debug, Serialize, Deserialize)]
/// Allow module name because that's what the action is called.
// TODO: should this be renamed?
#[allow(clippy::module_name_repetitions)]
pub struct BridgeUnlockMemo {
    // TODO: can we get rid of ethers types here?
    pub block_number: U64,
    pub transaction_hash: TxHash,
}

/// Memo format for a ICS20 withdrawal from the rollup which is sent to
/// an external IBC-enabled chain.
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    derive(serde::Deserialize)
)]
pub struct Ics20WithdrawalFromRollupMemo {
    pub memo: String,
    pub bridge_address: Address,
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
    fn ics20_withdrawal_from_rollup_memo_snapshot() {
        let memo = Ics20WithdrawalFromRollupMemo {
            memo: "hello".to_string(),
            bridge_address: Address::builder()
                .array([99; 20])
                .prefix("astria")
                .try_build()
                .unwrap(),
            block_number: 1,
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
