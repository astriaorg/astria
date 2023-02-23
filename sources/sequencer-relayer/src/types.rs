use serde::{Deserialize, Serialize};

/// cosmos-sdk RPC types.
/// see https://v1.cosmos.network/rpc/v0.41.4

#[derive(Serialize, Deserialize, Debug)]
pub struct EmptyRequest {}

#[derive(Serialize, Deserialize, Debug)]
pub struct LatestBlockResponse {
    block: Block,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block {
    header: Header,
    pub data: Data,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Header {
    // TODO
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Data {
    pub txs: Vec<String>,
}
