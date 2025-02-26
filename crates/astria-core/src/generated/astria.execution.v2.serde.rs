<<<<<<< HEAD
impl serde::Serialize for CommitmentState {
=======
impl serde::Serialize for BatchGetBlocksRequest {
>>>>>>> superfluffy/forma-restart-logic
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
<<<<<<< HEAD
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
            struct_ser.serialize_field("lowestCelestiaSearchHeight", ToString::to_string(&self.lowest_celestia_search_height).as_str())?;
=======
        if !self.identifiers.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.BatchGetBlocksRequest", len)?;
        if !self.identifiers.is_empty() {
            struct_ser.serialize_field("identifiers", &self.identifiers)?;
>>>>>>> superfluffy/forma-restart-logic
        }
        struct_ser.end()
    }
}
<<<<<<< HEAD
impl<'de> serde::Deserialize<'de> for CommitmentState {
=======
impl<'de> serde::Deserialize<'de> for BatchGetBlocksRequest {
>>>>>>> superfluffy/forma-restart-logic
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
<<<<<<< HEAD
            "soft_executed_block_metadata",
            "softExecutedBlockMetadata",
            "firm_executed_block_metadata",
            "firmExecutedBlockMetadata",
            "lowest_celestia_search_height",
            "lowestCelestiaSearchHeight",
=======
            "identifiers",
>>>>>>> superfluffy/forma-restart-logic
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
<<<<<<< HEAD
            SoftExecutedBlockMetadata,
            FirmExecutedBlockMetadata,
            LowestCelestiaSearchHeight,
=======
            Identifiers,
>>>>>>> superfluffy/forma-restart-logic
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
<<<<<<< HEAD
                            "softExecutedBlockMetadata" | "soft_executed_block_metadata" => Ok(GeneratedField::SoftExecutedBlockMetadata),
                            "firmExecutedBlockMetadata" | "firm_executed_block_metadata" => Ok(GeneratedField::FirmExecutedBlockMetadata),
                            "lowestCelestiaSearchHeight" | "lowest_celestia_search_height" => Ok(GeneratedField::LowestCelestiaSearchHeight),
=======
                            "identifiers" => Ok(GeneratedField::Identifiers),
>>>>>>> superfluffy/forma-restart-logic
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
<<<<<<< HEAD
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
=======
            type Value = BatchGetBlocksRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.BatchGetBlocksRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BatchGetBlocksRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut identifiers__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Identifiers => {
                            if identifiers__.is_some() {
                                return Err(serde::de::Error::duplicate_field("identifiers"));
                            }
                            identifiers__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(BatchGetBlocksRequest {
                    identifiers: identifiers__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.BatchGetBlocksRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BatchGetBlocksResponse {
>>>>>>> superfluffy/forma-restart-logic
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
<<<<<<< HEAD
        let len = 0;
        let struct_ser = serializer.serialize_struct("astria.execution.v2.CreateExecutionSessionRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CreateExecutionSessionRequest {
=======
        let mut len = 0;
        if !self.blocks.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.BatchGetBlocksResponse", len)?;
        if !self.blocks.is_empty() {
            struct_ser.serialize_field("blocks", &self.blocks)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BatchGetBlocksResponse {
>>>>>>> superfluffy/forma-restart-logic
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
<<<<<<< HEAD
=======
            "blocks",
>>>>>>> superfluffy/forma-restart-logic
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
<<<<<<< HEAD
=======
            Blocks,
>>>>>>> superfluffy/forma-restart-logic
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
<<<<<<< HEAD
                            Err(serde::de::Error::unknown_field(value, FIELDS))
=======
                        match value {
                            "blocks" => Ok(GeneratedField::Blocks),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
>>>>>>> superfluffy/forma-restart-logic
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
<<<<<<< HEAD
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
=======
            type Value = BatchGetBlocksResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.BatchGetBlocksResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BatchGetBlocksResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut blocks__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Blocks => {
                            if blocks__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blocks"));
                            }
                            blocks__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(BatchGetBlocksResponse {
                    blocks: blocks__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.BatchGetBlocksResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Block {
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
        if !self.parent_block_hash.is_empty() {
            len += 1;
        }
        if self.timestamp.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.Block", len)?;
        if self.number != 0 {
            struct_ser.serialize_field("number", &self.number)?;
        }
        if !self.hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("hash", pbjson::private::base64::encode(&self.hash).as_str())?;
        }
        if !self.parent_block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("parentBlockHash", pbjson::private::base64::encode(&self.parent_block_hash).as_str())?;
        }
        if let Some(v) = self.timestamp.as_ref() {
            struct_ser.serialize_field("timestamp", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Block {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "number",
            "hash",
            "parent_block_hash",
            "parentBlockHash",
            "timestamp",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Number,
            Hash,
            ParentBlockHash,
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
                            "number" => Ok(GeneratedField::Number),
                            "hash" => Ok(GeneratedField::Hash),
                            "parentBlockHash" | "parent_block_hash" => Ok(GeneratedField::ParentBlockHash),
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
            type Value = Block;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.Block")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Block, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut number__ = None;
                let mut hash__ = None;
                let mut parent_block_hash__ = None;
                let mut timestamp__ = None;
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
                            hash__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ParentBlockHash => {
                            if parent_block_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("parentBlockHash"));
                            }
                            parent_block_hash__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Block {
                    number: number__.unwrap_or_default(),
                    hash: hash__.unwrap_or_default(),
                    parent_block_hash: parent_block_hash__.unwrap_or_default(),
                    timestamp: timestamp__,
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.Block", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BlockIdentifier {
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
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.BlockIdentifier", len)?;
        if let Some(v) = self.identifier.as_ref() {
            match v {
                block_identifier::Identifier::BlockNumber(v) => {
                    struct_ser.serialize_field("blockNumber", v)?;
                }
                block_identifier::Identifier::BlockHash(v) => {
                    #[allow(clippy::needless_borrow)]
                    struct_ser.serialize_field("blockHash", pbjson::private::base64::encode(&v).as_str())?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BlockIdentifier {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "block_number",
            "blockNumber",
            "block_hash",
            "blockHash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BlockNumber,
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
                            "blockNumber" | "block_number" => Ok(GeneratedField::BlockNumber),
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
            type Value = BlockIdentifier;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.BlockIdentifier")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BlockIdentifier, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut identifier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BlockNumber => {
                            if identifier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockNumber"));
                            }
                            identifier__ = map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| block_identifier::Identifier::BlockNumber(x.0));
                        }
                        GeneratedField::BlockHash => {
                            if identifier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHash"));
                            }
                            identifier__ = map_.next_value::<::std::option::Option<::pbjson::private::BytesDeserialize<_>>>()?.map(|x| block_identifier::Identifier::BlockHash(x.0));
                        }
                    }
                }
                Ok(BlockIdentifier {
                    identifier: identifier__,
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.BlockIdentifier", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CommitmentState {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.soft.is_some() {
            len += 1;
        }
        if self.firm.is_some() {
            len += 1;
        }
        if self.base_celestia_height != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.CommitmentState", len)?;
        if let Some(v) = self.soft.as_ref() {
            struct_ser.serialize_field("soft", v)?;
        }
        if let Some(v) = self.firm.as_ref() {
            struct_ser.serialize_field("firm", v)?;
        }
        if self.base_celestia_height != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("baseCelestiaHeight", ToString::to_string(&self.base_celestia_height).as_str())?;
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
            "soft",
            "firm",
            "base_celestia_height",
            "baseCelestiaHeight",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Soft,
            Firm,
            BaseCelestiaHeight,
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
                            "soft" => Ok(GeneratedField::Soft),
                            "firm" => Ok(GeneratedField::Firm),
                            "baseCelestiaHeight" | "base_celestia_height" => Ok(GeneratedField::BaseCelestiaHeight),
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
                let mut soft__ = None;
                let mut firm__ = None;
                let mut base_celestia_height__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Soft => {
                            if soft__.is_some() {
                                return Err(serde::de::Error::duplicate_field("soft"));
                            }
                            soft__ = map_.next_value()?;
                        }
                        GeneratedField::Firm => {
                            if firm__.is_some() {
                                return Err(serde::de::Error::duplicate_field("firm"));
                            }
                            firm__ = map_.next_value()?;
                        }
                        GeneratedField::BaseCelestiaHeight => {
                            if base_celestia_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseCelestiaHeight"));
                            }
                            base_celestia_height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(CommitmentState {
                    soft: soft__,
                    firm: firm__,
                    base_celestia_height: base_celestia_height__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.CommitmentState", FIELDS, GeneratedVisitor)
>>>>>>> superfluffy/forma-restart-logic
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
<<<<<<< HEAD
        if !self.session_id.is_empty() {
            len += 1;
        }
        if !self.parent_hash.is_empty() {
=======
        if !self.prev_block_hash.is_empty() {
>>>>>>> superfluffy/forma-restart-logic
            len += 1;
        }
        if !self.transactions.is_empty() {
            len += 1;
        }
        if self.timestamp.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.ExecuteBlockRequest", len)?;
<<<<<<< HEAD
        if !self.session_id.is_empty() {
            struct_ser.serialize_field("sessionId", &self.session_id)?;
        }
        if !self.parent_hash.is_empty() {
            struct_ser.serialize_field("parentHash", &self.parent_hash)?;
=======
        if !self.prev_block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("prevBlockHash", pbjson::private::base64::encode(&self.prev_block_hash).as_str())?;
>>>>>>> superfluffy/forma-restart-logic
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
impl<'de> serde::Deserialize<'de> for ExecuteBlockRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
<<<<<<< HEAD
            "session_id",
            "sessionId",
            "parent_hash",
            "parentHash",
=======
            "prev_block_hash",
            "prevBlockHash",
>>>>>>> superfluffy/forma-restart-logic
            "transactions",
            "timestamp",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
<<<<<<< HEAD
            SessionId,
            ParentHash,
=======
            PrevBlockHash,
>>>>>>> superfluffy/forma-restart-logic
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
<<<<<<< HEAD
                            "sessionId" | "session_id" => Ok(GeneratedField::SessionId),
                            "parentHash" | "parent_hash" => Ok(GeneratedField::ParentHash),
=======
                            "prevBlockHash" | "prev_block_hash" => Ok(GeneratedField::PrevBlockHash),
>>>>>>> superfluffy/forma-restart-logic
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
            type Value = ExecuteBlockRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.ExecuteBlockRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ExecuteBlockRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
<<<<<<< HEAD
                let mut session_id__ = None;
                let mut parent_hash__ = None;
=======
                let mut prev_block_hash__ = None;
>>>>>>> superfluffy/forma-restart-logic
                let mut transactions__ = None;
                let mut timestamp__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
<<<<<<< HEAD
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
=======
                        GeneratedField::PrevBlockHash => {
                            if prev_block_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("prevBlockHash"));
                            }
                            prev_block_hash__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
>>>>>>> superfluffy/forma-restart-logic
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
                Ok(ExecuteBlockRequest {
<<<<<<< HEAD
                    session_id: session_id__.unwrap_or_default(),
                    parent_hash: parent_hash__.unwrap_or_default(),
=======
                    prev_block_hash: prev_block_hash__.unwrap_or_default(),
>>>>>>> superfluffy/forma-restart-logic
                    transactions: transactions__.unwrap_or_default(),
                    timestamp: timestamp__,
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.ExecuteBlockRequest", FIELDS, GeneratedVisitor)
    }
}
<<<<<<< HEAD
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
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.ExecutedBlockMetadata", len)?;
        if self.number != 0 {
            #[allow(clippy::needless_borrow)]
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
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Number,
            Hash,
            ParentHash,
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
                            "number" => Ok(GeneratedField::Number),
                            "hash" => Ok(GeneratedField::Hash),
                            "parentHash" | "parent_hash" => Ok(GeneratedField::ParentHash),
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
                    }
                }
                Ok(ExecutedBlockMetadata {
                    number: number__.unwrap_or_default(),
                    hash: hash__.unwrap_or_default(),
                    parent_hash: parent_hash__.unwrap_or_default(),
                    timestamp: timestamp__,
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
=======
impl serde::Serialize for GenesisInfo {
>>>>>>> superfluffy/forma-restart-logic
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
<<<<<<< HEAD
        if self.rollup_start_block_number != 0 {
            len += 1;
        }
        if self.rollup_end_block_number != 0 {
=======
        if self.sequencer_start_height != 0 {
            len += 1;
        }
        if self.celestia_block_variance != 0 {
            len += 1;
        }
        if self.rollup_start_block_number != 0 {
            len += 1;
        }
        if self.rollup_stop_block_number != 0 {
>>>>>>> superfluffy/forma-restart-logic
            len += 1;
        }
        if !self.sequencer_chain_id.is_empty() {
            len += 1;
        }
<<<<<<< HEAD
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
=======
        if !self.celestia_chain_id.is_empty() {
            len += 1;
        }
        if self.halt_at_rollup_stop_number {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.GenesisInfo", len)?;
        if let Some(v) = self.rollup_id.as_ref() {
            struct_ser.serialize_field("rollupId", v)?;
        }
        if self.sequencer_start_height != 0 {
            struct_ser.serialize_field("sequencerStartHeight", &self.sequencer_start_height)?;
        }
        if self.celestia_block_variance != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("celestiaBlockVariance", ToString::to_string(&self.celestia_block_variance).as_str())?;
        }
>>>>>>> superfluffy/forma-restart-logic
        if self.rollup_start_block_number != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("rollupStartBlockNumber", ToString::to_string(&self.rollup_start_block_number).as_str())?;
        }
<<<<<<< HEAD
        if self.rollup_end_block_number != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("rollupEndBlockNumber", ToString::to_string(&self.rollup_end_block_number).as_str())?;
=======
        if self.rollup_stop_block_number != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("rollupStopBlockNumber", ToString::to_string(&self.rollup_stop_block_number).as_str())?;
>>>>>>> superfluffy/forma-restart-logic
        }
        if !self.sequencer_chain_id.is_empty() {
            struct_ser.serialize_field("sequencerChainId", &self.sequencer_chain_id)?;
        }
<<<<<<< HEAD
        if self.sequencer_start_block_height != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("sequencerStartBlockHeight", ToString::to_string(&self.sequencer_start_block_height).as_str())?;
        }
        if !self.celestia_chain_id.is_empty() {
            struct_ser.serialize_field("celestiaChainId", &self.celestia_chain_id)?;
        }
        if self.celestia_search_height_max_look_ahead != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("celestiaSearchHeightMaxLookAhead", ToString::to_string(&self.celestia_search_height_max_look_ahead).as_str())?;
=======
        if !self.celestia_chain_id.is_empty() {
            struct_ser.serialize_field("celestiaChainId", &self.celestia_chain_id)?;
        }
        if self.halt_at_rollup_stop_number {
            struct_ser.serialize_field("haltAtRollupStopNumber", &self.halt_at_rollup_stop_number)?;
>>>>>>> superfluffy/forma-restart-logic
        }
        struct_ser.end()
    }
}
<<<<<<< HEAD
impl<'de> serde::Deserialize<'de> for ExecutionSessionParameters {
=======
impl<'de> serde::Deserialize<'de> for GenesisInfo {
>>>>>>> superfluffy/forma-restart-logic
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "rollup_id",
            "rollupId",
<<<<<<< HEAD
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
=======
            "sequencer_start_height",
            "sequencerStartHeight",
            "celestia_block_variance",
            "celestiaBlockVariance",
            "rollup_start_block_number",
            "rollupStartBlockNumber",
            "rollup_stop_block_number",
            "rollupStopBlockNumber",
            "sequencer_chain_id",
            "sequencerChainId",
            "celestia_chain_id",
            "celestiaChainId",
            "halt_at_rollup_stop_number",
            "haltAtRollupStopNumber",
>>>>>>> superfluffy/forma-restart-logic
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RollupId,
<<<<<<< HEAD
            RollupStartBlockNumber,
            RollupEndBlockNumber,
            SequencerChainId,
            SequencerStartBlockHeight,
            CelestiaChainId,
            CelestiaSearchHeightMaxLookAhead,
=======
            SequencerStartHeight,
            CelestiaBlockVariance,
            RollupStartBlockNumber,
            RollupStopBlockNumber,
            SequencerChainId,
            CelestiaChainId,
            HaltAtRollupStopNumber,
>>>>>>> superfluffy/forma-restart-logic
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
<<<<<<< HEAD
                            "rollupStartBlockNumber" | "rollup_start_block_number" => Ok(GeneratedField::RollupStartBlockNumber),
                            "rollupEndBlockNumber" | "rollup_end_block_number" => Ok(GeneratedField::RollupEndBlockNumber),
                            "sequencerChainId" | "sequencer_chain_id" => Ok(GeneratedField::SequencerChainId),
                            "sequencerStartBlockHeight" | "sequencer_start_block_height" => Ok(GeneratedField::SequencerStartBlockHeight),
                            "celestiaChainId" | "celestia_chain_id" => Ok(GeneratedField::CelestiaChainId),
                            "celestiaSearchHeightMaxLookAhead" | "celestia_search_height_max_look_ahead" => Ok(GeneratedField::CelestiaSearchHeightMaxLookAhead),
=======
                            "sequencerStartHeight" | "sequencer_start_height" => Ok(GeneratedField::SequencerStartHeight),
                            "celestiaBlockVariance" | "celestia_block_variance" => Ok(GeneratedField::CelestiaBlockVariance),
                            "rollupStartBlockNumber" | "rollup_start_block_number" => Ok(GeneratedField::RollupStartBlockNumber),
                            "rollupStopBlockNumber" | "rollup_stop_block_number" => Ok(GeneratedField::RollupStopBlockNumber),
                            "sequencerChainId" | "sequencer_chain_id" => Ok(GeneratedField::SequencerChainId),
                            "celestiaChainId" | "celestia_chain_id" => Ok(GeneratedField::CelestiaChainId),
                            "haltAtRollupStopNumber" | "halt_at_rollup_stop_number" => Ok(GeneratedField::HaltAtRollupStopNumber),
>>>>>>> superfluffy/forma-restart-logic
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
<<<<<<< HEAD
            type Value = ExecutionSessionParameters;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.ExecutionSessionParameters")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ExecutionSessionParameters, V::Error>
=======
            type Value = GenesisInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.GenesisInfo")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GenesisInfo, V::Error>
>>>>>>> superfluffy/forma-restart-logic
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut rollup_id__ = None;
<<<<<<< HEAD
                let mut rollup_start_block_number__ = None;
                let mut rollup_end_block_number__ = None;
                let mut sequencer_chain_id__ = None;
                let mut sequencer_start_block_height__ = None;
                let mut celestia_chain_id__ = None;
                let mut celestia_search_height_max_look_ahead__ = None;
=======
                let mut sequencer_start_height__ = None;
                let mut celestia_block_variance__ = None;
                let mut rollup_start_block_number__ = None;
                let mut rollup_stop_block_number__ = None;
                let mut sequencer_chain_id__ = None;
                let mut celestia_chain_id__ = None;
                let mut halt_at_rollup_stop_number__ = None;
>>>>>>> superfluffy/forma-restart-logic
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RollupId => {
                            if rollup_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupId"));
                            }
                            rollup_id__ = map_.next_value()?;
                        }
<<<<<<< HEAD
=======
                        GeneratedField::SequencerStartHeight => {
                            if sequencer_start_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequencerStartHeight"));
                            }
                            sequencer_start_height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::CelestiaBlockVariance => {
                            if celestia_block_variance__.is_some() {
                                return Err(serde::de::Error::duplicate_field("celestiaBlockVariance"));
                            }
                            celestia_block_variance__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
>>>>>>> superfluffy/forma-restart-logic
                        GeneratedField::RollupStartBlockNumber => {
                            if rollup_start_block_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupStartBlockNumber"));
                            }
                            rollup_start_block_number__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
<<<<<<< HEAD
                        GeneratedField::RollupEndBlockNumber => {
                            if rollup_end_block_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupEndBlockNumber"));
                            }
                            rollup_end_block_number__ = 
=======
                        GeneratedField::RollupStopBlockNumber => {
                            if rollup_stop_block_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupStopBlockNumber"));
                            }
                            rollup_stop_block_number__ = 
>>>>>>> superfluffy/forma-restart-logic
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::SequencerChainId => {
                            if sequencer_chain_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequencerChainId"));
                            }
                            sequencer_chain_id__ = Some(map_.next_value()?);
                        }
<<<<<<< HEAD
                        GeneratedField::SequencerStartBlockHeight => {
                            if sequencer_start_block_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequencerStartBlockHeight"));
                            }
                            sequencer_start_block_height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
=======
>>>>>>> superfluffy/forma-restart-logic
                        GeneratedField::CelestiaChainId => {
                            if celestia_chain_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("celestiaChainId"));
                            }
                            celestia_chain_id__ = Some(map_.next_value()?);
                        }
<<<<<<< HEAD
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
=======
                        GeneratedField::HaltAtRollupStopNumber => {
                            if halt_at_rollup_stop_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("haltAtRollupStopNumber"));
                            }
                            halt_at_rollup_stop_number__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(GenesisInfo {
                    rollup_id: rollup_id__,
                    sequencer_start_height: sequencer_start_height__.unwrap_or_default(),
                    celestia_block_variance: celestia_block_variance__.unwrap_or_default(),
                    rollup_start_block_number: rollup_start_block_number__.unwrap_or_default(),
                    rollup_stop_block_number: rollup_stop_block_number__.unwrap_or_default(),
                    sequencer_chain_id: sequencer_chain_id__.unwrap_or_default(),
                    celestia_chain_id: celestia_chain_id__.unwrap_or_default(),
                    halt_at_rollup_stop_number: halt_at_rollup_stop_number__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.GenesisInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetBlockRequest {
>>>>>>> superfluffy/forma-restart-logic
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
<<<<<<< HEAD
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.GetExecutedBlockMetadataRequest", len)?;
=======
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.GetBlockRequest", len)?;
>>>>>>> superfluffy/forma-restart-logic
        if let Some(v) = self.identifier.as_ref() {
            struct_ser.serialize_field("identifier", v)?;
        }
        struct_ser.end()
    }
}
<<<<<<< HEAD
impl<'de> serde::Deserialize<'de> for GetExecutedBlockMetadataRequest {
=======
impl<'de> serde::Deserialize<'de> for GetBlockRequest {
>>>>>>> superfluffy/forma-restart-logic
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
<<<<<<< HEAD
            type Value = GetExecutedBlockMetadataRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.GetExecutedBlockMetadataRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetExecutedBlockMetadataRequest, V::Error>
=======
            type Value = GetBlockRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.GetBlockRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetBlockRequest, V::Error>
>>>>>>> superfluffy/forma-restart-logic
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
<<<<<<< HEAD
                Ok(GetExecutedBlockMetadataRequest {
=======
                Ok(GetBlockRequest {
>>>>>>> superfluffy/forma-restart-logic
                    identifier: identifier__,
                })
            }
        }
<<<<<<< HEAD
        deserializer.deserialize_struct("astria.execution.v2.GetExecutedBlockMetadataRequest", FIELDS, GeneratedVisitor)
=======
        deserializer.deserialize_struct("astria.execution.v2.GetBlockRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetCommitmentStateRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("astria.execution.v2.GetCommitmentStateRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetCommitmentStateRequest {
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
            type Value = GetCommitmentStateRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.GetCommitmentStateRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetCommitmentStateRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetCommitmentStateRequest {
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.GetCommitmentStateRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetGenesisInfoRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("astria.execution.v2.GetGenesisInfoRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetGenesisInfoRequest {
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
            type Value = GetGenesisInfoRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.GetGenesisInfoRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetGenesisInfoRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetGenesisInfoRequest {
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.GetGenesisInfoRequest", FIELDS, GeneratedVisitor)
>>>>>>> superfluffy/forma-restart-logic
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
<<<<<<< HEAD
        if !self.session_id.is_empty() {
            len += 1;
        }
=======
>>>>>>> superfluffy/forma-restart-logic
        if self.commitment_state.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.UpdateCommitmentStateRequest", len)?;
<<<<<<< HEAD
        if !self.session_id.is_empty() {
            struct_ser.serialize_field("sessionId", &self.session_id)?;
        }
=======
>>>>>>> superfluffy/forma-restart-logic
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
<<<<<<< HEAD
            "session_id",
            "sessionId",
=======
>>>>>>> superfluffy/forma-restart-logic
            "commitment_state",
            "commitmentState",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
<<<<<<< HEAD
            SessionId,
=======
>>>>>>> superfluffy/forma-restart-logic
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
<<<<<<< HEAD
                            "sessionId" | "session_id" => Ok(GeneratedField::SessionId),
=======
>>>>>>> superfluffy/forma-restart-logic
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
<<<<<<< HEAD
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
=======
                let mut commitment_state__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
>>>>>>> superfluffy/forma-restart-logic
                        GeneratedField::CommitmentState => {
                            if commitment_state__.is_some() {
                                return Err(serde::de::Error::duplicate_field("commitmentState"));
                            }
                            commitment_state__ = map_.next_value()?;
                        }
                    }
                }
                Ok(UpdateCommitmentStateRequest {
<<<<<<< HEAD
                    session_id: session_id__.unwrap_or_default(),
=======
>>>>>>> superfluffy/forma-restart-logic
                    commitment_state: commitment_state__,
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.UpdateCommitmentStateRequest", FIELDS, GeneratedVisitor)
    }
}
