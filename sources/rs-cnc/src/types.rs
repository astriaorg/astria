use base64::{engine::general_purpose, Engine as _};
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};
use std::fmt;

#[derive(Clone)]
/// Base64String wraps a Vec<u8> for deserializing base64-encoded strings.
pub struct Base64String(pub Vec<u8>);

impl fmt::Debug for Base64String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&general_purpose::STANDARD.encode(&self.0))
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
        let bytes_res = &general_purpose::STANDARD.decode(value);
        match bytes_res {
            Ok(bytes) => Ok(Base64String(bytes.to_vec())),
            Err(e) => Err(E::custom(format!(
                "failed to decode string {} from base64: {:?}",
                value, e
            ))),
        }
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
