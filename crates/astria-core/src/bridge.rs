use crate::primitive::v1::Address;

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
    pub transaction_hash: [u8; 32],
}

/// Memo format for a ICS20 transfer to Astria which is sent to a
/// bridge account, which will then be deposited into the rollup.
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    derive(serde::Deserialize)
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Ics20TransferDepositMemo {
    /// the destination address for the deposit on the rollup
    #[serde(rename = "rollupAddress")]
    pub rollup_address: String,
}
