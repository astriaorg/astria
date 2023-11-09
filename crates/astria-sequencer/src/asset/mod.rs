use once_cell::sync::OnceCell;
use proto::native::sequencer::v1alpha1::asset::Denom;

pub(crate) static NATIVE_ASSET: OnceCell<Denom> = OnceCell::new();

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
