// allow: to be fixed in future PRs. This is used for testing and is not in a critical path.
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use std::any::Any;

pub mod matcher;
mod mock;
mod mock_server;
mod mock_set;
mod mounted_mock;
pub mod response;
mod verification;

pub use mock::Mock;
pub use mock_server::{
    MockGuard,
    MockServer,
};

pub type AnyMessage = Box<dyn ErasedMessage + Send + Sync>;
pub type AnyRequest = tonic::Request<AnyMessage>;
pub type AnyResponse = tonic::Response<AnyMessage>;

pub trait ErasedName {
    fn full_name(&self) -> String;

    fn type_url(&self) -> String;
}

impl<T> ErasedName for T
where
    T: prost::Name,
{
    fn full_name(&self) -> String {
        <T as prost::Name>::full_name()
    }

    fn type_url(&self) -> String {
        <T as prost::Name>::type_url()
    }
}

pub trait ErasedMessage: Any + erased_serde::Serialize + ErasedName {
    fn clone_box(&self) -> Box<dyn ErasedMessage + Send + Sync>;

    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    fn into_serialize(self: Box<Self>) -> Box<dyn erased_serde::Serialize>;

    fn as_any(&self) -> &dyn Any;

    fn as_name(&self) -> &dyn ErasedName;

    fn as_serialize(&self) -> &dyn erased_serde::Serialize;
}

impl<T> ErasedMessage for T
where
    T: Clone + erased_serde::Serialize + ErasedName + Send + Sync + 'static,
{
    fn clone_box(&self) -> Box<dyn ErasedMessage + Send + Sync> {
        Box::new(self.clone())
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn into_serialize(self: Box<Self>) -> Box<dyn erased_serde::Serialize> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_name(&self) -> &dyn ErasedName {
        self
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self
    }
}

impl Clone for AnyMessage {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

fn erase_request<T>(req: tonic::Request<T>) -> AnyRequest
where
    T: Clone + erased_serde::Serialize + prost::Name + Send + Sync + 'static,
{
    let (metadata, extensions, message) = req.into_parts();
    let boxed = Box::new(message) as AnyMessage;
    tonic::Request::from_parts(metadata, extensions, boxed)
}

fn erase_response<T>(rsp: tonic::Response<T>) -> AnyResponse
where
    T: Clone + erased_serde::Serialize + prost::Name + Send + Sync + 'static,
{
    let (metadata, message, extensions) = rsp.into_parts();
    let boxed = Box::new(message) as AnyMessage;
    tonic::Response::from_parts(metadata, boxed, extensions)
}

fn clone_request<T: Clone>(req: &tonic::Request<T>) -> tonic::Request<T> {
    let mut clone = tonic::Request::new(req.get_ref().clone());
    *clone.metadata_mut() = req.metadata().clone();
    clone
}

fn clone_response<T: Clone>(rsp: &tonic::Response<T>) -> tonic::Response<T> {
    let mut clone = tonic::Response::new(rsp.get_ref().clone());
    *clone.metadata_mut() = rsp.metadata().clone();
    clone
}
