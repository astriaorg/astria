pub(crate) mod query;
pub(crate) mod state_ext;

use anyhow::ensure;
use astria_core::primitive::v1::{
    Address,
    AddressError,
    ADDRESS_LEN,
};
#[cfg(not(test))]
pub(crate) use regular::*;
#[cfg(test)]
pub(crate) use testonly::*;

pub(crate) fn base_prefixed(arr: [u8; ADDRESS_LEN]) -> Address {
    Address::builder()
        .array(arr)
        .prefix(get_base_prefix())
        .try_build()
        .expect("the prefix must have been set as a valid bech32 prefix, so this should never fail")
}

pub(crate) fn try_base_prefixed(slice: &[u8]) -> Result<Address, AddressError> {
    Address::builder()
        .slice(slice)
        .prefix(get_base_prefix())
        .try_build()
}

pub(crate) fn ensure_base_prefix(address: &Address) -> anyhow::Result<()> {
    ensure!(
        get_base_prefix() == address.prefix(),
        "address has prefix `{}` but only `{}` is permitted",
        address.prefix(),
        crate::address::get_base_prefix(),
    );
    Ok(())
}

#[cfg(not(test))]
mod regular {
    //! Logic to be used for a normal debug or release build of sequencer.

    use std::sync::OnceLock;

    use anyhow::Context as _;

    static BASE_PREFIX: OnceLock<String> = OnceLock::new();

    pub(crate) fn initialize_base_prefix(base_prefix: &str) -> anyhow::Result<()> {
        assert!(
            BASE_PREFIX.get().is_some(),
            "the base prefix was already initialized; it must only be initialized once and upon \
             receiving an init-chain consensus request"
        );

        // construct a dummy address to see if we can construct it; fail otherwise.
        try_construct_dummy_address_from_prefix(base_prefix)
            .context("failed constructing a dummy address from the provided prefix")?;

        BASE_PREFIX
            .set(base_prefix.to_string())
            .expect("singleton base prefix is initialized once which is asserted above");

        Ok(())
    }

    pub(crate) fn get_base_prefix() -> &'static str {
        BASE_PREFIX
            .get()
            .expect(
                "the base prefix must have been set during chain init; if not set, the chain was \
                 initialized incorrectly",
            )
            .as_str()
    }

    fn try_construct_dummy_address_from_prefix(
        s: &str,
    ) -> Result<(), astria_core::primitive::v1::AddressError> {
        use astria_core::primitive::v1::{
            Address,
            ADDRESS_LEN,
        };
        // construct a dummy address to see if we can construct it; fail otherwise.
        Address::builder()
            .array([0u8; ADDRESS_LEN])
            .prefix(s)
            .try_build()
            .map(|_| ())
    }
}

#[cfg(test)]
mod testonly {
    pub(crate) fn initialize_base_prefix(base_prefix: &str) -> anyhow::Result<()> {
        assert_eq!(
            base_prefix,
            get_base_prefix(),
            "all tests should be initialized with a \"astria\" as the base prefix"
        );
        Ok(())
    }

    pub(crate) fn get_base_prefix() -> &'static str {
        "astria"
    }
}