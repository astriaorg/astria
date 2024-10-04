use std::any::TypeId;

use assert_json_diff::{
    assert_json_matches_no_panic,
    CompareMode,
};
use serde_json::Value;

use crate::mock::Match;

/// Returns a [`MessagePartialJsonMatcher`] to be included as an argument in
/// [`Mock::for_rpc_given`]. Matcher will return true if the given request's message is contained in
/// the expected message.
///
/// # Examples
///
/// ```rust
/// use astria_grpc_mock::matcher;
///
/// // returns a mock builder which will match any request with a message that contains "expected message"
/// let _mock_builder = astria_grpc_mock::Mock::for_rpc_given("rpc", matcher::message_partial_pbjson(&"expected message"));
/// ```
pub fn message_partial_pbjson<T: serde::Serialize>(value: &T) -> MessagePartialJsonMatcher {
    MessagePartialJsonMatcher(
        serde_json::to_value(value).expect("can map provided protobuf message to JSON"),
    )
}

/// A matcher returned by [`message_partial_pbjson`], which will match any JSON message contained by
/// the expected message.
pub struct MessagePartialJsonMatcher(Value);

impl Match for MessagePartialJsonMatcher {
    fn matches(&self, req: &tonic::Request<crate::AnyMessage>) -> bool {
        let req_json = serde_json::to_value(req.get_ref().as_serialize())
            .expect("can map provided gRPC request to JSON");
        let config = assert_json_diff::Config::new(CompareMode::Inclusive);
        assert_json_matches_no_panic(&req_json, &self.0, config).is_ok()
    }
}

/// Returns a [`MessageExactMatcher`] to be included as an argument in [`Mock::for_rpc_given`].
/// Matcher will return true only if the given request's message exactly matches the expected
/// message.
///
/// # Examples
///
/// ```rust
/// use astria_grpc_mock::matcher;
///
/// // returns a mock builder which will match any request whose message is "expected message"
/// let _mock_builder = astria_grpc_mock::Mock::for_rpc_given(
///     "rpc",
///     matcher::message_exact_pbjson(&"expected message"),
/// );
/// ```
pub fn message_exact_pbjson<T: serde::Serialize>(value: &T) -> MessageExactMatcher {
    MessageExactMatcher::json(value)
}

/// A matcher returned by [`message_exact_pbjson`], which will match only exact JSON messages.
pub enum MessageExactMatcher {
    Json(Value),
}

impl MessageExactMatcher {
    fn json<T: serde::Serialize>(value: &T) -> Self {
        Self::Json(serde_json::to_value(value).expect("can map provided protobuf message to JSON"))
    }
}

impl Match for MessageExactMatcher {
    fn matches(&self, req: &tonic::Request<crate::AnyMessage>) -> bool {
        match self {
            Self::Json(json) => {
                let req_json = serde_json::to_value(req.get_ref().as_serialize())
                    .expect("can map provided gRPC request to JSON");
                *json == req_json
            }
        }
    }
}

/// Returns a [`MessageTypeMatcher`] to be included as an argument in [`Mock::for_rpc_given`].
/// Matcher will return true if the given request's message is the same type as the expected
/// message.
///
/// # Examples
///
/// ```rust
/// use astria_grpc_mock::matcher;
///
/// // returns a mock builder which will match any request whose message is of type `&str`
/// let _mock_builder =
///     astria_grpc_mock::Mock::for_rpc_given("rpc", matcher::message_type::<&str>());
/// ```
#[must_use = "a matcher must be used in a mock to be useful"]
pub fn message_type<T: 'static>() -> MessageTypeMatcher {
    MessageTypeMatcher {
        type_name: TypeId::of::<T>(),
    }
}

/// A matcher returned by [`message_type`], which will match only messages of the same type ID.
pub struct MessageTypeMatcher {
    type_name: TypeId,
}

impl Match for MessageTypeMatcher {
    fn matches(&self, req: &tonic::Request<crate::AnyMessage>) -> bool {
        self.type_name == req.get_ref().as_any().type_id()
    }
}
