use std::{
    pin::Pin,
    task::{
        ready,
        Context,
        Poll,
    },
};

use anyhow::{
    bail,
    Context as _,
    Result,
};
use astria_core::slinky::{
    oracle::v1::{
        CurrencyPairState,
        QuotePrice,
    },
    types::v1::CurrencyPair,
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

const CURRENCY_PAIR_TO_ID_PREFIX: &str = "oraclecpid";
const ID_TO_CURRENCY_PAIR_PREFIX: &str = "oracleidcp";
const CURRENCY_PAIR_STATE_PREFIX: &str = "oraclecpstate";

const NUM_CURRENCY_PAIRS_KEY: &str = "oraclenumcps";
const NUM_REMOVED_CURRENCY_PAIRS_KEY: &str = "oraclenumremovedcps";
const NEXT_CURRENCY_PAIR_ID_KEY: &str = "oraclenextcpid";

fn currency_pair_to_id_storage_key(currency_pair: &CurrencyPair) -> String {
    format!("{CURRENCY_PAIR_TO_ID_PREFIX}/{currency_pair}",)
}

fn id_to_currency_pair_storage_key(id: u64) -> String {
    format!("{ID_TO_CURRENCY_PAIR_PREFIX}/{id}")
}

fn currency_pair_state_storage_key(currency_pair: &CurrencyPair) -> String {
    format!("{CURRENCY_PAIR_STATE_PREFIX}/{currency_pair}",)
}

/// Newtype wrapper to read and write a u64 from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Id(u64);

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
    St: Stream<Item = anyhow::Result<(String, Vec<u8>)>>,
{
    type Item = anyhow::Result<CurrencyPairWithId>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let (key, bytes) = match ready!(this.underlying.as_mut().poll_next(cx)) {
            Some(Ok(item)) => item,
            Some(Err(err)) => {
                return Poll::Ready(Some(Err(err).context("failed reading from state")));
            }
            None => return Poll::Ready(None),
        };
        let Id(id) = Id::try_from_slice(&bytes).with_context(|| {
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
    St: Stream<Item = anyhow::Result<String>>,
{
    type Item = anyhow::Result<CurrencyPair>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let key = match ready!(this.underlying.as_mut().poll_next(cx)) {
            Some(Ok(item)) => item,
            Some(Err(err)) => {
                return Poll::Ready(Some(Err(err).context("failed reading from state")));
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

fn extract_currency_pair_from_key(key: &str) -> anyhow::Result<CurrencyPair> {
    key.strip_prefix(CURRENCY_PAIR_TO_ID_PREFIX)
        .context("failed to strip prefix from currency pair state key")?
        .parse::<CurrencyPair>()
        .context("failed to parse storage key suffix as currency pair")
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    async fn get_currency_pair_id(&self, currency_pair: &CurrencyPair) -> Result<u64> {
        let Some(bytes) = self
            .get_raw(&currency_pair_to_id_storage_key(currency_pair))
            .await
            .context("failed reading currency pair id from state")?
        else {
            bail!("currency pair not found in state")
        };
        let Id(id) = Id::try_from_slice(&bytes).context("invalid currency pair id bytes")?;
        Ok(id)
    }

    #[instrument(skip_all)]
    async fn get_currency_pair(&self, id: u64) -> Result<Option<CurrencyPair>> {
        let bytes = self
            .get_raw(&id_to_currency_pair_storage_key(id))
            .await
            .context("failed to get currency pair from state")?;
        match bytes {
            Some(bytes) => {
                let currency_pair = serde_json::from_slice(&bytes)
                    .context("failed to deserialize currency pair")?;
                Ok(Some(currency_pair))
            }
            None => Ok(None),
        }
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
            .context("failed reading number of currency pairs from state")?
        else {
            return Ok(0);
        };
        let Count(num_currency_pairs) =
            Count::try_from_slice(&bytes).context("invalid number of currency pairs bytes")?;
        Ok(num_currency_pairs)
    }

    #[instrument(skip_all)]
    async fn get_num_removed_currency_pairs(&self) -> Result<u64> {
        let Some(bytes) = self
            .get_raw(NUM_REMOVED_CURRENCY_PAIRS_KEY)
            .await
            .context("failed reading number of removed currency pairs from state")?
        else {
            return Ok(0);
        };
        let Count(num_removed_currency_pairs) = Count::try_from_slice(&bytes)
            .context("invalid number of removed currency pairs bytes")?;
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
            .context("failed to get currency pair state from state")?;
        match bytes {
            Some(bytes) => {
                let currency_pair_state = serde_json::from_slice(&bytes)
                    .context("failed to deserialize currency pair state")?;
                Ok(Some(currency_pair_state))
            }
            None => Ok(None),
        }
    }

    #[instrument(skip_all)]
    async fn get_next_currency_pair_id(&self) -> Result<u64> {
        let Some(bytes) = self
            .get_raw(NEXT_CURRENCY_PAIR_ID_KEY)
            .await
            .context("failed reading next currency pair id from state")?
        else {
            return Ok(0);
        };
        let Id(next_currency_pair_id) =
            Id::try_from_slice(&bytes).context("invalid next currency pair id bytes")?;
        Ok(next_currency_pair_id)
    }
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_currency_pair_id(&mut self, currency_pair: &CurrencyPair, id: u64) -> Result<()> {
        let bytes = borsh::to_vec(&Id(id)).context("failed to serialize currency pair id")?;
        self.put_raw(currency_pair_to_id_storage_key(currency_pair), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_currency_pair(&mut self, id: u64, currency_pair: &CurrencyPair) -> Result<()> {
        let bytes =
            serde_json::to_vec(&currency_pair).context("failed to serialize currency pair")?;
        self.put_raw(id_to_currency_pair_storage_key(id), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_num_currency_pairs(&mut self, num_currency_pairs: u64) -> Result<()> {
        let bytes = borsh::to_vec(&Count(num_currency_pairs))
            .context("failed to serialize number of currency pairs")?;
        self.put_raw(NUM_CURRENCY_PAIRS_KEY.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_num_removed_currency_pairs(&mut self, num_removed_currency_pairs: u64) -> Result<()> {
        let bytes = borsh::to_vec(&Count(num_removed_currency_pairs))
            .context("failed to serialize number of removed currency pairs")?;
        self.put_raw(NUM_REMOVED_CURRENCY_PAIRS_KEY.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_currency_pair_state(
        &mut self,
        currency_pair: &CurrencyPair,
        currency_pair_state: CurrencyPairState,
    ) -> Result<()> {
        let bytes = serde_json::to_vec(&currency_pair_state)
            .context("failed to serialize currency pair state")?;
        self.put_raw(currency_pair_state_storage_key(currency_pair), bytes);
        self.put_currency_pair_id(currency_pair, currency_pair_state.id)
            .context("failed to put currency pair id")?;
        self.put_currency_pair(currency_pair_state.id, currency_pair)
            .context("failed to put currency pair")?;
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_next_currency_pair_id(&mut self, next_currency_pair_id: u64) -> Result<()> {
        let bytes = borsh::to_vec(&Id(next_currency_pair_id))
            .context("failed to serialize next currency pair id")?;
        self.put_raw(NEXT_CURRENCY_PAIR_ID_KEY.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    async fn put_price_for_currency_pair(
        &mut self,
        currency_pair: &CurrencyPair,
        price: QuotePrice,
    ) -> Result<()> {
        let state = if let Some(mut state) = self
            .get_currency_pair_state(currency_pair)
            .await
            .context("failed to get currency pair state")?
        {
            state.price = price;
            state.nonce.checked_add(1).context("nonce overflow")?;
            state
        } else {
            let id = self.get_next_currency_pair_id().await?;
            CurrencyPairState {
                price,
                nonce: 0,
                id,
            }
        };
        self.put_currency_pair_state(currency_pair, state)
            .context("failed to put currency pair state")?;
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}
