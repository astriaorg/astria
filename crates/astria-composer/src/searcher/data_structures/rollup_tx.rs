/// This enum represents all the Transaction types that
/// are currently supported by Astria
#[non_exhaustive]
pub(crate) enum RollupTx {
    EthersTx(ethers::types::Transaction),
}

/// Rollup transactions have to be serialized before passed off into the wire
pub(crate) trait WireFormat {
    fn serialize(&self) -> Box<[u8]>;
}

impl WireFormat for RollupTx {
    fn serialize(&self) -> Box<[u8]> {
        match self {
            Self::EthersTx(tx) => tx.serialize(),
            _ => unreachable!(),
        }
    }
}

impl WireFormat for ethers::types::Transaction {
    fn serialize(&self) -> Box<[u8]> {
        self.rlp().to_vec().into_boxed_slice()
    }
}
