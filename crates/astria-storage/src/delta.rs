use std::{any::Any, sync::Arc};

use futures::StreamExt;
use parking_lot::RwLock;
use tendermint::abci;

use crate::{
    future::{
        CacheFuture, StateDeltaNonconsensusPrefixRawStream, StateDeltaNonconsensusRangeRawStream,
        StateDeltaPrefixKeysStream, StateDeltaPrefixRawStream,
    },
    utils, Cache, EscapedByteSlice, StateRead, StateWrite,
};

/// An arbitrarily-deeply nested stack of delta updates to an underlying state.
///
/// This API allows exploring a tree of possible execution paths concurrently,
/// before finally selecting one and applying it to the underlying state.
///
/// Using this API requires understanding its invariants.
///
/// On creation, `StateDelta::new` takes ownership of a `StateRead + StateWrite`
/// instance, acquiring a "write lock" over the underlying state (since `&mut S`
/// is `StateWrite` if `S: StateWrite`, it's possible to pass a unique
/// reference).
///
/// The resulting `StateDelta` instance is a "leaf" state, and can be used for
/// reads and writes, following the some execution path.
///
/// When two potential execution paths diverge, `delta.fork()` can be used to
/// fork the state update.  The new forked `StateDelta` will include all
/// previous state writes made to the original (and its ancestors).  Any writes
/// made to the original `StateDelta` after `fork()` is called will not be seen
/// by the forked state.
///
/// Finally, after some execution path has been selected, calling
/// `delta.apply()` on one of the possible state updates will commit the changes
/// to the underlying state instance, and invalidate all other delta updates in
/// the same family.  It is a programming error to use the other delta updates
/// after `apply()` has been called, but ideally this should not be a problem in
/// practice: the API is intended to explore a tree of possible execution paths;
/// once one has been selected, the others should be discarded.
#[derive(Debug)]
pub struct StateDelta<S: StateRead> {
    /// The underlying state instance.
    ///
    /// The Arc<_> allows it to be shared between different stacks of delta updates,
    /// and the RwLock<Option<_>> allows it to be taken out when it's time to commit
    /// the changes from one of the stacks.
    state: Arc<RwLock<Option<S>>>,
    /// A stack of intermediate delta updates, with the "top" layers first.
    ///
    /// We store all the layers directly, rather than using a recursive structure,
    /// so that the type doesn't depend on how many layers are involved. We're only
    /// duplicating the Arc<_>, so this should be cheap.
    layers: Vec<Arc<RwLock<Option<Cache>>>>,
    /// The final delta update in the stack, the one we're currently working on.
    /// Storing this separately allows us to avoid lock contention during writes.
    /// In fact, this data shouldn't usually be shared at all; the only reason it's
    /// wrapped this way is so that prefix streams can have 'static lifetimes.
    /// We option-wrap it so it can be chained with the layers; it will never be None.
    leaf_cache: Arc<RwLock<Option<Cache>>>,
}

impl<S: StateRead> StateDelta<S> {
    /// Create a new tree of possible updates to an underlying `state`.
    pub fn new(state: S) -> Self {
        Self {
            state: Arc::new(RwLock::new(Some(state))),
            layers: Vec::default(),
            leaf_cache: Arc::new(RwLock::new(Some(Cache::default()))),
        }
    }

    /// Fork execution, returning a new child state that includes all previous changes.
    pub fn fork(&mut self) -> Self {
        // If we have writes in the leaf cache, we'll move them to a new layer,
        // ensuring that the new child only sees writes made to this state
        // *before* fork was called, and not after.
        //
        // Doing this only when the leaf cache is dirty means that we don't
        // add empty layers in repeated fork() calls without intervening writes.
        if self.leaf_cache.read().as_ref().unwrap().is_dirty() {
            let new_layer = std::mem::replace(
                &mut self.leaf_cache,
                Arc::new(RwLock::new(Some(Cache::default()))),
            );
            self.layers.push(new_layer);
        }

        Self {
            state: self.state.clone(),
            layers: self.layers.clone(),
            leaf_cache: Arc::new(RwLock::new(Some(Cache::default()))),
        }
    }

