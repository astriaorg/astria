//! This module provides the checked actions related to moving bridge funds.
//!
//! This exists as a separate submodule inside `checked_actions` so that logic shared by these three
//! actions is not public outside of this `bridge` module.

mod bridge_lock;
mod bridge_transfer;
mod bridge_unlock;

pub(crate) use bridge_lock::CheckedBridgeLock;
pub(crate) use bridge_transfer::CheckedBridgeTransfer;
pub(crate) use bridge_unlock::CheckedBridgeUnlock;
