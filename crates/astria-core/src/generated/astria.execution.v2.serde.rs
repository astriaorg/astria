impl serde::Serialize for CommitmentState {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.soft_executed_block_metadata.is_some() {
            len += 1;
        }
        if self.firm_executed_block_metadata.is_some() {
            len += 1;
        }
        if self.lowest_celestia_search_height != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.CommitmentState", len)?;
        if let Some(v) = self.soft_executed_block_metadata.as_ref() {
            struct_ser.serialize_field("softExecutedBlockMetadata", v)?;
        }
        if let Some(v) = self.firm_executed_block_metadata.as_ref() {
            struct_ser.serialize_field("firmExecutedBlockMetadata", v)?;
        }
        if self.lowest_celestia_search_height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("lowestCelestiaSearchHeight", ToString::to_string(&self.lowest_celestia_search_height).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CommitmentState {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "soft_executed_block_metadata",
            "softExecutedBlockMetadata",
            "firm_executed_block_metadata",
            "firmExecutedBlockMetadata",
            "lowest_celestia_search_height",
            "lowestCelestiaSearchHeight",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SoftExecutedBlockMetadata,
            FirmExecutedBlockMetadata,
            LowestCelestiaSearchHeight,
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
                            "softExecutedBlockMetadata" | "soft_executed_block_metadata" => Ok(GeneratedField::SoftExecutedBlockMetadata),
                            "firmExecutedBlockMetadata" | "firm_executed_block_metadata" => Ok(GeneratedField::FirmExecutedBlockMetadata),
                            "lowestCelestiaSearchHeight" | "lowest_celestia_search_height" => Ok(GeneratedField::LowestCelestiaSearchHeight),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CommitmentState;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.CommitmentState")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CommitmentState, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut soft_executed_block_metadata__ = None;
                let mut firm_executed_block_metadata__ = None;
                let mut lowest_celestia_search_height__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::SoftExecutedBlockMetadata => {
                            if soft_executed_block_metadata__.is_some() {
                                return Err(serde::de::Error::duplicate_field("softExecutedBlockMetadata"));
                            }
                            soft_executed_block_metadata__ = map_.next_value()?;
                        }
                        GeneratedField::FirmExecutedBlockMetadata => {
                            if firm_executed_block_metadata__.is_some() {
                                return Err(serde::de::Error::duplicate_field("firmExecutedBlockMetadata"));
                            }
                            firm_executed_block_metadata__ = map_.next_value()?;
                        }
                        GeneratedField::LowestCelestiaSearchHeight => {
                            if lowest_celestia_search_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("lowestCelestiaSearchHeight"));
                            }
                            lowest_celestia_search_height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(CommitmentState {
                    soft_executed_block_metadata: soft_executed_block_metadata__,
                    firm_executed_block_metadata: firm_executed_block_metadata__,
                    lowest_celestia_search_height: lowest_celestia_search_height__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.CommitmentState", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CreateExecutionSessionRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("astria.execution.v2.CreateExecutionSessionRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CreateExecutionSessionRequest {
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
            type Value = CreateExecutionSessionRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.CreateExecutionSessionRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CreateExecutionSessionRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(CreateExecutionSessionRequest {
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.CreateExecutionSessionRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ExecuteBlockRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.session_id.is_empty() {
            len += 1;
        }
        if !self.parent_hash.is_empty() {
            len += 1;
        }
        if !self.transactions.is_empty() {
            len += 1;
        }
        if self.timestamp.is_some() {
            len += 1;
        }
        if !self.sequencer_block_hash.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.ExecuteBlockRequest", len)?;
        if !self.session_id.is_empty() {
            struct_ser.serialize_field("sessionId", &self.session_id)?;
        }
        if !self.parent_hash.is_empty() {
            struct_ser.serialize_field("parentHash", &self.parent_hash)?;
        }
        if !self.transactions.is_empty() {
            struct_ser.serialize_field("transactions", &self.transactions)?;
        }
        if let Some(v) = self.timestamp.as_ref() {
            struct_ser.serialize_field("timestamp", v)?;
        }
        if !self.sequencer_block_hash.is_empty() {
            struct_ser.serialize_field("sequencerBlockHash", &self.sequencer_block_hash)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ExecuteBlockRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "session_id",
            "sessionId",
            "parent_hash",
            "parentHash",
            "transactions",
            "timestamp",
            "sequencer_block_hash",
            "sequencerBlockHash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SessionId,
            ParentHash,
            Transactions,
            Timestamp,
            SequencerBlockHash,
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
                            "sessionId" | "session_id" => Ok(GeneratedField::SessionId),
                            "parentHash" | "parent_hash" => Ok(GeneratedField::ParentHash),
                            "transactions" => Ok(GeneratedField::Transactions),
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            "sequencerBlockHash" | "sequencer_block_hash" => Ok(GeneratedField::SequencerBlockHash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ExecuteBlockRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.ExecuteBlockRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ExecuteBlockRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut session_id__ = None;
                let mut parent_hash__ = None;
                let mut transactions__ = None;
                let mut timestamp__ = None;
                let mut sequencer_block_hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::SessionId => {
                            if session_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sessionId"));
                            }
                            session_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ParentHash => {
                            if parent_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("parentHash"));
                            }
                            parent_hash__ = Some(map_.next_value()?);
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
                        GeneratedField::SequencerBlockHash => {
                            if sequencer_block_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequencerBlockHash"));
                            }
                            sequencer_block_hash__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(ExecuteBlockRequest {
                    session_id: session_id__.unwrap_or_default(),
                    parent_hash: parent_hash__.unwrap_or_default(),
                    transactions: transactions__.unwrap_or_default(),
                    timestamp: timestamp__,
                    sequencer_block_hash: sequencer_block_hash__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.ExecuteBlockRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ExecuteBlockResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.executed_block_metadata.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.ExecuteBlockResponse", len)?;
        if let Some(v) = self.executed_block_metadata.as_ref() {
            struct_ser.serialize_field("executedBlockMetadata", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ExecuteBlockResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "executed_block_metadata",
            "executedBlockMetadata",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ExecutedBlockMetadata,
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
                            "executedBlockMetadata" | "executed_block_metadata" => Ok(GeneratedField::ExecutedBlockMetadata),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ExecuteBlockResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.ExecuteBlockResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ExecuteBlockResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut executed_block_metadata__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ExecutedBlockMetadata => {
                            if executed_block_metadata__.is_some() {
                                return Err(serde::de::Error::duplicate_field("executedBlockMetadata"));
                            }
                            executed_block_metadata__ = map_.next_value()?;
                        }
                    }
                }
                Ok(ExecuteBlockResponse {
                    executed_block_metadata: executed_block_metadata__,
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.ExecuteBlockResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ExecutedBlockIdentifier {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.identifier.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.ExecutedBlockIdentifier", len)?;
        if let Some(v) = self.identifier.as_ref() {
            match v {
                executed_block_identifier::Identifier::Number(v) => {
                    #[allow(clippy::needless_borrow)]
                    #[allow(clippy::needless_borrows_for_generic_args)]
                    struct_ser.serialize_field("number", ToString::to_string(&v).as_str())?;
                }
                executed_block_identifier::Identifier::Hash(v) => {
                    struct_ser.serialize_field("hash", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ExecutedBlockIdentifier {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "number",
            "hash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Number,
            Hash,
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
                            "number" => Ok(GeneratedField::Number),
                            "hash" => Ok(GeneratedField::Hash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ExecutedBlockIdentifier;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.ExecutedBlockIdentifier")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ExecutedBlockIdentifier, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut identifier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Number => {
                            if identifier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("number"));
                            }
                            identifier__ = map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| executed_block_identifier::Identifier::Number(x.0));
                        }
                        GeneratedField::Hash => {
                            if identifier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            identifier__ = map_.next_value::<::std::option::Option<_>>()?.map(executed_block_identifier::Identifier::Hash);
                        }
                    }
                }
                Ok(ExecutedBlockIdentifier {
                    identifier: identifier__,
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.ExecutedBlockIdentifier", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ExecutedBlockMetadata {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.number != 0 {
            len += 1;
        }
        if !self.hash.is_empty() {
            len += 1;
        }
        if !self.parent_hash.is_empty() {
            len += 1;
        }
        if self.timestamp.is_some() {
            len += 1;
        }
        if !self.sequencer_block_hash.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.ExecutedBlockMetadata", len)?;
        if self.number != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("number", ToString::to_string(&self.number).as_str())?;
        }
        if !self.hash.is_empty() {
            struct_ser.serialize_field("hash", &self.hash)?;
        }
        if !self.parent_hash.is_empty() {
            struct_ser.serialize_field("parentHash", &self.parent_hash)?;
        }
        if let Some(v) = self.timestamp.as_ref() {
            struct_ser.serialize_field("timestamp", v)?;
        }
        if !self.sequencer_block_hash.is_empty() {
            struct_ser.serialize_field("sequencerBlockHash", &self.sequencer_block_hash)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ExecutedBlockMetadata {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "number",
            "hash",
            "parent_hash",
            "parentHash",
            "timestamp",
            "sequencer_block_hash",
            "sequencerBlockHash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Number,
            Hash,
            ParentHash,
            Timestamp,
            SequencerBlockHash,
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
                            "number" => Ok(GeneratedField::Number),
                            "hash" => Ok(GeneratedField::Hash),
                            "parentHash" | "parent_hash" => Ok(GeneratedField::ParentHash),
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            "sequencerBlockHash" | "sequencer_block_hash" => Ok(GeneratedField::SequencerBlockHash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ExecutedBlockMetadata;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.ExecutedBlockMetadata")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ExecutedBlockMetadata, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut number__ = None;
                let mut hash__ = None;
                let mut parent_hash__ = None;
                let mut timestamp__ = None;
                let mut sequencer_block_hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Number => {
                            if number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("number"));
                            }
                            number__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ParentHash => {
                            if parent_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("parentHash"));
                            }
                            parent_hash__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = map_.next_value()?;
                        }
                        GeneratedField::SequencerBlockHash => {
                            if sequencer_block_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequencerBlockHash"));
                            }
                            sequencer_block_hash__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(ExecutedBlockMetadata {
                    number: number__.unwrap_or_default(),
                    hash: hash__.unwrap_or_default(),
                    parent_hash: parent_hash__.unwrap_or_default(),
                    timestamp: timestamp__,
                    sequencer_block_hash: sequencer_block_hash__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.ExecutedBlockMetadata", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ExecutionSession {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.session_id.is_empty() {
            len += 1;
        }
        if self.execution_session_parameters.is_some() {
            len += 1;
        }
        if self.commitment_state.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.ExecutionSession", len)?;
        if !self.session_id.is_empty() {
            struct_ser.serialize_field("sessionId", &self.session_id)?;
        }
        if let Some(v) = self.execution_session_parameters.as_ref() {
            struct_ser.serialize_field("executionSessionParameters", v)?;
        }
        if let Some(v) = self.commitment_state.as_ref() {
            struct_ser.serialize_field("commitmentState", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ExecutionSession {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "session_id",
            "sessionId",
            "execution_session_parameters",
            "executionSessionParameters",
            "commitment_state",
            "commitmentState",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SessionId,
            ExecutionSessionParameters,
            CommitmentState,
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
                            "sessionId" | "session_id" => Ok(GeneratedField::SessionId),
                            "executionSessionParameters" | "execution_session_parameters" => Ok(GeneratedField::ExecutionSessionParameters),
                            "commitmentState" | "commitment_state" => Ok(GeneratedField::CommitmentState),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ExecutionSession;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.ExecutionSession")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ExecutionSession, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut session_id__ = None;
                let mut execution_session_parameters__ = None;
                let mut commitment_state__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::SessionId => {
                            if session_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sessionId"));
                            }
                            session_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ExecutionSessionParameters => {
                            if execution_session_parameters__.is_some() {
                                return Err(serde::de::Error::duplicate_field("executionSessionParameters"));
                            }
                            execution_session_parameters__ = map_.next_value()?;
                        }
                        GeneratedField::CommitmentState => {
                            if commitment_state__.is_some() {
                                return Err(serde::de::Error::duplicate_field("commitmentState"));
                            }
                            commitment_state__ = map_.next_value()?;
                        }
                    }
                }
                Ok(ExecutionSession {
                    session_id: session_id__.unwrap_or_default(),
                    execution_session_parameters: execution_session_parameters__,
                    commitment_state: commitment_state__,
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.ExecutionSession", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ExecutionSessionParameters {
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
        if self.rollup_start_block_number != 0 {
            len += 1;
        }
        if self.rollup_end_block_number != 0 {
            len += 1;
        }
        if !self.sequencer_chain_id.is_empty() {
            len += 1;
        }
        if self.sequencer_start_block_height != 0 {
            len += 1;
        }
        if !self.celestia_chain_id.is_empty() {
            len += 1;
        }
        if self.celestia_search_height_max_look_ahead != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.ExecutionSessionParameters", len)?;
        if let Some(v) = self.rollup_id.as_ref() {
            struct_ser.serialize_field("rollupId", v)?;
        }
        if self.rollup_start_block_number != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("rollupStartBlockNumber", ToString::to_string(&self.rollup_start_block_number).as_str())?;
        }
        if self.rollup_end_block_number != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("rollupEndBlockNumber", ToString::to_string(&self.rollup_end_block_number).as_str())?;
        }
        if !self.sequencer_chain_id.is_empty() {
            struct_ser.serialize_field("sequencerChainId", &self.sequencer_chain_id)?;
        }
        if self.sequencer_start_block_height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("sequencerStartBlockHeight", ToString::to_string(&self.sequencer_start_block_height).as_str())?;
        }
        if !self.celestia_chain_id.is_empty() {
            struct_ser.serialize_field("celestiaChainId", &self.celestia_chain_id)?;
        }
        if self.celestia_search_height_max_look_ahead != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("celestiaSearchHeightMaxLookAhead", ToString::to_string(&self.celestia_search_height_max_look_ahead).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ExecutionSessionParameters {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "rollup_id",
            "rollupId",
            "rollup_start_block_number",
            "rollupStartBlockNumber",
            "rollup_end_block_number",
            "rollupEndBlockNumber",
            "sequencer_chain_id",
            "sequencerChainId",
            "sequencer_start_block_height",
            "sequencerStartBlockHeight",
            "celestia_chain_id",
            "celestiaChainId",
            "celestia_search_height_max_look_ahead",
            "celestiaSearchHeightMaxLookAhead",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RollupId,
            RollupStartBlockNumber,
            RollupEndBlockNumber,
            SequencerChainId,
            SequencerStartBlockHeight,
            CelestiaChainId,
            CelestiaSearchHeightMaxLookAhead,
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
                            "rollupStartBlockNumber" | "rollup_start_block_number" => Ok(GeneratedField::RollupStartBlockNumber),
                            "rollupEndBlockNumber" | "rollup_end_block_number" => Ok(GeneratedField::RollupEndBlockNumber),
                            "sequencerChainId" | "sequencer_chain_id" => Ok(GeneratedField::SequencerChainId),
                            "sequencerStartBlockHeight" | "sequencer_start_block_height" => Ok(GeneratedField::SequencerStartBlockHeight),
                            "celestiaChainId" | "celestia_chain_id" => Ok(GeneratedField::CelestiaChainId),
                            "celestiaSearchHeightMaxLookAhead" | "celestia_search_height_max_look_ahead" => Ok(GeneratedField::CelestiaSearchHeightMaxLookAhead),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ExecutionSessionParameters;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.ExecutionSessionParameters")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ExecutionSessionParameters, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut rollup_id__ = None;
                let mut rollup_start_block_number__ = None;
                let mut rollup_end_block_number__ = None;
                let mut sequencer_chain_id__ = None;
                let mut sequencer_start_block_height__ = None;
                let mut celestia_chain_id__ = None;
                let mut celestia_search_height_max_look_ahead__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RollupId => {
                            if rollup_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupId"));
                            }
                            rollup_id__ = map_.next_value()?;
                        }
                        GeneratedField::RollupStartBlockNumber => {
                            if rollup_start_block_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupStartBlockNumber"));
                            }
                            rollup_start_block_number__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::RollupEndBlockNumber => {
                            if rollup_end_block_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupEndBlockNumber"));
                            }
                            rollup_end_block_number__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::SequencerChainId => {
                            if sequencer_chain_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequencerChainId"));
                            }
                            sequencer_chain_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::SequencerStartBlockHeight => {
                            if sequencer_start_block_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequencerStartBlockHeight"));
                            }
                            sequencer_start_block_height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::CelestiaChainId => {
                            if celestia_chain_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("celestiaChainId"));
                            }
                            celestia_chain_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::CelestiaSearchHeightMaxLookAhead => {
                            if celestia_search_height_max_look_ahead__.is_some() {
                                return Err(serde::de::Error::duplicate_field("celestiaSearchHeightMaxLookAhead"));
                            }
                            celestia_search_height_max_look_ahead__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(ExecutionSessionParameters {
                    rollup_id: rollup_id__,
                    rollup_start_block_number: rollup_start_block_number__.unwrap_or_default(),
                    rollup_end_block_number: rollup_end_block_number__.unwrap_or_default(),
                    sequencer_chain_id: sequencer_chain_id__.unwrap_or_default(),
                    sequencer_start_block_height: sequencer_start_block_height__.unwrap_or_default(),
                    celestia_chain_id: celestia_chain_id__.unwrap_or_default(),
                    celestia_search_height_max_look_ahead: celestia_search_height_max_look_ahead__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.ExecutionSessionParameters", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetExecutedBlockMetadataRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.identifier.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.GetExecutedBlockMetadataRequest", len)?;
        if let Some(v) = self.identifier.as_ref() {
            struct_ser.serialize_field("identifier", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetExecutedBlockMetadataRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "identifier",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Identifier,
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
                            "identifier" => Ok(GeneratedField::Identifier),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetExecutedBlockMetadataRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.GetExecutedBlockMetadataRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetExecutedBlockMetadataRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut identifier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Identifier => {
                            if identifier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("identifier"));
                            }
                            identifier__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GetExecutedBlockMetadataRequest {
                    identifier: identifier__,
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.GetExecutedBlockMetadataRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for UpdateCommitmentStateRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.session_id.is_empty() {
            len += 1;
        }
        if self.commitment_state.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.UpdateCommitmentStateRequest", len)?;
        if !self.session_id.is_empty() {
            struct_ser.serialize_field("sessionId", &self.session_id)?;
        }
        if let Some(v) = self.commitment_state.as_ref() {
            struct_ser.serialize_field("commitmentState", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for UpdateCommitmentStateRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "session_id",
            "sessionId",
            "commitment_state",
            "commitmentState",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SessionId,
            CommitmentState,
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
                            "sessionId" | "session_id" => Ok(GeneratedField::SessionId),
                            "commitmentState" | "commitment_state" => Ok(GeneratedField::CommitmentState),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = UpdateCommitmentStateRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.UpdateCommitmentStateRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<UpdateCommitmentStateRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut session_id__ = None;
                let mut commitment_state__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::SessionId => {
                            if session_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sessionId"));
                            }
                            session_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::CommitmentState => {
                            if commitment_state__.is_some() {
                                return Err(serde::de::Error::duplicate_field("commitmentState"));
                            }
                            commitment_state__ = map_.next_value()?;
                        }
                    }
                }
                Ok(UpdateCommitmentStateRequest {
                    session_id: session_id__.unwrap_or_default(),
                    commitment_state: commitment_state__,
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.UpdateCommitmentStateRequest", FIELDS, GeneratedVisitor)
    }
}
