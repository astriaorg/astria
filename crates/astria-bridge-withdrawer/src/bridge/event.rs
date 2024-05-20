use astria_core::protocol::transaction::v1alpha1::{
    action::BridgeUnlockAction,
    Action,
};

pub(crate) struct Event;

impl From<Event> for Action {
    fn from(e: Event) -> Self {
        Action::BridgeUnlock(BridgeUnlockAction::default())
    }
}
