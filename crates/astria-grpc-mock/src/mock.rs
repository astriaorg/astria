use std::ops::{
    Range,
    RangeBounds as _,
    RangeFrom,
    RangeFull,
    RangeInclusive,
    RangeTo,
    RangeToInclusive,
};

use super::{
    response::Respond,
    AnyMessage,
};
use crate::{
    mock_server::MockGuard,
    MockServer,
};

pub trait Match: Send + Sync {
    fn matches(&self, req: &tonic::Request<AnyMessage>) -> bool;
}

pub(crate) struct Matcher(Box<dyn Match>);

impl Match for Matcher {
    fn matches(&self, request: &tonic::Request<AnyMessage>) -> bool {
        self.0.matches(request)
    }
}

pub struct Mock {
    pub(crate) rpc: &'static str,
    pub(crate) matchers: Vec<Matcher>,
    pub(crate) response: Box<dyn Respond>,
    pub(crate) max_n_matches: Option<u64>,
    pub(crate) expectation_range: Times,
    pub(crate) name: Option<String>,
}

impl Mock {
    pub fn for_rpc_given(rpc: &'static str, matcher: impl Match + 'static) -> MockBuilder {
        MockBuilder {
            rpc,
            matchers: vec![Matcher(Box::new(matcher))],
        }
    }

    #[must_use = "a mock must be mounted on a server to be useful"]
    pub fn up_to_n_times(mut self, n: u64) -> Self {
        assert!(n > 0, "n must be strictly greater than 0!");
        self.max_n_matches = Some(n);
        self
    }

    #[must_use = "a mock must be mounted on a server to be useful"]
    pub fn expect<T: Into<Times>>(mut self, r: T) -> Self {
        let range = r.into();
        self.expectation_range = range;
        self
    }

    #[must_use = "a mock must be mounted on a server to be useful"]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name.replace(name.into());
        self
    }

    pub async fn mount(self, server: &MockServer) {
        server.register(self).await;
    }

    pub async fn mount_as_scoped(self, server: &MockServer) -> MockGuard {
        server.register_as_scoped(self).await
    }
}

pub struct MockBuilder {
    rpc: &'static str,
    matchers: Vec<Matcher>,
}

impl MockBuilder {
    pub fn and(mut self, matcher: impl Match + 'static) -> Self {
        self.matchers.push(Matcher(Box::new(matcher)));
        self
    }

    pub fn respond_with(self, rsp: impl Respond + 'static) -> Mock {
        let Self {
            rpc,
            matchers,
        } = self;
        Mock {
            rpc,
            matchers,
            response: Box::new(rsp),
            max_n_matches: None,
            name: None,
            expectation_range: Times(TimesEnum::Unbounded(RangeFull)),
        }
    }
}

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
