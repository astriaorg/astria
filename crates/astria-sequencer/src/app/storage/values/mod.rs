mod block_height;
mod block_timestamp;
mod chain_id;
mod revision_number;
mod storage_version;

use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

pub(in crate::app) use self::{
    block_height::BlockHeight,
    block_timestamp::BlockTimestamp,
    chain_id::ChainId,
    revision_number::RevisionNumber,
    storage_version::StorageVersion,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Value<'a>(ValueImpl<'a>);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum ValueImpl<'a> {
    ChainId(ChainId<'a>),
    RevisionNumber(RevisionNumber),
    BlockHeight(BlockHeight),
    BlockTimestamp(BlockTimestamp),
    StorageVersion(StorageVersion),
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use insta::assert_snapshot;

    use super::*;

    #[test]
    fn value_impl_chain_id_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_chain_id_discriminant",
            format!(
                "{:?}",
                discriminant(&ValueImpl::ChainId(
                    (&tendermint::chain::Id::try_from("test_prefix".to_string()).unwrap()).into()
                ))
            )
        );
    }

    #[test]
    fn value_impl_revision_number_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_revision_number_discriminant",
            format!("{:?}", discriminant(&ValueImpl::RevisionNumber(1.into())))
        );
    }

    #[test]
    fn value_impl_block_height_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_block_height_discriminant",
            format!("{:?}", discriminant(&ValueImpl::BlockHeight(1.into())))
        );
    }

    #[test]
    fn value_impl_block_timestamp_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_block_timestamp_discriminant",
            format!(
                "{:?}",
                discriminant(&ValueImpl::BlockTimestamp(tendermint::Time::now().into()))
            )
        );
    }

    #[test]
    fn value_impl_storage_version_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_storage_version_discriminant",
            format!("{:?}", discriminant(&ValueImpl::StorageVersion(1.into())))
        );
    }

    #[test]
    fn stored_value_app_discriminant_unchanged() {
        use crate::storage::StoredValue;
        assert_snapshot!(
            "stored_value_app_discriminant",
            format!(
                "{:?}",
                discriminant(&StoredValue::App(Value(ValueImpl::BlockHeight(1.into()))))
            )
        );
    }
}
