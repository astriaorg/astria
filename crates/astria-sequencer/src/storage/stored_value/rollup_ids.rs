use astria_core::primitive::v1::RollupId as DomainRollupId;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::{
    RollupId,
    StoredValue,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct RollupIds<'a>(Vec<RollupId<'a>>);

impl<'a, T: Iterator<Item = &'a DomainRollupId>> From<T> for RollupIds<'a> {
    fn from(rollup_id_iter: T) -> Self {
        RollupIds(rollup_id_iter.map(RollupId::from).collect())
    }
}

impl<'a> From<RollupIds<'a>> for Vec<DomainRollupId> {
    fn from(rollup_ids: RollupIds<'a>) -> Self {
        rollup_ids.0.into_iter().map(DomainRollupId::from).collect()
    }
}

impl<'a> TryFrom<StoredValue<'a>> for RollupIds<'a> {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::RollupIds(rollup_ids) = value else {
            return Err(super::type_mismatch("rollup ids", &value));
        };
        Ok(rollup_ids)
    }
}
