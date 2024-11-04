use std::{
    pin::Pin,
    task::{
        ready,
        Context,
        Poll,
    },
};

use astria_core::connect::{
    oracle::v2::{
        CurrencyPairState,
        QuotePrice,
    },
    types::v2::{
        CurrencyPair,
        CurrencyPairId,
        CurrencyPairNonce,
    },
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        ContextCompat as _,
        Result,
        WrapErr as _,
    },
};
use async_trait::async_trait;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use futures::Stream;
use pin_project_lite::pin_project;
use tracing::instrument;

mod in_state {
    //! Contains all borsh datatypes that are written to/read from state.

    use astria_eyre::eyre::{
        Result,
        WrapErr as _,
    };
    use borsh::{
        BorshDeserialize,
        BorshSerialize,
    };

    #[derive(BorshSerialize, BorshDeserialize, Debug)]
    pub(super) struct CurrencyPairId(pub(super) u64);

    impl From<CurrencyPairId> for super::CurrencyPairId {
        fn from(value: CurrencyPairId) -> Self {
            Self::new(value.0)
        }
    }

    impl From<super::CurrencyPairId> for CurrencyPairId {
        fn from(value: super::CurrencyPairId) -> Self {
            Self(value.get())
        }
    }

    #[derive(BorshSerialize, BorshDeserialize, Debug)]
    pub(super) struct CurrencyPairNonce(pub(super) u64);

    impl From<CurrencyPairNonce> for super::CurrencyPairNonce {
        fn from(value: CurrencyPairNonce) -> Self {
            Self::new(value.0)
        }
    }

    impl From<super::CurrencyPairNonce> for CurrencyPairNonce {
        fn from(value: super::CurrencyPairNonce) -> Self {
            Self(value.get())
        }
    }

    #[derive(BorshSerialize, BorshDeserialize, Debug)]
    pub(super) struct CurrencyPair {
        base: String,
        quote: String,
    }

    impl TryFrom<CurrencyPair> for super::CurrencyPair {
        type Error = astria_eyre::eyre::Error;

        fn try_from(value: CurrencyPair) -> Result<Self> {
            Ok(Self::from_parts(
                value.base.parse().with_context(|| {
                    format!(
                        "failed to parse state-fetched `{}` as currency pair base",
                        value.base
                    )
                })?,
                value.quote.parse().with_context(|| {
                    format!(
                        "failed to parse state-fetched `{}` as currency pair quote",
                        value.quote
                    )
                })?,
            ))
        }
    }

    impl From<super::CurrencyPair> for CurrencyPair {
        fn from(value: super::CurrencyPair) -> Self {
            let (base, quote) = value.into_parts();
            Self {
                base,
                quote,
            }
        }
    }

    #[derive(Debug, BorshSerialize, BorshDeserialize)]
    struct Timestamp {
        seconds: i64,
        nanos: i32,
    }

    impl From<astria_core::primitive::Timestamp> for Timestamp {
        fn from(value: astria_core::primitive::Timestamp) -> Self {
            Self {
                seconds: value.seconds,
                nanos: value.nanos,
            }
        }
    }

    impl From<Timestamp> for astria_core::primitive::Timestamp {
        fn from(value: Timestamp) -> Self {
            Self {
                seconds: value.seconds,
                nanos: value.nanos,
            }
        }
    }

    #[derive(Debug, BorshSerialize, BorshDeserialize)]
    struct Price(u128);

    impl From<astria_core::connect::types::v2::Price> for Price {
        fn from(value: astria_core::connect::types::v2::Price) -> Self {
            Self(value.get())
        }
    }

    impl From<Price> for astria_core::connect::types::v2::Price {
        fn from(value: Price) -> Self {
            Self::new(value.0)
        }
    }

    #[derive(Debug, BorshSerialize, BorshDeserialize)]
    pub(super) struct QuotePrice {
        price: Price,
        block_timestamp: Timestamp,
        block_height: u64,
    }

    impl From<super::QuotePrice> for QuotePrice {
        fn from(value: super::QuotePrice) -> Self {
            Self {
                price: value.price.into(),
                block_timestamp: value.block_timestamp.into(),
                block_height: value.block_height,
            }
        }
    }

