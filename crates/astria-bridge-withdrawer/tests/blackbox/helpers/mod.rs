use astria_core::protocol::transaction::v1alpha1::Action;

mod ethereum;
mod mock_cometbft;
mod mock_sequencer;
mod test_bridge_withdrawer;

fn compare_actions(expected: &Action, actual: &Action) {
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

use test_bridge_withdrawer::default_native_asset;

pub use self::{
    mock_sequencer::MockSequencerServer,
    test_bridge_withdrawer::TestBridgeWithdrawer,
};
