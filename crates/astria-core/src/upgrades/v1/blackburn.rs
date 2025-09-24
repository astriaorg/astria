use borsh::BorshSerialize;

use super::{
    Change,
    ChangeName,
    UpgradeName,
};
use crate::{
    generated::upgrades::v1::{
        blackburn::{
            AllowIbcRelayToFail as RawAllowIbcRelayToFail,
            DisableableBridgeAccountDeposits as RawDisableableBridgeAccountDeposits,
            Ics20TransferActionChange as RawIcs20TransferActionChange,
        },
        BaseUpgradeInfo as RawBaseUpgradeInfo,
        Blackburn as RawBlackburn,
    },
    Protobuf,
};

#[derive(Clone, Debug)]
pub struct Blackburn {
    activation_height: u64,
    app_version: u64,
    ics20_transfer_action_change: Ics20TransferActionChange,
    allow_ibc_relay_to_fail: AllowIbcRelayToFail,
    disableable_bridge_account_deposits: DisableableBridgeAccountDeposits,
}

impl Blackburn {
    pub const NAME: UpgradeName = UpgradeName::new("blackburn");

    #[must_use]
    pub fn activation_height(&self) -> u64 {
        self.activation_height
    }

    #[must_use]
    pub fn app_version(&self) -> u64 {
        self.app_version
    }

    #[must_use]
    pub fn ics20_transfer_action_change(&self) -> &Ics20TransferActionChange {
        &self.ics20_transfer_action_change
    }

    #[must_use]
    pub fn allow_ibc_relay_to_fail(&self) -> &AllowIbcRelayToFail {
        &self.allow_ibc_relay_to_fail
    }

    #[must_use]
    pub fn disableable_bridge_account_deposits(&self) -> &DisableableBridgeAccountDeposits {
        &self.disableable_bridge_account_deposits
    }

    pub fn changes(&self) -> impl Iterator<Item = &'_ dyn Change> {
        Some(&self.ics20_transfer_action_change as &dyn Change)
            .into_iter()
            .chain(Some(&self.allow_ibc_relay_to_fail as &dyn Change))
            .chain(Some(
                &self.disableable_bridge_account_deposits as &dyn Change,
            ))
    }
}

impl Protobuf for Blackburn {
    type Error = Error;
    type Raw = RawBlackburn;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let RawBaseUpgradeInfo {
            activation_height,
            app_version,
        } = *raw.base_info.as_ref().ok_or_else(Error::no_base_info)?;

        if raw.ics20_transfer_action_change.is_none() {
            return Err(Error::no_ics20_transfer_action_change());
        }

        if raw.allow_ibc_relay_to_fail.is_none() {
            return Err(Error::no_allow_ibc_relay_to_fail());
        }

        if raw.disableable_bridge_account_deposits.is_none() {
            return Err(Error::no_disableable_bridge_account_deposits());
        }

        let ics20_transfer_action_change = Ics20TransferActionChange {
            activation_height,
            app_version,
        };

        let allow_ibc_relay_to_fail = AllowIbcRelayToFail {
            activation_height,
            app_version,
        };

        let disableable_bridge_account_deposits = DisableableBridgeAccountDeposits {
            activation_height,
            app_version,
        };

        Ok(Self {
            activation_height,
            app_version,
            ics20_transfer_action_change,
            allow_ibc_relay_to_fail,
            disableable_bridge_account_deposits,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let base_info = Some(RawBaseUpgradeInfo {
            activation_height: self.activation_height,
            app_version: self.app_version,
        });
        RawBlackburn {
            base_info,
            ics20_transfer_action_change: Some(RawIcs20TransferActionChange {}),
            allow_ibc_relay_to_fail: Some(RawAllowIbcRelayToFail {}),
            disableable_bridge_account_deposits: Some(RawDisableableBridgeAccountDeposits {}),
        }
    }
}

#[derive(Clone, Debug, BorshSerialize)]
pub struct DisableableBridgeAccountDeposits {
    activation_height: u64,
    app_version: u64,
}

impl DisableableBridgeAccountDeposits {
    pub const NAME: ChangeName = ChangeName::new("disableable_bridge_account_deposits");
}

impl Change for DisableableBridgeAccountDeposits {
    fn name(&self) -> ChangeName {
        Self::NAME.clone()
    }

