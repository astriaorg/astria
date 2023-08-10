use ethers::types::Transaction as EthersTx;

#[non_exhaustive]
pub enum RollupTx {
    EthersTx(EthersTx),
}

pub(crate) type RollupChainId = String;
pub type RollupTxExt = (RollupTx, RollupChainId);
