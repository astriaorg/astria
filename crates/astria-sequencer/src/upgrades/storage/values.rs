use std::{
    borrow::Cow,
    fmt::{
        self,
        Debug,
        Formatter,
    },
};

use astria_core::upgrades::v1::{
    ChangeHash as DomainChangeHash,
    ChangeInfo as DomainChangeInfo,
    ChangeName as DomainChangeName,
};
use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use telemetry::display::base64;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Value<'a>(ValueImpl<'a>);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum ValueImpl<'a> {
    ChangeInfo(ChangeInfo<'a>),
}

#[derive(BorshSerialize, BorshDeserialize)]
pub(in crate::upgrades) struct ChangeInfo<'a> {
    name: Cow<'a, str>,
    activation_height: u64,
    app_version: u64,
    hash: Cow<'a, [u8; DomainChangeHash::LENGTH]>,
}

impl Debug for ChangeInfo<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChangeInfo")
            .field("name", &self.name)
            .field("activation_height", &self.activation_height)
            .field("app_version", &self.app_version)
            .field("hash", &format!("{}", base64(self.hash.as_slice())))
            .finish()
    }
}

impl<'a> From<&'a DomainChangeInfo> for ChangeInfo<'a> {
    fn from(change_info: &'a DomainChangeInfo) -> Self {
        ChangeInfo {
            name: Cow::Borrowed(change_info.name.as_str()),
            activation_height: change_info.activation_height,
            app_version: change_info.app_version,
            hash: Cow::Borrowed(change_info.hash.as_bytes()),
        }
    }
}

impl<'a> From<ChangeInfo<'a>> for DomainChangeInfo {
    fn from(change_info: ChangeInfo<'a>) -> Self {
        DomainChangeInfo {
            name: DomainChangeName::from(change_info.name.into_owned()),
            activation_height: change_info.activation_height,
            app_version: change_info.app_version,
            hash: DomainChangeHash::new(change_info.hash.into_owned()),
        }
    }
}

impl<'a> From<ChangeInfo<'a>> for crate::storage::StoredValue<'a> {
    fn from(change_info: ChangeInfo<'a>) -> Self {
        crate::storage::StoredValue::Upgrades(Value(ValueImpl::ChangeInfo(change_info)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for ChangeInfo<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Upgrades(Value(ValueImpl::ChangeInfo(change_info))) =
            value
        else {
            bail!("upgrades stored value type mismatch: expected change info, found {value:?}");
        };
        Ok(change_info)
    }
}
