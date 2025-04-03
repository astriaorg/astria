pub(in crate::app) const CHAIN_ID: &str = "app/chain_id";
pub(in crate::app) const REVISION_NUMBER: &str = "app/revision_number";
pub(in crate::app) const BLOCK_HEIGHT: &str = "app/block_height";
pub(in crate::app) const BLOCK_TIMESTAMP: &str = "app/block_timestamp";
pub(in crate::app) const CONSENSUS_PARAMS: &str = "app/consensus_params";

pub(in crate::app) fn storage_version_by_height(height: u64) -> String {
    format!("app/storage_version/{height}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPONENT_PREFIX: &str = "app/";

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!("chain_id_key", CHAIN_ID);
        insta::assert_snapshot!("revision_number_key", REVISION_NUMBER);
        insta::assert_snapshot!("block_height_key", BLOCK_HEIGHT);
        insta::assert_snapshot!("block_timestamp_key", BLOCK_TIMESTAMP);
        insta::assert_snapshot!("consensus_params_key", CONSENSUS_PARAMS);
        insta::assert_snapshot!("storage_version_key", storage_version_by_height(42));
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(CHAIN_ID.starts_with(COMPONENT_PREFIX));
        assert!(REVISION_NUMBER.starts_with(COMPONENT_PREFIX));
        assert!(BLOCK_HEIGHT.starts_with(COMPONENT_PREFIX));
        assert!(BLOCK_TIMESTAMP.starts_with(COMPONENT_PREFIX));
        assert!(CONSENSUS_PARAMS.starts_with(COMPONENT_PREFIX));
        assert!(storage_version_by_height(42).starts_with(COMPONENT_PREFIX));
    }
}
