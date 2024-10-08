use astria_core::primitive::v1::RollupId;

pub(in crate::grpc) fn block_hash_by_height(height: u64) -> Vec<u8> {
    [b"grpc/block_hash/".as_slice(), &height.to_le_bytes()].concat()
}

pub(in crate::grpc) fn sequencer_block_header_by_hash(hash: &[u8; 32]) -> Vec<u8> {
    [b"grpc/block_header/", hash.as_slice()].concat()
}

pub(in crate::grpc) fn rollup_data_by_hash_and_rollup_id(
    hash: &[u8; 32],
    rollup_id: &RollupId,
) -> Vec<u8> {
    [b"grpc/rollup_data/", hash.as_slice(), rollup_id.as_ref()].concat()
}

pub(in crate::grpc) fn rollup_ids_by_hash(hash: &[u8; 32]) -> Vec<u8> {
    [b"grpc/rollup_ids/", hash.as_slice()].concat()
}

pub(in crate::grpc) fn rollup_transactions_proof_by_hash(hash: &[u8; 32]) -> Vec<u8> {
    [b"grpc/rollup_txs_proof/", hash.as_slice()].concat()
}

pub(in crate::grpc) fn rollup_ids_proof_by_hash(hash: &[u8; 32]) -> Vec<u8> {
    [b"grpc/rollup_ids_proof/", hash.as_slice()].concat()
}

#[cfg(test)]
mod tests {
    use telemetry::display::base64;

    use super::*;

    const COMPONENT_PREFIX: &[u8] = b"grpc/";
    const HASH: [u8; 32] = [1; 32];
    const ROLLUP_ID: RollupId = RollupId::new([2; 32]);

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!(base64(&block_hash_by_height(42)));
        insta::assert_snapshot!(base64(&sequencer_block_header_by_hash(&HASH)));
        insta::assert_snapshot!(base64(&rollup_data_by_hash_and_rollup_id(
            &HASH, &ROLLUP_ID
        )));
        insta::assert_snapshot!(base64(&rollup_ids_by_hash(&HASH)));
        insta::assert_snapshot!(base64(&rollup_transactions_proof_by_hash(&HASH)));
        insta::assert_snapshot!(base64(&rollup_ids_proof_by_hash(&HASH)));
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(block_hash_by_height(42).starts_with(COMPONENT_PREFIX));
        assert!(sequencer_block_header_by_hash(&HASH).starts_with(COMPONENT_PREFIX));
        assert!(rollup_data_by_hash_and_rollup_id(&HASH, &ROLLUP_ID).starts_with(COMPONENT_PREFIX));
        assert!(rollup_ids_by_hash(&HASH).starts_with(COMPONENT_PREFIX));
        assert!(rollup_transactions_proof_by_hash(&HASH).starts_with(COMPONENT_PREFIX));
        assert!(rollup_ids_proof_by_hash(&HASH).starts_with(COMPONENT_PREFIX));
    }
}
