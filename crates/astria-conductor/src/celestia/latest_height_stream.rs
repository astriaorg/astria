use std::{
    pin::Pin,
    time::Duration,
};

use astria_eyre::eyre::{
    Result,
    WrapErr as _,
};
use celestia_client::{
    celestia_rpc::HeaderClient as _,
    jsonrpsee::http_client::HttpClient,
};
use futures::{
    Future,
    FutureExt as _,
    Stream,
    StreamExt as _,
};
use tokio_stream::wrappers::IntervalStream;

pub(super) fn stream_latest_heights(
    client: HttpClient,
    poll_period: Duration,
) -> LatestHeightStream {
    let f = Box::new(move |_: tokio::time::Instant| {
        let client = client.clone();
        async move {
            client
                .header_network_head()
                .await
                .wrap_err("failed to fetch network head")
                .map(|header| header.height().value())
        }
        .boxed()
    });
    let mut interval = tokio::time::interval(poll_period);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    LatestHeightStream {
        inner: IntervalStream::new(interval).then(f),
    }
}

type HeightFromHeaderFut = Pin<Box<dyn Future<Output = Result<u64>> + Send>>;

pub(super) struct LatestHeightStream {
    inner: futures::stream::Then<
        IntervalStream,
        HeightFromHeaderFut,
        Box<dyn FnMut(tokio::time::Instant) -> HeightFromHeaderFut + Send>,
    >,
}

impl Stream for LatestHeightStream {
    type Item = Result<u64>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.poll_next_unpin(cx)
    }
}
