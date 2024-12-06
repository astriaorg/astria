use std::{
    borrow::Cow,
    fmt::{
        self,
        Debug,
        Formatter,
    },
};

use astria_core::primitive::v1::RollupId as DomainRollupId;
use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use telemetry::display::base64;

use super::{
    Value,
    ValueImpl,
};

#[derive(BorshSerialize, BorshDeserialize)]
pub(in crate::bridge) struct RollupId<'a>(Cow<'a, [u8; 32]>);

impl<'a> Debug for RollupId<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", base64(self.0.as_slice()))
    }
}

impl<'a> From<&'a DomainRollupId> for RollupId<'a> {
    fn from(rollup_id: &'a DomainRollupId) -> Self {
        RollupId(Cow::Borrowed(rollup_id.as_bytes()))
    }
}

impl<'a> From<RollupId<'a>> for DomainRollupId {
    fn from(rollup_id: RollupId<'a>) -> Self {
        DomainRollupId::new(rollup_id.0.into_owned())
    }
}

impl<'a> From<RollupId<'a>> for crate::storage::StoredValue<'a> {
    fn from(rollup_id: RollupId<'a>) -> Self {
        crate::storage::StoredValue::Bridge(Value(ValueImpl::RollupId(rollup_id)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for RollupId<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Bridge(Value(ValueImpl::RollupId(rollup_id))) = value
        else {
            bail!("bridge stored value type mismatch: expected rollup id, found {value:?}");
        };
        Ok(rollup_id)
    }
}
