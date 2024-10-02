pub(in crate::app) const CHAIN_ID_KEY: &str = "app/chain_id";
pub(in crate::app) const REVISION_NUMBER_KEY: &str = "app/revision_number";
pub(in crate::app) const BLOCK_HEIGHT_KEY: &str = "app/block_height";
pub(in crate::app) const BLOCK_TIMESTAMP_KEY: &str = "app/block_timestamp";

pub(in crate::app) fn storage_version_by_height_key(height: u64) -> Vec<u8> {
    format!("app/storage_version/{height}").into_bytes()
}

#[cfg(test)]
mod tests {
    use telemetry::display::base64;

    use super::*;

    const COMPONENT_PREFIX: &str = "app/";

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!(CHAIN_ID_KEY);
        insta::assert_snapshot!(REVISION_NUMBER_KEY);
        insta::assert_snapshot!(BLOCK_HEIGHT_KEY);
        insta::assert_snapshot!(BLOCK_TIMESTAMP_KEY);
        insta::assert_snapshot!(base64(storage_version_by_height_key(42)));
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(CHAIN_ID_KEY.starts_with(COMPONENT_PREFIX));
        assert!(REVISION_NUMBER_KEY.starts_with(COMPONENT_PREFIX));
        assert!(BLOCK_HEIGHT_KEY.starts_with(COMPONENT_PREFIX));
        assert!(BLOCK_TIMESTAMP_KEY.starts_with(COMPONENT_PREFIX));
        assert!(storage_version_by_height_key(42).starts_with(COMPONENT_PREFIX.as_bytes()));
    }
}
