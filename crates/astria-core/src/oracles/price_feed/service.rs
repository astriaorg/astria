pub mod v2 {
    use indexmap::IndexMap;

    use crate::{
        generated::price_feed::service::v2 as raw,
        oracles::price_feed::types::v2::{
            CurrencyPair,
            CurrencyPairParseError,
            ParsePriceError,
            Price,
        },
    };

    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct QueryPricesResponseError(#[from] QueryPricesResponseErrorKind);

    #[derive(Debug, thiserror::Error)]
    #[error("failed validating wire type `{}`", raw::QueryPrices::full_name())]
    enum QueryPricesResponseErrorKind {
        #[error("failed to parse key `{input}` in `.prices` field as currency pair")]
        ParseCurrencyPair {
            input: String,
            source: CurrencyPairParseError,
        },
        #[error("failed to parse value `{input}` in `.prices` field at key `{key}` as price")]
        ParsePrice {
            input: String,
            key: String,
            source: ParsePriceError,
        },
    }

    pub struct QueryPricesResponse {
        pub prices: IndexMap<CurrencyPair, Price>,
        pub timestamp: Option<pbjson_types::Timestamp>,
        pub version: String,
    }

    impl QueryPricesResponse {
        /// Converts the on-wire [`raw::QueryPricesResponse`] to a validated domain type
        /// [`QueryPricesResponse`].
        ///
        /// # Errors
        /// Returns an error if:
        /// - A key in the `.prices` map could not be parsed as a [`CurrencyPair`].
        /// - A value in the `.prices` map could not be parsed as a [`Price`].
        pub fn try_from_raw(
            wire: raw::QueryPricesResponse,
        ) -> Result<QueryPricesResponse, QueryPricesResponseError> {
            let raw::QueryPricesResponse {
                prices,
                timestamp,
                version,
            } = wire;
            let prices = prices
                .into_iter()
                .map(|(key, value)| {
                    let currency_pair = match key.parse() {
                        Err(source) => {
                            return Err(QueryPricesResponseErrorKind::ParseCurrencyPair {
                                input: key,
                                source,
                            });
                        }
                        Ok(parsed) => parsed,
                    };
                    let price = value.parse().map_err(move |source| {
                        QueryPricesResponseErrorKind::ParsePrice {
                            input: value,
                            key,
                            source,
                        }
                    })?;
                    Ok((currency_pair, price))
                })
                .collect::<Result<_, _>>()?;
            Ok(Self {
                prices,
                timestamp,
                version,
            })
        }
    }
}