    /// Flatten all changes in this branch of the tree into a single [`Cache`],
    /// invalidating all other branches of the tree and releasing the underlying
    /// state back to the caller.
    ///
    /// The [`apply`](Self::apply) method is a convenience wrapper around this
    /// that applies the changes to the underlying state.
    pub fn flatten(self) -> (S, Cache) {
        tracing::trace!("flattening branch");
        // Take ownership of the underlying state, immediately invalidating all
        // other delta stacks in the same family.
        let state = self
            .state
            .write()
            .take()
            .expect("apply must be called only once");

        // Flatten the intermediate layers into a single cache, applying them from oldest
        // (bottom) to newest (top), so that newer writes clobber old ones.
        let mut changes = Cache::default();
        for layer in self.layers {
            let cache = layer
                .write()
                .take()
                .expect("cache must not have already been applied");
            changes.merge(cache);
        }
        // Last, apply the changes in the leaf cache.
        changes.merge(self.leaf_cache.write().take().unwrap());

        (state, changes)
    }
}

impl<S: StateRead + StateWrite> StateDelta<S> {
    /// Apply all changes in this branch of the tree to the underlying state,
    /// releasing it back to the caller and invalidating all other branches of
    /// the tree.
    pub fn apply(self) -> (S, Vec<abci::Event>) {
        let (mut state, mut changes) = self.flatten();
        let events = changes.take_events();

        // Apply the flattened changes to the underlying state.
        changes.apply_to(&mut state);

        // Finally, return ownership of the state back to the caller.
        (state, events)
    }
}

impl<S: StateRead + StateWrite> StateDelta<Arc<S>> {
    pub fn try_apply(self) -> anyhow::Result<(S, Vec<abci::Event>)> {
        let (arc_state, mut changes) = self.flatten();
        let events = std::mem::take(&mut changes.events);

        if let Ok(mut state) = Arc::try_unwrap(arc_state) {
            // Apply the flattened changes to the underlying state.
            changes.apply_to(&mut state);

            // Finally, return ownership of the state back to the caller.
            Ok((state, events))
        } else {
            Err(anyhow::anyhow!("did not have unique ownership of Arc<S>"))
        }
    }
}

impl<S: StateRead> StateRead for StateDelta<S> {
    type GetRawFut = CacheFuture<S::GetRawFut>;
    type PrefixRawStream = StateDeltaPrefixRawStream<S::PrefixRawStream>;
    type PrefixKeysStream = StateDeltaPrefixKeysStream<S::PrefixKeysStream>;
    type NonconsensusPrefixRawStream =
        StateDeltaNonconsensusPrefixRawStream<S::NonconsensusPrefixRawStream>;
    type NonconsensusRangeRawStream =
        StateDeltaNonconsensusRangeRawStream<S::NonconsensusRangeRawStream>;

    fn get_raw(&self, key: &str) -> Self::GetRawFut {
        // Check if we have a cache hit in the leaf cache.
        if let Some(entry) = self
            .leaf_cache
            .read()
            .as_ref()
            .unwrap()
            .unwritten_changes
            .get(key)
        {
            return CacheFuture::hit(entry.clone());
        }

        // Iterate through the stack, top to bottom, to see if we have a cache hit.
        for layer in self.layers.iter().rev() {
            if let Some(entry) = layer
                .read()
                .as_ref()
                .expect("delta must not have been applied")
                .unwritten_changes
                .get(key)
            {
                return CacheFuture::hit(entry.clone());
            }
        }

        // If we got here, the key must be in the underlying state or not present at all.
        CacheFuture::miss(
            self.state
                .read()
                .as_ref()
                .expect("delta must not have been applied")
                .get_raw(key),
        )
    }

    fn nonverifiable_get_raw(&self, key: &[u8]) -> Self::GetRawFut {
        // Check if we have a cache hit in the leaf cache.
        if let Some(entry) = self
            .leaf_cache
            .read()
            .as_ref()
            .unwrap()
            .nonverifiable_changes
            .get(key)
        {
            return CacheFuture::hit(entry.clone());
        }

        // Iterate through the stack, top to bottom, to see if we have a cache hit.
        for layer in self.layers.iter().rev() {
            if let Some(entry) = layer
                .read()
                .as_ref()
                .expect("delta must not have been applied")
                .nonverifiable_changes
                .get(key)
            {
                return CacheFuture::hit(entry.clone());
            }
        }

        // If we got here, the key must be in the underlying state or not present at all.
        CacheFuture::miss(
            self.state
                .read()
                .as_ref()
                .expect("delta must not have been applied")
                .nonverifiable_get_raw(key),
        )
    }

