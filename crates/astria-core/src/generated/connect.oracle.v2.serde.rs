impl serde::Serialize for CurrencyPairGenesis {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.currency_pair.is_some() {
            len += 1;
        }
        if self.currency_pair_price.is_some() {
            len += 1;
        }
        if self.nonce != 0 {
            len += 1;
        }
        if self.id != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("connect.oracle.v2.CurrencyPairGenesis", len)?;
        if let Some(v) = self.currency_pair.as_ref() {
            struct_ser.serialize_field("currencyPair", v)?;
        }
        if let Some(v) = self.currency_pair_price.as_ref() {
            struct_ser.serialize_field("currencyPairPrice", v)?;
        }
        if self.nonce != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("nonce", ToString::to_string(&self.nonce).as_str())?;
        }
        if self.id != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("id", ToString::to_string(&self.id).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CurrencyPairGenesis {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "currency_pair",
            "currencyPair",
            "currency_pair_price",
            "currencyPairPrice",
            "nonce",
            "id",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CurrencyPair,
            CurrencyPairPrice,
            Nonce,
            Id,
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
                            "currencyPair" | "currency_pair" => Ok(GeneratedField::CurrencyPair),
                            "currencyPairPrice" | "currency_pair_price" => Ok(GeneratedField::CurrencyPairPrice),
                            "nonce" => Ok(GeneratedField::Nonce),
                            "id" => Ok(GeneratedField::Id),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CurrencyPairGenesis;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.oracle.v2.CurrencyPairGenesis")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CurrencyPairGenesis, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut currency_pair__ = None;
                let mut currency_pair_price__ = None;
                let mut nonce__ = None;
                let mut id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::CurrencyPair => {
                            if currency_pair__.is_some() {
                                return Err(serde::de::Error::duplicate_field("currencyPair"));
                            }
                            currency_pair__ = map_.next_value()?;
                        }
                        GeneratedField::CurrencyPairPrice => {
                            if currency_pair_price__.is_some() {
                                return Err(serde::de::Error::duplicate_field("currencyPairPrice"));
                            }
                            currency_pair_price__ = map_.next_value()?;
                        }
                        GeneratedField::Nonce => {
                            if nonce__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nonce"));
                            }
                            nonce__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(CurrencyPairGenesis {
                    currency_pair: currency_pair__,
                    currency_pair_price: currency_pair_price__,
                    nonce: nonce__.unwrap_or_default(),
                    id: id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("connect.oracle.v2.CurrencyPairGenesis", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CurrencyPairState {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.price.is_some() {
            len += 1;
        }
        if self.nonce != 0 {
            len += 1;
        }
        if self.id != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("connect.oracle.v2.CurrencyPairState", len)?;
        if let Some(v) = self.price.as_ref() {
            struct_ser.serialize_field("price", v)?;
        }
        if self.nonce != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("nonce", ToString::to_string(&self.nonce).as_str())?;
        }
        if self.id != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("id", ToString::to_string(&self.id).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CurrencyPairState {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "price",
            "nonce",
            "id",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Price,
            Nonce,
            Id,
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
                            "price" => Ok(GeneratedField::Price),
                            "nonce" => Ok(GeneratedField::Nonce),
                            "id" => Ok(GeneratedField::Id),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CurrencyPairState;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.oracle.v2.CurrencyPairState")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CurrencyPairState, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut price__ = None;
                let mut nonce__ = None;
                let mut id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Price => {
                            if price__.is_some() {
                                return Err(serde::de::Error::duplicate_field("price"));
                            }
                            price__ = map_.next_value()?;
                        }
                        GeneratedField::Nonce => {
                            if nonce__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nonce"));
                            }
                            nonce__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(CurrencyPairState {
                    price: price__,
                    nonce: nonce__.unwrap_or_default(),
                    id: id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("connect.oracle.v2.CurrencyPairState", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GenesisState {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.currency_pair_genesis.is_empty() {
            len += 1;
        }
        if self.next_id != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("connect.oracle.v2.GenesisState", len)?;
        if !self.currency_pair_genesis.is_empty() {
            struct_ser.serialize_field("currencyPairGenesis", &self.currency_pair_genesis)?;
        }
        if self.next_id != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("nextId", ToString::to_string(&self.next_id).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GenesisState {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "currency_pair_genesis",
            "currencyPairGenesis",
            "next_id",
            "nextId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CurrencyPairGenesis,
            NextId,
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
                            "currencyPairGenesis" | "currency_pair_genesis" => Ok(GeneratedField::CurrencyPairGenesis),
                            "nextId" | "next_id" => Ok(GeneratedField::NextId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GenesisState;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.oracle.v2.GenesisState")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GenesisState, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut currency_pair_genesis__ = None;
                let mut next_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::CurrencyPairGenesis => {
                            if currency_pair_genesis__.is_some() {
                                return Err(serde::de::Error::duplicate_field("currencyPairGenesis"));
                            }
                            currency_pair_genesis__ = Some(map_.next_value()?);
                        }
                        GeneratedField::NextId => {
                            if next_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nextId"));
                            }
                            next_id__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(GenesisState {
                    currency_pair_genesis: currency_pair_genesis__.unwrap_or_default(),
                    next_id: next_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("connect.oracle.v2.GenesisState", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetAllCurrencyPairsRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("connect.oracle.v2.GetAllCurrencyPairsRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetAllCurrencyPairsRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
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
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetAllCurrencyPairsRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.oracle.v2.GetAllCurrencyPairsRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetAllCurrencyPairsRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetAllCurrencyPairsRequest {
                })
            }
        }
        deserializer.deserialize_struct("connect.oracle.v2.GetAllCurrencyPairsRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetAllCurrencyPairsResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.currency_pairs.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("connect.oracle.v2.GetAllCurrencyPairsResponse", len)?;
        if !self.currency_pairs.is_empty() {
            struct_ser.serialize_field("currencyPairs", &self.currency_pairs)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetAllCurrencyPairsResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "currency_pairs",
            "currencyPairs",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CurrencyPairs,
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
                            "currencyPairs" | "currency_pairs" => Ok(GeneratedField::CurrencyPairs),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetAllCurrencyPairsResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.oracle.v2.GetAllCurrencyPairsResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetAllCurrencyPairsResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut currency_pairs__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::CurrencyPairs => {
                            if currency_pairs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("currencyPairs"));
                            }
                            currency_pairs__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(GetAllCurrencyPairsResponse {
                    currency_pairs: currency_pairs__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("connect.oracle.v2.GetAllCurrencyPairsResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetCurrencyPairMappingRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("connect.oracle.v2.GetCurrencyPairMappingRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetCurrencyPairMappingRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
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
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetCurrencyPairMappingRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.oracle.v2.GetCurrencyPairMappingRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetCurrencyPairMappingRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetCurrencyPairMappingRequest {
                })
            }
        }
        deserializer.deserialize_struct("connect.oracle.v2.GetCurrencyPairMappingRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetCurrencyPairMappingResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.currency_pair_mapping.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("connect.oracle.v2.GetCurrencyPairMappingResponse", len)?;
        if !self.currency_pair_mapping.is_empty() {
            struct_ser.serialize_field("currencyPairMapping", &self.currency_pair_mapping)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetCurrencyPairMappingResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "currency_pair_mapping",
            "currencyPairMapping",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CurrencyPairMapping,
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
                            "currencyPairMapping" | "currency_pair_mapping" => Ok(GeneratedField::CurrencyPairMapping),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetCurrencyPairMappingResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.oracle.v2.GetCurrencyPairMappingResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetCurrencyPairMappingResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut currency_pair_mapping__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::CurrencyPairMapping => {
                            if currency_pair_mapping__.is_some() {
                                return Err(serde::de::Error::duplicate_field("currencyPairMapping"));
                            }
                            currency_pair_mapping__ = Some(
                                map_.next_value::<std::collections::BTreeMap<::pbjson::private::NumberDeserialize<u64>, _>>()?
                                    .into_iter().map(|(k,v)| (k.0, v)).collect()
                            );
                        }
                    }
                }
                Ok(GetCurrencyPairMappingResponse {
                    currency_pair_mapping: currency_pair_mapping__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("connect.oracle.v2.GetCurrencyPairMappingResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetPriceRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.currency_pair.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("connect.oracle.v2.GetPriceRequest", len)?;
        if !self.currency_pair.is_empty() {
            struct_ser.serialize_field("currencyPair", &self.currency_pair)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetPriceRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "currency_pair",
            "currencyPair",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CurrencyPair,
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
                            "currencyPair" | "currency_pair" => Ok(GeneratedField::CurrencyPair),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetPriceRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.oracle.v2.GetPriceRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetPriceRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut currency_pair__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::CurrencyPair => {
                            if currency_pair__.is_some() {
                                return Err(serde::de::Error::duplicate_field("currencyPair"));
                            }
                            currency_pair__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(GetPriceRequest {
                    currency_pair: currency_pair__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("connect.oracle.v2.GetPriceRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetPriceResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.price.is_some() {
            len += 1;
        }
        if self.nonce != 0 {
            len += 1;
        }
        if self.decimals != 0 {
            len += 1;
        }
        if self.id != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("connect.oracle.v2.GetPriceResponse", len)?;
        if let Some(v) = self.price.as_ref() {
            struct_ser.serialize_field("price", v)?;
        }
        if self.nonce != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("nonce", ToString::to_string(&self.nonce).as_str())?;
        }
        if self.decimals != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("decimals", ToString::to_string(&self.decimals).as_str())?;
        }
        if self.id != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("id", ToString::to_string(&self.id).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetPriceResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "price",
            "nonce",
            "decimals",
            "id",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Price,
            Nonce,
            Decimals,
            Id,
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
                            "price" => Ok(GeneratedField::Price),
                            "nonce" => Ok(GeneratedField::Nonce),
                            "decimals" => Ok(GeneratedField::Decimals),
                            "id" => Ok(GeneratedField::Id),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetPriceResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.oracle.v2.GetPriceResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetPriceResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut price__ = None;
                let mut nonce__ = None;
                let mut decimals__ = None;
                let mut id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Price => {
                            if price__.is_some() {
                                return Err(serde::de::Error::duplicate_field("price"));
                            }
                            price__ = map_.next_value()?;
                        }
                        GeneratedField::Nonce => {
                            if nonce__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nonce"));
                            }
                            nonce__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Decimals => {
                            if decimals__.is_some() {
                                return Err(serde::de::Error::duplicate_field("decimals"));
                            }
                            decimals__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(GetPriceResponse {
                    price: price__,
                    nonce: nonce__.unwrap_or_default(),
                    decimals: decimals__.unwrap_or_default(),
                    id: id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("connect.oracle.v2.GetPriceResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetPricesRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.currency_pair_ids.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("connect.oracle.v2.GetPricesRequest", len)?;
        if !self.currency_pair_ids.is_empty() {
            struct_ser.serialize_field("currencyPairIds", &self.currency_pair_ids)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetPricesRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "currency_pair_ids",
            "currencyPairIds",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CurrencyPairIds,
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
                            "currencyPairIds" | "currency_pair_ids" => Ok(GeneratedField::CurrencyPairIds),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetPricesRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.oracle.v2.GetPricesRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetPricesRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut currency_pair_ids__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::CurrencyPairIds => {
                            if currency_pair_ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("currencyPairIds"));
                            }
                            currency_pair_ids__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(GetPricesRequest {
                    currency_pair_ids: currency_pair_ids__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("connect.oracle.v2.GetPricesRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetPricesResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.prices.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("connect.oracle.v2.GetPricesResponse", len)?;
        if !self.prices.is_empty() {
            struct_ser.serialize_field("prices", &self.prices)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetPricesResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "prices",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Prices,
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
                            "prices" => Ok(GeneratedField::Prices),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetPricesResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.oracle.v2.GetPricesResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetPricesResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut prices__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Prices => {
                            if prices__.is_some() {
                                return Err(serde::de::Error::duplicate_field("prices"));
                            }
                            prices__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(GetPricesResponse {
                    prices: prices__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("connect.oracle.v2.GetPricesResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for QuotePrice {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.price.is_empty() {
            len += 1;
        }
        if self.block_timestamp.is_some() {
            len += 1;
        }
        if self.block_height != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("connect.oracle.v2.QuotePrice", len)?;
        if !self.price.is_empty() {
            struct_ser.serialize_field("price", &self.price)?;
        }
        if let Some(v) = self.block_timestamp.as_ref() {
            struct_ser.serialize_field("blockTimestamp", v)?;
        }
        if self.block_height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("blockHeight", ToString::to_string(&self.block_height).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for QuotePrice {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "price",
            "block_timestamp",
            "blockTimestamp",
            "block_height",
            "blockHeight",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Price,
            BlockTimestamp,
            BlockHeight,
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
                            "price" => Ok(GeneratedField::Price),
                            "blockTimestamp" | "block_timestamp" => Ok(GeneratedField::BlockTimestamp),
                            "blockHeight" | "block_height" => Ok(GeneratedField::BlockHeight),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = QuotePrice;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.oracle.v2.QuotePrice")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<QuotePrice, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut price__ = None;
                let mut block_timestamp__ = None;
                let mut block_height__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Price => {
                            if price__.is_some() {
                                return Err(serde::de::Error::duplicate_field("price"));
                            }
                            price__ = Some(map_.next_value()?);
                        }
                        GeneratedField::BlockTimestamp => {
                            if block_timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockTimestamp"));
                            }
                            block_timestamp__ = map_.next_value()?;
                        }
                        GeneratedField::BlockHeight => {
                            if block_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHeight"));
                            }
                            block_height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(QuotePrice {
                    price: price__.unwrap_or_default(),
                    block_timestamp: block_timestamp__,
                    block_height: block_height__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("connect.oracle.v2.QuotePrice", FIELDS, GeneratedVisitor)
    }
}
