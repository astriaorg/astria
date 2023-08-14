use ct_merkle::{
    inclusion::InclusionProof as CtInclusionProof,
    CtMerkleTree,
    RootHash,
};
use serde::{
    ser::SerializeStruct,
    Deserialize,
    Serialize,
};
use sha2::Sha256;
use tendermint::Hash;

#[derive(Debug, Default)]
pub struct MerkleTree(CtMerkleTree<Sha256, Vec<u8>>);

impl MerkleTree {
    #[must_use]
    pub fn new() -> Self {
        MerkleTree(CtMerkleTree::new())
    }

    #[must_use]
    pub fn from_leaves(leaves: Vec<Vec<u8>>) -> Self {
        let tree = leaves
            .into_iter()
            .fold(CtMerkleTree::new(), |mut tree, leaf| {
                tree.push(leaf);
                tree
            });
        MerkleTree(tree)
    }

    #[must_use]
    pub fn root(&self) -> Hash {
        let digest = sha2::digest::Output::<Sha256>::from_slice(self.0.root().as_bytes())
            .as_slice()
            .to_vec();
        Hash::from_bytes(tendermint::hash::Algorithm::Sha256, &digest)
            .expect("cannot fail since both hashes are 32 bytes")
    }

    /// Returns the inclusion proof for the leaf at the given index.
    ///
    /// # Errors
    ///
    /// - if the index is out of bounds
    pub fn prove_inclusion(&self, idx: usize) -> anyhow::Result<InclusionProof> {
        Ok(InclusionProof {
            value: self
                .0
                .get(idx)
                .ok_or(anyhow::anyhow!("index out of bounds of merkle tree"))?
                .clone(),
            idx,
            num_leaves: self.0.len(),
            inclusion_proof: self.0.prove_inclusion(idx),
        })
    }
}

/// A merkle proof of inclusion.
#[derive(Debug)]
#[allow(clippy::module_name_repetitions)] // TODO fix
pub struct InclusionProof {
    // value to be proven as included
    value: Vec<u8>,
    // leaf index of value to be proven
    idx: usize,
    // total number of leaves in the tree
    num_leaves: usize,
    // the merkle proof itself
    inclusion_proof: CtInclusionProof<Sha256>,
}

impl Serialize for InclusionProof {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("InclusionProof", 4)?;
        s.serialize_field("value", &hex::encode(&self.value))?;
        s.serialize_field("idx", &self.idx)?;
        s.serialize_field("num_leaves", &self.num_leaves)?;
        s.serialize_field(
            "inclusion_proof",
            &hex::encode(self.inclusion_proof.as_bytes()),
        )?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for InclusionProof {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct InclusionProofHelper {
            value: String,
            idx: usize,
            num_leaves: usize,
            inclusion_proof: String,
        }
        let helper = InclusionProofHelper::deserialize(deserializer)?;
        let value = hex::decode(helper.value).map_err(serde::de::Error::custom)?;
        // TODO: fix CtInclusionProof::from_bytes to not have panics
        let inclusion_proof = CtInclusionProof::from_bytes(
            hex::decode(helper.inclusion_proof).map_err(serde::de::Error::custom)?,
        );
        Ok(InclusionProof {
            value,
            idx: helper.idx,
            num_leaves: helper.num_leaves,
            inclusion_proof,
        })
    }
}

impl InclusionProof {
    /// Verify that the proof is valid for the given root hash.
    ///
    /// # Errors
    ///
    /// - if the proof is invalid
    pub fn verify(&self, root_hash: Hash) -> anyhow::Result<()> {
        let digest = *sha2::digest::Output::<Sha256>::from_slice(root_hash.as_bytes());
        let ct_root = RootHash::<Sha256>::new(digest, self.num_leaves);
        ct_root
            .verify_inclusion(&self.value, self.idx, &self.inclusion_proof)
            .map_err(|e| anyhow::anyhow!("failed to verify inclusion: {}", e))
    }
}

#[cfg(test)]
mod test {
    use ct_merkle::{
        self,
        CtMerkleTree,
    };
    use tendermint::merkle::simple_hash_from_byte_vectors;

    use super::*;

    #[test]
    fn ct_merkle_vs_tendermint() {
        let data: Vec<Vec<u8>> = vec![
            vec![1, 2, 3],
            vec![4, 5, 6],
            vec![7, 8, 9],
            vec![10, 11, 12],
            vec![13, 14, 15],
            vec![16, 17, 18],
            vec![19, 20, 21],
            vec![22, 23, 24],
        ];

        let tm_root = simple_hash_from_byte_vectors::<tendermint::crypto::default::Sha256>(&data);

        let mut ct_tree: CtMerkleTree<sha2::Sha256, Vec<u8>> = ct_merkle::CtMerkleTree::new();
        for d in data {
            ct_tree.push(d);
        }
        let ct_root = ct_tree.root();
        assert_eq!(ct_root.as_bytes().as_slice(), tm_root.as_slice());
    }

    #[test]
    fn inclusion_proof_serde_roundtrip() {
        let data: Vec<Vec<u8>> = vec![
            vec![1, 2, 3],
            vec![4, 5, 6],
            vec![7, 8, 9],
            vec![10, 11, 12],
            vec![13, 14, 15],
            vec![16, 17, 18],
            vec![19, 20, 21],
            vec![22, 23, 24],
        ];

        let mut ct_tree: CtMerkleTree<sha2::Sha256, Vec<u8>> = ct_merkle::CtMerkleTree::new();
        for d in &data {
            ct_tree.push(d.clone());
        }
        let ct_root = ct_tree.root();

        let idx = 0;
        let proof = InclusionProof {
            value: data[idx].clone(),
            idx,
            num_leaves: data.len(),
            inclusion_proof: ct_tree.prove_inclusion(idx),
        };

        let json = serde_json::to_string(&proof).unwrap();
        let proof: InclusionProof = serde_json::from_str(&json).unwrap();
        let tm_hash =
            tendermint::Hash::from_bytes(tendermint::hash::Algorithm::Sha256, ct_root.as_bytes())
                .unwrap();
        proof.verify(tm_hash).unwrap();
    }
}
