impl serde::Serialize for BaseBlock {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.sequencer_block_hash.is_empty() {
            len += 1;
        }
        if !self.transactions.is_empty() {
            len += 1;
        }
        if self.timestamp.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.optimistic_execution.v1alpha1.BaseBlock", len)?;
        if !self.sequencer_block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("sequencerBlockHash", pbjson::private::base64::encode(&self.sequencer_block_hash).as_str())?;
        }
        if !self.transactions.is_empty() {
            struct_ser.serialize_field("transactions", &self.transactions)?;
        }
        if let Some(v) = self.timestamp.as_ref() {
            struct_ser.serialize_field("timestamp", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BaseBlock {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "sequencer_block_hash",
            "sequencerBlockHash",
            "transactions",
            "timestamp",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SequencerBlockHash,
            Transactions,
            Timestamp,
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
                            "sequencerBlockHash" | "sequencer_block_hash" => Ok(GeneratedField::SequencerBlockHash),
                            "transactions" => Ok(GeneratedField::Transactions),
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BaseBlock;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.optimistic_execution.v1alpha1.BaseBlock")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BaseBlock, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut sequencer_block_hash__ = None;
                let mut transactions__ = None;
                let mut timestamp__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::SequencerBlockHash => {
                            if sequencer_block_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequencerBlockHash"));
                            }
                            sequencer_block_hash__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Transactions => {
                            if transactions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactions"));
                            }
                            transactions__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = map_.next_value()?;
                        }
                    }
                }
                Ok(BaseBlock {
                    sequencer_block_hash: sequencer_block_hash__.unwrap_or_default(),
                    transactions: transactions__.unwrap_or_default(),
                    timestamp: timestamp__,
                })
            }
        }
        deserializer.deserialize_struct("astria.optimistic_execution.v1alpha1.BaseBlock", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ExecuteOptimisticBlockStreamRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.base_block.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.optimistic_execution.v1alpha1.ExecuteOptimisticBlockStreamRequest", len)?;
        if let Some(v) = self.base_block.as_ref() {
            struct_ser.serialize_field("baseBlock", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ExecuteOptimisticBlockStreamRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "base_block",
            "baseBlock",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BaseBlock,
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
                            "baseBlock" | "base_block" => Ok(GeneratedField::BaseBlock),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ExecuteOptimisticBlockStreamRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.optimistic_execution.v1alpha1.ExecuteOptimisticBlockStreamRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ExecuteOptimisticBlockStreamRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut base_block__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BaseBlock => {
                            if base_block__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseBlock"));
                            }
                            base_block__ = map_.next_value()?;
                        }
                    }
                }
                Ok(ExecuteOptimisticBlockStreamRequest {
                    base_block: base_block__,
                })
            }
        }
        deserializer.deserialize_struct("astria.optimistic_execution.v1alpha1.ExecuteOptimisticBlockStreamRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ExecuteOptimisticBlockStreamResponse {
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
        if !self.base_sequencer_block_hash.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.optimistic_execution.v1alpha1.ExecuteOptimisticBlockStreamResponse", len)?;
        if let Some(v) = self.block.as_ref() {
            struct_ser.serialize_field("block", v)?;
        }
        if !self.base_sequencer_block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("baseSequencerBlockHash", pbjson::private::base64::encode(&self.base_sequencer_block_hash).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ExecuteOptimisticBlockStreamResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "block",
            "base_sequencer_block_hash",
            "baseSequencerBlockHash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Block,
            BaseSequencerBlockHash,
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
                            "baseSequencerBlockHash" | "base_sequencer_block_hash" => Ok(GeneratedField::BaseSequencerBlockHash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ExecuteOptimisticBlockStreamResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.optimistic_execution.v1alpha1.ExecuteOptimisticBlockStreamResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ExecuteOptimisticBlockStreamResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut block__ = None;
                let mut base_sequencer_block_hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Block => {
                            if block__.is_some() {
                                return Err(serde::de::Error::duplicate_field("block"));
                            }
                            block__ = map_.next_value()?;
                        }
                        GeneratedField::BaseSequencerBlockHash => {
                            if base_sequencer_block_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseSequencerBlockHash"));
                            }
                            base_sequencer_block_hash__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(ExecuteOptimisticBlockStreamResponse {
                    block: block__,
                    base_sequencer_block_hash: base_sequencer_block_hash__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.optimistic_execution.v1alpha1.ExecuteOptimisticBlockStreamResponse", FIELDS, GeneratedVisitor)
    }
}
