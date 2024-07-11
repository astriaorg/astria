use astria_core::protocol::transaction::v1alpha1::Action;

mod ethereum;
mod mock_cometbft;
mod mock_sequencer;
mod test_bridge_withdrawer;

pub fn compare_actions(expected: &Action, actual: &Action) {
    match (expected, actual) {
        (Action::BridgeUnlock(expected), Action::BridgeUnlock(actual)) => {
            assert_eq!(expected, actual, "BridgeUnlock actions do not match");
        }
        (Action::Ics20Withdrawal(expected), Action::Ics20Withdrawal(actual)) => {
            assert_eq!(expected, actual, "Ics20Withdrawal actions do not match");
        }
        _ => panic!("Actions do not match"),
    }
}

pub use self::{
    mock_cometbft::*,
    mock_sequencer::MockSequencerServer,
    test_bridge_withdrawer::{
        astria_address,
        default_native_asset,
        make_bridge_unlock_action,
        make_ics20_withdrawal_action,
        TestBridgeWithdrawer,
        TestBridgeWithdrawerConfig,
    },
};
