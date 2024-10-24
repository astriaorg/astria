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
    Fees(crate::fees::storage::Value),
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
