pub mod v1 {
    use indexmap::IndexMap;
    use tendermint::{
        abci::types::ExtendedCommitInfo,
        block::Round,
    };

    use crate::{
        generated::{
            astria::protocol::price_feed::v1 as raw,
            astria_vendored,
        },
        oracles::price_feed::types::v2::{
            CurrencyPair,
            CurrencyPairError,
            CurrencyPairId,
        },
    };

    impl From<astria_vendored::tendermint::abci::ExtendedCommitInfo>
        for tendermint_proto::abci::ExtendedCommitInfo
    {
        fn from(value: astria_vendored::tendermint::abci::ExtendedCommitInfo) -> Self {
            tendermint_proto::abci::ExtendedCommitInfo {
                round: value.round,
                votes: value.votes.into_iter().map(Into::into).collect(),
            }
        }
    }

    impl From<astria_vendored::tendermint::abci::ExtendedVoteInfo>
        for tendermint_proto::abci::ExtendedVoteInfo
    {
        fn from(value: astria_vendored::tendermint::abci::ExtendedVoteInfo) -> Self {
            tendermint_proto::abci::ExtendedVoteInfo {
                validator: value.validator.map(Into::into),
                vote_extension: value.vote_extension,
                extension_signature: value.extension_signature,
                block_id_flag: value.block_id_flag,
            }
        }
    }

    impl From<astria_vendored::tendermint::abci::Validator> for tendermint_proto::abci::Validator {
        fn from(value: astria_vendored::tendermint::abci::Validator) -> Self {
            tendermint_proto::abci::Validator {
                address: value.address,
                power: value.power,
            }
        }
    }

    impl From<tendermint_proto::abci::ExtendedCommitInfo>
        for astria_vendored::tendermint::abci::ExtendedCommitInfo
    {
        fn from(value: tendermint_proto::abci::ExtendedCommitInfo) -> Self {
            astria_vendored::tendermint::abci::ExtendedCommitInfo {
                round: value.round,
                votes: value.votes.into_iter().map(Into::into).collect(),
            }
        }
    }

    impl From<tendermint_proto::abci::ExtendedVoteInfo>
        for astria_vendored::tendermint::abci::ExtendedVoteInfo
    {
        fn from(value: tendermint_proto::abci::ExtendedVoteInfo) -> Self {
            astria_vendored::tendermint::abci::ExtendedVoteInfo {
                validator: value.validator.map(Into::into),
                vote_extension: value.vote_extension,
                extension_signature: value.extension_signature,
                block_id_flag: value.block_id_flag,
            }
        }
    }

