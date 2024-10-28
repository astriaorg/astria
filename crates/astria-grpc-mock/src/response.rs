//! Contains the logic for constructing responses for a [`Mock`](`crate::Mock`).

use std::{
    marker::PhantomData,
    time::Duration,
};

use super::{
    clone_response,
    AnyMessage,
};
use crate::erase_response;

/// Constructs a [`ResponseTemplate`] that will respond with [`tonic::Response<T>`]
/// where `T` is the type of `value` and the `message` of the response is `value`.
///
/// # Examples
///
/// ```rust
/// use astria_grpc_mock::{
///     matcher,
///     response,
///     AnyMessage,
///     Mock,
///     MockServer,
/// };
/// use futures::{
///     executor::block_on,
///     future::join,
/// };
///
/// // `MockMessage` implementation hidden for brevity
/// # #[derive(serde::Serialize, ::prost::Message, Clone, PartialEq)]
/// # struct MockMessage {
/// #     #[prost(string, tag = "1")]
/// #     name: String,
/// # }
/// # impl ::prost::Name for MockMessage {
/// #     const NAME: &'static str = "MockMessage";
/// #     const PACKAGE: &'static str = "test";
/// #
/// #     fn full_name() -> ::prost::alloc::string::String {
/// #         ::prost::alloc::format!("test.{}", Self::NAME)
/// #     }
/// # }
///
/// let mock_message = MockMessage {
///     name: "test".to_string(),
/// };
/// let server = MockServer::new();
/// let mock_fut = Mock::for_rpc_given("rpc", matcher::message_exact_pbjson(&mock_message))
///     .respond_with(response::constant_response(mock_message.clone()))
///     .mount(&server);
/// let rsp_fut = server.handle_request::<MockMessage, MockMessage>(
///     "rpc",
///     tonic::Request::new(mock_message.clone()),
/// );
/// let (_, rsp) = block_on(join(mock_fut, rsp_fut));
/// assert_eq!(rsp.unwrap().into_inner(), mock_message);
/// ```
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

/// Constructs a [`ResponseTemplate`] that will respond with [`tonic::Response<T>`]
/// where the constant response is the default value of `T`.
///
/// # Examples
///
/// ```rust
/// use astria_grpc_mock::{
///     matcher,
///     response,
///     AnyMessage,
///     Mock,
///     MockServer,
/// };
/// use futures::{
///     executor::block_on,
///     future::join,
/// };
///
/// // `MockMessage` implementation hidden for brevity
/// # #[derive(serde::Serialize, ::prost::Message, Clone, PartialEq)]
/// # struct MockMessage {
/// #     #[prost(string, tag = "1")]
/// #     name: String,
/// # }
/// # impl ::prost::Name for MockMessage {
/// #     const NAME: &'static str = "MockMessage";
/// #     const PACKAGE: &'static str = "test";
/// #
/// #     fn full_name() -> ::prost::alloc::string::String {
/// #         ::prost::alloc::format!("test.{}", Self::NAME)
/// #     }
/// # }
///
/// let mock_message = MockMessage {
///     name: String::new(),
/// };
/// let server = MockServer::new();
/// let mock_fut = Mock::for_rpc_given("rpc", matcher::message_exact_pbjson(&mock_message))
///     .respond_with(response::default_response::<MockMessage>())
///     .mount(&server);
/// let rsp_fut = server.handle_request::<MockMessage, MockMessage>(
///     "rpc",
///     tonic::Request::new(mock_message.clone()),
/// );
/// let (_, rsp) = block_on(join(mock_fut, rsp_fut));
/// assert_eq!(rsp.unwrap().into_inner(), mock_message);
/// ```
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

