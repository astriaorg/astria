use astria_core::protocol::transaction::v1alpha1::action_groups::BundlableGeneralAction;

#[derive(Debug)]
pub(crate) struct Batch {
    /// The withdrawal payloads
    pub(crate) actions: Vec<BundlableGeneralAction>,
    /// The corresponding rollup block height
    pub(crate) rollup_height: u64,
}
