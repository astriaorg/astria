mod address_bytes;
mod block_height;
mod deposits;
mod ibc_prefixed_denom;
mod rollup_id;
mod transaction_id;

use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

pub(in crate::bridge) use self::{
    address_bytes::AddressBytes,
    block_height::BlockHeight,
    deposits::Deposits,
    ibc_prefixed_denom::IbcPrefixedDenom,
    rollup_id::RollupId,
    transaction_id::TransactionId,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Value<'a>(ValueImpl<'a>);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum ValueImpl<'a> {
    RollupId(RollupId<'a>),
    IbcPrefixedDenom(IbcPrefixedDenom<'a>),
    AddressBytes(AddressBytes<'a>),
    BlockHeight(BlockHeight),
    Deposits(Deposits<'a>),
    TransactionId(TransactionId<'a>),
}

#[cfg(test)]
mod tests {
    use astria_core::{
        crypto::ADDRESS_LENGTH,
        primitive::v1::{
            asset::IbcPrefixed as DomainIbcPrefixed,
            RollupId as DomainRollupId,
            TransactionId as DomainTransactionId,
            ROLLUP_ID_LEN,
            TRANSACTION_ID_LEN,
        },
        sequencerblock::v1::block::Deposit as DomainDeposit,
    };
    use insta::assert_snapshot;

    use super::*;
    use crate::test_utils::borsh_then_hex;

    #[test]
    fn value_impl_existing_variants_unchanged() {
        assert_snapshot!(
            "value_impl_rollup_id",
            borsh_then_hex(&ValueImpl::RollupId(
                (&DomainRollupId::new([0; ROLLUP_ID_LEN])).into()
            ))
        );
        assert_snapshot!(
            "value_impl_ibc_prefixed_denom",
            borsh_then_hex(&ValueImpl::IbcPrefixedDenom(
                (&DomainIbcPrefixed::new([0; 32])).into()
            ))
        );
        assert_snapshot!(
            "value_impl_address_bytes",
            borsh_then_hex(&ValueImpl::AddressBytes((&[0; ADDRESS_LENGTH]).into()))
        );
        assert_snapshot!(
            "value_impl_block_height",
            borsh_then_hex(&ValueImpl::BlockHeight(1.into()))
        );
        assert_snapshot!(
            "value_impl_deposits",
            borsh_then_hex(&ValueImpl::Deposits(
                Vec::<DomainDeposit>::new().iter().into()
            ))
        );
        assert_snapshot!(
            "value_impl_transaction_id",
            borsh_then_hex(&ValueImpl::TransactionId(
                (&DomainTransactionId::new([0; TRANSACTION_ID_LEN])).into()
            ))
        );
    }

    // Note: This test must be here instead of in `crate::storage` since `ValueImpl` is not
    // re-exported.
    #[test]
    fn stored_value_bridge_variant_unchanged() {
        use crate::storage::StoredValue;
        assert_snapshot!(
            "stored_value_bridge_variant",
            borsh_then_hex(&StoredValue::Bridge(Value(ValueImpl::BlockHeight(
                1.into()
            ))))
        );
    }
}
