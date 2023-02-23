use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct EmptyRequest {}

#[derive(Serialize, Deserialize)]
pub struct LatestBlockResponse {
    block: Block,
}

#[derive(Serialize, Deserialize)]
pub struct Block {
    header: Header,
    data: Data,
}

#[derive(Serialize, Deserialize)]
pub struct Header {
    // TODO
}

#[derive(Serialize, Deserialize)]
pub struct Data {
    txs: Vec<String>,
}
