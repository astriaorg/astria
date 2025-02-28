impl serde::Serialize for PublicKey {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.sum.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria_vendored.tendermint.crypto.PublicKey", len)?;
        if let Some(v) = self.sum.as_ref() {
            match v {
                public_key::Sum::Ed25519(v) => {
                    #[allow(clippy::needless_borrow)]
                    #[allow(clippy::needless_borrows_for_generic_args)]
                    struct_ser.serialize_field("ed25519", pbjson::private::base64::encode(&v).as_str())?;
                }
                public_key::Sum::Secp256k1(v) => {
                    #[allow(clippy::needless_borrow)]
                    #[allow(clippy::needless_borrows_for_generic_args)]
                    struct_ser.serialize_field("secp256k1", pbjson::private::base64::encode(&v).as_str())?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PublicKey {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "ed25519",
            "secp256k1",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Ed25519,
            Secp256k1,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "ed25519" => Ok(GeneratedField::Ed25519),
                            "secp256k1" => Ok(GeneratedField::Secp256k1),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PublicKey;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria_vendored.tendermint.crypto.PublicKey")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PublicKey, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut sum__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Ed25519 => {
                            if sum__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ed25519"));
                            }
                            sum__ = map_.next_value::<::std::option::Option<::pbjson::private::BytesDeserialize<_>>>()?.map(|x| public_key::Sum::Ed25519(x.0));
                        }
                        GeneratedField::Secp256k1 => {
                            if sum__.is_some() {
                                return Err(serde::de::Error::duplicate_field("secp256k1"));
                            }
                            sum__ = map_.next_value::<::std::option::Option<::pbjson::private::BytesDeserialize<_>>>()?.map(|x| public_key::Sum::Secp256k1(x.0));
                        }
                    }
                }
                Ok(PublicKey {
                    sum: sum__,
                })
            }
        }
        deserializer.deserialize_struct("astria_vendored.tendermint.crypto.PublicKey", FIELDS, GeneratedVisitor)
    }
}
