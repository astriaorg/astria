use crate::ds::RollupTxExt;
use async_trait::async_trait;

/// The Strategy crate defines arbitrary MEV strategies executed on a set of received
/// rollup transactions
#[async_trait]
pub(crate) trait Strategy{
    type Error;
    /// Accepts an ordered list of Rollup transactions and returns an (ordered) list of Rollup transactions
    /// The returned list is ordered to execute the arbitrary strategy (may have new transactions, a different order, etc.)
    async fn execute(input: Vec<RollupTxExt>) -> Result<Vec<RollupTxExt>, Self::Error>;
}

pub(crate) struct NoStrategy {}

#[async_trait]
impl Strategy for NoStrategy {
    type Error = color_eyre::eyre::Error;
    async fn execute(input: Vec<RollupTxExt>) -> Result<Vec<RollupTxExt>, Self::Error> {
        Ok(input)
    }
}