impl serde::Serialize for ExtendedCommitInfoWithCurrencyPairMapping {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.extended_commit_info.is_some() {
            len += 1;
        }
        if !self.id_to_currency_pair.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.price_feed.v1.ExtendedCommitInfoWithCurrencyPairMapping", len)?;
        if let Some(v) = self.extended_commit_info.as_ref() {
            struct_ser.serialize_field("extendedCommitInfo", v)?;
        }
        if !self.id_to_currency_pair.is_empty() {
            struct_ser.serialize_field("idToCurrencyPair", &self.id_to_currency_pair)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ExtendedCommitInfoWithCurrencyPairMapping {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "extended_commit_info",
            "extendedCommitInfo",
            "id_to_currency_pair",
            "idToCurrencyPair",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ExtendedCommitInfo,
            IdToCurrencyPair,
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
                            "extendedCommitInfo" | "extended_commit_info" => Ok(GeneratedField::ExtendedCommitInfo),
                            "idToCurrencyPair" | "id_to_currency_pair" => Ok(GeneratedField::IdToCurrencyPair),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ExtendedCommitInfoWithCurrencyPairMapping;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.price_feed.v1.ExtendedCommitInfoWithCurrencyPairMapping")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ExtendedCommitInfoWithCurrencyPairMapping, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut extended_commit_info__ = None;
                let mut id_to_currency_pair__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ExtendedCommitInfo => {
                            if extended_commit_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("extendedCommitInfo"));
                            }
                            extended_commit_info__ = map_.next_value()?;
                        }
                        GeneratedField::IdToCurrencyPair => {
                            if id_to_currency_pair__.is_some() {
                                return Err(serde::de::Error::duplicate_field("idToCurrencyPair"));
                            }
                            id_to_currency_pair__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(ExtendedCommitInfoWithCurrencyPairMapping {
                    extended_commit_info: extended_commit_info__,
                    id_to_currency_pair: id_to_currency_pair__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.price_feed.v1.ExtendedCommitInfoWithCurrencyPairMapping", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for IdWithCurrencyPair {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.id != 0 {
            len += 1;
        }
        if self.currency_pair.is_some() {
            len += 1;
        }
        if self.decimals != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.price_feed.v1.IdWithCurrencyPair", len)?;
        if self.id != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("id", ToString::to_string(&self.id).as_str())?;
        }
        if let Some(v) = self.currency_pair.as_ref() {
            struct_ser.serialize_field("currencyPair", v)?;
        }
        if self.decimals != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("decimals", ToString::to_string(&self.decimals).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for IdWithCurrencyPair {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "currency_pair",
            "currencyPair",
            "decimals",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            CurrencyPair,
            Decimals,
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
                            "id" => Ok(GeneratedField::Id),
                            "currencyPair" | "currency_pair" => Ok(GeneratedField::CurrencyPair),
                            "decimals" => Ok(GeneratedField::Decimals),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = IdWithCurrencyPair;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.price_feed.v1.IdWithCurrencyPair")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<IdWithCurrencyPair, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut currency_pair__ = None;
                let mut decimals__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::CurrencyPair => {
                            if currency_pair__.is_some() {
                                return Err(serde::de::Error::duplicate_field("currencyPair"));
                            }
                            currency_pair__ = map_.next_value()?;
                        }
                        GeneratedField::Decimals => {
                            if decimals__.is_some() {
                                return Err(serde::de::Error::duplicate_field("decimals"));
                            }
                            decimals__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(IdWithCurrencyPair {
                    id: id__.unwrap_or_default(),
                    currency_pair: currency_pair__,
                    decimals: decimals__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.price_feed.v1.IdWithCurrencyPair", FIELDS, GeneratedVisitor)
    }
}
