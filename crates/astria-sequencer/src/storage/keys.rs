use std::{
    borrow::Cow,
    fmt::{
        self,
        Display,
        Formatter,
    },
    str::FromStr,
};

use astria_core::{
    primitive::v1::asset::{
        denom::ParseIbcPrefixedError,
        IbcPrefixed,
    },
    protocol::orderbook::v1::OrderSide,
};

use crate::accounts::AddressBytes;

/// Helper struct whose `Display` impl outputs the prefix followed by the hex-encoded address.
pub(crate) struct AccountPrefixer<'a, T> {
    prefix: &'static str,
    address: &'a T,
}

impl<'a, T> AccountPrefixer<'a, T> {
    pub(crate) fn new(prefix: &'static str, address: &'a T) -> Self {
        Self {
            prefix,
            address,
        }
    }
}

impl<T: AddressBytes> Display for AccountPrefixer<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use base64::{
            display::Base64Display,
            engine::general_purpose::URL_SAFE,
        };
        f.write_str(self.prefix)?;
        Base64Display::new(self.address.address_bytes(), &URL_SAFE).fmt(f)
    }
}

/// Helper struct whose `Display` impl outputs the hex-encoded ibc-prefixed address, and that can be
/// parsed from such a hex-encoded form.
#[cfg_attr(test, derive(Debug, PartialEq))]
pub(crate) struct Asset<'a>(Cow<'a, IbcPrefixed>);

impl Asset<'_> {
    pub(crate) fn get(self) -> IbcPrefixed {
        self.0.into_owned()
    }
}

impl Display for Asset<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a, T> From<&'a T> for Asset<'a>
where
    &'a T: Into<Cow<'a, IbcPrefixed>>,
{
    fn from(value: &'a T) -> Self {
        Self(value.into())
    }
}

impl FromStr for Asset<'_> {
    type Err = ParseIbcPrefixedError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Cow::Owned(s.parse()?)))
    }
}

// Order book keys

// Prefix for all orderbook keys
const ORDERBOOK_PREFIX: &str = "orderbook/";
// Specific orderbook key prefixes
const ORDERBOOK_MARKETS: &str = "orderbook/markets/";
const ORDERBOOK_MARKET_PARAMS: &str = "orderbook/market_params/";
const ORDERBOOK_ORDERS: &str = "orderbook/orders/";
const ORDERBOOK_MARKET_ORDERS: &str = "orderbook/market_orders/";
const ORDERBOOK_MARKET_SIDE_ORDERS: &str = "orderbook/market_side_orders/";
const ORDERBOOK_MARKET_SIDE_PRICE_ORDERS: &str = "orderbook/market_side_price_orders/";
const ORDERBOOK_MARKET_PRICE_LEVELS: &str = "orderbook/market_price_levels/";
const ORDERBOOK_OWNER_ORDERS: &str = "orderbook/owner_orders/";
const ORDERBOOK_MARKET_TRADES: &str = "orderbook/market_trades/";
const ORDERBOOK_ALL_MARKETS: &str = "orderbook/all_markets";
const ORDERBOOK_NEXT_ORDER_ID: &str = "orderbook/next_order_id";
const ORDERBOOK_NEXT_TRADE_ID: &str = "orderbook/next_trade_id";

// Get the list of all markets
pub fn orderbook_markets() -> String {
    ORDERBOOK_MARKETS.to_string()
}

// Get a specific market
pub fn orderbook_market(market: &str) -> String {
    format!("{}{}", ORDERBOOK_MARKETS, market)
}

// Get market parameters
pub fn orderbook_market_params(market: &str) -> String {
    format!("{}{}", ORDERBOOK_MARKET_PARAMS, market)
}

// Get an order by ID
pub fn orderbook_order(order_id: &str) -> String {
    format!("{}{}", ORDERBOOK_ORDERS, order_id)
}

// Get all orders for a market
pub fn orderbook_market_orders(market: &str) -> String {
    format!("{}{}/", ORDERBOOK_MARKET_ORDERS, market)
}

