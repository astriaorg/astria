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
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("number", ToString::to_string(&self.number).as_str())?;
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
                block_identifier::Identifier::Number(v) => {
                    #[allow(clippy::needless_borrow)]
                    struct_ser.serialize_field("number", ToString::to_string(&v).as_str())?;
                }
                block_identifier::Identifier::Hash(v) => {
                    #[allow(clippy::needless_borrow)]
                    struct_ser.serialize_field("hash", pbjson::private::base64::encode(&v).as_str())?;
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
                        GeneratedField::Number => {
                            if identifier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("number"));
                            }
                            identifier__ = map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| block_identifier::Identifier::Number(x.0));
                        }
                        GeneratedField::Hash => {
                            if identifier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            identifier__ = map_.next_value::<::std::option::Option<::pbjson::private::BytesDeserialize<_>>>()?.map(|x| block_identifier::Identifier::Hash(x.0));
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
        if !self.prev_block_hash.is_empty() {
            len += 1;
        }
        if !self.transactions.is_empty() {
            len += 1;
        }
        if self.timestamp.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.ExecuteBlockRequest", len)?;
        if !self.session_id.is_empty() {
            struct_ser.serialize_field("sessionId", &self.session_id)?;
        }
        if !self.prev_block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("prevBlockHash", pbjson::private::base64::encode(&self.prev_block_hash).as_str())?;
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
            "session_id",
            "sessionId",
            "prev_block_hash",
            "prevBlockHash",
            "transactions",
            "timestamp",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SessionId,
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
                            "sessionId" | "session_id" => Ok(GeneratedField::SessionId),
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
                formatter.write_str("struct astria.execution.v2.ExecuteBlockRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ExecuteBlockRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut session_id__ = None;
                let mut prev_block_hash__ = None;
                let mut transactions__ = None;
                let mut timestamp__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::SessionId => {
                            if session_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sessionId"));
                            }
                            session_id__ = Some(map_.next_value()?);
                        }
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
                    session_id: session_id__.unwrap_or_default(),
                    prev_block_hash: prev_block_hash__.unwrap_or_default(),
                    transactions: transactions__.unwrap_or_default(),
                    timestamp: timestamp__,
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.ExecuteBlockRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ExecutionConfig {
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
        if self.rollup_first_block_number != 0 {
            len += 1;
        }
        if self.rollup_stop_block_number != 0 {
            len += 1;
        }
        if !self.sequencer_chain_id.is_empty() {
            len += 1;
        }
        if self.sequencer_first_block_height != 0 {
            len += 1;
        }
        if !self.celestia_chain_id.is_empty() {
            len += 1;
        }
        if self.celestia_block_variance != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.ExecutionConfig", len)?;
        if let Some(v) = self.rollup_id.as_ref() {
            struct_ser.serialize_field("rollupId", v)?;
        }
        if self.rollup_first_block_number != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("rollupFirstBlockNumber", ToString::to_string(&self.rollup_first_block_number).as_str())?;
        }
        if self.rollup_stop_block_number != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("rollupStopBlockNumber", ToString::to_string(&self.rollup_stop_block_number).as_str())?;
        }
        if !self.sequencer_chain_id.is_empty() {
            struct_ser.serialize_field("sequencerChainId", &self.sequencer_chain_id)?;
        }
        if self.sequencer_first_block_height != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("sequencerFirstBlockHeight", ToString::to_string(&self.sequencer_first_block_height).as_str())?;
        }
        if !self.celestia_chain_id.is_empty() {
            struct_ser.serialize_field("celestiaChainId", &self.celestia_chain_id)?;
        }
        if self.celestia_block_variance != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("celestiaBlockVariance", ToString::to_string(&self.celestia_block_variance).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ExecutionConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "rollup_id",
            "rollupId",
            "rollup_first_block_number",
            "rollupFirstBlockNumber",
            "rollup_stop_block_number",
            "rollupStopBlockNumber",
            "sequencer_chain_id",
            "sequencerChainId",
            "sequencer_first_block_height",
            "sequencerFirstBlockHeight",
            "celestia_chain_id",
            "celestiaChainId",
            "celestia_block_variance",
            "celestiaBlockVariance",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RollupId,
            RollupFirstBlockNumber,
            RollupStopBlockNumber,
            SequencerChainId,
            SequencerFirstBlockHeight,
            CelestiaChainId,
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
                            "rollupFirstBlockNumber" | "rollup_first_block_number" => Ok(GeneratedField::RollupFirstBlockNumber),
                            "rollupStopBlockNumber" | "rollup_stop_block_number" => Ok(GeneratedField::RollupStopBlockNumber),
                            "sequencerChainId" | "sequencer_chain_id" => Ok(GeneratedField::SequencerChainId),
                            "sequencerFirstBlockHeight" | "sequencer_first_block_height" => Ok(GeneratedField::SequencerFirstBlockHeight),
                            "celestiaChainId" | "celestia_chain_id" => Ok(GeneratedField::CelestiaChainId),
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
            type Value = ExecutionConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.ExecutionConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ExecutionConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut rollup_id__ = None;
                let mut rollup_first_block_number__ = None;
                let mut rollup_stop_block_number__ = None;
                let mut sequencer_chain_id__ = None;
                let mut sequencer_first_block_height__ = None;
                let mut celestia_chain_id__ = None;
                let mut celestia_block_variance__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RollupId => {
                            if rollup_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupId"));
                            }
                            rollup_id__ = map_.next_value()?;
                        }
                        GeneratedField::RollupFirstBlockNumber => {
                            if rollup_first_block_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupFirstBlockNumber"));
                            }
                            rollup_first_block_number__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::RollupStopBlockNumber => {
                            if rollup_stop_block_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupStopBlockNumber"));
                            }
                            rollup_stop_block_number__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::SequencerChainId => {
                            if sequencer_chain_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequencerChainId"));
                            }
                            sequencer_chain_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::SequencerFirstBlockHeight => {
                            if sequencer_first_block_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequencerFirstBlockHeight"));
                            }
                            sequencer_first_block_height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::CelestiaChainId => {
                            if celestia_chain_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("celestiaChainId"));
                            }
                            celestia_chain_id__ = Some(map_.next_value()?);
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
                Ok(ExecutionConfig {
                    rollup_id: rollup_id__,
                    rollup_first_block_number: rollup_first_block_number__.unwrap_or_default(),
                    rollup_stop_block_number: rollup_stop_block_number__.unwrap_or_default(),
                    sequencer_chain_id: sequencer_chain_id__.unwrap_or_default(),
                    sequencer_first_block_height: sequencer_first_block_height__.unwrap_or_default(),
                    celestia_chain_id: celestia_chain_id__.unwrap_or_default(),
                    celestia_block_variance: celestia_block_variance__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.ExecutionConfig", FIELDS, GeneratedVisitor)
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
        if self.commitment_state.is_some() {
            len += 1;
        }
        if self.execution_config.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.ExecutionSession", len)?;
        if !self.session_id.is_empty() {
            struct_ser.serialize_field("sessionId", &self.session_id)?;
        }
        if let Some(v) = self.commitment_state.as_ref() {
            struct_ser.serialize_field("commitmentState", v)?;
        }
        if let Some(v) = self.execution_config.as_ref() {
            struct_ser.serialize_field("executionConfig", v)?;
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
            "commitment_state",
            "commitmentState",
            "execution_config",
            "executionConfig",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SessionId,
            CommitmentState,
            ExecutionConfig,
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
                            "executionConfig" | "execution_config" => Ok(GeneratedField::ExecutionConfig),
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
                let mut commitment_state__ = None;
                let mut execution_config__ = None;
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
                        GeneratedField::ExecutionConfig => {
                            if execution_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("executionConfig"));
                            }
                            execution_config__ = map_.next_value()?;
                        }
                    }
                }
                Ok(ExecutionSession {
                    session_id: session_id__.unwrap_or_default(),
                    commitment_state: commitment_state__,
                    execution_config: execution_config__,
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.ExecutionSession", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.execution.v2.GetBlockRequest", len)?;
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
                formatter.write_str("struct astria.execution.v2.GetBlockRequest")
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
impl serde::Serialize for NewExecutionSessionRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("astria.execution.v2.NewExecutionSessionRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for NewExecutionSessionRequest {
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
            type Value = NewExecutionSessionRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.execution.v2.NewExecutionSessionRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<NewExecutionSessionRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(NewExecutionSessionRequest {
                })
            }
        }
        deserializer.deserialize_struct("astria.execution.v2.NewExecutionSessionRequest", FIELDS, GeneratedVisitor)
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
