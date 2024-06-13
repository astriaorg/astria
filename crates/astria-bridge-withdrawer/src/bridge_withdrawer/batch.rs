use astria_core::protocol::transaction::v1alpha1::Action;

pub(crate) struct Batch {
    /// The withdrawal payloads
    pub(crate) actions: Vec<Action>,
    /// The corresponding rollup block height
    pub(crate) rollup_height: u64,
}
