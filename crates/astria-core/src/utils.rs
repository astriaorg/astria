/// Utility to calculate the sha256 hash of protobuf encoded astria types.
pub fn sha256_of_proto<T, U>(val: &T) -> [u8; 32]
where
    T: crate::Protobuf<Raw = U>,
    <T as crate::Protobuf>::Raw: prost::Message,
{
    use sha2::{
        Digest as _,
        Sha256,
    };
    Sha256::digest(val.to_raw().encode_to_vec()).into()
}
