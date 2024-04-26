mod bridge_lock_action;
pub(crate) mod component;
pub(crate) mod init_bridge_account_action;
pub(crate) mod state_ext;

#[cfg(test)]
pub(crate) use bridge_lock_action::DEPOSIT_BYTE_LEN;
