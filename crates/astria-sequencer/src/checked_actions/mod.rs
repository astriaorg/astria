//! This module defines wrapper types for all actions.
//!
//! Each wrapped action has immutable and mutable checks. Immutable checks are ones that will always
//! succeed or always fail, regardless of global state. Mutable checks have the potential to pass,
//! but later to fail due to changes in global state (e.g. the sudo address being changed).
//!
//! Account balance checks are not part of the mutable checks, as the Mempool ensures actions put
//! forward for execution have sufficient balances.
//!
//! Immutable and mutable checks are run during construction, and should they fail, the type will
//! error on construction.
//!
//! If construction succeeds, the immutable checks are never rerun.
//!
//! The mutable checks are rerun by a checked action when the transaction holding the action is
//! being executed.
//!
//! Failure in this case results in a failed transaction execution and removal from the Mempool.

mod action_ref;
mod bridge;
mod bridge_sudo_change;
mod checked_action;
mod currency_pairs_change;
mod error;
mod fee_asset_change;
mod fee_change;
mod ibc_relay;
mod ibc_relayer_change;
mod ibc_sudo_change;
mod ics20_withdrawal;
mod init_bridge_account;
mod markets_change;
mod recover_ibc_client;
mod rollup_data_submission;
mod sudo_address_change;
#[cfg(test)]
pub(crate) mod test_utils;
mod transfer;
pub(crate) mod utils;
mod validator_update;

use std::fmt::{
    self,
    Debug,
    Formatter,
};

pub(crate) use action_ref::ActionRef;
use astria_core::{
    crypto::ADDRESS_LENGTH,
    primitive::v1::asset::IbcPrefixed,
};
pub(crate) use bridge::{
    CheckedBridgeLock,
    CheckedBridgeTransfer,
    CheckedBridgeUnlock,
};
pub(crate) use bridge_sudo_change::CheckedBridgeSudoChange;
pub(crate) use checked_action::CheckedAction;
pub(crate) use currency_pairs_change::CheckedCurrencyPairsChange;
pub(crate) use error::{
    CheckedActionExecutionError,
    CheckedActionFeeError,
    CheckedActionInitialCheckError,
    CheckedActionMutableCheckError,
};
pub(crate) use fee_asset_change::CheckedFeeAssetChange;
pub(crate) use fee_change::CheckedFeeChange;
pub(crate) use ibc_relay::CheckedIbcRelay;
pub(crate) use ibc_relayer_change::CheckedIbcRelayerChange;
pub(crate) use ibc_sudo_change::CheckedIbcSudoChange;
pub(crate) use ics20_withdrawal::CheckedIcs20Withdrawal;
pub(crate) use init_bridge_account::CheckedInitBridgeAccount;
pub(crate) use markets_change::CheckedMarketsChange;
pub(crate) use recover_ibc_client::CheckedRecoverIbcClient;
pub(crate) use rollup_data_submission::CheckedRollupDataSubmission;
pub(crate) use sudo_address_change::CheckedSudoAddressChange;
pub(crate) use transfer::CheckedTransfer;
pub(crate) use validator_update::{
    use_pre_aspen_validator_updates,
    CheckedValidatorUpdate,
};

use crate::accounts::AddressBytes;

struct TransactionSignerAddressBytes([u8; ADDRESS_LENGTH]);

impl TransactionSignerAddressBytes {
    #[must_use]
    fn as_bytes(&self) -> &[u8; ADDRESS_LENGTH] {
        &self.0
    }
}

impl From<[u8; ADDRESS_LENGTH]> for TransactionSignerAddressBytes {
    fn from(address_bytes: [u8; ADDRESS_LENGTH]) -> Self {
        Self(address_bytes)
    }
}

impl Debug for TransactionSignerAddressBytes {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", telemetry::display::base64(&self.0))
    }
}

impl AddressBytes for TransactionSignerAddressBytes {
    fn address_bytes(&self) -> &[u8; ADDRESS_LENGTH] {
        &self.0
    }
}

/// A trait to be implemented on all checked actions, providing transfer details of the action.
trait AssetTransfer {
    /// The asset and amount of any balance transfer performed by the action.
    fn transfer_asset_and_amount(&self) -> Option<(IbcPrefixed, u128)>;
}
