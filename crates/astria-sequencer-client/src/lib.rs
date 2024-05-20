#[cfg(any(feature = "http", feature = "websocket"))]
pub mod extension_trait;

#[cfg(not(any(feature = "http", feature = "websocket")))]
compile_error!("at least one of the `http` or `websocket` features must be enabled");

use std::{
    future::Future,
    pin::Pin,
    time::Duration,
};

#[cfg(any(feature = "http", feature = "websocket"))]
pub use __feature_gated_exports::*;
pub use astria_core::{
    primitive::v1::Address,
    protocol::{
        account::v1alpha1::{
            BalanceResponse,
            NonceResponse,
        },
        transaction::v1alpha1::SignedTransaction,
    },
    sequencerblock::v1alpha1::SequencerBlock,
};
use futures_util::{
    FutureExt,
    Stream,
    StreamExt,
};
pub use tendermint;
use tendermint::block::Height;
pub use tendermint_proto;
pub use tendermint_rpc;
#[cfg(feature = "http")]
pub use tendermint_rpc::HttpClient;
#[cfg(feature = "websocket")]
pub use tendermint_rpc::WebSocketClient;
use tokio_stream::wrappers::IntervalStream;
#[cfg(any(feature = "http", feature = "websocket"))]
mod __feature_gated_exports {
    pub use tendermint_rpc::{
        Client,
        SubscriptionClient,
    };

    pub use crate::extension_trait::{
        NewBlockStreamError,
        SequencerClientExt,
        SequencerSubscriptionClientExt,
    };
}

pub trait StreamLatestHeight {
    fn stream_latest_height(&self, poll_period: Duration) -> LatestHeightStream;
}

#[cfg(feature = "http")]
impl StreamLatestHeight for HttpClient {
    fn stream_latest_height(&self, poll_period: Duration) -> LatestHeightStream {
        let client = self.clone();
        let f = Box::new(move |_: tokio::time::Instant| {
            let client = client.clone();
            async move {
                let info = match client.abci_info().await {
                    Ok(info) => info,
                    Err(e) => return Err(e),
                };
                Ok(info.last_block_height)
            }
            .boxed()
        });
        let mut interval = tokio::time::interval(poll_period);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        LatestHeightStream {
            inner: IntervalStream::new(interval).then(f),
        }
    }
}

type HeightFromAbciFut =
    Pin<Box<dyn Future<Output = Result<Height, tendermint_rpc::Error>> + Send>>;

pub struct LatestHeightStream {
    inner: futures_util::stream::Then<
        IntervalStream,
        HeightFromAbciFut,
        Box<dyn FnMut(tokio::time::Instant) -> HeightFromAbciFut + Send>,
    >,
}

impl Stream for LatestHeightStream {
    type Item = Result<Height, tendermint_rpc::Error>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.poll_next_unpin(cx)
    }
}

#[cfg(test)]
mod tests;
