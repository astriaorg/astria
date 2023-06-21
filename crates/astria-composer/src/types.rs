use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct ChainId(String);
