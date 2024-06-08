use crate::primitive::v1::Address;

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
