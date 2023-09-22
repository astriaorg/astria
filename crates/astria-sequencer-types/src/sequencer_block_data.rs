use std::{
    array::TryFromSliceError,
    collections::BTreeMap,
};

use base64::{
    engine::general_purpose,
    Engine as _,
};
use proto::DecodeError;
use sequencer_validation::{
    InclusionProof,
    MerkleTree,
};
use serde::{
    Deserialize,
    Serialize,
};
use tendermint::{
    block::{
        Commit,
        Header,
    },
    Block,
    Hash,
};
use thiserror::Error;
use tracing::debug;

use crate::tendermint::calculate_last_commit_hash;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed converting bytes to action tree root: expected 32 bytes")]
    ActionTreeRootConversion(#[source] TryFromSliceError),
    #[error("failed converting bytes to chain IDs commitment: expected 32 bytes")]
    ChainIdsCommitmentConversion(#[source] TryFromSliceError),
    #[error(
        "data hash stored tendermint header does not match action tree root reconstructed from \
         data"
    )]
    CometBftDataHashReconstructedHashMismatch,
    #[error(
        "block hash calculated from tendermint header does not match block hash stored in \
         sequencer block"
    )]
    HashOfHeaderBlockHashMismatach,
    #[error("failed to generate inclusion proof for action tree root")]
    InclusionProof(sequencer_validation::IndexOutOfBounds),
    #[error("chain ID must be 32 bytes or less")]
    InvalidChainIdLength,
    #[error(
        "last commit hash does not match the one calculated from the block's commit signatures"
    )]
    LastCommitHashMismatch,
    #[error("the sequencer block contained neither action tree root nor transaction data")]
    NoData,
    #[error("block has no data hash")]
    MissingDataHash,
    #[error("block has no last commit hash")]
    MissingLastCommitHash,
    #[error("failed decoding bytes to protobuf signed transaction")]
    SignedTransactionProtobufDecode(#[source] DecodeError),
    #[error("failed constructing native signed transaction from raw protobuf signed transaction")]
    RawSignedTransactionConversion(
        #[source] proto::native::sequencer::v1alpha1::SignedTransactionError,
    ),
    #[error("failed deserializing sequencer block data from json bytes")]
    ReadingJson(#[source] serde_json::Error),
    #[error("chain IDs commitment does not match the one calculated from the rollup data")]
    ReconstructedChainIdsCommitmentMismatch,
    #[error(
        "failed to verify data hash in cometbft header against inclusion proof and action tree \
         root in sequencer block body"
    )]
    Verification(#[source] sequencer_validation::VerificationFailure),
    #[error("failed writing sequencer block data as json")]
    WritingJson(#[source] serde_json::Error),
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChainId(#[serde(with = "hex::serde")] Vec<u8>);

impl ChainId {
    /// Creates a new `ChainId` from the given bytes.
    ///
    /// # Errors
    ///
    /// - if the given bytes are longer than 32 bytes
    pub fn new(inner: Vec<u8>) -> Result<Self, Error> {
        if inner.len() > 32 {
            return Err(Error::InvalidChainIdLength);
        }

        Ok(Self(inner))
    }
}

impl AsRef<[u8]> for ChainId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// `SequencerBlockData` represents a sequencer block's data
/// to be submitted to the DA layer.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(try_from = "RawSequencerBlockData")]
#[serde(into = "RawSequencerBlockData")]
pub struct SequencerBlockData {
    block_hash: Hash,
    header: Header,
    /// This field should be set for every block with height > 1.
    last_commit: Option<Commit>,
    /// chain ID -> rollup transactions
    rollup_data: BTreeMap<ChainId, Vec<Vec<u8>>>,
    /// The root of the action tree for this block.
    action_tree_root: [u8; 32],
    /// The inclusion proof that the action tree root is included
    /// in `Header::data_hash`.
    action_tree_root_inclusion_proof: InclusionProof,
    /// The commitment to the chain IDs of the rollup data.
    /// The merkle root of the tree where the leaves are the chain IDs.
    chain_ids_commitment: [u8; 32],
}

impl SequencerBlockData {
    /// Creates a new `SequencerBlockData` from the given data.
    ///
    /// Note that this is only constructable for blocks with height >= 1.
    ///
    /// # Errors
    ///
    /// - if the block hash calculated from the header does not match the block hash stored
    ///  in the sequencer block
    /// - if the block has no data hash
    /// - if the block's action tree root inclusion proof cannot be verified
    /// - if the block's height is >1 and it does not contain a last commit or last commit hash
    /// - if the block's last commit hash does not match the one calculated from the block's commit
    pub fn try_from_raw(raw: RawSequencerBlockData) -> Result<Self, Error> {
        let RawSequencerBlockData {
            block_hash,
            header,
            last_commit,
            rollup_data,
            action_tree_root,
            action_tree_root_inclusion_proof,
            chain_ids_commitment,
        } = raw;

        let calculated_block_hash = header.hash();
        if block_hash != calculated_block_hash {
            return Err(Error::HashOfHeaderBlockHashMismatach);
        }

        let data_hash = header.data_hash.ok_or(Error::MissingDataHash)?;
        action_tree_root_inclusion_proof
            .verify(&action_tree_root, data_hash)
            .map_err(Error::Verification)?;

        // genesis and height 1 do not have a last commit
        if header.height.value() <= 1 {
            return Ok(Self {
                block_hash,
                header,
                last_commit: None,
                rollup_data,
                action_tree_root,
                action_tree_root_inclusion_proof,
                chain_ids_commitment,
            });
        }

        // calculate and verify last_commit_hash
        let last_commit_hash = header
            .last_commit_hash
            .ok_or(Error::MissingLastCommitHash)?;

        let calculated_last_commit_hash = calculate_last_commit_hash(
            last_commit
                .as_ref()
                .expect("last_commit must be set if last_commit_hash is set"),
        );
        if calculated_last_commit_hash != last_commit_hash {
            return Err(Error::LastCommitHashMismatch)?;
        }
        Ok(Self {
            block_hash,
            header,
            last_commit,
            rollup_data,
            action_tree_root,
            action_tree_root_inclusion_proof,
            chain_ids_commitment,
        })
    }

    #[must_use]
    pub fn block_hash(&self) -> Hash {
        self.block_hash
    }

    #[must_use]
    pub fn header(&self) -> &Header {
        &self.header
    }

    #[must_use]
    pub fn last_commit(&self) -> &Option<Commit> {
        &self.last_commit
    }

    #[must_use]
    pub fn rollup_data(&self) -> &BTreeMap<ChainId, Vec<Vec<u8>>> {
        &self.rollup_data
    }

    /// Returns the [`SequencerBlockData`] as a [`RawSequencerBlockData`].
    #[must_use]
    pub fn into_raw(self) -> RawSequencerBlockData {
        let Self {
            block_hash,
            header,
            last_commit,
            rollup_data,
            action_tree_root,
            action_tree_root_inclusion_proof,
            chain_ids_commitment,
        } = self;

        RawSequencerBlockData {
            block_hash,
            header,
            last_commit,
            rollup_data,
            action_tree_root,
            action_tree_root_inclusion_proof,
            chain_ids_commitment,
        }
    }

    /// Converts the `SequencerBlockData` into bytes using json.
    ///
    /// # Errors
    ///
    /// - if the data cannot be serialized into json
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        serde_json::to_vec(self).map_err(Error::WritingJson)
    }

    /// Converts json-encoded bytes into a `SequencerBlockData`.
    ///
    /// # Errors
    ///
    /// - if the data cannot be deserialized from json
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(bytes).map_err(Error::ReadingJson)
    }

    /// Converts a Tendermint block into a `SequencerBlockData`.
    /// it parses the block for `SequenceAction`s and namespaces them accordingly.
    ///
    /// # Errors
    ///
    /// - if the block has no data hash
    /// - if the block has no transactions
    /// - if the block's first transaction is not the 32-byte action tree root
    /// - if a transaction in the block cannot be parsed
    /// - if the block's `data_hash` does not match the one calculated from the transactions
    /// - if the inclusion proof of the action tree root in the block's `data_hash` cannot be
    ///   generated
    ///
    /// See `specs/sequencer-inclusion-proofs.md` for most details on the action tree root
    /// and inclusion proof purpose.
    pub fn from_tendermint_block(b: Block) -> Result<Self, Error> {
        use proto::{
            generated::sequencer::v1alpha1 as raw,
            native::sequencer::v1alpha1::SignedTransaction,
            Message as _,
        };
        let data_hash = b.header.data_hash.ok_or(Error::MissingDataHash)?;

        let mut datas = b.data.iter();

        let action_tree_root: [u8; 32] = datas
            .next()
            .map(Vec::as_slice)
            .ok_or(Error::NoData)?
            .try_into()
            .map_err(Error::ActionTreeRootConversion)?;

        let chain_ids_commitment: [u8; 32] = datas
            .next()
            .map(Vec::as_slice)
            .ok_or(Error::NoData)?
            .try_into()
            .map_err(Error::ChainIdsCommitmentConversion)?;

        // we unwrap sequencer txs into rollup-specific data here,
        // and namespace them correspondingly
        let mut rollup_data = BTreeMap::new();

        // the first two transactions is skipped as it's the action tree root,
        // not a user-submitted transaction.
        for (index, tx) in datas.enumerate() {
            debug!(
                index,
                bytes = general_purpose::STANDARD.encode(tx.as_slice()),
                "parsing data from tendermint block",
            );

            let raw_tx = raw::SignedTransaction::decode(&**tx)
                .map_err(Error::SignedTransactionProtobufDecode)?;
            let tx = SignedTransaction::try_from_raw(raw_tx)
                .map_err(Error::RawSignedTransactionConversion)?;
            tx.actions().iter().for_each(|action| {
                if let Some(action) = action.as_sequence() {
                    // TODO(https://github.com/astriaorg/astria/issues/318): intern
                    // these namespaces so they don't get rebuild on every iteration.
                    rollup_data
                        .entry(ChainId(action.chain_id.clone()))
                        .and_modify(|data: &mut Vec<Vec<u8>>| {
                            data.push(action.data.clone());
                        })
                        .or_insert_with(|| vec![action.data.clone()]);
                }
            });
        }

        // generate the action tree root proof of inclusion in `Header::data_hash`
        let (calculated_data_hash, tx_tree) = calculate_data_hash_and_tx_tree(&b.data);
        if calculated_data_hash != data_hash.as_bytes() {
            return Err(Error::CometBftDataHashReconstructedHashMismatch);
        }
        let action_tree_root_inclusion_proof = tx_tree
            .prove_inclusion(0) // action tree root is always the first tx in a block
            .map_err(Error::InclusionProof)?;

        // ensure the chain IDs commitment matches the one calculated from the rollup data
        let chain_ids = rollup_data
            .keys()
            .cloned()
            .map(|chain_id| chain_id.0)
            .collect::<Vec<_>>();
        let calculated_chain_ids_commitment = MerkleTree::from_leaves(chain_ids).root();
        if calculated_chain_ids_commitment != chain_ids_commitment {
            return Err(Error::ReconstructedChainIdsCommitmentMismatch);
        }

        let data = Self {
            block_hash: b.header.hash(),
            header: b.header,
            last_commit: b.last_commit,
            rollup_data,
            action_tree_root,
            action_tree_root_inclusion_proof,
            chain_ids_commitment,
        };
        Ok(data)
    }
}

fn calculate_data_hash_and_tx_tree(txs: &[Vec<u8>]) -> ([u8; 32], MerkleTree) {
    let hashed_txs = txs.iter().map(|tx| sha256_hash(tx)).collect::<Vec<_>>();
    let tx_tree = MerkleTree::from_leaves(hashed_txs);
    let calculated_data_hash = tx_tree.root();
    (calculated_data_hash, tx_tree)
}

fn sha256_hash(data: &[u8]) -> Vec<u8> {
    use sha2::Digest as _;
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// An unverified version of [`SequencerBlockData`], primarily used for
/// serialization/deserialization.
#[allow(clippy::module_name_repetitions)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RawSequencerBlockData {
    pub block_hash: Hash,
    pub header: Header,
    /// This field should be set for every block with height > 1.
    pub last_commit: Option<Commit>,
    /// namespace -> rollup data (chain ID and transactions)
    pub rollup_data: BTreeMap<ChainId, Vec<Vec<u8>>>,
    /// The root of the action tree for this block.
    pub action_tree_root: [u8; 32],
    /// The inclusion proof that the action tree root is included
    /// in `Header::data_hash`.
    pub action_tree_root_inclusion_proof: InclusionProof,
    /// The commitment to the chain IDs of the rollup data.
    /// The merkle root of the tree where the leaves are the chain IDs.
    pub chain_ids_commitment: [u8; 32],
}

impl TryFrom<RawSequencerBlockData> for SequencerBlockData {
    type Error = Error;

    fn try_from(raw: RawSequencerBlockData) -> Result<Self, Self::Error> {
        Self::try_from_raw(raw)
    }
}

impl From<SequencerBlockData> for RawSequencerBlockData {
    fn from(data: SequencerBlockData) -> Self {
        data.into_raw()
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use sequencer_validation::MerkleTree;
    use tendermint::Hash;

    use super::SequencerBlockData;
    use crate::{
        test_utils::make_test_commit_and_hash,
        RawSequencerBlockData,
    };

    #[test]
    fn new_sequencer_block() {
        let mut header = crate::test_utils::default_header();
        let (action_tree_root, action_tree_root_inclusion_proof, data_hash) = {
            let action_tree_root = [9u8; 32];
            let transactions = vec![
                action_tree_root.to_vec(),
                vec![0x11, 0x22, 0x33],
                vec![0x44, 0x55, 0x66],
                vec![0x77, 0x88, 0x99],
            ];
            let tree = MerkleTree::from_leaves(transactions);
            (
                action_tree_root,
                tree.prove_inclusion(0).unwrap(),
                tree.root(),
            )
        };

        let (last_commit_hash, last_commit) = make_test_commit_and_hash();
        header.last_commit_hash = Some(last_commit_hash);

        header.data_hash = Some(Hash::try_from(data_hash.to_vec()).unwrap());
        let block_hash = header.hash();

        let chain_ids_commitment = MerkleTree::from_leaves(vec![]).root();
        SequencerBlockData::try_from_raw(RawSequencerBlockData {
            block_hash,
            header,
            last_commit: Some(last_commit),
            rollup_data: BTreeMap::new(),
            action_tree_root,
            action_tree_root_inclusion_proof,
            chain_ids_commitment,
        })
        .unwrap();
    }

    #[test]
    fn sequencer_block_to_bytes() {
        let mut header = crate::test_utils::default_header();
        let (action_tree_root, action_tree_root_inclusion_proof, data_hash) = {
            let action_tree_root = [9u8; 32];
            let transactions = vec![
                action_tree_root.to_vec(),
                vec![0x11, 0x22, 0x33],
                vec![0x44, 0x55, 0x66],
                vec![0x77, 0x88, 0x99],
            ];
            let tree = MerkleTree::from_leaves(transactions);
            (
                action_tree_root,
                tree.prove_inclusion(0).unwrap(),
                tree.root(),
            )
        };

        let (last_commit_hash, last_commit) = make_test_commit_and_hash();
        header.last_commit_hash = Some(last_commit_hash);

        header.data_hash = Some(Hash::try_from(data_hash.to_vec()).unwrap());
        let block_hash = header.hash();

        let chain_ids_commitment = MerkleTree::from_leaves(vec![]).root();
        let data = SequencerBlockData::try_from_raw(RawSequencerBlockData {
            block_hash,
            header,
            last_commit: Some(last_commit),
            rollup_data: BTreeMap::new(),
            action_tree_root,
            action_tree_root_inclusion_proof,
            chain_ids_commitment,
        })
        .unwrap();

        let bytes = data.to_bytes().unwrap();
        let actual = SequencerBlockData::from_bytes(&bytes).unwrap();
        assert_eq!(data, actual);
    }

    #[test]
    fn test_calculate_last_commit_hash() {
        use tendermint::block::Commit;

        // these values were retrieved by running the cometbft v0.37 + the sequencer app and
        // requesting the following:
        //
        // curl http://localhost:26657/commit?height=79
        // curl http://localhost:26657/block?height=80 | grep last_commit_hash
        //
        // the heights are arbitrary; you just need to pick two successive blocks and take the
        // commit of the first one, and the `last_commit_hash` of the second one.
        //
        // note: this will work with any ABCI app, not just the sequencer app, as commits are
        // generated entirely within cometbft.
        let commit_str = r#"{"height":"79","round":0,"block_id":{"hash":"74BD4E7F7EF902A84D55589F2AA60B332F1C2F34DDE7652C80BFEB8E7471B1DA","parts":{"total":1,"hash":"7632FFB5D84C3A64279BC9EA86992418ED23832C66E0C3504B7025A9AF42C8C4"}},"signatures":[{"block_id_flag":2,"validator_address":"D223B03AE01B4A0296053E01A41AE1E2F9CDEBC9","timestamp":"2023-07-05T19:02:55.206600022Z","signature":"qy9vEjqSrF+8sD0K0IAXA398xN1s3QI2rBBDbBMWf0rw0L+B9Z92DZEptf6bPYWuKUFdEc0QFKhUMQA8HjBaAw=="}]}"#;
        let expected_last_commit_hash =
            "EF285154CDF29146FF423EB48BC7F88A0B57022C9B63455EC7AE876F4EA45B6F"
                .parse::<Hash>()
                .unwrap();
        let commit = serde_json::from_str::<Commit>(commit_str).unwrap();
        let last_commit_hash = crate::calculate_last_commit_hash(&commit);
        assert!(matches!(last_commit_hash, Hash::Sha256(_)));
        assert!(expected_last_commit_hash.as_bytes() == last_commit_hash.as_bytes());
    }

    #[test]
    fn calculate_data_hash() {
        use base64::{
            engine::general_purpose::STANDARD,
            Engine as _,
        };

        // data_hash is calculated from the txs in a block, where the leaves of the merkle tree are
        // the sha256 hashes of the txs
        //
        // this tx and the resultant data_hash were generated by running cometbft v0.37 + sequencer
        // app and submitting a transaction
        //
        // for example, run the code in the readme here: https://github.com/astriaorg/go-sequencer-client
        // check the sequencer app logs for the encoded transaction + block number,
        // then run `curl http://localhost:26657/block?height=<HEIGHT> | grep data_hash`
        // to obtain the respective `data_hash`.
        let tx = STANDARD.decode("CscBCsQBCg0vU2VxdWVuY2VyTXNnErIBCghldGhlcmV1bRJ4Avh1ggU5gIRZaC8AhQUD1cTyglIIlBtwp0/22gQLMRmQwVX9/9u8AvfuiA3gtrOnZAAAgMABoLnRqksJblEaolE6wbsAHYTAiSlA14+B5nvWuFrIfevnoBg+UGcWLC4eg1lZylqLnrL8okBc3vTS4qOO/J5sRtVDGixtZXRybzFsbDJobHAzM3J4eTdwN2s2YXhoeDRjdnFtdGcwY3hkZjZnemY5ahJ0Ck4KRgofL2Nvc21vcy5jcnlwdG8uc2VjcDI1NmsxLlB1YktleRIjCiEDJ/LvaMZTBcGX66geJOEmTm/fyyPTZKMUJoDtMDUmSPkSBAoCCAESGAoQCgV1dGljaxIHMTAwMDAwMBCAlOvcAyIIZXRoZXJldW0aQMhoTCUr84xgTkYxsFWDfHH2k+oHCPsKvbTpz8m5YrHfYMv6gdou6V8oj1v0B9ySD5VjMXQi1kJ9DZN6wD2buo8=").unwrap();
        let expected_data_hash = STANDARD
            .decode("rRDu3aQf1V37yGSTdf2fv9GSPeZ6/p0wJ9pjBl8IqFc=")
            .unwrap();
        let (data_hash, _) = super::calculate_data_hash_and_tx_tree(&[tx]);
        assert_eq!(data_hash.to_vec(), expected_data_hash);
    }
}