    impl From<QuotePrice> for super::QuotePrice {
        fn from(value: QuotePrice) -> Self {
            Self {
                price: value.price.into(),
                block_timestamp: value.block_timestamp.into(),
                block_height: value.block_height,
            }
        }
    }

    #[derive(Debug, BorshSerialize, BorshDeserialize)]
    pub(super) struct CurrencyPairState {
        pub(super) price: QuotePrice,
        pub(super) nonce: CurrencyPairNonce,
        pub(super) id: CurrencyPairId,
    }

    impl From<super::CurrencyPairState> for CurrencyPairState {
        fn from(value: super::CurrencyPairState) -> Self {
            Self {
                price: value.price.into(),
                nonce: value.nonce.into(),
                id: value.id.into(),
            }
        }
    }

    impl From<CurrencyPairState> for super::CurrencyPairState {
        fn from(value: CurrencyPairState) -> Self {
            Self {
                price: value.price.into(),
                nonce: value.nonce.into(),
                id: value.id.into(),
            }
        }
    }
}

const CURRENCY_PAIR_TO_ID_PREFIX: &str = "oraclecpid";
const ID_TO_CURRENCY_PAIR_PREFIX: &str = "oracleidcp";
const CURRENCY_PAIR_STATE_PREFIX: &str = "oraclecpstate";

const NUM_CURRENCY_PAIRS_KEY: &str = "oraclenumcps";
const NUM_REMOVED_CURRENCY_PAIRS_KEY: &str = "oraclenumremovedcps";
const NEXT_CURRENCY_PAIR_ID_KEY: &str = "oraclenextcpid";

fn currency_pair_to_id_storage_key(currency_pair: &CurrencyPair) -> String {
    format!("{CURRENCY_PAIR_TO_ID_PREFIX}/{currency_pair}",)
}

fn id_to_currency_pair_storage_key(id: CurrencyPairId) -> String {
    format!("{ID_TO_CURRENCY_PAIR_PREFIX}/{id}")
}

fn currency_pair_state_storage_key(currency_pair: &CurrencyPair) -> String {
    format!("{CURRENCY_PAIR_STATE_PREFIX}/{currency_pair}",)
}

/// Newtype wrapper to read and write a u64 from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Count(u64);

pin_project! {
    pub(crate) struct CurrencyPairsWithIdsStream<St> {
        #[pin]
        underlying: St,
    }
}

pub(crate) struct CurrencyPairWithId {
    pub(crate) id: u64,
    pub(crate) currency_pair: CurrencyPair,
}

impl<St> Stream for CurrencyPairsWithIdsStream<St>
where
    St: Stream<Item = astria_eyre::anyhow::Result<(String, Vec<u8>)>>,
{
    type Item = Result<CurrencyPairWithId>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let (key, bytes) = match ready!(this.underlying.as_mut().poll_next(cx)) {
            Some(Ok(item)) => item,
            Some(Err(err)) => {
                return Poll::Ready(Some(
                    Err(anyhow_to_eyre(err)).wrap_err("failed reading from state"),
                ));
            }
            None => return Poll::Ready(None),
        };
        let in_state::CurrencyPairId(id) = in_state::CurrencyPairId::try_from_slice(&bytes)
            .with_context(|| {
                "failed decoding bytes read from state as currency pair ID for key `{key}`"
            })?;
        let currency_pair = match extract_currency_pair_from_key(&key) {
            Err(err) => {
                return Poll::Ready(Some(Err(err).with_context(|| {
                    format!("failed to extract currency pair from key `{key}`")
                })));
            }
            Ok(parsed) => parsed,
        };
        Poll::Ready(Some(Ok(CurrencyPairWithId {
            id,
            currency_pair,
        })))
    }
}

pin_project! {
    pub(crate) struct CurrencyPairsStream<St> {
        #[pin]
        underlying: St,
    }
}

