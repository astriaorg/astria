pub(crate) mod query;
pub(crate) mod state_ext;

use std::sync::OnceLock;

use astria_core::primitive::v1::asset::Denom;
#[cfg(test)]
pub(crate) use intests::*;
#[cfg(not(test))]
pub(crate) use regular::*;

pub(crate) static NATIVE_ASSET: OnceLock<Denom> = OnceLock::new();

#[cfg(not(test))]
mod regular {

    pub(crate) fn initialize_native_asset(native_asset: &str) {
        if super::NATIVE_ASSET.get().is_some() {
            tracing::error!("native asset should only be set once");
            return;
        }

        let denom = native_asset
            .parse()
            .expect("being unable to parse the native asset breaks sequencer");
        super::NATIVE_ASSET
            .set(denom)
            .expect("native asset should only be set once");
    }

    pub(crate) fn get_native_asset() -> &'static super::Denom {
        super::NATIVE_ASSET
            .get()
            .expect("native asset should be set")
    }
}

#[cfg(test)]
mod intests {
    pub(crate) fn initialize_native_asset(native_asset: &str) {
        assert_eq!(
            "nria", native_asset,
            "all tests should be initialized with \"nria\" as the native asset"
        );
    }

    pub(crate) fn get_native_asset() -> &'static super::Denom {
        super::NATIVE_ASSET.get_or_init(|| {
            "nria"
                .parse()
                .expect("being unable to parse the native asset breaks sequencer")
        })
    }
}
