impl serde::Serialize for EnshrinedAuctioneerEntry {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.auctioneer_address.is_some() {
            len += 1;
        }
        if self.staker_address.is_some() {
            len += 1;
        }
        if self.staked_amount.is_some() {
            len += 1;
        }
        if !self.fee_asset.is_empty() {
            len += 1;
        }
        if !self.asset.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.auctioneer.v1.EnshrinedAuctioneerEntry", len)?;
        if let Some(v) = self.auctioneer_address.as_ref() {
            struct_ser.serialize_field("auctioneerAddress", v)?;
        }
        if let Some(v) = self.staker_address.as_ref() {
            struct_ser.serialize_field("stakerAddress", v)?;
        }
        if let Some(v) = self.staked_amount.as_ref() {
            struct_ser.serialize_field("stakedAmount", v)?;
        }
        if !self.fee_asset.is_empty() {
            struct_ser.serialize_field("feeAsset", &self.fee_asset)?;
        }
        if !self.asset.is_empty() {
            struct_ser.serialize_field("asset", &self.asset)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for EnshrinedAuctioneerEntry {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "auctioneer_address",
            "auctioneerAddress",
            "staker_address",
            "stakerAddress",
            "staked_amount",
            "stakedAmount",
            "fee_asset",
            "feeAsset",
            "asset",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            AuctioneerAddress,
            StakerAddress,
            StakedAmount,
            FeeAsset,
            Asset,
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
                            "auctioneerAddress" | "auctioneer_address" => Ok(GeneratedField::AuctioneerAddress),
                            "stakerAddress" | "staker_address" => Ok(GeneratedField::StakerAddress),
                            "stakedAmount" | "staked_amount" => Ok(GeneratedField::StakedAmount),
                            "feeAsset" | "fee_asset" => Ok(GeneratedField::FeeAsset),
                            "asset" => Ok(GeneratedField::Asset),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = EnshrinedAuctioneerEntry;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.auctioneer.v1.EnshrinedAuctioneerEntry")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<EnshrinedAuctioneerEntry, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut auctioneer_address__ = None;
                let mut staker_address__ = None;
                let mut staked_amount__ = None;
                let mut fee_asset__ = None;
                let mut asset__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::AuctioneerAddress => {
                            if auctioneer_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("auctioneerAddress"));
                            }
                            auctioneer_address__ = map_.next_value()?;
                        }
                        GeneratedField::StakerAddress => {
                            if staker_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stakerAddress"));
                            }
                            staker_address__ = map_.next_value()?;
                        }
                        GeneratedField::StakedAmount => {
                            if staked_amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stakedAmount"));
                            }
                            staked_amount__ = map_.next_value()?;
                        }
                        GeneratedField::FeeAsset => {
                            if fee_asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeAsset"));
                            }
                            fee_asset__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Asset => {
                            if asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("asset"));
                            }
                            asset__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(EnshrinedAuctioneerEntry {
                    auctioneer_address: auctioneer_address__,
                    staker_address: staker_address__,
                    staked_amount: staked_amount__,
                    fee_asset: fee_asset__.unwrap_or_default(),
                    asset: asset__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.auctioneer.v1.EnshrinedAuctioneerEntry", FIELDS, GeneratedVisitor)
    }
}
