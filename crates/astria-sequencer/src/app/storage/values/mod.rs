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
    use insta::assert_snapshot;

    use super::*;
    use crate::test_utils::borsh_then_hex;

    #[test]
    fn value_impl_existing_variants_unchanged() {
        assert_snapshot!(
            "value_impl_chain_id",
            borsh_then_hex(&ValueImpl::ChainId(
                (&tendermint::chain::Id::try_from("test_prefix".to_string()).unwrap()).into()
            ))
        );
        assert_snapshot!(
            "value_impl_revision_number",
            borsh_then_hex(&ValueImpl::RevisionNumber(1.into()))
        );
        assert_snapshot!(
            "value_impl_block_height",
            borsh_then_hex(&ValueImpl::BlockHeight(1.into()))
        );
        assert_snapshot!(
            "value_impl_block_timestamp",
            borsh_then_hex(&ValueImpl::BlockTimestamp(
                tendermint::Time::unix_epoch().into()
            ))
        );
        assert_snapshot!(
            "value_impl_storage_version",
            borsh_then_hex(&ValueImpl::StorageVersion(1.into()))
        );
    }

    #[test]
    fn stored_value_app_variant_unchanged() {
        use crate::storage::StoredValue;
        assert_snapshot!(
            "stored_value_app_variant",
            borsh_then_hex(&StoredValue::App(Value(ValueImpl::BlockHeight(1.into()))))
        );
    }
}
