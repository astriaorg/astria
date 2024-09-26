use std::{
    marker::PhantomData,
    time::Duration,
};

use super::{
    clone_response,
    AnyMessage,
};
use crate::erase_response;

pub fn constant_response<
    T: erased_serde::Serialize + prost::Name + Clone + Default + Send + Sync + 'static,
>(
    value: T,
) -> ResponseTemplate {
    ResponseTemplate {
        response: Box::new(ConstantResponse {
            type_name: std::any::type_name::<T>(),
            response: erase_response(tonic::Response::new(value)),
        }),
        delay: None,
    }
}

struct ConstantResponse {
    type_name: &'static str,
    response: tonic::Response<AnyMessage>,
}

impl Respond for ConstantResponse {
    fn respond(&self, _req: &tonic::Request<AnyMessage>) -> ResponseResult {
        Ok(MockResponse {
            type_name: self.type_name,
            inner: clone_response(&self.response),
        })
    }
}

#[must_use]
pub fn default_response<
    T: erased_serde::Serialize + prost::Name + Clone + Default + Send + Sync + 'static,
>() -> ResponseTemplate {
    let response = T::default();
    ResponseTemplate {
        response: Box::new(ConstantResponse {
            type_name: std::any::type_name::<T>(),
            response: erase_response(tonic::Response::new(response)),
        }),
        delay: None,
    }
}

pub fn dynamic_response<I, O, F>(responder: F) -> ResponseTemplate
where
    O: erased_serde::Serialize + prost::Name + Clone + 'static,
    F: Send + Sync + 'static + Fn(&I) -> O,
    I: Send + Sync + 'static,
{
    ResponseTemplate {
        response: Box::new(DynamicResponse {
            type_name: std::any::type_name::<O>(),
            responder: Box::new(responder),
            _phantom_data: PhantomData,
        }),
        delay: None,
    }
}

pub struct DynamicResponse<I, O, F> {
    type_name: &'static str,
    responder: Box<F>,
    _phantom_data: PhantomData<(I, O)>,
}

struct ErrorResponse {
    status: tonic::Status,
}

impl Respond for ErrorResponse {
    fn respond(&self, _req: &tonic::Request<AnyMessage>) -> ResponseResult {
        Err(self.status.clone())
    }
}

#[must_use]
pub fn error_response(code: tonic::Code) -> ResponseTemplate {
    ResponseTemplate {
        response: Box::new(ErrorResponse {
            status: tonic::Status::new(code, "error"),
        }),
        delay: None,
    }
}

impl<I, O, F> Respond for DynamicResponse<I, O, F>
where
    I: Send + Sync + 'static,
    O: erased_serde::Serialize + prost::Name + Clone + 'static,
    F: Send + Sync + Fn(&I) -> O,
{
    fn respond(&self, outer_req: &tonic::Request<AnyMessage>) -> ResponseResult {
        let erased_req = outer_req.get_ref();
        let Some(req) = erased_req.as_any().downcast_ref::<I>() else {
            let actual = erased_req.as_name().full_name();
            let expected = std::any::type_name::<I>();
            let req_as_json = serde_json::to_string(erased_req.as_serialize())
                .expect("can map registered protobuf response to json");
            let msg = format!(
                "failed downcasting request to concrete type; expected type of request: \
                 `{expected}`, actual type of request: `{actual}`, request: {req_as_json}",
            );
            return Err(tonic::Status::internal(msg));
        };

        let resp = (self.responder)(req);
        Ok(MockResponse {
            type_name: self.type_name,
            inner: erase_response(tonic::Response::new(resp)),
        })
    }
}

pub struct MockResponse {
    pub(crate) type_name: &'static str,
    pub(crate) inner: tonic::Response<AnyMessage>,
}

pub type ResponseResult = Result<MockResponse, tonic::Status>;

impl Clone for MockResponse {
    fn clone(&self) -> Self {
        let inner = clone_response(&self.inner);
        Self {
            type_name: self.type_name,
            inner,
        }
    }
}

pub struct ResponseTemplate {
    response: Box<dyn Respond>,
    delay: Option<Duration>,
}

impl ResponseTemplate {
    pub(crate) fn respond(
        &self,
        req: &tonic::Request<AnyMessage>,
    ) -> (ResponseResult, Option<Duration>) {
        (self.response.respond(req), self.delay)
    }

    #[must_use]
    pub fn set_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }
}

pub trait Respond: Send + Sync {
    fn respond(&self, req: &tonic::Request<AnyMessage>) -> ResponseResult;
}
