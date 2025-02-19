mod block_hash;
mod proof;
mod rollup_ids;
mod rollup_transactions;
mod sequencer_block_header;

use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

pub(in crate::grpc) use self::{
    block_hash::BlockHash,
    proof::Proof,
    rollup_ids::RollupIds,
    rollup_transactions::RollupTransactions,
    sequencer_block_header::SequencerBlockHeader,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Value<'a>(ValueImpl<'a>);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum ValueImpl<'a> {
    RollupIds(RollupIds<'a>),
    BlockHash(BlockHash<'a>),
    SequencerBlockHeader(SequencerBlockHeader<'a>),
    RollupTransactions(RollupTransactions<'a>),
    Proof(Proof<'a>),
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use astria_core::{
        primitive::v1::RollupId as DomainRollupId,
        sequencerblock::v1::block::{
            Hash,
            RollupTransactions as DomainRollupTransactions,
            RollupTransactionsParts,
            SequencerBlockHeader as DomainSequencerBlockHeader,
            SequencerBlockHeaderParts,
        },
    };
    use insta::assert_snapshot;

    use super::*;

    #[test]
    fn value_impl_rollup_ids_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_rollup_ids_discriminant",
            format!(
                "{:?}",
                discriminant(&ValueImpl::RollupIds(
                    Vec::<DomainRollupId>::new().iter().into()
                ))
            )
        );
    }

    #[test]
    fn value_impl_block_hash_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_block_hash_discriminant",
            format!(
                "{:?}",
                discriminant(&ValueImpl::BlockHash((&Hash::new([0; 32])).into()))
            )
        );
    }

    #[test]
    fn value_impl_sequencer_block_header_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_sequencer_block_header_discriminant",
            format!(
                "{:?}",
                discriminant(&ValueImpl::SequencerBlockHeader(
                    (&DomainSequencerBlockHeader::unchecked_from_parts(
                        SequencerBlockHeaderParts {
                            chain_id: "test_chain_id".to_string().try_into().unwrap(),
                            height: 0u32.into(),
                            time: tendermint::Time::now(),
                            rollup_transactions_root: [0; 32],
                            data_hash: [0; 32],
                            proposer_address: tendermint::account::Id::new([0; 20])
                        }
                    ))
                        .into()
                ))
            )
        );
    }

    #[test]
    fn value_impl_rollup_transactions_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_rollup_transactions_discriminant",
            format!(
                "{:?}",
                discriminant(&ValueImpl::RollupTransactions(
                    (&DomainRollupTransactions::unchecked_from_parts(RollupTransactionsParts {
                        rollup_id: DomainRollupId::new([0; 32]),
                        transactions: Vec::new(),
                        proof: merkle::Proof::unchecked_from_parts(merkle::audit::UncheckedProof {
                            audit_path: vec![],
                            leaf_index: 1,
                            tree_size: 1,
                        }),
                    }))
                        .into()
                ))
            )
        );
    }

    #[test]
    fn value_impl_proof_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_proof_discriminant",
            format!(
                "{:?}",
                discriminant(&ValueImpl::Proof(
                    (&merkle::Proof::unchecked_from_parts(merkle::audit::UncheckedProof {
                        audit_path: vec![],
                        leaf_index: 1,
                        tree_size: 1,
                    }))
                        .into()
                ))
            )
        );
    }

    // Note: This test must be here instead of in `crate::storage` since `ValueImpl` is not
    // re-exported.
    #[test]
    fn stored_value_grpc_discriminant_unchanged() {
        use crate::storage::StoredValue;
        assert_snapshot!(
            "stored_value_grpc_discriminant",
            format!(
                "{:?}",
                discriminant(&StoredValue::Grpc(Value(ValueImpl::BlockHash(
                    (&Hash::new([0; 32])).into()
                ))))
            )
        );
    }
}
