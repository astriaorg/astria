use crate::{
    accounts::AddressBytes,
    storage::keys::AccountPrefixer,
};

pub(in crate::auctioneer) const ENSHRINED_AUCTIONEER_PREFIX: &str = "auctioneer/";

pub(in crate::auctioneer) fn enshrined_auctioneer_key<T: AddressBytes>(address: &T) -> String {
    format!(
        "{}/auctioneer",
        AccountPrefixer::new(ENSHRINED_AUCTIONEER_PREFIX, address)
    )
}
