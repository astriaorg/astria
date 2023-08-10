use crate::ds::RollupTxExt;
use async_trait::async_trait;

/// The Strategy trait defines arbitrary MEV strategies executed on a list of received
/// rollup transactions
#[async_trait]
pub(crate) trait Strategy{
    type Error;
    /// Accepts a list of rollup transactions and returns an (ordered) list of rollup transactions
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