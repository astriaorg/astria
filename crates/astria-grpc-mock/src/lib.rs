// allow: to be fixed in future PRs. This is used for testing and is not in a critical path.
#![expect(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::arithmetic_side_effects,
    reason = "only used for testing"
)]

use std::any::Any;

pub mod matcher;
mod mock;
mod mock_server;
mod mock_set;
mod mounted_mock;
pub mod response;
mod verification;

pub use mock::{
    Match,
    Mock,
    Times,
};
pub use mock_server::{
    MockGuard,
    MockServer,
};

/// A generic boxed gRPC message type, which can be used in both [`AnyRequest`] and [`AnyResponse`].
///
/// Types wished to be passed as this generic type must implement [`ErasedMessage`].
pub type AnyMessage = Box<dyn ErasedMessage + Send + Sync>;

/// Type alias for a tonic request with an [`AnyMessage`] as its message body.
pub type AnyRequest = tonic::Request<AnyMessage>;

/// Type alias for a tonic response with an [`AnyMessage`] as its message body.
pub type AnyResponse = tonic::Response<AnyMessage>;

/// Provides functionality for obtaining the name and type URL of an erased type. It is already
/// implemented for all types that implement [`prost::Name`].
pub trait ErasedName {
    /// Returns the full name of the erased type.
    fn full_name(&self) -> String;

    /// Returns the type URL of the erased type.
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

/// A trait for working with type erased messages. Types that implement this trait must also
/// implement `Any`, `erased_serde::Serialize`, and [`ErasedName`].
pub trait ErasedMessage: Any + erased_serde::Serialize + ErasedName {
    /// Clones `self` into a boxed trait object.
    fn clone_box(&self) -> Box<dyn ErasedMessage + Send + Sync>;

    /// Converts `self` into a boxed `Any` trait object, consuming `self` in the process.
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    /// Converts `self` into a boxed `erased_serde::Serialize` trait object, consuming `self` in the
    /// process.
    fn into_serialize(self: Box<Self>) -> Box<dyn erased_serde::Serialize>;

    /// Returns a reference to `self` as a boxed `Any` trait object.
    fn as_any(&self) -> &dyn Any;

    /// Returns a reference to `self` as a boxed [`ErasedName`] trait object.
    fn as_name(&self) -> &dyn ErasedName;

    /// Returns a reference to `self` as a boxed [`erased_serde::Serialize`] trait object.
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
