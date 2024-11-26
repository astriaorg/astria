impl serde::Serialize for GetBlockCommitmentStreamRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("astria.sequencerblock.optimistic.v1alpha1.GetBlockCommitmentStreamRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetBlockCommitmentStreamRequest {
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
            type Value = GetBlockCommitmentStreamRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.optimistic.v1alpha1.GetBlockCommitmentStreamRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetBlockCommitmentStreamRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetBlockCommitmentStreamRequest {
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.optimistic.v1alpha1.GetBlockCommitmentStreamRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetBlockCommitmentStreamResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.commitment.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.optimistic.v1alpha1.GetBlockCommitmentStreamResponse", len)?;
        if let Some(v) = self.commitment.as_ref() {
            struct_ser.serialize_field("commitment", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetBlockCommitmentStreamResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "commitment",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Commitment,
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
                            "commitment" => Ok(GeneratedField::Commitment),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetBlockCommitmentStreamResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.optimistic.v1alpha1.GetBlockCommitmentStreamResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetBlockCommitmentStreamResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut commitment__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Commitment => {
                            if commitment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("commitment"));
                            }
                            commitment__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GetBlockCommitmentStreamResponse {
                    commitment: commitment__,
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.optimistic.v1alpha1.GetBlockCommitmentStreamResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetOptimisticBlockStreamRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.rollup_id.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.optimistic.v1alpha1.GetOptimisticBlockStreamRequest", len)?;
        if let Some(v) = self.rollup_id.as_ref() {
            struct_ser.serialize_field("rollupId", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetOptimisticBlockStreamRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "rollup_id",
            "rollupId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RollupId,
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
                            "rollupId" | "rollup_id" => Ok(GeneratedField::RollupId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetOptimisticBlockStreamRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.optimistic.v1alpha1.GetOptimisticBlockStreamRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetOptimisticBlockStreamRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut rollup_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RollupId => {
                            if rollup_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupId"));
                            }
                            rollup_id__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GetOptimisticBlockStreamRequest {
                    rollup_id: rollup_id__,
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.optimistic.v1alpha1.GetOptimisticBlockStreamRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetOptimisticBlockStreamResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.block.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.optimistic.v1alpha1.GetOptimisticBlockStreamResponse", len)?;
        if let Some(v) = self.block.as_ref() {
            struct_ser.serialize_field("block", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetOptimisticBlockStreamResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "block",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Block,
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
                            "block" => Ok(GeneratedField::Block),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetOptimisticBlockStreamResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.optimistic.v1alpha1.GetOptimisticBlockStreamResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetOptimisticBlockStreamResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut block__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Block => {
                            if block__.is_some() {
                                return Err(serde::de::Error::duplicate_field("block"));
                            }
                            block__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GetOptimisticBlockStreamResponse {
                    block: block__,
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.optimistic.v1alpha1.GetOptimisticBlockStreamResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SequencerBlockCommit {
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
        if !self.block_hash.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.optimistic.v1alpha1.SequencerBlockCommit", len)?;
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        if !self.block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("blockHash", pbjson::private::base64::encode(&self.block_hash).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SequencerBlockCommit {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "block_hash",
            "blockHash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            BlockHash,
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
                            "blockHash" | "block_hash" => Ok(GeneratedField::BlockHash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SequencerBlockCommit;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.optimistic.v1alpha1.SequencerBlockCommit")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SequencerBlockCommit, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut block_hash__ = None;
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
                        GeneratedField::BlockHash => {
                            if block_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHash"));
                            }
                            block_hash__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(SequencerBlockCommit {
                    height: height__.unwrap_or_default(),
                    block_hash: block_hash__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.optimistic.v1alpha1.SequencerBlockCommit", FIELDS, GeneratedVisitor)
    }
}
