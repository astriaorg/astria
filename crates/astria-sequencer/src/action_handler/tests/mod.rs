use astria_core::primitive::v1::asset;

mod bridge_sudo_change;
mod bridge_unlock;
mod fee_change;
mod ics20_withdrawal;

fn test_asset() -> asset::Denom {
    "test".parse().unwrap()
}