    impl From<tendermint_proto::abci::Validator> for astria_vendored::tendermint::abci::Validator {
        fn from(value: tendermint_proto::abci::Validator) -> Self {
            astria_vendored::tendermint::abci::Validator {
                address: value.address,
                power: value.power,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct CurrencyPairInfo {
        pub currency_pair: CurrencyPair,
        pub decimals: u8,
    }

    impl std::fmt::Display for CurrencyPairInfo {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "CurrencyPairInfo {{ currency_pair: {}, decimals: {} }}",
                self.currency_pair, self.decimals
            )
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ExtendedCommitInfoWithCurrencyPairMapping {
        pub extended_commit_info: ExtendedCommitInfo,
        pub id_to_currency_pair: IndexMap<CurrencyPairId, CurrencyPairInfo>,
    }

    impl ExtendedCommitInfoWithCurrencyPairMapping {
        #[must_use]
        pub fn new(
            extended_commit_info: ExtendedCommitInfo,
            id_to_currency_pair: IndexMap<CurrencyPairId, CurrencyPairInfo>,
        ) -> Self {
            Self {
                extended_commit_info,
                id_to_currency_pair,
            }
        }

        #[must_use]
        pub fn empty(round: Round) -> Self {
            Self {
                extended_commit_info: ExtendedCommitInfo {
                    round,
                    votes: Vec::new(),
                },
                id_to_currency_pair: IndexMap::new(),
            }
        }

        /// Converts from a protobuf `ExtendedCommitInfoWithCurrencyPairMapping` to the native type.
        ///
        /// # Errors
        ///
        /// - if the `extended_commit_info` field is not set
        /// - if the `extended_commit_info` field is invalid
        /// - if a `currency_pair` field is not set within a `id_to_currency_pair` item
        /// - if a currency pair is invalid
        pub fn try_from_raw(
            raw: raw::ExtendedCommitInfoWithCurrencyPairMapping,
        ) -> Result<Self, ExtendedCommitInfoWithCurrencyPairMappingError> {
            let Some(extended_commit_info) = raw.extended_commit_info else {
                return Err(
                    ExtendedCommitInfoWithCurrencyPairMappingError::field_not_set(
                        "extended_commit_info",
                    ),
                );
            };
            let extended_commit_info = ExtendedCommitInfo::try_from(
                tendermint_proto::abci::ExtendedCommitInfo::from(extended_commit_info),
            )
            .map_err(ExtendedCommitInfoWithCurrencyPairMappingError::extended_commit_info)?;
            let id_to_currency_pair = raw
                .id_to_currency_pair
                .into_iter()
                .map(|id_with_currency_pair| {
                    let currency_pair_id = CurrencyPairId::new(id_with_currency_pair.id);
                    let Some(currency_pair) = id_with_currency_pair.currency_pair else {
                        return Err(
                            ExtendedCommitInfoWithCurrencyPairMappingError::field_not_set(
                                "currency_pair",
                            ),
                        );
                    };
                    let currency_pair = CurrencyPair::try_from_raw(currency_pair)
                        .map_err(ExtendedCommitInfoWithCurrencyPairMappingError::currency_pair)?;
                    let decimals = id_with_currency_pair.decimals.try_into().map_err(|_| {
                        ExtendedCommitInfoWithCurrencyPairMappingError::decimals_too_large(
                            id_with_currency_pair.decimals,
                        )
                    })?;
                    Ok((
                        currency_pair_id,
                        CurrencyPairInfo {
                            currency_pair,
                            decimals,
                        },
                    ))
                })
                .collect::<Result<
                    IndexMap<CurrencyPairId, CurrencyPairInfo>,
                    ExtendedCommitInfoWithCurrencyPairMappingError,
                >>()?;
            Ok(Self {
                extended_commit_info,
                id_to_currency_pair,
            })
        }

        #[must_use]
        pub fn into_raw(self) -> raw::ExtendedCommitInfoWithCurrencyPairMapping {
            let extended_commit_info: tendermint_proto::abci::ExtendedCommitInfo =
                self.extended_commit_info.into();
            let id_to_currency_pair = self
                .id_to_currency_pair
                .into_iter()
                .map(
                    |(currency_pair_id, currency_pair_info)| raw::IdWithCurrencyPair {
                        id: currency_pair_id.get(),
                        currency_pair: Some(currency_pair_info.currency_pair.into_raw()),
                        decimals: currency_pair_info.decimals.into(),
                    },
                )
                .collect();
            raw::ExtendedCommitInfoWithCurrencyPairMapping {
                extended_commit_info: Some(extended_commit_info.into()),
                id_to_currency_pair,
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct ExtendedCommitInfoWithCurrencyPairMappingError(
        ExtendedCommitInfoWithCurrencyPairMappingErrorKind,
    );

    impl ExtendedCommitInfoWithCurrencyPairMappingError {
        fn field_not_set(field: &'static str) -> Self {
            Self(
                ExtendedCommitInfoWithCurrencyPairMappingErrorKind::FieldNotSet {
                    field,
                },
            )
        }

        fn currency_pair(error: CurrencyPairError) -> Self {
            Self(ExtendedCommitInfoWithCurrencyPairMappingErrorKind::CurrencyPair(error))
        }

        fn extended_commit_info(error: tendermint::Error) -> Self {
            Self(ExtendedCommitInfoWithCurrencyPairMappingErrorKind::ExtendedCommitInfo(error))
        }

        fn decimals_too_large(decimals: u64) -> Self {
            Self(
                ExtendedCommitInfoWithCurrencyPairMappingErrorKind::DecimalsTooLarge {
                    decimals,
                },
            )
        }
    }

    #[derive(Debug, thiserror::Error)]
    enum ExtendedCommitInfoWithCurrencyPairMappingErrorKind {
        #[error("field not set: {field}")]
        FieldNotSet { field: &'static str },
        #[error("invalid currency pair")]
        CurrencyPair(#[from] CurrencyPairError),
        #[error("invalid extended commit info")]
        ExtendedCommitInfo(#[from] tendermint::Error),
        #[error("decimals too large: {decimals}; must fit in a u8")]
        DecimalsTooLarge { decimals: u64 },
    }
}