    fn object_type(&self, key: &'static str) -> Option<std::any::TypeId> {
        // Check if we have a cache hit in the leaf cache.
        if let Some(entry) = self
            .leaf_cache
            .read()
            .as_ref()
            .expect("delta must not have been applied")
            .ephemeral_objects
            .get(key)
        {
            // We have to explicitly call `Any::type_id(&**v)` here because this ensures that we are
            // asking for the type of the `Any` *inside* the `Box<dyn Any>`, rather than the type of
            // `Box<dyn Any>` itself.
            return entry.as_ref().map(|v| std::any::Any::type_id(&**v));
        }

        // Iterate through the stack, top to bottom, to see if we have a cache hit.
        for layer in self.layers.iter().rev() {
            if let Some(entry) = layer
                .read()
                .as_ref()
                .expect("delta must not have been applied")
                .ephemeral_objects
                .get(key)
            {
                // We have to explicitly call `Any::type_id(&**v)` here because this ensures that we are
                // asking for the type of the `Any` *inside* the `Box<dyn Any>`, rather than the type of
                // `Box<dyn Any>` itself.
                return entry.as_ref().map(|v| std::any::Any::type_id(&**v));
            }
        }

        // Fall through to the underlying store.
        self.state
            .read()
            .as_ref()
            .expect("delta must not have been applied")
            .object_type(key)
    }

    fn object_get<T: std::any::Any + Send + Sync + Clone>(&self, key: &'static str) -> Option<T> {
        // Check if we have a cache hit in the leaf cache.
        if let Some(entry) = self
            .leaf_cache
            .read()
            .as_ref()
            .expect("delta must not have been applied")
            .ephemeral_objects
            .get(key)
        {
            return entry
                .as_ref()
                .map(|v| {
                    v.downcast_ref().unwrap_or_else(|| panic!("unexpected type for key \"{key}\" in `StateDelta::object_get`: expected type {}", std::any::type_name::<T>()))
                })
                .cloned();
        }

        // Iterate through the stack, top to bottom, to see if we have a cache hit.
        for layer in self.layers.iter().rev() {
            if let Some(entry) = layer
                .read()
                .as_ref()
                .expect("delta must not have been applied")
                .ephemeral_objects
                .get(key)
            {
                return entry
                    .as_ref()
                    .map(|v| {
                    v.downcast_ref().unwrap_or_else(|| panic!("unexpected type for key \"{key}\" in `StateDelta::object_get`: expected type {}", std::any::type_name::<T>()))
                }).cloned();
            }
        }

        // Fall through to the underlying store.
        self.state
            .read()
            .as_ref()
            .expect("delta must not have been applied")
            .object_get(key)
    }

    fn prefix_raw(&self, prefix: &str) -> Self::PrefixRawStream {
        let underlying = self
            .state
            .read()
            .as_ref()
            .expect("delta must not have been applied")
            .prefix_raw(prefix)
            .peekable();
        StateDeltaPrefixRawStream {
            underlying,
            layers: self.layers.clone(),
            leaf_cache: self.leaf_cache.clone(),
            last_key: None,
            prefix: prefix.to_owned(),
        }
    }

    fn prefix_keys(&self, prefix: &str) -> Self::PrefixKeysStream {
        let underlying = self
            .state
            .read()
            .as_ref()
            .expect("delta must not have been applied")
            .prefix_keys(prefix)
            .peekable();
        StateDeltaPrefixKeysStream {
            underlying,
            layers: self.layers.clone(),
            leaf_cache: self.leaf_cache.clone(),
            last_key: None,
            prefix: prefix.to_owned(),
        }
    }