impl<St> Stream for CurrencyPairsStream<St>
where
    St: Stream<Item = astria_eyre::anyhow::Result<String>>,
{
    type Item = Result<CurrencyPair>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let key = match ready!(this.underlying.as_mut().poll_next(cx)) {
            Some(Ok(item)) => item,
            Some(Err(err)) => {
                return Poll::Ready(Some(
                    Err(anyhow_to_eyre(err)).wrap_err("failed reading from state"),
                ));
            }
            None => return Poll::Ready(None),
        };
        let currency_pair = match extract_currency_pair_from_key(&key) {
            Err(err) => {
                return Poll::Ready(Some(Err(err).with_context(|| {
                    format!("failed to extract currency pair from key `{key}`")
                })));
            }
            Ok(parsed) => parsed,
        };
        Poll::Ready(Some(Ok(currency_pair)))
    }
}

fn extract_currency_pair_from_key(key: &str) -> Result<CurrencyPair> {
    key.strip_prefix(CURRENCY_PAIR_TO_ID_PREFIX)
        .wrap_err("failed to strip prefix from currency pair state key")?
        .parse::<CurrencyPair>()
        .wrap_err("failed to parse storage key suffix as currency pair")
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    async fn get_currency_pair_id(
        &self,
        currency_pair: &CurrencyPair,
    ) -> Result<Option<CurrencyPairId>> {
        let Some(bytes) = self
            .get_raw(&currency_pair_to_id_storage_key(currency_pair))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading currency pair id from state")?
        else {
            return Ok(None);
        };
        in_state::CurrencyPairId::try_from_slice(&bytes)
            .wrap_err("invalid currency pair id bytes")
            .map(|id| Some(id.into()))
    }

    #[instrument(skip_all)]
    async fn get_currency_pair(&self, id: CurrencyPairId) -> Result<Option<CurrencyPair>> {
        let Some(bytes) = self
            .get_raw(&id_to_currency_pair_storage_key(id))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading currency pair from state")?
        else {
            return Ok(None);
        };
        let currency_pair = borsh::from_slice::<in_state::CurrencyPair>(&bytes)
            .wrap_err("failed to deserialize bytes read from state as currency pair")?
            .try_into()
            .wrap_err("failed converting in-state currency pair into domain type currency pair")?;
        Ok(Some(currency_pair))
    }

    #[instrument(skip_all)]
    fn currency_pairs_with_ids(&self) -> CurrencyPairsWithIdsStream<Self::PrefixRawStream> {
        CurrencyPairsWithIdsStream {
            underlying: self.prefix_raw(CURRENCY_PAIR_TO_ID_PREFIX),
        }
    }

    #[instrument(skip_all)]
    fn currency_pairs(&self) -> CurrencyPairsStream<Self::PrefixKeysStream> {
        CurrencyPairsStream {
            underlying: self.prefix_keys(CURRENCY_PAIR_STATE_PREFIX),
        }
    }

    #[instrument(skip_all)]
    async fn get_num_currency_pairs(&self) -> Result<u64> {
        let Some(bytes) = self
            .get_raw(NUM_CURRENCY_PAIRS_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading number of currency pairs from state")?
        else {
            return Ok(0);
        };
        let Count(num_currency_pairs) =
            Count::try_from_slice(&bytes).wrap_err("invalid number of currency pairs bytes")?;
        Ok(num_currency_pairs)
    }

    #[instrument(skip_all)]
    async fn get_num_removed_currency_pairs(&self) -> Result<u64> {
        let Some(bytes) = self
            .get_raw(NUM_REMOVED_CURRENCY_PAIRS_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading number of removed currency pairs from state")?
        else {
            return Ok(0);
        };
        let Count(num_removed_currency_pairs) = Count::try_from_slice(&bytes)
            .wrap_err("invalid number of removed currency pairs bytes")?;
        Ok(num_removed_currency_pairs)
    }

    #[instrument(skip_all)]
    async fn get_currency_pair_state(
        &self,
        currency_pair: &CurrencyPair,
    ) -> Result<Option<CurrencyPairState>> {
        let bytes = self
            .get_raw(&currency_pair_state_storage_key(currency_pair))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to get currency pair state from state")?;
        bytes
            .map(|bytes| {
                borsh::from_slice::<in_state::CurrencyPairState>(&bytes)
                    .wrap_err("failed to deserialize bytes read from state as currency pair state")
                    .map(Into::into)
            })
            .transpose()
    }

    #[instrument(skip_all)]
    async fn get_next_currency_pair_id(&self) -> Result<CurrencyPairId> {
        let Some(bytes) = self
            .get_raw(NEXT_CURRENCY_PAIR_ID_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading next currency pair id from state")?
        else {
            return Ok(CurrencyPairId::new(0));
        };
        let next_currency_pair_id = in_state::CurrencyPairId::try_from_slice(&bytes)
            .wrap_err("invalid next currency pair id bytes")?
            .into();
        Ok(next_currency_pair_id)
    }
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_currency_pair_id(
        &mut self,
        currency_pair: &CurrencyPair,
        id: CurrencyPairId,
    ) -> Result<()> {
        let bytes = borsh::to_vec(&in_state::CurrencyPairId::from(id))
            .wrap_err("failed to serialize currency pair id")?;
        self.put_raw(currency_pair_to_id_storage_key(currency_pair), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_currency_pair(&mut self, id: CurrencyPairId, currency_pair: CurrencyPair) -> Result<()> {
        let bytes = borsh::to_vec(&in_state::CurrencyPair::from(currency_pair))
            .wrap_err("failed to serialize currency pair")?;
        self.put_raw(id_to_currency_pair_storage_key(id), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_num_currency_pairs(&mut self, num_currency_pairs: u64) -> Result<()> {
        let bytes = borsh::to_vec(&Count(num_currency_pairs))
            .wrap_err("failed to serialize number of currency pairs")?;
        self.put_raw(NUM_CURRENCY_PAIRS_KEY.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_num_removed_currency_pairs(&mut self, num_removed_currency_pairs: u64) -> Result<()> {
        let bytes = borsh::to_vec(&Count(num_removed_currency_pairs))
            .wrap_err("failed to serialize number of removed currency pairs")?;
        self.put_raw(NUM_REMOVED_CURRENCY_PAIRS_KEY.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_currency_pair_state(
        &mut self,
        currency_pair: CurrencyPair,
        currency_pair_state: CurrencyPairState,
    ) -> Result<()> {
        let currency_pair_id = currency_pair_state.id;
        let bytes = borsh::to_vec(&in_state::CurrencyPairState::from(currency_pair_state))
            .wrap_err("failed to serialize currency pair state")?;
        self.put_raw(currency_pair_state_storage_key(&currency_pair), bytes);

        self.put_currency_pair_id(&currency_pair, currency_pair_id)
            .wrap_err("failed to put currency pair id")?;
        self.put_currency_pair(currency_pair_id, currency_pair)
            .wrap_err("failed to put currency pair")?;
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_next_currency_pair_id(&mut self, next_currency_pair_id: CurrencyPairId) -> Result<()> {
        let bytes = borsh::to_vec(&in_state::CurrencyPairId::from(next_currency_pair_id))
            .wrap_err("failed to serialize next currency pair id")?;
        self.put_raw(NEXT_CURRENCY_PAIR_ID_KEY.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    async fn put_price_for_currency_pair(
        &mut self,
        currency_pair: CurrencyPair,
        price: QuotePrice,
    ) -> Result<()> {
        let state = if let Some(mut state) = self
            .get_currency_pair_state(&currency_pair)
            .await
            .wrap_err("failed to get currency pair state")?
        {
            state.price = price;
            state.nonce = state
                .nonce
                .increment()
                .wrap_err("increment nonce overflowed")?;
            state
        } else {
            let id = self
                .get_next_currency_pair_id()
                .await
                .wrap_err("failed to read next currency pair ID")?;
            let next_id = id.increment().wrap_err("increment ID overflowed")?;
            self.put_next_currency_pair_id(next_id)
                .wrap_err("failed to put next currency pair ID")?;
            CurrencyPairState {
                price,
                nonce: CurrencyPairNonce::new(0),
                id,
            }
        };
        self.put_currency_pair_state(currency_pair, state)
            .wrap_err("failed to put currency pair state")?;
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}
