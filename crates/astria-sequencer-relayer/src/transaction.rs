use tendermint::{
    hash::Hash as TmHash,
    merkle,
};

pub fn txs_to_data_hash(hashed_txs: &[Vec<u8>]) -> TmHash {
    TmHash::Sha256(merkle::simple_hash_from_byte_vectors::<
        tendermint::crypto::default::Sha256,
    >(hashed_txs))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::base64_string::Base64String;

    fn sha256_hash(data: &[u8]) -> Vec<u8> {
        use sha2::Digest as _;
        let mut hasher = sha2::Sha256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }

    #[test]
    fn txs_to_data_hash_test() {
        // data_hash is calculated from the txs in a block, where the leaves of the merkle tree are
        // the sha256 hashes of the txs
        let tx = Base64String::from_string("CscBCsQBCg0vU2VxdWVuY2VyTXNnErIBCghldGhlcmV1bRJ4Avh1ggU5gIRZaC8AhQUD1cTyglIIlBtwp0/22gQLMRmQwVX9/9u8AvfuiA3gtrOnZAAAgMABoLnRqksJblEaolE6wbsAHYTAiSlA14+B5nvWuFrIfevnoBg+UGcWLC4eg1lZylqLnrL8okBc3vTS4qOO/J5sRtVDGixtZXRybzFsbDJobHAzM3J4eTdwN2s2YXhoeDRjdnFtdGcwY3hkZjZnemY5ahJ0Ck4KRgofL2Nvc21vcy5jcnlwdG8uc2VjcDI1NmsxLlB1YktleRIjCiEDJ/LvaMZTBcGX66geJOEmTm/fyyPTZKMUJoDtMDUmSPkSBAoCCAESGAoQCgV1dGljaxIHMTAwMDAwMBCAlOvcAyIIZXRoZXJldW0aQMhoTCUr84xgTkYxsFWDfHH2k+oHCPsKvbTpz8m5YrHfYMv6gdou6V8oj1v0B9ySD5VjMXQi1kJ9DZN6wD2buo8=".to_string()).unwrap();
        let hash = sha256_hash(&tx.0);

        let expected_hash =
            Base64String::from_string("rRDu3aQf1V37yGSTdf2fv9GSPeZ6/p0wJ9pjBl8IqFc=".to_string())
                .unwrap();
        let res = txs_to_data_hash(&[hash]);
        assert_eq!(res.as_bytes(), expected_hash.0);
    }
}
