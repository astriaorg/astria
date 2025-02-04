impl serde::Serialize for CurrencyPair {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.base.is_empty() {
            len += 1;
        }
        if !self.quote.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("connect.types.v2.CurrencyPair", len)?;
        if !self.base.is_empty() {
            struct_ser.serialize_field("Base", &self.base)?;
        }
        if !self.quote.is_empty() {
            struct_ser.serialize_field("Quote", &self.quote)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CurrencyPair {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "Base",
            "Quote",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Base,
            Quote,
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
                            "Base" => Ok(GeneratedField::Base),
                            "Quote" => Ok(GeneratedField::Quote),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CurrencyPair;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.types.v2.CurrencyPair")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CurrencyPair, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut base__ = None;
                let mut quote__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Base => {
                            if base__.is_some() {
                                return Err(serde::de::Error::duplicate_field("Base"));
                            }
                            base__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Quote => {
                            if quote__.is_some() {
                                return Err(serde::de::Error::duplicate_field("Quote"));
                            }
                            quote__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(CurrencyPair {
                    base: base__.unwrap_or_default(),
                    quote: quote__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("connect.types.v2.CurrencyPair", FIELDS, GeneratedVisitor)
    }
}
