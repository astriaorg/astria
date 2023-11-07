use std::collections::BTreeMap;

use once_cell::sync::OnceCell;
use proto::native::sequencer::asset::*;

pub(crate) static KNOWN_ASSETS: OnceCell<BTreeMap<Id, String>> = OnceCell::new();

pub(crate) fn initialize_known_assets() {
    let mut known_assets = BTreeMap::new();
    let denom = Denom::from_base_denom("utia");
    known_assets.insert(denom.id().clone(), denom.base_denom().to_string());
    KNOWN_ASSETS
        .set(known_assets)
        .expect("known assets should only be set once");
}
