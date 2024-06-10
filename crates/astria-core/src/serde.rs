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
