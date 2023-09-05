// Calculates the `last_commit_hash` given a Tendermint [`Commit`].
//
// It merkleizes the commit and returns the root. The leaves of the merkle tree
// are the protobuf-encoded [`CommitSig`]s; ie. the signatures that the commit consist of.
//
// See https://github.com/cometbft/cometbft/blob/539985efc7d461668ffb46dff88b3f7bb9275e5a/types/block.go#L922
#[must_use]
pub fn calculate_last_commit_hash(commit: &tendermint::block::Commit) -> tendermint::Hash {
    use prost::Message as _;
    use tendermint::{
        crypto,
        merkle,
    };
    use tendermint_proto::types::CommitSig;

    let signatures = commit
        .signatures
        .iter()
        .map(|commit_sig| CommitSig::from(commit_sig.clone()).encode_to_vec())
        .collect::<Vec<_>>();
    tendermint::Hash::Sha256(merkle::simple_hash_from_byte_vectors::<
        crypto::default::Sha256,
    >(&signatures))
}
