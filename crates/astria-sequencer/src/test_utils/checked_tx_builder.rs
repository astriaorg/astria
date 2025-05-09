use std::sync::Arc;

use astria_core::{
    crypto::SigningKey,
    primitive::v1::RollupId,
    protocol::transaction::v1::{
        action::RollupDataSubmission,
        Action,
        TransactionBodyBuilder,
    },
    Protobuf as _,
};
use bytes::Bytes;
use cnidarium::{
    Snapshot,
    StateDelta,
};
use prost::Message as _;

use super::{
    Fixture,
    SUDO,
};
use crate::{
    checked_transaction::CheckedTransaction,
    test_utils::nria,
};

/// A builder for a [`CheckedTransaction`].
///
/// An instance can be constructed via `Fixture::chain_initializer()`.
///
/// By default, the following values are used:
///   * `nonce`: 0
///   * `signer`: `SUDO`
///   * `actions`: a single `RollupDataSubmission` with values:
///     * `rollup_id`: `[1; 32]`
///     * `data`: `[1, 2, 3]`
///     * `fee_asset`: nria
pub(crate) struct CheckedTxBuilder<'a> {
    state: &'a StateDelta<Snapshot>,
    nonce: u32,
    signer: SigningKey,
    actions: Vec<Action>,
}

impl<'a> CheckedTxBuilder<'a> {
    pub(super) fn new(fixture: &'a Fixture) -> Self {
        Self {
            state: fixture.state(),
            nonce: 0,
            signer: SUDO.clone(),
            actions: vec![],
        }
    }

    pub(crate) fn with_nonce(mut self, nonce: u32) -> Self {
        self.nonce = nonce;
        self
    }

    pub(crate) fn with_signer(mut self, signer: SigningKey) -> Self {
        self.signer = signer;
        self
    }

    /// Appends an action to the existing collection of actions.
    pub(crate) fn with_action<T: Into<Action>>(mut self, action: T) -> Self {
        self.actions.push(action.into());
        self
    }

    /// Appends a `RollupDataSubmission` action to the existing collection of actions.
    ///
    /// This is equivalent to calling `CheckedTxBuilder::with_action` with a `RollupDataSubmission`
    /// where the rollup ID is `[1; 32]` and the fee asset is `nria()`.
    pub(crate) fn with_rollup_data_submission(mut self, data: Vec<u8>) -> Self {
        self.actions
            .push(Action::RollupDataSubmission(RollupDataSubmission {
                rollup_id: RollupId::new([1; 32]),
                data: Bytes::from(data),
                fee_asset: nria().into(),
            }));
        self
    }

    /// Constructs and returns the checked transaction.
    ///
    /// Panics if construction fails.
    pub(crate) async fn build(mut self) -> Arc<CheckedTransaction> {
        if self.actions.is_empty() {
            self = self.with_rollup_data_submission(vec![1, 2, 3]);
        }
        let Self {
            state,
            nonce,
            signer,
            actions,
        } = self;
        let tx = TransactionBodyBuilder::new()
            .nonce(nonce)
            .chain_id("test".to_string())
            .actions(actions)
            .try_build()
            .unwrap()
            .sign(&signer);
        let encoded_tx = Bytes::from(tx.into_raw().encode_to_vec());
        Arc::new(CheckedTransaction::new(encoded_tx, state).await.unwrap())
    }
}
