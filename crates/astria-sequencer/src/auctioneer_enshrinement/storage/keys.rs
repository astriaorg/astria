use crate::accounts::AddressBytes;
use crate::storage::keys::AccountPrefixer;

pub(in crate::auctioneer_enshrinement) const ENSHRINED_AUCTIONEER_PREFIX: &str = "auctioneer_enshrinement/";

pub(in crate::auctioneer_enshrinement) fn enshrined_auctioneer_key<T: AddressBytes>(address: &T) -> String {
    format!(
        "{}/auctioneer",
        AccountPrefixer::new(ENSHRINED_AUCTIONEER_PREFIX, address)
    )
}