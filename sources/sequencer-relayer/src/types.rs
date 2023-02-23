use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct EmptyRequest {}

#[derive(Serialize, Deserialize, Debug)]
pub struct LatestBlockResponse {
    block: Block,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block {
    header: Header,
    data: Data,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Header {
    // TODO
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Data {
    txs: Vec<String>,
}
