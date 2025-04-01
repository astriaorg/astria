use astria_core::{
    self,
    crypto::ADDRESS_LENGTH,
    primitive::v1::Address,
    protocol::{
        memos::v1::Ics20WithdrawalFromRollup,
        transaction::v1::action::{
            BridgeUnlock,
            Ics20Withdrawal,
        },
    },
};
use ibc_types::core::{
    channel::ChannelId,
    client::Height,
};

mod mock_rollup;
pub(crate) mod test_bridge_signer;

pub(crate) fn make_ics20_withdrawal() -> Ics20Withdrawal {
    let memo = serde_json::to_string(&Ics20WithdrawalFromRollup {
        rollup_block_number: 0,
        // Will be parsed to block hash and event index
        rollup_withdrawal_event_id:
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef.\
             0x0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
        rollup_return_address: "0x0000000000000000000000000000000000000000000000000000000000000000"
            .to_string(),
        memo: String::new(),
    })
    .unwrap();
    Ics20Withdrawal {
        amount: 1000,
        denom: "port-no-0/channel-0/nria".parse().unwrap(),
        destination_chain_address: "astria".to_string(),
        return_address: Address::builder()
            .array([0; ADDRESS_LENGTH])
            .prefix("astria")
            .try_build()
            .unwrap(),
        timeout_height: Height::new(u64::MAX, u64::MAX).unwrap(),
        timeout_time: 100,
        source_channel: ChannelId::new(0),
        fee_asset: "nria".parse().unwrap(),
        memo,
        bridge_address: Some(
            Address::builder()
                .array([0; ADDRESS_LENGTH])
                .prefix("astria")
                .try_build()
                .unwrap(),
        ),
        use_compat_address: false,
    }
}

pub(crate) fn make_bridge_unlock() -> BridgeUnlock {
    BridgeUnlock {
        to: Address::builder()
            .array([0; ADDRESS_LENGTH])
            .prefix("astria")
            .try_build()
            .unwrap(),
        amount: 1000,
        fee_asset: "nria".parse().unwrap(),
        bridge_address: Address::builder()
            .array([0; ADDRESS_LENGTH])
            .prefix("astria")
            .try_build()
            .unwrap(),
        memo: String::new(),
        rollup_block_number: 1,
        rollup_withdrawal_event_id:
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef.\
             0x0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
    }
}
