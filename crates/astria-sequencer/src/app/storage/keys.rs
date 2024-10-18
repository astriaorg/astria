pub(in crate::app) const CHAIN_ID: &str = "app/chain_id";
pub(in crate::app) const REVISION_NUMBER: &str = "app/revision_number";
pub(in crate::app) const BLOCK_HEIGHT: &str = "app/block_height";
pub(in crate::app) const BLOCK_TIMESTAMP: &str = "app/block_timestamp";

pub(in crate::app) fn storage_version_by_height(height: u64) -> String {
    format!("app/storage_version/{height}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPONENT_PREFIX: &str = "app/";

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!(CHAIN_ID);
        insta::assert_snapshot!(REVISION_NUMBER);
        insta::assert_snapshot!(BLOCK_HEIGHT);
        insta::assert_snapshot!(BLOCK_TIMESTAMP);
        insta::assert_snapshot!(storage_version_by_height(42));
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(CHAIN_ID.starts_with(COMPONENT_PREFIX));
        assert!(REVISION_NUMBER.starts_with(COMPONENT_PREFIX));
        assert!(BLOCK_HEIGHT.starts_with(COMPONENT_PREFIX));
        assert!(BLOCK_TIMESTAMP.starts_with(COMPONENT_PREFIX));
        assert!(storage_version_by_height(42).starts_with(COMPONENT_PREFIX));
    }
}
