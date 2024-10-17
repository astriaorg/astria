//! Routing of abci query requests to handlers.
//!
//! This module contains types and traits to allow handling
//! abci info queries with type erased callback functions.
//! The implementation here is heavily inspired by axum's [`Router`]
//! implementation.
//!
//! [`Router`]: https://docs.rs/axum/0.6.20/axum/struct.Router.html
//!
//! # High level implementation overview
//!
//! The implementation in terms of type-erased traits follow directly
//! from the requirement to make registering function handlers ergonomic:
//!
//! 1. Ideally, a new path can be registered with `router.insert("/some/path", some_handler)`.
//! 2. Because the handlers are async functions, their types are anonymous as `async fn -> T`
//!    desugars to something like `fn -> impl Future<Output T>`.
//! 3. This means that either the function signature of all handlers have to be changed from `async
//!    fn -> T` to `fn -> Box<dyn Future<Output T>>`, or the functions themselves have to be boxed.
//! 4. This implementation defines the `AbciQueryHandler` trait to box the handler functions to box
//!    the handler functions, which is more ergonomic.
//!
//! The next requirements come from the `Info` service needing to be `Clone`, and
//! its futures `Send` and `Sync`:
//!
//! 1. the trait has bounds `AbciQueryHandler: Clone + Sized + Send + 'static`, which means it
//!    cannot be directly made into a trait object because `Clone` and `Sized` types are not object
//!    safe.
//! 2. this requires the definition of an `ErasedAbciQueryHandler: Send` that is object safe (not
//!    `Clone`, not `Sized`), but that defines a method `fn clone_box` returning a boxed trait
//!    object.
//! 3. `BoxedAbciQueryHandler` is a wrapper around a boxed `ErasedAbciQueryHandler` that implements
//!    `Clone` to fulfill the `Clone` requirement of the `Info` service.
//! 4. finally `MakeErasedAbciQueryHandler<H>` is the glue that allows to go from a non-object safe
//!    `AbciQueryHandler` to an object-safe `ErasedAbciQueryHandler`.
use std::{
    future::Future,
    pin::Pin,
};

use matchit::{
    InsertError,
    Match,
    MatchError,
};
use tendermint::abci::{
    request,
    response,
};

use crate::storage::Storage;

/// `Router` is a wrapper around [`matchit::Router`] to route abci queries
/// to handlers.
#[derive(Clone)]
pub(super) struct Router {
    query_router: matchit::Router<BoxedAbciQueryHandler>,
}

impl Router {
    pub(super) fn new() -> Self {
        Self {
            query_router: matchit::Router::new(),
        }
    }

    pub(super) fn at<'m, 'p>(
        &'m self,
        path: &'p str,
    ) -> Result<Match<'m, 'p, &'m BoxedAbciQueryHandler>, MatchError> {
        self.query_router.at(path)
    }

    pub(super) fn insert(
        &mut self,
        route: impl Into<String>,
        handler: impl AbciQueryHandler,
    ) -> Result<(), InsertError> {
        self.query_router
            .insert(route, BoxedAbciQueryHandler::from_handler(handler))
    }
}

pub(super) struct BoxedAbciQueryHandler(Box<dyn ErasedAbciQueryHandler>);

impl BoxedAbciQueryHandler {
    fn from_handler<H>(handler: H) -> Self
    where
        H: AbciQueryHandler,
    {
        Self(Box::new(MakeErasedAbciQueryHandler {
            handler,
        }))
    }

    pub(super) async fn call(
        self,
        storage: Storage,
        request: request::Query,
        params: Vec<(String, String)>,
    ) -> response::Query {
        self.0.call(storage, request, params).await
    }
}

impl Clone for BoxedAbciQueryHandler {
    fn clone(&self) -> Self {
        Self(self.0.clone_box())
    }
}

pub(super) trait ErasedAbciQueryHandler: Send {
    fn clone_box(&self) -> Box<dyn ErasedAbciQueryHandler>;

    fn call(
        self: Box<Self>,
        storage: Storage,
        request: request::Query,
        params: Vec<(String, String)>,
    ) -> Pin<Box<dyn Future<Output = response::Query> + Send>>;
}

struct MakeErasedAbciQueryHandler<H> {
    handler: H,
}

impl<H> Clone for MakeErasedAbciQueryHandler<H>
where
    H: Clone,
{
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
        }
    }
}

impl<H> ErasedAbciQueryHandler for MakeErasedAbciQueryHandler<H>
where
    H: AbciQueryHandler + Clone + Send + 'static,
{
    fn clone_box(&self) -> Box<dyn ErasedAbciQueryHandler> {
        Box::new(self.clone())
    }

    fn call(
        self: Box<Self>,
        storage: Storage,
        request: request::Query,
        params: Vec<(String, String)>,
    ) -> Pin<Box<dyn Future<Output = response::Query> + Send>> {
        self.handler.call(storage, request, params)
    }
}

pub(super) trait AbciQueryHandler: Clone + Send + Sized + 'static {
    fn call(
        self,
        storage: Storage,
        request: request::Query,
        params: Vec<(String, String)>,
    ) -> Pin<Box<dyn Future<Output = response::Query> + Send>>;
}

impl<F, Fut> AbciQueryHandler for F
where
    F: FnOnce(Storage, request::Query, Vec<(String, String)>) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = response::Query> + Send,
{
    fn call(
        self,
        storage: Storage,
        request: request::Query,
        params: Vec<(String, String)>,
    ) -> Pin<Box<dyn Future<Output = response::Query> + Send>> {
        Box::pin(async move { self(storage, request, params).await })
    }
}
