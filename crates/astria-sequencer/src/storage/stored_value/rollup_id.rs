use std::borrow::Cow;

use astria_core::primitive::v1::RollupId as DomainRollupId;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::StoredValue;

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct RollupId<'a>(Cow<'a, [u8; 32]>);

impl<'a> From<&'a DomainRollupId> for RollupId<'a> {
    fn from(rollup_id: &'a DomainRollupId) -> Self {
        RollupId(Cow::Borrowed(rollup_id.get()))
    }
}

impl<'a> From<RollupId<'a>> for DomainRollupId {
    fn from(rollup_id: RollupId<'a>) -> Self {
        DomainRollupId::new(rollup_id.0.into_owned())
    }
}

impl<'a> TryFrom<StoredValue<'a>> for RollupId<'a> {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::RollupId(rollup_id) = value else {
            return Err(super::type_mismatch("rollup id", &value));
        };
        Ok(rollup_id)
    }
}
