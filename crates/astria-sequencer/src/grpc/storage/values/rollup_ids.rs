use std::{
    borrow::Cow,
    fmt::{
        self,
        Display,
        Formatter,
    },
};

use astria_core::primitive::v1::RollupId as DomainRollupId;
use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use itertools::Itertools as _;
use telemetry::display::base64;

use super::{
    Value,
    ValueImpl,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(super) struct RollupId<'a>(Cow<'a, [u8; 32]>);

impl<'a> Display for RollupId<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        base64(self.0.as_slice()).fmt(f)
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

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::grpc) struct RollupIds<'a>(Vec<RollupId<'a>>);

impl<'a> Display for RollupIds<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.0.iter().join(", "))
    }
}

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

impl<'a> From<RollupIds<'a>> for crate::storage::StoredValue<'a> {
    fn from(rollup_ids: RollupIds<'a>) -> Self {
        crate::storage::StoredValue::Grpc(Value(ValueImpl::RollupIds(rollup_ids)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for RollupIds<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Grpc(Value(ValueImpl::RollupIds(rollup_ids))) = value
        else {
            bail!("grpc stored value type mismatch: expected rollup ids, found {value}");
        };
        Ok(rollup_ids)
    }
}
