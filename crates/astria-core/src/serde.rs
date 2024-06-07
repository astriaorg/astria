use base64_serde::base64_serde_type;
use serde::Serializer;

base64_serde_type!(pub(crate) Base64Standard, base64::engine::general_purpose::STANDARD);
pub(crate) fn base64_serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: AsRef<[u8]>,
    S: Serializer,
{
    Base64Standard::serialize(value, serializer)
}

pub(crate) fn base64_deserialize_address<'de, D>(deserializer: D) -> Result<[u8; 20], D::Error>
where
    D: serde::Deserializer<'de>,
{
    let bytes: Vec<u8> =
        Base64Standard::deserialize(deserializer).map_err(serde::de::Error::custom)?;
    let address = bytes
        .try_into()
        .map_err(|_| serde::de::Error::custom("address bytes length was not 20"))?;
    Ok(address)
}
