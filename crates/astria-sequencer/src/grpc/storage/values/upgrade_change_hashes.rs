use std::{
    borrow::Cow,
    fmt::{
        self,
        Debug,
        Formatter,
    },
};

use astria_core::upgrades::v1::ChangeHash as DomainChangeHash;
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
pub(super) struct ChangeHash<'a>(Cow<'a, [u8; 32]>);

impl Debug for ChangeHash<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", base64(self.0.as_slice()))
    }
}

impl<'a> From<&'a DomainChangeHash> for ChangeHash<'a> {
    fn from(change_hash: &'a DomainChangeHash) -> Self {
        ChangeHash(Cow::Borrowed(change_hash.as_bytes()))
    }
}

impl<'a> From<ChangeHash<'a>> for DomainChangeHash {
    fn from(change_hash: ChangeHash<'a>) -> Self {
        DomainChangeHash::new(change_hash.0.into_owned())
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::grpc) struct UpgradeChangeHashes<'a>(Vec<ChangeHash<'a>>);

impl<'a, T: Iterator<Item = &'a DomainChangeHash>> From<T> for UpgradeChangeHashes<'a> {
    fn from(change_hashes_iter: T) -> Self {
        UpgradeChangeHashes(change_hashes_iter.map(ChangeHash::from).collect())
    }
}

impl<'a> From<UpgradeChangeHashes<'a>> for Vec<DomainChangeHash> {
    fn from(change_hashes: UpgradeChangeHashes<'a>) -> Self {
        change_hashes
            .0
            .into_iter()
            .map(DomainChangeHash::from)
            .collect()
    }
}

impl<'a> From<UpgradeChangeHashes<'a>> for crate::storage::StoredValue<'a> {
    fn from(change_hashes: UpgradeChangeHashes<'a>) -> Self {
        crate::storage::StoredValue::Grpc(Value(ValueImpl::UpgradeChangeHashes(change_hashes)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for UpgradeChangeHashes<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Grpc(Value(ValueImpl::UpgradeChangeHashes(change_hashes))) =
            value
        else {
            bail!(
                "grpc stored value type mismatch: expected upgrade change hashes, found {value:?}"
            );
        };
        Ok(change_hashes)
    }
}
