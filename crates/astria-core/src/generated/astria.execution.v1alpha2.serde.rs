impl serde::Serialize for BatchGetBlocksRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.identifiers.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v1alpha2.BatchGetBlocksRequest", len)?;
        if !self.identifiers.is_empty() {
            struct_ser.serialize_field("identifiers", &self.identifiers)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BatchGetBlocksRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "identifiers",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Identifiers,
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
                            "identifiers" => Ok(GeneratedField::Identifiers),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BatchGetBlocksRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v1alpha2.BatchGetBlocksRequest")
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
        deserializer.deserialize_struct("astria.execution.v1alpha2.BatchGetBlocksRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BatchGetBlocksResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.blocks.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v1alpha2.BatchGetBlocksResponse", len)?;
        if !self.blocks.is_empty() {
            struct_ser.serialize_field("blocks", &self.blocks)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BatchGetBlocksResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "blocks",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Blocks,
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
                            "blocks" => Ok(GeneratedField::Blocks),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BatchGetBlocksResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v1alpha2.BatchGetBlocksResponse")
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
        deserializer.deserialize_struct("astria.execution.v1alpha2.BatchGetBlocksResponse", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.execution.v1alpha2.Block", len)?;
        if self.number != 0 {
            struct_ser.serialize_field("number", &self.number)?;
        }
        if !self.hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("hash", pbjson::private::base64::encode(&self.hash).as_str())?;
        }
        if !self.parent_block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("parent_block_hash", pbjson::private::base64::encode(&self.parent_block_hash).as_str())?;
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
                formatter.write_str("struct astria.execution.v1alpha2.Block")
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
        deserializer.deserialize_struct("astria.execution.v1alpha2.Block", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.execution.v1alpha2.BlockIdentifier", len)?;
        if let Some(v) = self.identifier.as_ref() {
            match v {
                block_identifier::Identifier::BlockNumber(v) => {
                    struct_ser.serialize_field("block_number", v)?;
                }
                block_identifier::Identifier::BlockHash(v) => {
                    #[allow(clippy::needless_borrow)]
                    struct_ser.serialize_field("block_hash", pbjson::private::base64::encode(&v).as_str())?;
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
                formatter.write_str("struct astria.execution.v1alpha2.BlockIdentifier")
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
        deserializer.deserialize_struct("astria.execution.v1alpha2.BlockIdentifier", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.execution.v1alpha2.CommitmentState", len)?;
        if let Some(v) = self.soft.as_ref() {
            struct_ser.serialize_field("soft", v)?;
        }
        if let Some(v) = self.firm.as_ref() {
            struct_ser.serialize_field("firm", v)?;
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
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Soft,
            Firm,
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
                formatter.write_str("struct astria.execution.v1alpha2.CommitmentState")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CommitmentState, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut soft__ = None;
                let mut firm__ = None;
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
                    }
                }
                Ok(CommitmentState {
                    soft: soft__,
                    firm: firm__,
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v1alpha2.CommitmentState", FIELDS, GeneratedVisitor)
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
        if !self.prev_block_hash.is_empty() {
            len += 1;
        }
        if !self.transactions.is_empty() {
            len += 1;
        }
        if self.timestamp.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v1alpha2.ExecuteBlockRequest", len)?;
        if !self.prev_block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("prev_block_hash", pbjson::private::base64::encode(&self.prev_block_hash).as_str())?;
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
            "prev_block_hash",
            "prevBlockHash",
            "transactions",
            "timestamp",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            PrevBlockHash,
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
                            "prevBlockHash" | "prev_block_hash" => Ok(GeneratedField::PrevBlockHash),
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
                formatter.write_str("struct astria.execution.v1alpha2.ExecuteBlockRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ExecuteBlockRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut prev_block_hash__ = None;
                let mut transactions__ = None;
                let mut timestamp__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::PrevBlockHash => {
                            if prev_block_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("prevBlockHash"));
                            }
                            prev_block_hash__ = 
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
                Ok(ExecuteBlockRequest {
                    prev_block_hash: prev_block_hash__.unwrap_or_default(),
                    transactions: transactions__.unwrap_or_default(),
                    timestamp: timestamp__,
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v1alpha2.ExecuteBlockRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GenesisInfo {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.rollup_id.is_empty() {
            len += 1;
        }
        if self.sequencer_genesis_block_height != 0 {
            len += 1;
        }
        if self.celestia_base_block_height != 0 {
            len += 1;
        }
        if self.celestia_block_variance != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v1alpha2.GenesisInfo", len)?;
        if !self.rollup_id.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("rollup_id", pbjson::private::base64::encode(&self.rollup_id).as_str())?;
        }
        if self.sequencer_genesis_block_height != 0 {
            struct_ser.serialize_field("sequencer_genesis_block_height", &self.sequencer_genesis_block_height)?;
        }
        if self.celestia_base_block_height != 0 {
            struct_ser.serialize_field("celestia_base_block_height", &self.celestia_base_block_height)?;
        }
        if self.celestia_block_variance != 0 {
            struct_ser.serialize_field("celestia_block_variance", &self.celestia_block_variance)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GenesisInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "rollup_id",
            "rollupId",
            "sequencer_genesis_block_height",
            "sequencerGenesisBlockHeight",
            "celestia_base_block_height",
            "celestiaBaseBlockHeight",
            "celestia_block_variance",
            "celestiaBlockVariance",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RollupId,
            SequencerGenesisBlockHeight,
            CelestiaBaseBlockHeight,
            CelestiaBlockVariance,
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
                            "sequencerGenesisBlockHeight" | "sequencer_genesis_block_height" => Ok(GeneratedField::SequencerGenesisBlockHeight),
                            "celestiaBaseBlockHeight" | "celestia_base_block_height" => Ok(GeneratedField::CelestiaBaseBlockHeight),
                            "celestiaBlockVariance" | "celestia_block_variance" => Ok(GeneratedField::CelestiaBlockVariance),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GenesisInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v1alpha2.GenesisInfo")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GenesisInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut rollup_id__ = None;
                let mut sequencer_genesis_block_height__ = None;
                let mut celestia_base_block_height__ = None;
                let mut celestia_block_variance__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RollupId => {
                            if rollup_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupId"));
                            }
                            rollup_id__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::SequencerGenesisBlockHeight => {
                            if sequencer_genesis_block_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequencerGenesisBlockHeight"));
                            }
                            sequencer_genesis_block_height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::CelestiaBaseBlockHeight => {
                            if celestia_base_block_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("celestiaBaseBlockHeight"));
                            }
                            celestia_base_block_height__ = 
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
                    }
                }
                Ok(GenesisInfo {
                    rollup_id: rollup_id__.unwrap_or_default(),
                    sequencer_genesis_block_height: sequencer_genesis_block_height__.unwrap_or_default(),
                    celestia_base_block_height: celestia_base_block_height__.unwrap_or_default(),
                    celestia_block_variance: celestia_block_variance__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v1alpha2.GenesisInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetBlockRequest {
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
        let mut struct_ser = serializer.serialize_struct("astria.execution.v1alpha2.GetBlockRequest", len)?;
        if let Some(v) = self.identifier.as_ref() {
            struct_ser.serialize_field("identifier", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetBlockRequest {
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
            type Value = GetBlockRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v1alpha2.GetBlockRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetBlockRequest, V::Error>
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
                Ok(GetBlockRequest {
                    identifier: identifier__,
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v1alpha2.GetBlockRequest", FIELDS, GeneratedVisitor)
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
        let struct_ser = serializer.serialize_struct("astria.execution.v1alpha2.GetCommitmentStateRequest", len)?;
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
                formatter.write_str("struct astria.execution.v1alpha2.GetCommitmentStateRequest")
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
        deserializer.deserialize_struct("astria.execution.v1alpha2.GetCommitmentStateRequest", FIELDS, GeneratedVisitor)
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
        let struct_ser = serializer.serialize_struct("astria.execution.v1alpha2.GetGenesisInfoRequest", len)?;
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
                formatter.write_str("struct astria.execution.v1alpha2.GetGenesisInfoRequest")
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
        deserializer.deserialize_struct("astria.execution.v1alpha2.GetGenesisInfoRequest", FIELDS, GeneratedVisitor)
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
        if self.commitment_state.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v1alpha2.UpdateCommitmentStateRequest", len)?;
        if let Some(v) = self.commitment_state.as_ref() {
            struct_ser.serialize_field("commitment_state", v)?;
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
            "commitment_state",
            "commitmentState",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
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
                formatter.write_str("struct astria.execution.v1alpha2.UpdateCommitmentStateRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<UpdateCommitmentStateRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut commitment_state__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::CommitmentState => {
                            if commitment_state__.is_some() {
                                return Err(serde::de::Error::duplicate_field("commitmentState"));
                            }
                            commitment_state__ = map_.next_value()?;
                        }
                    }
                }
                Ok(UpdateCommitmentStateRequest {
                    commitment_state: commitment_state__,
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v1alpha2.UpdateCommitmentStateRequest", FIELDS, GeneratedVisitor)
    }
}
