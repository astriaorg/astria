use std::ops::{
    Range,
    RangeBounds as _,
    RangeFrom,
    RangeFull,
    RangeInclusive,
    RangeTo,
    RangeToInclusive,
};

use super::AnyMessage;
use crate::{
    mock_server::MockGuard,
    response::ResponseTemplate,
    MockServer,
};

/// Provides the method for determining whether a message matches the given matcher.
///
/// It is implemented for the following types:
/// * [`crate::matcher::MessagePartialJsonMatcher`]
/// * [`crate::matcher::MessageExactMatcher`]
/// * [`crate::matcher::MessageTypeMatcher`]
///
/// This trait can also be implemented to use custom matchers.
pub trait Match: Send + Sync {
    /// Returns true if the given request fulfills the matcher's criteria, false if otherwise.
    fn matches(&self, req: &tonic::Request<AnyMessage>) -> bool;
}

pub(crate) struct Matcher(Box<dyn Match>);

impl Match for Matcher {
    fn matches(&self, request: &tonic::Request<AnyMessage>) -> bool {
        self.0.matches(request)
    }
}

/// A mock that can be mounted on a [`MockServer`] which will respond to matching requests with a
/// gRPC response.
///
/// Given an rpc whose request fulfills all the mock's matchers, it will respond with
/// `response::respond()` up to `n` times (if it has been set). The mock can be set to expect a
/// range in number of requests and can also be given a name, which is best practice.
///
/// # Examples
/// ```rust
/// use astria_grpc_mock::{
///     matcher,
///     response,
///     AnyMessage,
///     Mock,
///     MockServer,
/// };
/// use futures::executor::block_on;
///
/// let server = MockServer::new();
/// let mock = Mock::for_rpc_given("rpc", matcher::message_type::<AnyMessage>())
///     .respond_with(response::error_response(0.into()))
///     .up_to_n_times(1)
///     .expect(0)
///     .with_name("mock name");
/// let _mock_guard = block_on(mock.mount_as_scoped(&server));
/// block_on(server.verify());
/// ```
pub struct Mock {
    pub(crate) rpc: &'static str,
    pub(crate) matchers: Vec<Matcher>,
    pub(crate) response: ResponseTemplate,
    pub(crate) max_n_matches: Option<u64>,
    pub(crate) expectation_range: Times,
    pub(crate) name: Option<String>,
}

impl Mock {
    /// Creates a mock builder for the given `rpc` with a given `Matcher`. See
    /// [`Mock`] docs for example usage.
    pub fn for_rpc_given(rpc: &'static str, matcher: impl Match + 'static) -> MockBuilder {
        MockBuilder {
            rpc,
            matchers: vec![Matcher(Box::new(matcher))],
        }
    }

    /// Sets the maximum number of times (inclusive) that the [`Mock`] will respond. See [`Mock`]
    /// docs for example usage.
    #[must_use = "a mock must be mounted on a server to be useful"]
    pub fn up_to_n_times(mut self, n: u64) -> Self {
        assert!(n > 0, "n must be strictly greater than 0!");
        self.max_n_matches = Some(n);
        self
    }

    /// Sets the range of times that the [`Mock`] should expect to receive a matching
    /// request. If the mock is mounted via [`mount_as_scoped`](`Mock::mount_as_scoped`), this range
    /// will be verified either upon calling [`MockGuard::wait_until_satisfied`] or when the
    /// guard is dropped. See [`Mock`] docs for example usage.
    #[must_use = "a mock must be mounted on a server to be useful"]
    pub fn expect<T: Into<Times>>(mut self, r: T) -> Self {
        let range = r.into();
        self.expectation_range = range;
        self
    }

    /// Sets the name of the given [`Mock`]. Calling this is best practice, as the name will be
    /// displayed if validation (of the server or mock) fails. See [`Mock`] docs for example usage.
    #[must_use = "a mock must be mounted on a server to be useful"]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name.replace(name.into());
        self
    }

    /// Registers the [`Mock`] on the given [`MockServer`]. See [`Mock`] docs for example usage.
    pub async fn mount(self, server: &MockServer) {
        server.register(self).await;
    }

    /// Registers the [`Mock`] on the given [`MockServer`] and returns a [`MockGuard`],
    /// which can be evaluated for verification that the mock was called the expected
    /// and returned no bad responses. See [`Mock`] docs for example usage.
    pub async fn mount_as_scoped(self, server: &MockServer) -> MockGuard {
        server.register_as_scoped(self).await
    }
}

