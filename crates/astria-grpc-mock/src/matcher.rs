use std::any::TypeId;

use assert_json_diff::{
    assert_json_matches_no_panic,
    CompareMode,
};
use serde_json::Value;

use crate::mock::Match;

pub fn message_partial_pbjson<T: serde::Serialize>(value: &T) -> MessagePartialJsonMatcher {
    MessagePartialJsonMatcher(
        serde_json::to_value(value).expect("can map provided protobuf message to JSON"),
    )
}

pub struct MessagePartialJsonMatcher(Value);

impl Match for MessagePartialJsonMatcher {
    fn matches(&self, req: &tonic::Request<crate::AnyMessage>) -> bool {
        let req_json = serde_json::to_value(req.get_ref().as_serialize())
            .expect("can map provided gRPC request to JSON");
        let config = assert_json_diff::Config::new(CompareMode::Inclusive);
        assert_json_matches_no_panic(&req_json, &self.0, config).is_ok()
    }
}

pub fn message_exact_pbjson<T: serde::Serialize>(value: &T) -> MessageExactMatcher {
    MessageExactMatcher::json(value)
}

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

#[must_use = "a matcher must be used in a mock to be useful"]
pub fn message_type<T: 'static>() -> MessageTypeMatcher {
    MessageTypeMatcher {
        type_name: TypeId::of::<T>(),
    }
}

pub struct MessageTypeMatcher {
    type_name: TypeId,
}

impl Match for MessageTypeMatcher {
    fn matches(&self, req: &tonic::Request<crate::AnyMessage>) -> bool {
        self.type_name == req.get_ref().as_any().type_id()
    }
}
