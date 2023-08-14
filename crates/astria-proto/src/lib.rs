pub use prost::{
    DecodeError,
    EncodeError,
    Message,
};

#[allow(warnings)]
#[allow(unreachable_pub)]
#[rustfmt::skip]
mod proto;
pub mod native;

pub use proto::generated;
