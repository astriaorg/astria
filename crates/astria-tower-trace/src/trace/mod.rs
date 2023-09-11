use std::fmt;
use tower_service::Service;
use tracing::Level;

pub mod request_span;
pub mod service_span;

pub type InstrumentedService<S, R> = service_span::Service<request_span::Service<S, R>>;

pub trait InstrumentableService<Request>
where
    Self: Service<Request> + Sized,
{
    fn instrument<G>(self, svc_span: G) -> InstrumentedService<Self, Request>
    where
        G: GetSpan<Self>,
        Request: fmt::Debug,
    {
        let req_span: fn(&Request) -> tracing::Span =
            |request| tracing::span!(Level::TRACE, "request", ?request);
        let svc_span = svc_span.span_for(&self);
        self.trace_requests(req_span).trace_service(svc_span)
    }

    fn trace_requests<G>(self, get_span: G) -> request_span::Service<Self, Request, G>
    where
        G: GetSpan<Request> + Clone,
    {
        request_span::Service::new(self, get_span)
    }

    fn trace_service<G>(self, get_span: G) -> service_span::Service<Self>
    where
        G: GetSpan<Self>,
    {
        let span = get_span.span_for(&self);
        service_span::Service::new(self, span)
    }
}

pub trait InstrumentMake<T, R>
where
    Self: tower::make::MakeService<T, R> + Sized,
{
    fn with_traced_service<G>(self, get_span: G) -> service_span::MakeService<Self, T, R, G>
    where
        G: GetSpan<T>,
    {
        service_span::MakeService::new(self, get_span)
    }

    fn with_traced_requests<G>(self, get_span: G) -> request_span::MakeService<Self, R, G>
    where
        G: GetSpan<R> + Clone,
    {
        request_span::MakeService::new(self, get_span)
    }
}

impl<S, R> InstrumentableService<R> for S where S: Service<R> + Sized {}

impl<M, T, R> InstrumentMake<T, R> for M where M: tower::make::MakeService<T, R> {}

pub trait GetSpan<T>: sealed::Sealed<T> {
    fn span_for(&self, target: &T) -> tracing::Span;
}

impl<T, F> sealed::Sealed<T> for F where F: Fn(&T) -> tracing::Span {}

impl<T, F> GetSpan<T> for F
where
    F: Fn(&T) -> tracing::Span,
{
    #[inline]
    fn span_for(&self, target: &T) -> tracing::Span {
        (self)(target)
    }
}

impl<T> sealed::Sealed<T> for tracing::Span {}

impl<T> GetSpan<T> for tracing::Span {
    #[inline]
    fn span_for(&self, _: &T) -> tracing::Span {
        self.clone()
    }
}

mod sealed {
    pub trait Sealed<T = ()> {}
}
