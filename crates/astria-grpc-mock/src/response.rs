use std::{
    marker::PhantomData,
    time::Duration,
};

use super::{
    clone_response,
    AnyMessage,
};
use crate::{
    erase_response,
    AnyResponse,
};

pub struct ResponseTemplate {
    pub(crate) type_name: &'static str,
    pub(crate) response: AnyResponse,
    pub(crate) delay: Option<Duration>,
}

impl ResponseTemplate {
    #[must_use]
    pub fn set_delay(mut self, delay: Option<Duration>) -> Self {
        self.delay = delay;
        self
    }
}

impl Respond for ResponseTemplate {
    fn respond(&self, _req: &tonic::Request<AnyMessage>) -> ResponseResult {
        Ok(self.clone())
    }
}

impl Clone for ResponseTemplate {
    fn clone(&self) -> Self {
        Self {
            type_name: self.type_name,
            response: clone_response(&self.response),
            delay: self.delay,
        }
    }
}

#[must_use]
pub fn constant_response<
    T: erased_serde::Serialize + prost::Name + Clone + Default + Send + Sync + 'static,
>(
    value: T,
) -> ResponseTemplate {
    ResponseTemplate {
        type_name: std::any::type_name::<T>(),
        response: erase_response(tonic::Response::new(value)),
        delay: None,
    }
}

#[must_use]
pub fn default_response<
    T: erased_serde::Serialize + prost::Name + Clone + Default + Send + Sync + 'static,
>() -> ResponseTemplate {
    let response = T::default();
    ResponseTemplate {
        type_name: std::any::type_name::<T>(),
        response: erase_response(tonic::Response::new(response)),
        delay: None,
    }
}

pub fn dynamic_response<I, O, F>(responder: F) -> DynamicResponse<I, O, F>
where
    O: erased_serde::Serialize + prost::Name + Clone + 'static,
    F: Fn(&I) -> O,
{
    DynamicResponse {
        type_name: std::any::type_name::<O>(),
        responder: Box::new(responder),
        _phantom_data: PhantomData,
    }
}

pub struct DynamicResponse<I, O, F> {
    type_name: &'static str,
    responder: Box<F>,
    _phantom_data: PhantomData<(I, O)>,
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
        Ok(ResponseTemplate {
            type_name: self.type_name,
            response: erase_response(tonic::Response::new(resp)),
            delay: None,
        })
    }
}

pub type ResponseResult = Result<ResponseTemplate, tonic::Status>;

pub trait Respond: Send + Sync {
    fn respond(&self, req: &tonic::Request<AnyMessage>) -> ResponseResult;
}
