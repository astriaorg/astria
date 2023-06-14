use prost;
use tendermint::{
    hash::Hash as TmHash,
    merkle,
};

pub fn txs_to_data_hash(txs: &Vec<Vec<u8>>) -> TmHash {
    TmHash::Sha256(merkle::simple_hash_from_byte_vectors::<
        tendermint::crypto::default::Sha256,
    >(txs))
}
