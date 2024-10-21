//! Helpers to support deriving serde functions that encode/decode bytes to/from base64.
//!
//! # Example
//! ```
//! #[derive(serde::Serialize, serde::Deserialize)]
//! pub struct Bytes {
//!     #[serde(with = "astria_core_utils::base64::serde")]
//!     inner: Vec<u8>,
//! }
//! ```

use serde::{
    Deserialize,
    Deserializer,
    Serializer,
};

/// Base64-encode `input`.
///
/// # Errors
///
/// Returns an error if allocation fails.
pub fn serialize<S: Serializer, T: AsRef<[u8]>>(
    input: T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&super::encode(input))
}

/// Base64-decode to `T`.
///
/// # Errors
///
/// Returns an error if decoding fails.
pub fn deserialize<'de, D: Deserializer<'de>, T: From<Vec<u8>>>(
    deserializer: D,
) -> Result<T, D::Error> {
    let output = String::deserialize(deserializer)?;
    super::decode(output)
        .map(T::from)
        .map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    #[test]
    fn should_serialize_to_base64() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct Bytes {
            #[serde(with = "crate::base64::serde")]
            inner: Vec<u8>,
        }

        let bytes = Bytes {
            inner: vec![0, 5, 10, 20, 40, 80, 160],
        };

        let encoded = serde_json::to_string(&bytes).unwrap();
        assert_eq!(encoded, r#"{"inner":"AAUKFChQoA=="}"#);

        let decoded: Bytes = serde_json::from_str(&encoded).unwrap();
        assert_eq!(bytes.inner, decoded.inner);
    }
}
