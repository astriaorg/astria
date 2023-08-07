use tendermint::merkle::MerkleHash;
use tendermint::Hash;
use digest::{consts::U32, Digest, FixedOutputReset};

/// A merkle proof.
pub(crate) struct Proof {
    /// root hash of the merkle tree
    root_hash: Hash,
    /// leaf data before being hashed
    leaf_data: Vec<u8>,
    /// merkle proof
    proof: Vec<Hash>,
}

impl Proof {
    pub(crate) fn verify<H: MerkleHash + Default + Digest<OutputSize = U32> + FixedOutputReset>(&self) -> bool {
        let mut hasher = H::default();
        let mut hash = hasher.leaf_hash(&self.leaf_data);

        for sibling in &self.proof {
            // TOOD
        }
        hash == self.root_hash.as_bytes()
    }
}
