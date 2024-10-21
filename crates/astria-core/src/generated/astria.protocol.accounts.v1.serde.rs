impl serde::Serialize for AssetBalance {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.denom.is_empty() {
            len += 1;
        }
        if self.balance.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.accounts.v1.AssetBalance", len)?;
        if !self.denom.is_empty() {
            struct_ser.serialize_field("denom", &self.denom)?;
        }
        if let Some(v) = self.balance.as_ref() {
            struct_ser.serialize_field("balance", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AssetBalance {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "denom",
            "balance",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Denom,
            Balance,
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
                            "denom" => Ok(GeneratedField::Denom),
                            "balance" => Ok(GeneratedField::Balance),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AssetBalance;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.accounts.v1.AssetBalance")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AssetBalance, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut denom__ = None;
                let mut balance__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Denom => {
                            if denom__.is_some() {
                                return Err(serde::de::Error::duplicate_field("denom"));
                            }
                            denom__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Balance => {
                            if balance__.is_some() {
                                return Err(serde::de::Error::duplicate_field("balance"));
                            }
                            balance__ = map_.next_value()?;
                        }
                    }
                }
                Ok(AssetBalance {
                    denom: denom__.unwrap_or_default(),
                    balance: balance__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.accounts.v1.AssetBalance", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BalanceResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.height != 0 {
            len += 1;
        }
        if !self.balances.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.accounts.v1.BalanceResponse", len)?;
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        if !self.balances.is_empty() {
            struct_ser.serialize_field("balances", &self.balances)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BalanceResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "balances",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            Balances,
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
                            "height" => Ok(GeneratedField::Height),
                            "balances" => Ok(GeneratedField::Balances),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BalanceResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.accounts.v1.BalanceResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BalanceResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut balances__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Balances => {
                            if balances__.is_some() {
                                return Err(serde::de::Error::duplicate_field("balances"));
                            }
                            balances__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(BalanceResponse {
                    height: height__.unwrap_or_default(),
                    balances: balances__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.accounts.v1.BalanceResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for NonceResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.height != 0 {
            len += 1;
        }
        if self.nonce != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.accounts.v1.NonceResponse", len)?;
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        if self.nonce != 0 {
            struct_ser.serialize_field("nonce", &self.nonce)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for NonceResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "nonce",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            Nonce,
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
                            "height" => Ok(GeneratedField::Height),
                            "nonce" => Ok(GeneratedField::Nonce),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = NonceResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.accounts.v1.NonceResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<NonceResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut nonce__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Nonce => {
                            if nonce__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nonce"));
                            }
                            nonce__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(NonceResponse {
                    height: height__.unwrap_or_default(),
                    nonce: nonce__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.accounts.v1.NonceResponse", FIELDS, GeneratedVisitor)
    }
}
