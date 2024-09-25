use std::fmt::{
    self,
    Display,
    Formatter,
};

use astria_eyre::{
    eyre::WrapErr as _,
    Result,
};
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) enum StoredValue<'a> {
    Unit,
    Address(crate::address::storage::Value<'a>),
    Assets(crate::assets::storage::Value<'a>),
    Accounts(crate::accounts::storage::Value),
    Authority(crate::authority::storage::Value<'a>),
    Sequence(crate::sequence::storage::Value),
    Bridge(crate::bridge::storage::Value<'a>),
    Ibc(crate::ibc::storage::Value<'a>),
    App(crate::app::storage::Value<'a>),
    Grpc(crate::grpc::storage::Value<'a>),
}

impl<'a> StoredValue<'a> {
    pub(crate) fn serialize(&self) -> Result<Vec<u8>> {
        borsh::to_vec(&self).wrap_err("failed to serialize stored value")
    }

    pub(crate) fn deserialize(bytes: &[u8]) -> Result<Self> {
        borsh::from_slice(bytes).wrap_err("failed to deserialize stored value")
    }
}

impl<'a> Display for StoredValue<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            StoredValue::Unit => write!(f, "unit stored value"),
            StoredValue::Address(value) => write!(f, "address stored value {value}"),
            StoredValue::Assets(value) => write!(f, "assets stored value {value}"),
            StoredValue::Accounts(value) => write!(f, "accounts stored value {value}"),
            StoredValue::Authority(value) => write!(f, "authority stored value {value}"),
            StoredValue::Sequence(value) => write!(f, "sequence stored value {value}"),
            StoredValue::Bridge(value) => write!(f, "bridge stored value {value}"),
            StoredValue::Ibc(value) => write!(f, "ibc stored value {value}"),
            StoredValue::App(value) => write!(f, "app stored value {value}"),
            StoredValue::Grpc(value) => write!(f, "grpc stored value {value}"),
        }
    }
}
