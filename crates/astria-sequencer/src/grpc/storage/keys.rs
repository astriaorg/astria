use astria_core::primitive::v1::RollupId;
use base64::{
    display::Base64Display,
    engine::general_purpose::URL_SAFE,
};

pub(in crate::grpc) fn block_hash_by_height(height: u64) -> String {
    format!("grpc/block_hash/{height}")
}

pub(in crate::grpc) fn sequencer_block_header_by_hash(hash: &[u8; 32]) -> String {
    format!("grpc/block_header/{}", Base64Display::new(hash, &URL_SAFE))
}

pub(in crate::grpc) fn rollup_data_by_hash_and_rollup_id(
    hash: &[u8; 32],
    rollup_id: &RollupId,
) -> String {
    format!(
        "grpc/rollup_data/{}/{rollup_id}",
        Base64Display::new(hash, &URL_SAFE),
    )
}

pub(in crate::grpc) fn rollup_ids_by_hash(hash: &[u8; 32]) -> String {
    format!("grpc/rollup_ids/{}", Base64Display::new(hash, &URL_SAFE))
}

pub(in crate::grpc) fn rollup_transactions_proof_by_hash(hash: &[u8; 32]) -> String {
    format!(
        "grpc/rollup_txs_proof/{}",
        Base64Display::new(hash, &URL_SAFE)
    )
}

pub(in crate::grpc) fn rollup_ids_proof_by_hash(hash: &[u8; 32]) -> String {
    format!(
        "grpc/rollup_ids_proof/{}",
        Base64Display::new(hash, &URL_SAFE)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPONENT_PREFIX: &str = "grpc/";
    const HASH: [u8; 32] = [1; 32];
    const ROLLUP_ID: RollupId = RollupId::new([2; 32]);

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!("block_hash_by_height_key", block_hash_by_height(42));
        insta::assert_snapshot!(
            "sequencer_block_header_by_hash_key",
            sequencer_block_header_by_hash(&HASH)
        );
        insta::assert_snapshot!(
            "rollup_data_by_has_and_id_key",
            rollup_data_by_hash_and_rollup_id(&HASH, &ROLLUP_ID)
        );
        insta::assert_snapshot!("rollup_ids_by_hash_key", rollup_ids_by_hash(&HASH));
        insta::assert_snapshot!(
            "rollup_transactions_proof_by_hash_key",
            rollup_transactions_proof_by_hash(&HASH)
        );
        insta::assert_snapshot!("rollup_ids_proof_by_hash", rollup_ids_proof_by_hash(&HASH));
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
