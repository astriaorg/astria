use std::sync::Arc;

use borsh::BorshSerialize;

use super::{
    Change,
    ChangeName,
    UpgradeName,
};
use crate::{
    generated::upgrades::v1::{
        upgrade1::{
            ConnectOracleChange as RawConnectOracleChange,
            ValidatorUpdateActionChange as RawValidatorUpdateActionChange,
        },
        BaseUpgradeInfo as RawBaseUpgradeInfo,
        Upgrade1 as RawUpgrade1,
    },
    protocol::genesis::v1::{
        ConnectGenesis,
        ConnectGenesisError,
    },
    Protobuf,
};

#[derive(Clone, Debug)]
pub struct Upgrade1 {
    activation_height: u64,
    app_version: u64,
    connect_oracle_change: ConnectOracleChange,
    validator_update_action_change: ValidatorUpdateActionChange,
}

impl Upgrade1 {
    pub const NAME: UpgradeName = UpgradeName::new("upgrade_1");

    #[must_use]
    pub fn activation_height(&self) -> u64 {
        self.activation_height
    }

    #[must_use]
    pub fn app_version(&self) -> u64 {
        self.app_version
    }

    #[must_use]
    pub fn connect_oracle_change(&self) -> &ConnectOracleChange {
        &self.connect_oracle_change
    }

    #[must_use]
    pub fn validator_update_action_change(&self) -> &ValidatorUpdateActionChange {
        &self.validator_update_action_change
    }

    pub fn changes(&self) -> impl Iterator<Item = &'_ dyn Change> {
        Some(&self.connect_oracle_change as &dyn Change)
            .into_iter()
            .chain(Some(&self.validator_update_action_change as &dyn Change))
    }
}

impl Protobuf for Upgrade1 {
    type Error = Error;
    type Raw = RawUpgrade1;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let RawBaseUpgradeInfo {
            activation_height,
            app_version,
        } = raw
            .base_info
            .as_ref()
            .ok_or_else(Error::no_base_info)?
            .clone();

        let connect_oracle_change = raw
            .connect_oracle_change
            .as_ref()
            .ok_or_else(Error::no_connect_oracle_change)?;

        let genesis = connect_oracle_change
            .genesis
            .as_ref()
            .ok_or_else(Error::no_connect_genesis)
            .and_then(|raw_genesis| {
                ConnectGenesis::try_from_raw_ref(raw_genesis).map_err(Error::connect_genesis)
            })?;

        if raw.validator_update_action_change.is_none() {
            return Err(Error::no_validator_update_action_change());
        }

        let connect_oracle_change = ConnectOracleChange {
            activation_height,
            app_version,
            genesis: Arc::new(genesis),
        };

        let validator_update_action_change = ValidatorUpdateActionChange {
            activation_height,
            app_version,
        };

        Ok(Self {
            activation_height,
            app_version,
            connect_oracle_change,
            validator_update_action_change,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let base_info = Some(RawBaseUpgradeInfo {
            activation_height: self.activation_height,
            app_version: self.app_version,
        });
        let connect_oracle_change = Some(RawConnectOracleChange {
            genesis: Some(self.connect_oracle_change.genesis.to_raw()),
        });
        RawUpgrade1 {
            base_info,
            connect_oracle_change,
            validator_update_action_change: Some(RawValidatorUpdateActionChange {}),
        }
    }
}

/// This change enables vote extensions and starts to provide price feed data from the Connect
/// Oracle sidecar (if enabled) via the vote extensions.
///
/// The vote extensions are enabled in the block immediately after `activation_height`, meaning the
/// price feed data is available no earlier than two blocks after `activation_height`.
#[derive(Clone, Debug, BorshSerialize)]
pub struct ConnectOracleChange {
    activation_height: u64,
    app_version: u64,
    genesis: Arc<ConnectGenesis>,
}

impl ConnectOracleChange {
    pub const NAME: ChangeName = ChangeName::new("connect_oracle_change");

    #[must_use]
    pub fn genesis(&self) -> &Arc<ConnectGenesis> {
        &self.genesis
    }
}

impl Change for ConnectOracleChange {
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

/// This change introduces new sequencer `Action`s to support updating the validator set.
#[derive(Clone, Debug, BorshSerialize)]
pub struct ValidatorUpdateActionChange {
    activation_height: u64,
    app_version: u64,
}

impl ValidatorUpdateActionChange {
    pub const NAME: ChangeName = ChangeName::new("validator_update_action_change");
}

impl Change for ValidatorUpdateActionChange {
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

/// An error when transforming a [`RawConnectOracleUpgrade`] into a [`ConnectOracleChange`].
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error(ErrorKind);

impl Error {
    fn no_base_info() -> Self {
        Self(ErrorKind::FieldNotSet("base_info"))
    }

    fn no_connect_oracle_change() -> Self {
        Self(ErrorKind::FieldNotSet("connect_oracle_change"))
    }

    fn no_validator_update_action_change() -> Self {
        Self(ErrorKind::FieldNotSet("validator_update_action_change"))
    }

    fn no_connect_genesis() -> Self {
        Self(ErrorKind::FieldNotSet("connect_oracle_change.genesis"))
    }

    fn connect_genesis(source: ConnectGenesisError) -> Self {
        Self(ErrorKind::ConnectGenesis {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("`{0}` field was not set")]
    FieldNotSet(&'static str),
    #[error("`connect_oracle_change.genesis` field was invalid")]
    ConnectGenesis { source: ConnectGenesisError },
}