// Get a specific order in a market
pub fn orderbook_market_order(market: &str, order_id: &str) -> String {
    format!("{}{}/{}", ORDERBOOK_MARKET_ORDERS, market, order_id)
}

// Get all orders for a market and side
pub fn orderbook_market_side_orders(market: &str, side: OrderSide) -> String {
    format!("{}{}/{:?}/", ORDERBOOK_MARKET_SIDE_ORDERS, market, side)
}

// Get a specific order for a market and side
pub fn orderbook_market_side_order(market: &str, side: OrderSide, order_id: &str) -> String {
    format!("{}{}/{:?}/{}", ORDERBOOK_MARKET_SIDE_ORDERS, market, side, order_id)
}

// Get all orders at a specific price level
pub fn orderbook_market_side_price_orders(
    market: &str,
    side: OrderSide,
    price: &str,
) -> String {
    format!(
        "{}{}/{:?}/{}/",
        ORDERBOOK_MARKET_SIDE_PRICE_ORDERS, market, side, price
    )
}

// Get a specific order at a price level
pub fn orderbook_market_side_price_order(
    market: &str,
    side: OrderSide,
    price: &str,
    order_id: &str,
) -> String {
    format!(
        "{}{}/{:?}/{}/{}",
        ORDERBOOK_MARKET_SIDE_PRICE_ORDERS, market, side, price, order_id
    )
}

// Get a price level
pub fn orderbook_market_price_level(market: &str, side: OrderSide, price: &str) -> String {
    format!("{}{}/{:?}/{}", ORDERBOOK_MARKET_PRICE_LEVELS, market, side, price)
}

// Get all price levels for a market and side
pub fn orderbook_market_price_levels(market: &str, side: OrderSide) -> String {
    format!("{}{}/{:?}/", ORDERBOOK_MARKET_PRICE_LEVELS, market, side)
}

// Get all orders for an owner
pub fn orderbook_owner_orders(owner: &str) -> String {
    format!("{}{}/", ORDERBOOK_OWNER_ORDERS, owner)
}

// Get a specific order for an owner
pub fn orderbook_owner_order(owner: &str, order_id: &str) -> String {
    format!("{}{}/{}", ORDERBOOK_OWNER_ORDERS, owner, order_id)
}

// Get all trades for a market
pub fn orderbook_market_trades(market: &str) -> String {
    format!("{}{}/", ORDERBOOK_MARKET_TRADES, market)
}

// Get a specific trade
pub fn orderbook_market_trade(market: &str, trade_id: &str) -> String {
    format!("{}{}/{}", ORDERBOOK_MARKET_TRADES, market, trade_id)
}

// Get all market IDs
pub fn orderbook_all_markets() -> String {
    ORDERBOOK_ALL_MARKETS.to_string()
}

// Get the next order ID
pub fn orderbook_next_order_id() -> String {
    ORDERBOOK_NEXT_ORDER_ID.to_string()
}

// Get the next trade ID
pub fn orderbook_next_trade_id() -> String {
    ORDERBOOK_NEXT_TRADE_ID.to_string()
}

/// A key builder for constructing hierarchical keys with segments
#[derive(Debug, Clone)]
pub struct Key {
    segments: Vec<String>,
}

impl Key {
    /// Create a new empty key
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Push a segment to the key
    pub fn push_segment<S: Into<String>>(mut self, segment: S) -> Self {
        self.segments.push(segment.into());
        self
    }

    /// Convert the key to a string representation
    pub fn to_string(&self) -> String {
        self.segments.join("/")
    }

    /// Get the key as bytes
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().into_bytes()
    }
}

impl Default for Key {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Key> for String {
    fn from(key: Key) -> Self {
        key.to_string()
    }
}

impl AsRef<[u8]> for Key {
    fn as_ref(&self) -> &[u8] {
        self.segments.join("/").as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::Asset;

    #[test]
    fn asset_key_to_string_parse_roundtrip() {
        let asset = "an/asset/with/a/prefix"
            .parse::<astria_core::primitive::v1::asset::Denom>()
            .unwrap();
        let expected = Asset::from(&asset);
        let actual = expected.to_string().parse::<Asset>().unwrap();
        assert_eq!(expected, actual);
    }
}
