use std::{
    borrow::Cow,
    fmt::{
        self,
        Debug,
        Formatter,
    },
};

use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use bytes::Bytes;
use telemetry::display::base64;

use super::{
    Value,
    ValueImpl,
};

#[derive(BorshSerialize, BorshDeserialize)]
pub(in crate::grpc) struct ExtendedCommitInfo<'a>(Cow<'a, Bytes>);

impl From<ExtendedCommitInfo<'_>> for Bytes {
    fn from(commit_info: ExtendedCommitInfo) -> Self {
        commit_info.0.into_owned()
    }
}

impl<'a> From<&'a Bytes> for ExtendedCommitInfo<'a> {
    fn from(commit_info: &'a Bytes) -> Self {
        ExtendedCommitInfo(Cow::Borrowed(commit_info))
    }
}

impl<'a> From<ExtendedCommitInfo<'a>> for crate::storage::StoredValue<'a> {
    fn from(commit_info: ExtendedCommitInfo<'a>) -> Self {
        crate::storage::StoredValue::Grpc(Value(ValueImpl::ExtendedCommitInfo(commit_info)))
    }
}

impl Debug for ExtendedCommitInfo<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", base64(self.0.as_ref()))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for ExtendedCommitInfo<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Grpc(Value(ValueImpl::ExtendedCommitInfo(commit_info))) =
            value
        else {
            bail!(
                "grpc stored value type mismatch: expected extended commit info bytes, found \
                 {value:?}"
            );
        };
        Ok(commit_info)
    }
}
