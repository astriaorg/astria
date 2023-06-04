use serde::{
    Deserialize,
    Serialize,
};
// use tendermint::Block;
use tendermint_proto::types::{
    Block as RawBlock,
    BlockId as RawBlockId,
};

/// cosmos-sdk (Tendermint) RPC types.
/// see https://v1.cosmos.network/rpc/v0.41.4

#[derive(Serialize, Debug)]
pub struct EmptyRequest {}

#[derive(Deserialize, Debug)]
pub struct BlockResponse {
    pub block_id: RawBlockId,
    pub block: RawBlock,
}
