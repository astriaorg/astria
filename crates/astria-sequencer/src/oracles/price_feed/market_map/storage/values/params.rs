use std::{
    borrow::Cow,
    fmt::{
        self,
        Debug,
        Formatter,
    },
};

use astria_core::{
    oracles::price_feed::market_map::v2::Params as DomainParams,
    primitive::v1::{
        Address,
        ADDRESS_LEN,
    },
};
use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use itertools::Itertools as _;
use telemetry::display::base64;

use super::{
    Value,
    ValueImpl,
};

#[derive(BorshSerialize, BorshDeserialize)]
pub(in crate::oracles::price_feed::market_map) struct Params<'a> {
    market_authorities: Vec<Cow<'a, [u8; ADDRESS_LEN]>>,
    admin: Cow<'a, [u8; ADDRESS_LEN]>,
    // NOTE: All addresses in the `market_authorities` and `admin` have the same prefix.
    prefix: Cow<'a, str>,
}

impl Debug for Params<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Params")
            .field(
                "market_authorities",
                &self
                    .market_authorities
                    .iter()
                    .map(|address| base64(address.as_slice()))
                    .join(", "),
            )
            .field("admin", &base64(self.admin.as_slice()).to_string())
            .field("prefix", &self.prefix)
            .finish()
    }
}

impl<'a> From<&'a DomainParams> for Params<'a> {
    fn from(params: &'a DomainParams) -> Self {
        Params {
            market_authorities: params
                .market_authorities
                .iter()
                .map(|address| Cow::Borrowed(address.as_bytes()))
                .collect(),
            admin: Cow::Borrowed(params.admin.as_bytes()),
            prefix: Cow::Borrowed(params.admin.prefix()),
        }
    }
}

impl<'a> From<Params<'a>> for DomainParams {
    fn from(params: Params<'a>) -> Self {
        DomainParams::unchecked_from_parts(
            params
                .market_authorities
                .into_iter()
                .map(|address_bytes| {
                    Address::unchecked_from_parts(
                        address_bytes.into_owned(),
                        params.prefix.as_ref(),
                    )
                })
                .collect(),
            Address::unchecked_from_parts(params.admin.into_owned(), params.prefix.as_ref()),
        )
    }
}

impl<'a> From<Params<'a>> for crate::storage::StoredValue<'a> {
    fn from(params: Params<'a>) -> Self {
        crate::storage::StoredValue::PriceFeedMarketMap(Value(ValueImpl::Params(params)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for Params<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::PriceFeedMarketMap(Value(ValueImpl::Params(params))) =
            value
        else {
            bail!(
                "price feed market map stored value type mismatch: expected params, found \
                 {value:?}"
            );
        };
        Ok(params)
    }
}
