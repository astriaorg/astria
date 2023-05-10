use base64::{engine::general_purpose, Engine as _};
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::fmt;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Base64String(pub Vec<u8>);

impl Base64String {
    pub fn from_string(s: String) -> Result<Base64String, base64::DecodeError> {
        general_purpose::STANDARD.decode(s).map(Base64String)
    }

    pub fn from_bytes(bytes: &[u8]) -> Base64String {
        Base64String(bytes.to_vec())
    }
}

impl fmt::Display for Base64String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", general_purpose::STANDARD.encode(&self.0))
    }
}

impl fmt::Debug for Base64String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&general_purpose::STANDARD.encode(&self.0))
    }
}

impl Serialize for Base64String {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&general_purpose::STANDARD.encode(&self.0))
    }
}

impl<'de> Deserialize<'de> for Base64String {
    fn deserialize<D>(deserializer: D) -> Result<Base64String, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(Base64StringVisitor)
    }
}

struct Base64StringVisitor;

impl Base64StringVisitor {
    fn decode_string<E>(self, value: &str) -> Result<Base64String, E>
    where
        E: de::Error,
    {
        general_purpose::STANDARD
            .decode(value)
            .map(Base64String)
            .map_err(|e| {
                E::custom(format!(
                    "failed to decode string {} from base64: {:?}",
                    value, e
                ))
            })
    }
}

impl<'de> Visitor<'de> for Base64StringVisitor {
    type Value = Base64String;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a base64-encoded string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.decode_string(value)
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.decode_string(&value)
    }
}
