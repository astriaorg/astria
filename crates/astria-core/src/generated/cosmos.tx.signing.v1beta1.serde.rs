impl serde::Serialize for SignMode {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "SIGN_MODE_UNSPECIFIED",
            Self::Direct => "SIGN_MODE_DIRECT",
            Self::Textual => "SIGN_MODE_TEXTUAL",
            Self::DirectAux => "SIGN_MODE_DIRECT_AUX",
            Self::LegacyAminoJson => "SIGN_MODE_LEGACY_AMINO_JSON",
            Self::Eip191 => "SIGN_MODE_EIP_191",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for SignMode {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "SIGN_MODE_UNSPECIFIED",
            "SIGN_MODE_DIRECT",
            "SIGN_MODE_TEXTUAL",
            "SIGN_MODE_DIRECT_AUX",
            "SIGN_MODE_LEGACY_AMINO_JSON",
            "SIGN_MODE_EIP_191",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SignMode;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "SIGN_MODE_UNSPECIFIED" => Ok(SignMode::Unspecified),
                    "SIGN_MODE_DIRECT" => Ok(SignMode::Direct),
                    "SIGN_MODE_TEXTUAL" => Ok(SignMode::Textual),
                    "SIGN_MODE_DIRECT_AUX" => Ok(SignMode::DirectAux),
                    "SIGN_MODE_LEGACY_AMINO_JSON" => Ok(SignMode::LegacyAminoJson),
                    "SIGN_MODE_EIP_191" => Ok(SignMode::Eip191),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
