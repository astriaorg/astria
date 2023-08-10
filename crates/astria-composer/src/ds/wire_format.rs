use ethers::types::Transaction as EthersTx;

pub(crate) trait WireFormat {
    fn serialize(&self) -> Box<[u8]>;
}

impl WireFormat for EthersTx {
    fn serialize(&self) -> Box<[u8]> {
        self.rlp().to_vec().into_boxed_slice()
    }
}