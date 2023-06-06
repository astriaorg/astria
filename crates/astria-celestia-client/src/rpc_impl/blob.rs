use jsonrpsee::proc_macros::rpc;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct BlobWithCommitment {
    blob: crate::Blob,
    #[serde(with = "crate::serde::Base64Standard")]
    commitment: Vec<u8>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Commitment(#[serde(with = "crate::serde::Base64Standard")] pub Vec<u8>);

#[derive(Clone, Debug, Serialize)]
pub struct NamespaceId(#[serde(with = "crate::serde::Base64Standard")] pub Vec<u8>);

#[derive(Debug, Deserialize, Serialize)]
pub struct Node(#[serde(with = "crate::serde::Base64Standard")] pub Vec<u8>);

#[derive(Debug, Deserialize, Serialize)]
pub struct Proof {
    pub start: i64,
    pub end: i64,
    pub nodes: Vec<Node>,
    pub leaf_hash: Node,
    pub is_max_namespace_id_ignored: bool,
}

#[rpc(client)]
trait Block {
    #[method(name = "block.Submit")]
    async fn submit(&self, blobs: &[BlobWithCommitment]) -> Result<u64, Error>;

    #[method(name = "block.Get")]
    async fn get(
        &self,
        height: u64,
        namespace_id: Vec<u8>,
        commitment: &Commitment,
    ) -> Result<BlobWithCommitment, Error>;

    #[method(name = "block.GetAll")]
    async fn get_all(
        &self,
        height: u64,
        namespace_ids: &[NamespaceId],
    ) -> Result<Vec<BlobWithCommitment>, Error>;

    #[method(name = "block.GetProof")]
    async fn get_proof(
        &self,
        height: u64,
        namespace_id: NamespaceId,
        commitment: &Commitment,
    ) -> Result<Vec<Proof>, Error>;

    #[method(name = "block.Included")]
    async fn included(
        &self,
        height: u64,
        namespace_id: NamespaceId,
        proofs: &[Proof],
        commitment: &Commitment,
    ) -> Result<bool, Error>;
}
