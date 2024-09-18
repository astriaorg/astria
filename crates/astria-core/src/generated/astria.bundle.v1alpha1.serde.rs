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
        let mut struct_ser = serializer.serialize_struct("astria.bundle.v1alpha1.BaseBlock", len)?;
        if !self.sequencer_block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
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
                formatter.write_str("struct astria.bundle.v1alpha1.BaseBlock")
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
        deserializer.deserialize_struct("astria.bundle.v1alpha1.BaseBlock", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Bundle {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.fee != 0 {
            len += 1;
        }
        if !self.transactions.is_empty() {
            len += 1;
        }
        if !self.base_sequencer_block_hash.is_empty() {
            len += 1;
        }
        if !self.prev_rollup_block_hash.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.bundle.v1alpha1.Bundle", len)?;
        if self.fee != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("fee", ToString::to_string(&self.fee).as_str())?;
        }
        if !self.transactions.is_empty() {
            struct_ser.serialize_field("transactions", &self.transactions.iter().map(pbjson::private::base64::encode).collect::<Vec<_>>())?;
        }
        if !self.base_sequencer_block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("baseSequencerBlockHash", pbjson::private::base64::encode(&self.base_sequencer_block_hash).as_str())?;
        }
        if !self.prev_rollup_block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("prevRollupBlockHash", pbjson::private::base64::encode(&self.prev_rollup_block_hash).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Bundle {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "fee",
            "transactions",
            "base_sequencer_block_hash",
            "baseSequencerBlockHash",
            "prev_rollup_block_hash",
            "prevRollupBlockHash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Fee,
            Transactions,
            BaseSequencerBlockHash,
            PrevRollupBlockHash,
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
                            "fee" => Ok(GeneratedField::Fee),
                            "transactions" => Ok(GeneratedField::Transactions),
                            "baseSequencerBlockHash" | "base_sequencer_block_hash" => Ok(GeneratedField::BaseSequencerBlockHash),
                            "prevRollupBlockHash" | "prev_rollup_block_hash" => Ok(GeneratedField::PrevRollupBlockHash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Bundle;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.bundle.v1alpha1.Bundle")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Bundle, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut fee__ = None;
                let mut transactions__ = None;
                let mut base_sequencer_block_hash__ = None;
                let mut prev_rollup_block_hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Fee => {
                            if fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("fee"));
                            }
                            fee__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Transactions => {
                            if transactions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactions"));
                            }
                            transactions__ = 
                                Some(map_.next_value::<Vec<::pbjson::private::BytesDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                        GeneratedField::BaseSequencerBlockHash => {
                            if base_sequencer_block_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseSequencerBlockHash"));
                            }
                            base_sequencer_block_hash__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PrevRollupBlockHash => {
                            if prev_rollup_block_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("prevRollupBlockHash"));
                            }
                            prev_rollup_block_hash__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Bundle {
                    fee: fee__.unwrap_or_default(),
                    transactions: transactions__.unwrap_or_default(),
                    base_sequencer_block_hash: base_sequencer_block_hash__.unwrap_or_default(),
                    prev_rollup_block_hash: prev_rollup_block_hash__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.bundle.v1alpha1.Bundle", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StreamBundlesRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("astria.bundle.v1alpha1.StreamBundlesRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StreamBundlesRequest {
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
            type Value = StreamBundlesRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.bundle.v1alpha1.StreamBundlesRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<StreamBundlesRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(StreamBundlesRequest {
                })
            }
        }
        deserializer.deserialize_struct("astria.bundle.v1alpha1.StreamBundlesRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StreamExecuteOptimisticBlockRequest {
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
        let mut struct_ser = serializer.serialize_struct("astria.bundle.v1alpha1.StreamExecuteOptimisticBlockRequest", len)?;
        if let Some(v) = self.block.as_ref() {
            struct_ser.serialize_field("block", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StreamExecuteOptimisticBlockRequest {
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
            type Value = StreamExecuteOptimisticBlockRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.bundle.v1alpha1.StreamExecuteOptimisticBlockRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<StreamExecuteOptimisticBlockRequest, V::Error>
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
                Ok(StreamExecuteOptimisticBlockRequest {
                    block: block__,
                })
            }
        }
        deserializer.deserialize_struct("astria.bundle.v1alpha1.StreamExecuteOptimisticBlockRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StreamExecuteOptimisticBlockResponse {
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
        let mut struct_ser = serializer.serialize_struct("astria.bundle.v1alpha1.StreamExecuteOptimisticBlockResponse", len)?;
        if let Some(v) = self.block.as_ref() {
            struct_ser.serialize_field("block", v)?;
        }
        if !self.base_sequencer_block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("baseSequencerBlockHash", pbjson::private::base64::encode(&self.base_sequencer_block_hash).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StreamExecuteOptimisticBlockResponse {
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
            type Value = StreamExecuteOptimisticBlockResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.bundle.v1alpha1.StreamExecuteOptimisticBlockResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<StreamExecuteOptimisticBlockResponse, V::Error>
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
                Ok(StreamExecuteOptimisticBlockResponse {
                    block: block__,
                    base_sequencer_block_hash: base_sequencer_block_hash__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.bundle.v1alpha1.StreamExecuteOptimisticBlockResponse", FIELDS, GeneratedVisitor)
    }
}
