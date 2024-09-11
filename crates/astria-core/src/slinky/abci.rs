pub mod v1 {
    use bytes::Bytes;
    use indexmap::IndexMap;

    use crate::{
        generated::astria_vendored::slinky::abci::v1 as raw,
        slinky::types::v1::{
            CurrencyPairId,
            Price,
        },
    };

    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct OracleVoteExtensionError(#[from] OracleVoteExtensionErrorKind);

    #[derive(Debug, thiserror::Error)]
    #[error("failed to validate astria_vendored.slinky.abci.v1.OracleVoteExtension")]
    enum OracleVoteExtensionErrorKind {
        #[error("failed decoding price value in .prices field for key `{id}`")]
        DecodePrice {
            id: u64,
            source: crate::slinky::types::v1::DecodePriceError,
        },
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OracleVoteExtension {
        pub prices: IndexMap<CurrencyPairId, Price>,
    }

    impl OracleVoteExtension {
        pub fn try_from_raw(
            raw: raw::OracleVoteExtension,
        ) -> Result<Self, OracleVoteExtensionError> {
            let prices = raw
                .prices
                .into_iter()
                .map(|(id, price)| {
                    let price = Price::try_from(price).map_err(|source| {
                        OracleVoteExtensionErrorKind::DecodePrice {
                            id,
                            source,
                        }
                    })?;
                    Ok::<_, OracleVoteExtensionErrorKind>((CurrencyPairId::new(id), price))
                })
                .collect::<Result<_, _>>()?;
            Ok(Self {
                prices,
            })
        }

        #[must_use]
        pub fn into_raw(self) -> raw::OracleVoteExtension {
            fn encode_price(input: Price) -> Bytes {
                Bytes::copy_from_slice(&input.get().to_be_bytes())
            }

            raw::OracleVoteExtension {
                prices: self
                    .prices
                    .into_iter()
                    .map(|(id, price)| (id.get(), encode_price(price)))
                    .collect(),
            }
        }
    }
}
