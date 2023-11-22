use std::sync::OnceLock;

use proto::native::sequencer::v1alpha1::asset::Denom;

pub(crate) static NATIVE_ASSET: OnceLock<Denom> = OnceLock::new();

pub(crate) fn initialize_native_asset(native_asset: &str) {
    if NATIVE_ASSET.get().is_some() {
        tracing::error!("native asset should only be set once");
        return;
    }

    let denom = Denom::from_base_denom(native_asset);
    NATIVE_ASSET
        .set(denom)
        .expect("native asset should only be set once");
}

pub(crate) fn get_native_asset() -> &'static Denom {
    NATIVE_ASSET.get().expect("native asset should be set")
}
