impl serde::Serialize for StakedBuilderEntry {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.creator_address.is_some() {
            len += 1;
        }
        if self.builder_address.is_some() {
            len += 1;
        }
        if self.staked_amount.is_some() {
            len += 1;
        }
        if !self.asset.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.enshrinedbuilder.v1alpha1.StakedBuilderEntry", len)?;
        if let Some(v) = self.creator_address.as_ref() {
            struct_ser.serialize_field("creatorAddress", v)?;
        }
        if let Some(v) = self.builder_address.as_ref() {
            struct_ser.serialize_field("builderAddress", v)?;
        }
        if let Some(v) = self.staked_amount.as_ref() {
            struct_ser.serialize_field("stakedAmount", v)?;
        }
        if !self.asset.is_empty() {
            struct_ser.serialize_field("asset", &self.asset)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StakedBuilderEntry {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "creator_address",
            "creatorAddress",
            "builder_address",
            "builderAddress",
            "staked_amount",
            "stakedAmount",
            "asset",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CreatorAddress,
            BuilderAddress,
            StakedAmount,
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
                            "creatorAddress" | "creator_address" => Ok(GeneratedField::CreatorAddress),
                            "builderAddress" | "builder_address" => Ok(GeneratedField::BuilderAddress),
                            "stakedAmount" | "staked_amount" => Ok(GeneratedField::StakedAmount),
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
            type Value = StakedBuilderEntry;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.enshrinedbuilder.v1alpha1.StakedBuilderEntry")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<StakedBuilderEntry, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut creator_address__ = None;
                let mut builder_address__ = None;
                let mut staked_amount__ = None;
                let mut asset__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::CreatorAddress => {
                            if creator_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("creatorAddress"));
                            }
                            creator_address__ = map_.next_value()?;
                        }
                        GeneratedField::BuilderAddress => {
                            if builder_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("builderAddress"));
                            }
                            builder_address__ = map_.next_value()?;
                        }
                        GeneratedField::StakedAmount => {
                            if staked_amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stakedAmount"));
                            }
                            staked_amount__ = map_.next_value()?;
                        }
                        GeneratedField::Asset => {
                            if asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("asset"));
                            }
                            asset__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(StakedBuilderEntry {
                    creator_address: creator_address__,
                    builder_address: builder_address__,
                    staked_amount: staked_amount__,
                    asset: asset__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.enshrinedbuilder.v1alpha1.StakedBuilderEntry", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for UnstakedBuilderEntry {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.creator_address.is_some() {
            len += 1;
        }
        if self.builder_address.is_some() {
            len += 1;
        }
        if self.time.is_some() {
            len += 1;
        }
        if !self.asset.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.enshrinedbuilder.v1alpha1.UnstakedBuilderEntry", len)?;
        if let Some(v) = self.creator_address.as_ref() {
            struct_ser.serialize_field("creatorAddress", v)?;
        }
        if let Some(v) = self.builder_address.as_ref() {
            struct_ser.serialize_field("builderAddress", v)?;
        }
        if let Some(v) = self.time.as_ref() {
            struct_ser.serialize_field("time", v)?;
        }
        if !self.asset.is_empty() {
            struct_ser.serialize_field("asset", &self.asset)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for UnstakedBuilderEntry {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "creator_address",
            "creatorAddress",
            "builder_address",
            "builderAddress",
            "time",
            "asset",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CreatorAddress,
            BuilderAddress,
            Time,
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
                            "creatorAddress" | "creator_address" => Ok(GeneratedField::CreatorAddress),
                            "builderAddress" | "builder_address" => Ok(GeneratedField::BuilderAddress),
                            "time" => Ok(GeneratedField::Time),
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
            type Value = UnstakedBuilderEntry;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.enshrinedbuilder.v1alpha1.UnstakedBuilderEntry")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<UnstakedBuilderEntry, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut creator_address__ = None;
                let mut builder_address__ = None;
                let mut time__ = None;
                let mut asset__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::CreatorAddress => {
                            if creator_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("creatorAddress"));
                            }
                            creator_address__ = map_.next_value()?;
                        }
                        GeneratedField::BuilderAddress => {
                            if builder_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("builderAddress"));
                            }
                            builder_address__ = map_.next_value()?;
                        }
                        GeneratedField::Time => {
                            if time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("time"));
                            }
                            time__ = map_.next_value()?;
                        }
                        GeneratedField::Asset => {
                            if asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("asset"));
                            }
                            asset__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(UnstakedBuilderEntry {
                    creator_address: creator_address__,
                    builder_address: builder_address__,
                    time: time__,
                    asset: asset__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.enshrinedbuilder.v1alpha1.UnstakedBuilderEntry", FIELDS, GeneratedVisitor)
    }
}