/// Constructs a [`ResponseTemplate`] that will respond with [`tonic::Response<T>`],
/// where the response is the return value of `responder({Request})`.
///
/// # Examples
///
/// ```rust
/// use astria_grpc_mock::{
///     matcher,
///     response,
///     AnyMessage,
///     Mock,
///     MockServer,
/// };
/// use futures::{
///     executor::block_on,
///     future::join,
/// };
///
/// // `MockMessage` implementation hidden for brevity
/// # #[derive(serde::Serialize, ::prost::Message, Clone, PartialEq)]
/// # struct MockMessage {
/// #     #[prost(string, tag = "1")]
/// #     name: String,
/// # }
/// # impl ::prost::Name for MockMessage {
/// #     const NAME: &'static str = "MockMessage";
/// #     const PACKAGE: &'static str = "test";
/// #
/// #     fn full_name() -> ::prost::alloc::string::String {
/// #         ::prost::alloc::format!("test.{}", Self::NAME)
/// #     }
/// # }
///
/// let mock_message = MockMessage {
///     name: "test".to_string(),
/// };
/// let server = MockServer::new();
/// let responder = |req: &MockMessage| MockMessage {
///     name: req.name.clone(),
/// };
/// let mock_fut = Mock::for_rpc_given("rpc", matcher::message_exact_pbjson(&mock_message))
///     .respond_with(response::dynamic_response(responder))
///     .mount(&server);
/// let rsp_fut = server.handle_request::<MockMessage, MockMessage>(
///     "rpc",
///     tonic::Request::new(mock_message.clone()),
/// );
/// let (_, rsp) = block_on(join(mock_fut, rsp_fut));
/// assert_eq!(rsp.unwrap().into_inner(), mock_message);
/// ```
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

struct DynamicResponse<I, O, F> {
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

/// Constructs a [`ResponseTemplate`] that will respond with a [`tonic::Status`] error containing
/// the given [`tonic::Code`].
///
/// # Examples
///
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
/// // `MockMessage` implementation hidden for brevity
/// # #[derive(serde::Serialize, ::prost::Message, Clone, PartialEq)]
/// # struct MockMessage {
/// #     #[prost(string, tag = "1")]
/// #     name: String,
/// # }
/// # impl ::prost::Name for MockMessage {
/// #     const NAME: &'static str = "MockMessage";
/// #     const PACKAGE: &'static str = "test";
/// #
/// #     fn full_name() -> ::prost::alloc::string::String {
/// #        ::prost::alloc::format!("test.{}", Self::NAME)
/// #    }
/// # }
/// #
///
/// let mock_message = MockMessage {
///     name: "test".to_string(),
/// };
/// let server = MockServer::new();
///
/// block_on(
///     Mock::for_rpc_given("rpc", matcher::message_exact_pbjson(&mock_message))
///         .respond_with(response::error_response(2.into())) // mount `Code::Unknown`
///         .mount(&server),
/// );
///
/// let rsp = block_on(server.handle_request::<MockMessage, MockMessage>(
///     "rpc",
///     tonic::Request::new(mock_message.clone()),
/// ));
/// assert_eq!(rsp.unwrap_err().code(), tonic::Code::Unknown);
/// ```
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

/// The `Ok` variant of the `Result` returned by [`Respond::respond`]. It consists
/// of the type name of the inner message and a [`tonic::Response`] containing the inner message.
pub struct MockResponse {
    pub(crate) type_name: &'static str,
    pub(crate) inner: tonic::Response<AnyMessage>,
}

/// The return type of [`Respond::respond`].
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

/// A template for response that is used to construct a [`Mock`](`crate::Mock`). When a request is
/// made which satisfies the mock's matcher, the template's [`Respond`] implementation is called.
/// There is an optional delay that can be set on the response as well.
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

    /// Sets the delay of the response.
    ///
    /// # Examples
    /// ```rust
    /// use astria_grpc_mock::{
    ///     matcher,
    ///     response,
    ///     AnyMessage,
    ///     Mock,
    /// };
    ///
    /// // The below mock's response will be delayed by 1 second.
    /// let _mock = Mock::for_rpc_given("rpc", matcher::message_type::<AnyMessage>()).respond_with(
    ///     response::error_response(0.into()).set_delay(std::time::Duration::from_secs(1)),
    /// );
    /// ```
    #[must_use]
    pub fn set_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }
}

/// The trait which houses the logic for responding to a request.
///
/// It is already implemented for the following response types:
/// * [`constant_response`]
/// * [`dynamic_response`]
/// * [`error_response`]
///
/// This trait can also be implemented to use custom response logic.
pub trait Respond: Send + Sync {
    fn respond(&self, req: &tonic::Request<AnyMessage>) -> ResponseResult;
}
