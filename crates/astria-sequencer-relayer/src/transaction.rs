use prost;
use tendermint::{
    hash::Hash as TmHash,
    merkle,
};

fn tx_to_prost_bytes(tx: Vec<u8>) -> prost::alloc::vec::Vec<u8> {
    let mut buf = prost::alloc::vec::Vec::new();
    prost::encoding::bytes::encode(1, &tx, &mut buf);
    buf
}

pub fn txs_to_data_hash(txs: &Vec<Vec<u8>>) -> TmHash {
    TmHash::Sha256(merkle::simple_hash_from_byte_vectors::<
        tendermint::crypto::default::Sha256,
    >(txs))
}
