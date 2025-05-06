use std::{
    marker::PhantomData,
    pin::Pin,
    task::{
        ready,
        Poll,
    },
};

use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use futures::{
    Future,
    FutureExt as _,
    Stream,
    StreamExt as _,
};
use http::{
    Request,
    Response,
};
use pin_project_lite::pin_project;
use tonic::transport::Channel;
use tower::{
    util::BoxCloneService,
    ServiceBuilder,
};
use tower_http::{
    map_response_body::MapResponseBodyLayer,
    trace::{
        DefaultMakeSpan,
        TraceLayer,
    },
};

pub(crate) type InstrumentedChannel = BoxCloneService<
    Request<tonic::body::BoxBody>,
    Response<tonic::body::BoxBody>,
    tonic::transport::Error,
>;

pub(crate) fn make_instrumented_channel(uri: &str) -> eyre::Result<InstrumentedChannel> {
    // TODO(janis): understand what an appropriate setting for timeout or connect_timeout would be.
    //
    // We *do not* set timeouts because it's not clear what the correct behavior with streams is
    // intended to be.
    //
    // On a first connection to the auction service of the rollup, it can take several seconds
    // before any bids are received (because auctioneer first has to forward proposed blocks,
    // followed by an optimistic execution, and finally geth receiving bids based on that
    // optimistically executed block which it can then forward to auctioneer).
    //
    // We *do* set connect_timeout because that is for establishing the stream in the first place
    // irrespective of receiving over it. We rely on the reconnect logic of the underlying tonic
    // channel for things like network failure or geth not yet being online.
    let channel = Channel::from_shared(uri.to_string())
        .wrap_err("failed to create a channel to the provided uri")?
        .connect_timeout(std::time::Duration::from_secs(5))
        .connect_lazy();

    let channel = ServiceBuilder::new()
        .layer(MapResponseBodyLayer::new(tonic::body::BoxBody::new))
        .layer(
            TraceLayer::new_for_grpc().make_span_with(DefaultMakeSpan::new().include_headers(true)),
        )
        .service(channel);

    Ok(InstrumentedChannel::new(channel))
}

pub(crate) fn restarting_stream<F, Fut, S, T, E>(f: F) -> RestartingStream<F, Fut, S, T, E>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<S, E>>,
    S: Stream<Item = Result<T, E>>,
{
    let opening_stream = Some(f());
    RestartingStream {
        f,
        opening_stream,
        running_stream: None,
        _phantom_data: PhantomData,
    }
}

// TODO: Adds logs.
//
// Specifically explain why Fut returns Option<S>, and how to return
// an error to the user (tracing).
pin_project! {
    pub(crate) struct RestartingStream<F, Fut, S, T, E> {
        f: F,
        #[pin]
        opening_stream: Option<Fut>,
        #[pin]
        running_stream: Option<S>,
        _phantom_data: PhantomData<Result<T, E>>,
    }
}

impl<F, Fut, S, T, E> Stream for RestartingStream<F, Fut, S, T, E>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<S, E>>,
    S: Stream<Item = Result<T, E>>,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if this.opening_stream.is_some() {
            debug_assert!(this.running_stream.is_none());

            let open_output = ready!(this
                .opening_stream
                .as_mut()
                .as_pin_mut()
                .expect("inside a branch that checks opening_stream == Some")
                .poll_unpin(cx));

            // The future has completed, unset it so it will not be polled again.
            Pin::set(&mut this.opening_stream, None);
            match open_output {
                Ok(stream) => Pin::set(&mut this.running_stream, Some(stream)),
                Err(err) => return Poll::Ready(Some(Err(err))),
            }
        }

        if this.running_stream.is_some() {
            debug_assert!(this.opening_stream.is_none());

            if let Some(item) = ready!(this
                .running_stream
                .as_mut()
                .as_pin_mut()
                .expect("inside a branch that checks running_stream == Some")
                .poll_next_unpin(cx))
            {
                return Poll::Ready(Some(item));
            }

            Pin::set(&mut this.running_stream, None);
            Pin::set(&mut this.opening_stream, Some((*this.f)()));
            return Poll::Pending;
        }

        Poll::Ready(None)
    }
}
