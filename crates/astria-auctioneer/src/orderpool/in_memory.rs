//! The in-memory storage of active orders.

use std::{
    hash::RandomState,
    sync::Arc,
};

use alloy_primitives::B256;
use jiff::Timestamp;
use papaya::{
    Compute,
    LocalGuard,
    Operation,
};
use tracing::{
    info,
    instrument,
};
use uuid::Uuid;

use super::Bundle;

pub(crate) enum InsertedOrReplaced {
    #[allow(dead_code)]
    Inserted {
        uuid: Uuid,
        timestamp: Timestamp,
        bundle_hash: B256,
    },
    Replaced {
        old: Arc<Bundle>,
        new: Arc<Bundle>,
    },
    Aborted {
        requested: Arc<Bundle>,
        in_storage: Arc<Bundle>,
    },
}

pub(crate) enum RemovedOrNotFound {
    Removed(Arc<Bundle>),
    #[allow(dead_code)]
    NotFound(Uuid),
    #[allow(dead_code)]
    Aborted {
        requested_timestamp: Timestamp,
        in_storage_bundle: Arc<Bundle>,
    },
}

#[derive(Clone)]
pub(super) struct Storage {
    /// The collection of currently active bundles.
    uuid_to_bundle: Arc<papaya::HashMap<Uuid, Arc<crate::bundle::Bundle>>>,
}

impl Storage {
    pub(super) fn new() -> Self {
        Self {
            uuid_to_bundle: Arc::new(papaya::HashMap::new()),
        }
    }

    /// Inserts `bundle` into storage or replaces a previous bundle with the same `bundle.uuid`.
    pub(super) fn insert_or_replace(&self, bundle: Arc<Bundle>) -> InsertedOrReplaced {
        match self
            .uuid_to_bundle
            .pin()
            .compute(*bundle.uuid(), move |entry| match entry {
                Some((_key, stored_bundle)) => {
                    if bundle.timestamp() > stored_bundle.timestamp() {
                        Operation::Insert(bundle.clone())
                    } else {
                        Operation::Abort((bundle.clone(), stored_bundle.clone()))
                    }
                }
                None => Operation::Insert(bundle.clone()),
            }) {
            Compute::Inserted(uuid, bundle) => InsertedOrReplaced::Inserted {
                uuid: *uuid,
                timestamp: *bundle.timestamp(),
                bundle_hash: *bundle.hash(),
            },
            Compute::Updated {
                old: (_uuid_old, old),
                new: (_uuid_new, new),
            } => InsertedOrReplaced::Replaced {
                old: old.clone(),
                new: new.clone(),
            },
            Compute::Aborted((requested, in_storage)) => InsertedOrReplaced::Aborted {
                requested,
                in_storage,
            },
            Compute::Removed(..) => {
                unreachable!("inserting or replacing a bundle should never result in a removal")
            }
        }
    }

    /// Unfortunately we are leaking the details of [`papaya::HashMap`] because the iterator
    /// is always with respect to a stack allocated object.
    pub(super) fn pin(
        &self,
    ) -> papaya::HashMapRef<'_, Uuid, Arc<Bundle>, RandomState, LocalGuard<'_>> {
        self.uuid_to_bundle.pin()
    }

    pub(super) fn remove(&self, uuid: Uuid, timestamp: Timestamp) -> RemovedOrNotFound {
        match self
            .uuid_to_bundle
            .pin()
            .remove_if(&uuid, |_uuid, bundle| &timestamp > bundle.timestamp())
        {
            Ok(Some((_uuid, bundle))) => RemovedOrNotFound::Removed(bundle.clone()),
            Ok(None) => RemovedOrNotFound::NotFound(uuid),
            Err((_uuid, bundle)) => RemovedOrNotFound::Aborted {
                requested_timestamp: timestamp,
                in_storage_bundle: bundle.clone(),
            },
        }
    }

    /// Removes all bundles up to (but not including) `height`.
    ///
    /// Returns the total number of bundles removed from storage.
    #[instrument(
        skip_all,
        follows_from = [follows_from],
        fields(height),
        ret
    )]
    pub(super) fn remove_up_to_height(
        &self,
        follows_from: Option<tracing::Id>,
        height: u64,
    ) -> usize {
        let guard = self.uuid_to_bundle.guard();
        let mut evicted = 0;
        for uuid in self.uuid_to_bundle.keys(&guard) {
            if let Ok(Some((_uuid, bundle))) =
                self.uuid_to_bundle
                    .remove_if(uuid, |_, bundle| bundle.is_valid_at(height), &guard)
            {
                info!(
                    %uuid,
                    bundle_hash = %bundle.hash(),
                    timestamp = %bundle.timestamp(),
                    good_until = bundle.block(),
                    "evicted order",
                );
                evicted += 1;
            }
        }
        evicted
    }
}
