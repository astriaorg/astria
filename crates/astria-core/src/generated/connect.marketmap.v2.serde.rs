impl serde::Serialize for GenesisState {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.market_map.is_some() {
            len += 1;
        }
        if self.last_updated != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("connect.marketmap.v2.GenesisState", len)?;
        if let Some(v) = self.market_map.as_ref() {
            struct_ser.serialize_field("marketMap", v)?;
        }
        if self.last_updated != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("lastUpdated", ToString::to_string(&self.last_updated).as_str())?;
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
            "market_map",
            "marketMap",
            "last_updated",
            "lastUpdated",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            MarketMap,
            LastUpdated,
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
                            "marketMap" | "market_map" => Ok(GeneratedField::MarketMap),
                            "lastUpdated" | "last_updated" => Ok(GeneratedField::LastUpdated),
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
                formatter.write_str("struct connect.marketmap.v2.GenesisState")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GenesisState, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut market_map__ = None;
                let mut last_updated__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::MarketMap => {
                            if market_map__.is_some() {
                                return Err(serde::de::Error::duplicate_field("marketMap"));
                            }
                            market_map__ = map_.next_value()?;
                        }
                        GeneratedField::LastUpdated => {
                            if last_updated__.is_some() {
                                return Err(serde::de::Error::duplicate_field("lastUpdated"));
                            }
                            last_updated__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(GenesisState {
                    market_map: market_map__,
                    last_updated: last_updated__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("connect.marketmap.v2.GenesisState", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for LastUpdatedRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("connect.marketmap.v2.LastUpdatedRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for LastUpdatedRequest {
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
            type Value = LastUpdatedRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.marketmap.v2.LastUpdatedRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<LastUpdatedRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(LastUpdatedRequest {
                })
            }
        }
        deserializer.deserialize_struct("connect.marketmap.v2.LastUpdatedRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for LastUpdatedResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.last_updated != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("connect.marketmap.v2.LastUpdatedResponse", len)?;
        if self.last_updated != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("lastUpdated", ToString::to_string(&self.last_updated).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for LastUpdatedResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "last_updated",
            "lastUpdated",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            LastUpdated,
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
                            "lastUpdated" | "last_updated" => Ok(GeneratedField::LastUpdated),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = LastUpdatedResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.marketmap.v2.LastUpdatedResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<LastUpdatedResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut last_updated__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::LastUpdated => {
                            if last_updated__.is_some() {
                                return Err(serde::de::Error::duplicate_field("lastUpdated"));
                            }
                            last_updated__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(LastUpdatedResponse {
                    last_updated: last_updated__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("connect.marketmap.v2.LastUpdatedResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Market {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.ticker.is_some() {
            len += 1;
        }
        if !self.provider_configs.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("connect.marketmap.v2.Market", len)?;
        if let Some(v) = self.ticker.as_ref() {
            struct_ser.serialize_field("ticker", v)?;
        }
        if !self.provider_configs.is_empty() {
            struct_ser.serialize_field("providerConfigs", &self.provider_configs)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Market {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "ticker",
            "provider_configs",
            "providerConfigs",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Ticker,
            ProviderConfigs,
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
                            "ticker" => Ok(GeneratedField::Ticker),
                            "providerConfigs" | "provider_configs" => Ok(GeneratedField::ProviderConfigs),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Market;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.marketmap.v2.Market")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Market, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut ticker__ = None;
                let mut provider_configs__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Ticker => {
                            if ticker__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ticker"));
                            }
                            ticker__ = map_.next_value()?;
                        }
                        GeneratedField::ProviderConfigs => {
                            if provider_configs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("providerConfigs"));
                            }
                            provider_configs__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Market {
                    ticker: ticker__,
                    provider_configs: provider_configs__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("connect.marketmap.v2.Market", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MarketMap {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.markets.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("connect.marketmap.v2.MarketMap", len)?;
        if !self.markets.is_empty() {
            struct_ser.serialize_field("markets", &self.markets)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MarketMap {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "markets",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Markets,
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
                            "markets" => Ok(GeneratedField::Markets),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MarketMap;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.marketmap.v2.MarketMap")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<MarketMap, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut markets__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Markets => {
                            if markets__.is_some() {
                                return Err(serde::de::Error::duplicate_field("markets"));
                            }
                            markets__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                    }
                }
                Ok(MarketMap {
                    markets: markets__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("connect.marketmap.v2.MarketMap", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MarketMapRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("connect.marketmap.v2.MarketMapRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MarketMapRequest {
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
            type Value = MarketMapRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.marketmap.v2.MarketMapRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<MarketMapRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(MarketMapRequest {
                })
            }
        }
        deserializer.deserialize_struct("connect.marketmap.v2.MarketMapRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MarketMapResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.market_map.is_some() {
            len += 1;
        }
        if self.last_updated != 0 {
            len += 1;
        }
        if !self.chain_id.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("connect.marketmap.v2.MarketMapResponse", len)?;
        if let Some(v) = self.market_map.as_ref() {
            struct_ser.serialize_field("marketMap", v)?;
        }
        if self.last_updated != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("lastUpdated", ToString::to_string(&self.last_updated).as_str())?;
        }
        if !self.chain_id.is_empty() {
            struct_ser.serialize_field("chainId", &self.chain_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MarketMapResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "market_map",
            "marketMap",
            "last_updated",
            "lastUpdated",
            "chain_id",
            "chainId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            MarketMap,
            LastUpdated,
            ChainId,
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
                            "marketMap" | "market_map" => Ok(GeneratedField::MarketMap),
                            "lastUpdated" | "last_updated" => Ok(GeneratedField::LastUpdated),
                            "chainId" | "chain_id" => Ok(GeneratedField::ChainId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MarketMapResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.marketmap.v2.MarketMapResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<MarketMapResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut market_map__ = None;
                let mut last_updated__ = None;
                let mut chain_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::MarketMap => {
                            if market_map__.is_some() {
                                return Err(serde::de::Error::duplicate_field("marketMap"));
                            }
                            market_map__ = map_.next_value()?;
                        }
                        GeneratedField::LastUpdated => {
                            if last_updated__.is_some() {
                                return Err(serde::de::Error::duplicate_field("lastUpdated"));
                            }
                            last_updated__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ChainId => {
                            if chain_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("chainId"));
                            }
                            chain_id__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(MarketMapResponse {
                    market_map: market_map__,
                    last_updated: last_updated__.unwrap_or_default(),
                    chain_id: chain_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("connect.marketmap.v2.MarketMapResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MarketRequest {
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
        let mut struct_ser = serializer.serialize_struct("connect.marketmap.v2.MarketRequest", len)?;
        if let Some(v) = self.currency_pair.as_ref() {
            struct_ser.serialize_field("currencyPair", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MarketRequest {
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
            type Value = MarketRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.marketmap.v2.MarketRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<MarketRequest, V::Error>
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
                            currency_pair__ = map_.next_value()?;
                        }
                    }
                }
                Ok(MarketRequest {
                    currency_pair: currency_pair__,
                })
            }
        }
        deserializer.deserialize_struct("connect.marketmap.v2.MarketRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MarketResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.market.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("connect.marketmap.v2.MarketResponse", len)?;
        if let Some(v) = self.market.as_ref() {
            struct_ser.serialize_field("market", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MarketResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "market",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Market,
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
                            "market" => Ok(GeneratedField::Market),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MarketResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.marketmap.v2.MarketResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<MarketResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut market__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Market => {
                            if market__.is_some() {
                                return Err(serde::de::Error::duplicate_field("market"));
                            }
                            market__ = map_.next_value()?;
                        }
                    }
                }
                Ok(MarketResponse {
                    market: market__,
                })
            }
        }
        deserializer.deserialize_struct("connect.marketmap.v2.MarketResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ProviderConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.name.is_empty() {
            len += 1;
        }
        if !self.off_chain_ticker.is_empty() {
            len += 1;
        }
        if self.normalize_by_pair.is_some() {
            len += 1;
        }
        if self.invert {
            len += 1;
        }
        if !self.metadata_json.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("connect.marketmap.v2.ProviderConfig", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.off_chain_ticker.is_empty() {
            struct_ser.serialize_field("offChainTicker", &self.off_chain_ticker)?;
        }
        if let Some(v) = self.normalize_by_pair.as_ref() {
            struct_ser.serialize_field("normalizeByPair", v)?;
        }
        if self.invert {
            struct_ser.serialize_field("invert", &self.invert)?;
        }
        if !self.metadata_json.is_empty() {
            struct_ser.serialize_field("metadataJSON", &self.metadata_json)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ProviderConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "off_chain_ticker",
            "offChainTicker",
            "normalize_by_pair",
            "normalizeByPair",
            "invert",
            "metadata_JSON",
            "metadataJSON",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            OffChainTicker,
            NormalizeByPair,
            Invert,
            MetadataJson,
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
                            "name" => Ok(GeneratedField::Name),
                            "offChainTicker" | "off_chain_ticker" => Ok(GeneratedField::OffChainTicker),
                            "normalizeByPair" | "normalize_by_pair" => Ok(GeneratedField::NormalizeByPair),
                            "invert" => Ok(GeneratedField::Invert),
                            "metadataJSON" | "metadata_JSON" => Ok(GeneratedField::MetadataJson),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ProviderConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.marketmap.v2.ProviderConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ProviderConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut off_chain_ticker__ = None;
                let mut normalize_by_pair__ = None;
                let mut invert__ = None;
                let mut metadata_json__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::OffChainTicker => {
                            if off_chain_ticker__.is_some() {
                                return Err(serde::de::Error::duplicate_field("offChainTicker"));
                            }
                            off_chain_ticker__ = Some(map_.next_value()?);
                        }
                        GeneratedField::NormalizeByPair => {
                            if normalize_by_pair__.is_some() {
                                return Err(serde::de::Error::duplicate_field("normalizeByPair"));
                            }
                            normalize_by_pair__ = map_.next_value()?;
                        }
                        GeneratedField::Invert => {
                            if invert__.is_some() {
                                return Err(serde::de::Error::duplicate_field("invert"));
                            }
                            invert__ = Some(map_.next_value()?);
                        }
                        GeneratedField::MetadataJson => {
                            if metadata_json__.is_some() {
                                return Err(serde::de::Error::duplicate_field("metadataJSON"));
                            }
                            metadata_json__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(ProviderConfig {
                    name: name__.unwrap_or_default(),
                    off_chain_ticker: off_chain_ticker__.unwrap_or_default(),
                    normalize_by_pair: normalize_by_pair__,
                    invert: invert__.unwrap_or_default(),
                    metadata_json: metadata_json__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("connect.marketmap.v2.ProviderConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Ticker {
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
        if self.decimals != 0 {
            len += 1;
        }
        if self.min_provider_count != 0 {
            len += 1;
        }
        if self.enabled {
            len += 1;
        }
        if !self.metadata_json.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("connect.marketmap.v2.Ticker", len)?;
        if let Some(v) = self.currency_pair.as_ref() {
            struct_ser.serialize_field("currencyPair", v)?;
        }
        if self.decimals != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("decimals", ToString::to_string(&self.decimals).as_str())?;
        }
        if self.min_provider_count != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("minProviderCount", ToString::to_string(&self.min_provider_count).as_str())?;
        }
        if self.enabled {
            struct_ser.serialize_field("enabled", &self.enabled)?;
        }
        if !self.metadata_json.is_empty() {
            struct_ser.serialize_field("metadataJSON", &self.metadata_json)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Ticker {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "currency_pair",
            "currencyPair",
            "decimals",
            "min_provider_count",
            "minProviderCount",
            "enabled",
            "metadata_JSON",
            "metadataJSON",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CurrencyPair,
            Decimals,
            MinProviderCount,
            Enabled,
            MetadataJson,
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
                            "decimals" => Ok(GeneratedField::Decimals),
                            "minProviderCount" | "min_provider_count" => Ok(GeneratedField::MinProviderCount),
                            "enabled" => Ok(GeneratedField::Enabled),
                            "metadataJSON" | "metadata_JSON" => Ok(GeneratedField::MetadataJson),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Ticker;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct connect.marketmap.v2.Ticker")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Ticker, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut currency_pair__ = None;
                let mut decimals__ = None;
                let mut min_provider_count__ = None;
                let mut enabled__ = None;
                let mut metadata_json__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
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
                        GeneratedField::MinProviderCount => {
                            if min_provider_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("minProviderCount"));
                            }
                            min_provider_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Enabled => {
                            if enabled__.is_some() {
                                return Err(serde::de::Error::duplicate_field("enabled"));
                            }
                            enabled__ = Some(map_.next_value()?);
                        }
                        GeneratedField::MetadataJson => {
                            if metadata_json__.is_some() {
                                return Err(serde::de::Error::duplicate_field("metadataJSON"));
                            }
                            metadata_json__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Ticker {
                    currency_pair: currency_pair__,
                    decimals: decimals__.unwrap_or_default(),
                    min_provider_count: min_provider_count__.unwrap_or_default(),
                    enabled: enabled__.unwrap_or_default(),
                    metadata_json: metadata_json__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("connect.marketmap.v2.Ticker", FIELDS, GeneratedVisitor)
    }
}