/// A builder for [`Mock`] which is returned by [`Mock::for_rpc_given`].
/// Takes a given `rpc` and a list of `Matcher`s, and returns a [`Mock`].
/// Optionally, additional matchers can be added with the [`MockBuilder::and`] method.
///
/// # Examples
/// ```rust
/// use astria_grpc_mock::{
///     matcher,
///     response,
///     AnyMessage,
///     Mock,
///     MockServer,
/// };
/// use futures::executor::block_on;
///
/// let server = MockServer::new();
/// let mock_builder = Mock::for_rpc_given("rpc", matcher::message_type::<AnyMessage>())
///     .and(matcher::message_type::<AnyMessage>());
/// let mock = mock_builder.respond_with(response::error_response(0.into()));
/// block_on(mock.mount(&server));
/// block_on(server.verify());
/// ```
pub struct MockBuilder {
    rpc: &'static str,
    matchers: Vec<Matcher>,
}

impl MockBuilder {
    /// Adds an additional `Matcher` to the [`MockBuilder`]'s `matchers` set. There is no maximum
    /// number of matchers that can be pushed. See [`MockBuilder`] docs for example usage.
    pub fn and(mut self, matcher: impl Match + 'static) -> Self {
        self.matchers.push(Matcher(Box::new(matcher)));
        self
    }

    /// Returns a [`Mock`] which will respond with the given [`ResponseTemplate`]'s `respond()`
    /// implementation. See [`MockBuilder`] docs for example usage.
    pub fn respond_with(self, rsp: ResponseTemplate) -> Mock {
        let Self {
            rpc,
            matchers,
        } = self;
        Mock {
            rpc,
            matchers,
            response: rsp,
            max_n_matches: None,
            name: None,
            expectation_range: Times(TimesEnum::Unbounded(RangeFull)),
        }
    }
}

/// A range used by [`Mock`] to specify how many times it should expect to receive
/// a matching request.
#[derive(Clone, Debug)]
pub struct Times(TimesEnum);

impl Times {
    pub(crate) fn contains(&self, n_calls: u64) -> bool {
        match &self.0 {
            TimesEnum::Exact(e) => e == &n_calls,
            TimesEnum::Unbounded(r) => r.contains(&n_calls),
            TimesEnum::Range(r) => r.contains(&n_calls),
            TimesEnum::RangeFrom(r) => r.contains(&n_calls),
            TimesEnum::RangeTo(r) => r.contains(&n_calls),
            TimesEnum::RangeToInclusive(r) => r.contains(&n_calls),
            TimesEnum::RangeInclusive(r) => r.contains(&n_calls),
        }
    }
}

impl std::fmt::Display for Times {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            TimesEnum::Exact(e) => write!(f, "== {e}"),
            TimesEnum::Unbounded(_) => write!(f, "0 <= x"),
            TimesEnum::Range(r) => write!(f, "{} <= x < {}", r.start, r.end),
            TimesEnum::RangeFrom(r) => write!(f, "{} <= x", r.start),
            TimesEnum::RangeTo(r) => write!(f, "0 <= x < {}", r.end),
            TimesEnum::RangeToInclusive(r) => write!(f, "0 <= x <= {}", r.end),
            TimesEnum::RangeInclusive(r) => write!(f, "{} <= x <= {}", r.start(), r.end()),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum TimesEnum {
    Exact(u64),
    Unbounded(RangeFull),
    Range(Range<u64>),
    RangeFrom(RangeFrom<u64>),
    RangeTo(RangeTo<u64>),
    RangeToInclusive(RangeToInclusive<u64>),
    RangeInclusive(RangeInclusive<u64>),
}

impl From<u64> for Times {
    fn from(x: u64) -> Self {
        Times(TimesEnum::Exact(x))
    }
}

// A quick macro to help easing the implementation pain.
macro_rules! impl_from_for_range {
    ($type_name:ident) => {
        impl From<$type_name<u64>> for Times {
            fn from(r: $type_name<u64>) -> Self {
                Times(TimesEnum::$type_name(r))
            }
        }
    };
}

impl_from_for_range!(Range);
impl_from_for_range!(RangeTo);
impl_from_for_range!(RangeFrom);
impl_from_for_range!(RangeInclusive);
impl_from_for_range!(RangeToInclusive);
