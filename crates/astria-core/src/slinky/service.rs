pub mod v1 {
    use indexmap::IndexMap;

    use crate::{
        generated::astria_vendored::slinky::service::v1 as raw,
        slinky::types::v1::{
            CurrencyPair,
            CurrencyPairParseError,
            ParsePriceError,
            Price,
        },
    };

    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct QueryPriceResponseError(#[from] QueryPriceResponseErrorKind);

    #[derive(Debug, thiserror::Error)]
    #[error(
        "failed validating wire type {}",
        raw::QueryPriceResponseError::full_name()
    )]
    enum QueryPriceResponseErrorKind {
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
        pub timestamp: ::core::option::Option<::pbjson_types::Timestamp>,
    }

    impl QueryPricesResponse {
        pub fn try_from_raw(
            wire: raw::QueryPricesResponse,
        ) -> Result<QueryPricesResponse, QueryPriceResponseError> {
            let raw::QueryPricesResponse {
                prices,
                timestamp,
            } = wire;
            let prices = prices
                .into_iter()
                .map(|(key, value)| {
                    let currency_pair = match key.parse() {
                        Err(source) => {
                            return Err(QueryPriceResponseErrorKind::ParseCurrencyPair {
                                input: key,
                                source,
                            });
                        }
                        Ok(parsed) => parsed,
                    };
                    let price = value.parse().map_err(move |source| {
                        QueryPriceResponseErrorKind::ParsePrice {
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
            })
        }
    }
}
