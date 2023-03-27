use prost;
use tendermint::hash::Hash as TmHash;
use tendermint::merkle;

use crate::base64_string::Base64String;

fn tx_to_prost_bytes(tx: Vec<u8>) -> prost::alloc::vec::Vec<u8> {
    let mut buf = prost::alloc::vec::Vec::new();
    prost::encoding::bytes::encode(1, &tx, &mut buf);
    buf
}

pub(crate) fn txs_to_data_hash(txs: &[Base64String]) -> TmHash {
    let txs = txs
        .iter()
        .map(|tx| tx_to_prost_bytes(tx.0.clone()))
        .collect::<Vec<Vec<u8>>>();

    TmHash::Sha256(merkle::simple_hash_from_byte_vectors::<
        tendermint::crypto::default::Sha256,
    >(&txs))
}

#[cfg(test)]
mod test {
    use super::txs_to_data_hash;
    use crate::sequencer::SequencerClient;

    #[tokio::test]
    async fn test_txs_to_data_hash() {
        let cosmos_endpoint = "http://localhost:1317".to_string();
        let client = SequencerClient::new(cosmos_endpoint).unwrap();
        let resp = client.get_latest_block().await.unwrap();
        let data_hash = txs_to_data_hash(&resp.block.data.txs);
        assert_eq!(
            data_hash.as_bytes(),
            &resp.block.header.data_hash.unwrap().0
        );
    }
}
