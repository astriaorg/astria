impl serde::Serialize for IbcRelay {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.raw_action.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria_vendored.penumbra.core.component.ibc.v1.IbcRelay", len)?;
        if let Some(v) = self.raw_action.as_ref() {
            struct_ser.serialize_field("rawAction", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for IbcRelay {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "raw_action",
            "rawAction",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RawAction,
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
                            "rawAction" | "raw_action" => Ok(GeneratedField::RawAction),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = IbcRelay;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria_vendored.penumbra.core.component.ibc.v1.IbcRelay")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<IbcRelay, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut raw_action__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RawAction => {
                            if raw_action__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rawAction"));
                            }
                            raw_action__ = map_.next_value()?;
                        }
                    }
                }
                Ok(IbcRelay {
                    raw_action: raw_action__,
                })
            }
        }
        deserializer.deserialize_struct("astria_vendored.penumbra.core.component.ibc.v1.IbcRelay", FIELDS, GeneratedVisitor)
    }
}
