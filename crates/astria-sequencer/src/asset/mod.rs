use once_cell::sync::OnceCell;
use proto::native::sequencer::asset::Denom;

pub(crate) static NATIVE_ASSET: OnceCell<Denom> = OnceCell::new();

pub(crate) fn initialize_native_asset() {
    let denom = Denom::from_base_denom("uria");
    NATIVE_ASSET
        .set(denom)
        .expect("native asset should only be set once");
}