    fn nonverifiable_prefix_raw(&self, prefix: &[u8]) -> Self::NonconsensusPrefixRawStream {
        let underlying = self
            .state
            .read()
            .as_ref()
            .expect("delta must not have been applied")
            .nonverifiable_prefix_raw(prefix)
            .peekable();
        StateDeltaNonconsensusPrefixRawStream {
            underlying,
            layers: self.layers.clone(),
            leaf_cache: self.leaf_cache.clone(),
            last_key: None,
            prefix: prefix.to_vec(),
        }
    }

    fn nonverifiable_range_raw(
        &self,
        prefix: Option<&[u8]>,
        range: impl std::ops::RangeBounds<Vec<u8>>,
    ) -> anyhow::Result<Self::NonconsensusRangeRawStream> {
        let (range, (start, end)) = utils::convert_bounds(range)?;
        let underlying = self
            .state
            .read()
            .as_ref()
            .expect("delta must not have been applied")
            .nonverifiable_range_raw(prefix, range)?
            .peekable();
        Ok(StateDeltaNonconsensusRangeRawStream {
            underlying,
            layers: self.layers.clone(),
            leaf_cache: self.leaf_cache.clone(),
            last_key: None,
            prefix: prefix.map(|p| p.to_vec()),
            range: (start, end),
        })
    }
}

impl<S: StateRead> StateWrite for StateDelta<S> {
    fn put_raw(&mut self, key: String, value: jmt::OwnedValue) {
        self.leaf_cache
            .write()
            .as_mut()
            .unwrap()
            .unwritten_changes
            .insert(key, Some(value));
    }

    fn delete(&mut self, key: String) {
        self.leaf_cache
            .write()
            .as_mut()
            .unwrap()
            .unwritten_changes
            .insert(key, None);
    }

    fn nonverifiable_delete(&mut self, key: Vec<u8>) {
        tracing::trace!(key = ?EscapedByteSlice(&key), "deleting key");
        self.leaf_cache
            .write()
            .as_mut()
            .unwrap()
            .nonverifiable_changes
            .insert(key, None);
    }

    fn nonverifiable_put_raw(&mut self, key: Vec<u8>, value: Vec<u8>) {
        tracing::trace!(key = ?EscapedByteSlice(&key), value = ?EscapedByteSlice(&value), "insert nonverifiable change");
        self.leaf_cache
            .write()
            .as_mut()
            .unwrap()
            .nonverifiable_changes
            .insert(key, Some(value));
    }

    fn object_put<T: Clone + Any + Send + Sync>(&mut self, key: &'static str, value: T) {
        if let Some(previous_type) = self.object_type(key) {
            if std::any::TypeId::of::<T>() != previous_type {
                panic!(
                    "unexpected type for key \"{key}\" in `StateDelta::object_put`: expected type {expected}",
                    expected = std::any::type_name::<T>(),
                );
            }
        }
        self.leaf_cache
            .write()
            .as_mut()
            .unwrap()
            .ephemeral_objects
            .insert(key, Some(Box::new(value)));
    }

    fn object_delete(&mut self, key: &'static str) {
        self.leaf_cache
            .write()
            .as_mut()
            .unwrap()
            .ephemeral_objects
            .insert(key, None);
    }

    fn object_merge(
        &mut self,
        objects: std::collections::BTreeMap<&'static str, Option<Box<dyn Any + Send + Sync>>>,
    ) {
        self.leaf_cache
            .write()
            .as_mut()
            .unwrap()
            .ephemeral_objects
            .extend(objects);
    }

    fn record(&mut self, event: abci::Event) {
        self.leaf_cache.write().as_mut().unwrap().events.push(event)
    }
}

/// Extension trait providing `try_begin_transaction()` on `Arc<StateDelta<S>>`.
pub trait ArcStateDeltaExt: Sized {
    type S: StateRead;
    /// Attempts to begin a transaction on this `Arc<State>`, returning `None` if the `Arc` is shared.
    fn try_begin_transaction(&'_ mut self) -> Option<StateDelta<&'_ mut StateDelta<Self::S>>>;
}

impl<S: StateRead> ArcStateDeltaExt for Arc<StateDelta<S>> {
    type S = S;
    fn try_begin_transaction(&'_ mut self) -> Option<StateDelta<&'_ mut StateDelta<S>>> {
        Arc::get_mut(self).map(StateDelta::new)
    }
}
