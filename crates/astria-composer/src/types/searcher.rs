//! TODO: explain searcher engine & types
use std::pin::Pin;

use async_trait::async_trait;
use color_eyre::eyre;
use tokio_stream::Stream;

/// A stream of events emitted by a [Collector](Collector).
// TODO
pub type CollectorStream<'a, E> = Pin<Box<dyn Stream<Item = E> + Send + 'a>>;

/// Collector trait, which defines a source of events.
#[async_trait]
pub trait Collector<E>: Send + Sync {
    /// Returns the core event stream for the collector.
    async fn get_event_stream(&self) -> eyre::Result<CollectorStream<'_, E>>;
}

/// Strategy trait, which defines the core logic for each opportunity.
#[async_trait]
pub trait Strategy<E, A>: Send + Sync {
    /// Sync the initial state of the strategy if needed, usually by fetching
    /// onchain data.
    async fn sync_state(&mut self) -> eyre::Result<()>;

    /// Process an event, and return an action if needed.
    async fn process_event(&mut self, event: E) -> Option<A>;
}

/// Executor trait, responsible for executing actions returned by strategies.
#[async_trait]
pub trait Executor<A>: Send + Sync {
    // Execute an action.
    async fn execute(&self, action: A) -> eyre::Result<()>;
}

pub enum Events {}

pub enum Actions {}
