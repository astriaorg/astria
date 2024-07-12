use base64_serde::base64_serde_type;
use serde::{
    Deserializer,
    Serializer,
};

base64_serde_type!(pub(crate) Base64Standard, base64::engine::general_purpose::STANDARD);
pub(crate) fn base64_serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: AsRef<[u8]>,
    S: Serializer,
{
    Base64Standard::serialize(value, serializer)
}

pub(crate) fn base64_deserialize_array<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: TryFrom<Vec<u8>>,
    D: Deserializer<'de>,
{
    let bytes = Base64Standard::deserialize(deserializer)?;
    T::try_from(bytes).map_err(|_| serde::de::Error::custom("invalid array length"))
}

pub(crate) mod address_string {
    use serde::{
        Deserialize as _,
        Deserializer,
        Serializer,
    };

    use crate::primitive::v1::Address;

    pub(crate) fn serialize<S: Serializer>(address: &Address, ser: S) -> Result<S::Ok, S::Error> {
        ser.collect_str(address)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deser: D) -> Result<Address, D::Error> {
        use serde::de::Error as _;
        let s = std::borrow::Cow::<'_, str>::deserialize(deser)?;
        s.trim().parse().map_err(D::Error::custom)
    }
}