    fn activation_height(&self) -> u64 {
        self.activation_height
    }

    fn app_version(&self) -> u64 {
        self.app_version
    }
}

/// This change alters the `IbcRelay` action to only allow denoms that are allowed fee assets in
/// ICS20 transfers.
#[derive(Clone, Debug, BorshSerialize)]
pub struct Ics20TransferActionChange {
    activation_height: u64,
    app_version: u64,
}

impl Ics20TransferActionChange {
    pub const NAME: ChangeName = ChangeName::new("ics20_transfer_action_change");
}

impl Change for Ics20TransferActionChange {
    fn name(&self) -> ChangeName {
        Self::NAME.clone()
    }

    fn activation_height(&self) -> u64 {
        self.activation_height
    }

    fn app_version(&self) -> u64 {
        self.app_version
    }
}

/// This change alters the `IbcRelay` action to allow it to fail execution, but still be included
/// in the CometBFT `txs`.
#[derive(Clone, Debug, BorshSerialize)]
pub struct AllowIbcRelayToFail {
    activation_height: u64,
    app_version: u64,
}

impl AllowIbcRelayToFail {
    pub const NAME: ChangeName = ChangeName::new("allow_ibc_relay_to_fail");
}

impl Change for AllowIbcRelayToFail {
    fn name(&self) -> ChangeName {
        Self::NAME.clone()
    }

    fn activation_height(&self) -> u64 {
        self.activation_height
    }

    fn app_version(&self) -> u64 {
        self.app_version
    }
}

/// An error when transforming a [`RawBlackburn`] into a [`Blackburn`].
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error(ErrorKind);

impl Error {
    fn no_base_info() -> Self {
        Self(ErrorKind::FieldNotSet("base_info"))
    }

    fn no_ics20_transfer_action_change() -> Self {
        Self(ErrorKind::FieldNotSet("ics20_transfer_action_change"))
    }

    fn no_allow_ibc_relay_to_fail() -> Self {
        Self(ErrorKind::FieldNotSet("allow_ibc_relay_to_fail"))
    }

    fn no_disableable_bridge_account_deposits() -> Self {
        Self(ErrorKind::FieldNotSet(
            "disableable_bridge_account_deposits",
        ))
    }
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("`{0}` field was not set")]
    FieldNotSet(&'static str),
}

#[cfg(test)]
mod tests {
    use crate::upgrades::{
        test_utils::UpgradesBuilder,
        v1::change::DeterministicSerialize,
    };

    #[test]
    fn serialized_ics20_transfer_action_change_should_not_change() {
        let ics20_transfer_action_change = UpgradesBuilder::new()
            .build()
            .blackburn()
            .unwrap()
            .ics20_transfer_action_change()
            .to_vec();
        let serialized_ics20_transfer_action_change = hex::encode(ics20_transfer_action_change);
        insta::assert_snapshot!(
            "ics20_transfer_action_change",
            serialized_ics20_transfer_action_change
        );
    }

    #[test]
    fn serialized_allow_ibc_relay_to_fail_should_not_change() {
        let allow_ibc_relay_to_fail = UpgradesBuilder::new()
            .build()
            .blackburn()
            .unwrap()
            .allow_ibc_relay_to_fail()
            .to_vec();
        let serialized_allow_ibc_relay_to_fail = hex::encode(allow_ibc_relay_to_fail);
        insta::assert_snapshot!(
            "allow_ibc_relay_to_fail",
            serialized_allow_ibc_relay_to_fail
        );
    }
}
