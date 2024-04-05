use super::{
    clone_response,
    AnyMessage,
};
use crate::erase_response;

pub fn constant_response<
    T: erased_serde::Serialize + prost::Name + Clone + Default + Send + Sync + 'static,
>(
    value: T,
) -> ConstantResponse {
    ConstantResponse {
        type_name: std::any::type_name::<T>(),
        response: erase_response(tonic::Response::new(value)),
    }
}

pub struct ConstantResponse {
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
>() -> DefaultResponse {
    let response = T::default();
    DefaultResponse {
        type_name: std::any::type_name::<T>(),
        response: erase_response(tonic::Response::new(response)),
    }
}

pub struct DefaultResponse {
    type_name: &'static str,
    response: tonic::Response<AnyMessage>,
}

impl Respond for DefaultResponse {
    fn respond(&self, _req: &tonic::Request<AnyMessage>) -> ResponseResult {
        Ok(MockResponse {
            type_name: self.type_name,
            inner: clone_response(&self.response),
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

pub trait Respond: Send + Sync {
    fn respond(&self, req: &tonic::Request<AnyMessage>) -> ResponseResult;
}
