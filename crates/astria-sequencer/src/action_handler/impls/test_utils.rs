use astria_core::primitive::v1::asset;

pub(super) fn test_asset() -> asset::Denom {
    "test".parse().unwrap()
}
