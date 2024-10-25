impl serde::Serialize for Deposit {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.bridge_address.is_some() {
            len += 1;
        }
        if self.rollup_id.is_some() {
            len += 1;
        }
        if self.amount.is_some() {
            len += 1;
        }
        if !self.asset.is_empty() {
            len += 1;
        }
        if !self.destination_chain_address.is_empty() {
            len += 1;
        }
        if self.source_transaction_id.is_some() {
            len += 1;
        }
        if self.source_action_index != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.Deposit", len)?;
        if let Some(v) = self.bridge_address.as_ref() {
            struct_ser.serialize_field("bridgeAddress", v)?;
        }
        if let Some(v) = self.rollup_id.as_ref() {
            struct_ser.serialize_field("rollupId", v)?;
        }
        if let Some(v) = self.amount.as_ref() {
            struct_ser.serialize_field("amount", v)?;
        }
        if !self.asset.is_empty() {
            struct_ser.serialize_field("asset", &self.asset)?;
        }
        if !self.destination_chain_address.is_empty() {
            struct_ser.serialize_field("destinationChainAddress", &self.destination_chain_address)?;
        }
        if let Some(v) = self.source_transaction_id.as_ref() {
            struct_ser.serialize_field("sourceTransactionId", v)?;
        }
        if self.source_action_index != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("sourceActionIndex", ToString::to_string(&self.source_action_index).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Deposit {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "bridge_address",
            "bridgeAddress",
            "rollup_id",
            "rollupId",
            "amount",
            "asset",
            "destination_chain_address",
            "destinationChainAddress",
            "source_transaction_id",
            "sourceTransactionId",
            "source_action_index",
            "sourceActionIndex",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BridgeAddress,
            RollupId,
            Amount,
            Asset,
            DestinationChainAddress,
            SourceTransactionId,
            SourceActionIndex,
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
                            "bridgeAddress" | "bridge_address" => Ok(GeneratedField::BridgeAddress),
                            "rollupId" | "rollup_id" => Ok(GeneratedField::RollupId),
                            "amount" => Ok(GeneratedField::Amount),
                            "asset" => Ok(GeneratedField::Asset),
                            "destinationChainAddress" | "destination_chain_address" => Ok(GeneratedField::DestinationChainAddress),
                            "sourceTransactionId" | "source_transaction_id" => Ok(GeneratedField::SourceTransactionId),
                            "sourceActionIndex" | "source_action_index" => Ok(GeneratedField::SourceActionIndex),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Deposit;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1alpha1.Deposit")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Deposit, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut bridge_address__ = None;
                let mut rollup_id__ = None;
                let mut amount__ = None;
                let mut asset__ = None;
                let mut destination_chain_address__ = None;
                let mut source_transaction_id__ = None;
                let mut source_action_index__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BridgeAddress => {
                            if bridge_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeAddress"));
                            }
                            bridge_address__ = map_.next_value()?;
                        }
                        GeneratedField::RollupId => {
                            if rollup_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupId"));
                            }
                            rollup_id__ = map_.next_value()?;
                        }
                        GeneratedField::Amount => {
                            if amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("amount"));
                            }
                            amount__ = map_.next_value()?;
                        }
                        GeneratedField::Asset => {
                            if asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("asset"));
                            }
                            asset__ = Some(map_.next_value()?);
                        }
                        GeneratedField::DestinationChainAddress => {
                            if destination_chain_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("destinationChainAddress"));
                            }
                            destination_chain_address__ = Some(map_.next_value()?);
                        }
                        GeneratedField::SourceTransactionId => {
                            if source_transaction_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sourceTransactionId"));
                            }
                            source_transaction_id__ = map_.next_value()?;
                        }
                        GeneratedField::SourceActionIndex => {
                            if source_action_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sourceActionIndex"));
                            }
                            source_action_index__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Deposit {
                    bridge_address: bridge_address__,
                    rollup_id: rollup_id__,
                    amount: amount__,
                    asset: asset__.unwrap_or_default(),
                    destination_chain_address: destination_chain_address__.unwrap_or_default(),
                    source_transaction_id: source_transaction_id__,
                    source_action_index: source_action_index__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.Deposit", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for FilteredSequencerBlock {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.block_hash.is_empty() {
            len += 1;
        }
        if self.header.is_some() {
            len += 1;
        }
        if !self.rollup_transactions.is_empty() {
            len += 1;
        }
        if self.rollup_transactions_proof.is_some() {
            len += 1;
        }
        if !self.all_rollup_ids.is_empty() {
            len += 1;
        }
        if self.rollup_ids_proof.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.FilteredSequencerBlock", len)?;
        if !self.block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("blockHash", pbjson::private::base64::encode(&self.block_hash).as_str())?;
        }
        if let Some(v) = self.header.as_ref() {
            struct_ser.serialize_field("header", v)?;
        }
        if !self.rollup_transactions.is_empty() {
            struct_ser.serialize_field("rollupTransactions", &self.rollup_transactions)?;
        }
        if let Some(v) = self.rollup_transactions_proof.as_ref() {
            struct_ser.serialize_field("rollupTransactionsProof", v)?;
        }
        if !self.all_rollup_ids.is_empty() {
            struct_ser.serialize_field("allRollupIds", &self.all_rollup_ids)?;
        }
        if let Some(v) = self.rollup_ids_proof.as_ref() {
            struct_ser.serialize_field("rollupIdsProof", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FilteredSequencerBlock {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "block_hash",
            "blockHash",
            "header",
            "rollup_transactions",
            "rollupTransactions",
            "rollup_transactions_proof",
            "rollupTransactionsProof",
            "all_rollup_ids",
            "allRollupIds",
            "rollup_ids_proof",
            "rollupIdsProof",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BlockHash,
            Header,
            RollupTransactions,
            RollupTransactionsProof,
            AllRollupIds,
            RollupIdsProof,
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
                            "blockHash" | "block_hash" => Ok(GeneratedField::BlockHash),
                            "header" => Ok(GeneratedField::Header),
                            "rollupTransactions" | "rollup_transactions" => Ok(GeneratedField::RollupTransactions),
                            "rollupTransactionsProof" | "rollup_transactions_proof" => Ok(GeneratedField::RollupTransactionsProof),
                            "allRollupIds" | "all_rollup_ids" => Ok(GeneratedField::AllRollupIds),
                            "rollupIdsProof" | "rollup_ids_proof" => Ok(GeneratedField::RollupIdsProof),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FilteredSequencerBlock;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1alpha1.FilteredSequencerBlock")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<FilteredSequencerBlock, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut block_hash__ = None;
                let mut header__ = None;
                let mut rollup_transactions__ = None;
                let mut rollup_transactions_proof__ = None;
                let mut all_rollup_ids__ = None;
                let mut rollup_ids_proof__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BlockHash => {
                            if block_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHash"));
                            }
                            block_hash__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Header => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("header"));
                            }
                            header__ = map_.next_value()?;
                        }
                        GeneratedField::RollupTransactions => {
                            if rollup_transactions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupTransactions"));
                            }
                            rollup_transactions__ = Some(map_.next_value()?);
                        }
                        GeneratedField::RollupTransactionsProof => {
                            if rollup_transactions_proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupTransactionsProof"));
                            }
                            rollup_transactions_proof__ = map_.next_value()?;
                        }
                        GeneratedField::AllRollupIds => {
                            if all_rollup_ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("allRollupIds"));
                            }
                            all_rollup_ids__ = Some(map_.next_value()?);
                        }
                        GeneratedField::RollupIdsProof => {
                            if rollup_ids_proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupIdsProof"));
                            }
                            rollup_ids_proof__ = map_.next_value()?;
                        }
                    }
                }
                Ok(FilteredSequencerBlock {
                    block_hash: block_hash__.unwrap_or_default(),
                    header: header__,
                    rollup_transactions: rollup_transactions__.unwrap_or_default(),
                    rollup_transactions_proof: rollup_transactions_proof__,
                    all_rollup_ids: all_rollup_ids__.unwrap_or_default(),
                    rollup_ids_proof: rollup_ids_proof__,
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.FilteredSequencerBlock", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetBlockCommitmentStreamRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.GetBlockCommitmentStreamRequest", len)?;
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
                formatter.write_str("struct astria.sequencerblock.v1alpha1.GetBlockCommitmentStreamRequest")
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
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.GetBlockCommitmentStreamRequest", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.GetBlockCommitmentStreamResponse", len)?;
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
                formatter.write_str("struct astria.sequencerblock.v1alpha1.GetBlockCommitmentStreamResponse")
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
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.GetBlockCommitmentStreamResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetFilteredSequencerBlockRequest {
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
        if !self.rollup_ids.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.GetFilteredSequencerBlockRequest", len)?;
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        if !self.rollup_ids.is_empty() {
            struct_ser.serialize_field("rollupIds", &self.rollup_ids)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetFilteredSequencerBlockRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "rollup_ids",
            "rollupIds",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            RollupIds,
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
                            "rollupIds" | "rollup_ids" => Ok(GeneratedField::RollupIds),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetFilteredSequencerBlockRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1alpha1.GetFilteredSequencerBlockRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetFilteredSequencerBlockRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut rollup_ids__ = None;
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
                        GeneratedField::RollupIds => {
                            if rollup_ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupIds"));
                            }
                            rollup_ids__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(GetFilteredSequencerBlockRequest {
                    height: height__.unwrap_or_default(),
                    rollup_ids: rollup_ids__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.GetFilteredSequencerBlockRequest", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.GetOptimisticBlockStreamRequest", len)?;
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
                formatter.write_str("struct astria.sequencerblock.v1alpha1.GetOptimisticBlockStreamRequest")
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
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.GetOptimisticBlockStreamRequest", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.GetOptimisticBlockStreamResponse", len)?;
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
                formatter.write_str("struct astria.sequencerblock.v1alpha1.GetOptimisticBlockStreamResponse")
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
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.GetOptimisticBlockStreamResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetPendingNonceRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.address.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.GetPendingNonceRequest", len)?;
        if let Some(v) = self.address.as_ref() {
            struct_ser.serialize_field("address", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetPendingNonceRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "address",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Address,
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
                            "address" => Ok(GeneratedField::Address),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetPendingNonceRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1alpha1.GetPendingNonceRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetPendingNonceRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut address__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Address => {
                            if address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("address"));
                            }
                            address__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GetPendingNonceRequest {
                    address: address__,
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.GetPendingNonceRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetPendingNonceResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.inner != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.GetPendingNonceResponse", len)?;
        if self.inner != 0 {
            struct_ser.serialize_field("inner", &self.inner)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetPendingNonceResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "inner",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Inner,
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
                            "inner" => Ok(GeneratedField::Inner),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetPendingNonceResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1alpha1.GetPendingNonceResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetPendingNonceResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut inner__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Inner => {
                            if inner__.is_some() {
                                return Err(serde::de::Error::duplicate_field("inner"));
                            }
                            inner__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(GetPendingNonceResponse {
                    inner: inner__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.GetPendingNonceResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetSequencerBlockRequest {
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
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.GetSequencerBlockRequest", len)?;
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetSequencerBlockRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
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
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetSequencerBlockRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1alpha1.GetSequencerBlockRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetSequencerBlockRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
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
                    }
                }
                Ok(GetSequencerBlockRequest {
                    height: height__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.GetSequencerBlockRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RollupData {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.value.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.RollupData", len)?;
        if let Some(v) = self.value.as_ref() {
            match v {
                rollup_data::Value::SequencedData(v) => {
                    #[allow(clippy::needless_borrow)]
                    struct_ser.serialize_field("sequencedData", pbjson::private::base64::encode(&v).as_str())?;
                }
                rollup_data::Value::Deposit(v) => {
                    struct_ser.serialize_field("deposit", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RollupData {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "sequenced_data",
            "sequencedData",
            "deposit",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SequencedData,
            Deposit,
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
                            "sequencedData" | "sequenced_data" => Ok(GeneratedField::SequencedData),
                            "deposit" => Ok(GeneratedField::Deposit),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RollupData;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1alpha1.RollupData")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RollupData, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut value__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::SequencedData => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequencedData"));
                            }
                            value__ = map_.next_value::<::std::option::Option<::pbjson::private::BytesDeserialize<_>>>()?.map(|x| rollup_data::Value::SequencedData(x.0));
                        }
                        GeneratedField::Deposit => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("deposit"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(rollup_data::Value::Deposit)
;
                        }
                    }
                }
                Ok(RollupData {
                    value: value__,
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.RollupData", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RollupTransactions {
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
        if !self.transactions.is_empty() {
            len += 1;
        }
        if self.proof.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.RollupTransactions", len)?;
        if let Some(v) = self.rollup_id.as_ref() {
            struct_ser.serialize_field("rollupId", v)?;
        }
        if !self.transactions.is_empty() {
            struct_ser.serialize_field("transactions", &self.transactions.iter().map(pbjson::private::base64::encode).collect::<Vec<_>>())?;
        }
        if let Some(v) = self.proof.as_ref() {
            struct_ser.serialize_field("proof", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RollupTransactions {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "rollup_id",
            "rollupId",
            "transactions",
            "proof",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RollupId,
            Transactions,
            Proof,
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
                            "transactions" => Ok(GeneratedField::Transactions),
                            "proof" => Ok(GeneratedField::Proof),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RollupTransactions;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1alpha1.RollupTransactions")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RollupTransactions, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut rollup_id__ = None;
                let mut transactions__ = None;
                let mut proof__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RollupId => {
                            if rollup_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupId"));
                            }
                            rollup_id__ = map_.next_value()?;
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
                        GeneratedField::Proof => {
                            if proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("proof"));
                            }
                            proof__ = map_.next_value()?;
                        }
                    }
                }
                Ok(RollupTransactions {
                    rollup_id: rollup_id__,
                    transactions: transactions__.unwrap_or_default(),
                    proof: proof__,
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.RollupTransactions", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SequencerBlock {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.header.is_some() {
            len += 1;
        }
        if !self.rollup_transactions.is_empty() {
            len += 1;
        }
        if self.rollup_transactions_proof.is_some() {
            len += 1;
        }
        if self.rollup_ids_proof.is_some() {
            len += 1;
        }
        if !self.block_hash.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.SequencerBlock", len)?;
        if let Some(v) = self.header.as_ref() {
            struct_ser.serialize_field("header", v)?;
        }
        if !self.rollup_transactions.is_empty() {
            struct_ser.serialize_field("rollupTransactions", &self.rollup_transactions)?;
        }
        if let Some(v) = self.rollup_transactions_proof.as_ref() {
            struct_ser.serialize_field("rollupTransactionsProof", v)?;
        }
        if let Some(v) = self.rollup_ids_proof.as_ref() {
            struct_ser.serialize_field("rollupIdsProof", v)?;
        }
        if !self.block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("blockHash", pbjson::private::base64::encode(&self.block_hash).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SequencerBlock {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "header",
            "rollup_transactions",
            "rollupTransactions",
            "rollup_transactions_proof",
            "rollupTransactionsProof",
            "rollup_ids_proof",
            "rollupIdsProof",
            "block_hash",
            "blockHash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Header,
            RollupTransactions,
            RollupTransactionsProof,
            RollupIdsProof,
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
                            "header" => Ok(GeneratedField::Header),
                            "rollupTransactions" | "rollup_transactions" => Ok(GeneratedField::RollupTransactions),
                            "rollupTransactionsProof" | "rollup_transactions_proof" => Ok(GeneratedField::RollupTransactionsProof),
                            "rollupIdsProof" | "rollup_ids_proof" => Ok(GeneratedField::RollupIdsProof),
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
            type Value = SequencerBlock;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1alpha1.SequencerBlock")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SequencerBlock, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut header__ = None;
                let mut rollup_transactions__ = None;
                let mut rollup_transactions_proof__ = None;
                let mut rollup_ids_proof__ = None;
                let mut block_hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Header => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("header"));
                            }
                            header__ = map_.next_value()?;
                        }
                        GeneratedField::RollupTransactions => {
                            if rollup_transactions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupTransactions"));
                            }
                            rollup_transactions__ = Some(map_.next_value()?);
                        }
                        GeneratedField::RollupTransactionsProof => {
                            if rollup_transactions_proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupTransactionsProof"));
                            }
                            rollup_transactions_proof__ = map_.next_value()?;
                        }
                        GeneratedField::RollupIdsProof => {
                            if rollup_ids_proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupIdsProof"));
                            }
                            rollup_ids_proof__ = map_.next_value()?;
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
                Ok(SequencerBlock {
                    header: header__,
                    rollup_transactions: rollup_transactions__.unwrap_or_default(),
                    rollup_transactions_proof: rollup_transactions_proof__,
                    rollup_ids_proof: rollup_ids_proof__,
                    block_hash: block_hash__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.SequencerBlock", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.SequencerBlockCommit", len)?;
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
                formatter.write_str("struct astria.sequencerblock.v1alpha1.SequencerBlockCommit")
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
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.SequencerBlockCommit", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SequencerBlockHeader {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.chain_id.is_empty() {
            len += 1;
        }
        if self.height != 0 {
            len += 1;
        }
        if self.time.is_some() {
            len += 1;
        }
        if !self.data_hash.is_empty() {
            len += 1;
        }
        if !self.proposer_address.is_empty() {
            len += 1;
        }
        if !self.rollup_transactions_root.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.SequencerBlockHeader", len)?;
        if !self.chain_id.is_empty() {
            struct_ser.serialize_field("chainId", &self.chain_id)?;
        }
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        if let Some(v) = self.time.as_ref() {
            struct_ser.serialize_field("time", v)?;
        }
        if !self.data_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("dataHash", pbjson::private::base64::encode(&self.data_hash).as_str())?;
        }
        if !self.proposer_address.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("proposerAddress", pbjson::private::base64::encode(&self.proposer_address).as_str())?;
        }
        if !self.rollup_transactions_root.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("rollupTransactionsRoot", pbjson::private::base64::encode(&self.rollup_transactions_root).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SequencerBlockHeader {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "chain_id",
            "chainId",
            "height",
            "time",
            "data_hash",
            "dataHash",
            "proposer_address",
            "proposerAddress",
            "rollup_transactions_root",
            "rollupTransactionsRoot",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ChainId,
            Height,
            Time,
            DataHash,
            ProposerAddress,
            RollupTransactionsRoot,
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
                            "chainId" | "chain_id" => Ok(GeneratedField::ChainId),
                            "height" => Ok(GeneratedField::Height),
                            "time" => Ok(GeneratedField::Time),
                            "dataHash" | "data_hash" => Ok(GeneratedField::DataHash),
                            "proposerAddress" | "proposer_address" => Ok(GeneratedField::ProposerAddress),
                            "rollupTransactionsRoot" | "rollup_transactions_root" => Ok(GeneratedField::RollupTransactionsRoot),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SequencerBlockHeader;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1alpha1.SequencerBlockHeader")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SequencerBlockHeader, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut chain_id__ = None;
                let mut height__ = None;
                let mut time__ = None;
                let mut data_hash__ = None;
                let mut proposer_address__ = None;
                let mut rollup_transactions_root__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ChainId => {
                            if chain_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("chainId"));
                            }
                            chain_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Time => {
                            if time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("time"));
                            }
                            time__ = map_.next_value()?;
                        }
                        GeneratedField::DataHash => {
                            if data_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dataHash"));
                            }
                            data_hash__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ProposerAddress => {
                            if proposer_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("proposerAddress"));
                            }
                            proposer_address__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::RollupTransactionsRoot => {
                            if rollup_transactions_root__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupTransactionsRoot"));
                            }
                            rollup_transactions_root__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(SequencerBlockHeader {
                    chain_id: chain_id__.unwrap_or_default(),
                    height: height__.unwrap_or_default(),
                    time: time__,
                    data_hash: data_hash__.unwrap_or_default(),
                    proposer_address: proposer_address__.unwrap_or_default(),
                    rollup_transactions_root: rollup_transactions_root__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.SequencerBlockHeader", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SubmittedMetadata {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.block_hash.is_empty() {
            len += 1;
        }
        if self.header.is_some() {
            len += 1;
        }
        if !self.rollup_ids.is_empty() {
            len += 1;
        }
        if self.rollup_transactions_proof.is_some() {
            len += 1;
        }
        if self.rollup_ids_proof.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.SubmittedMetadata", len)?;
        if !self.block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("blockHash", pbjson::private::base64::encode(&self.block_hash).as_str())?;
        }
        if let Some(v) = self.header.as_ref() {
            struct_ser.serialize_field("header", v)?;
        }
        if !self.rollup_ids.is_empty() {
            struct_ser.serialize_field("rollupIds", &self.rollup_ids)?;
        }
        if let Some(v) = self.rollup_transactions_proof.as_ref() {
            struct_ser.serialize_field("rollupTransactionsProof", v)?;
        }
        if let Some(v) = self.rollup_ids_proof.as_ref() {
            struct_ser.serialize_field("rollupIdsProof", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SubmittedMetadata {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "block_hash",
            "blockHash",
            "header",
            "rollup_ids",
            "rollupIds",
            "rollup_transactions_proof",
            "rollupTransactionsProof",
            "rollup_ids_proof",
            "rollupIdsProof",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BlockHash,
            Header,
            RollupIds,
            RollupTransactionsProof,
            RollupIdsProof,
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
                            "blockHash" | "block_hash" => Ok(GeneratedField::BlockHash),
                            "header" => Ok(GeneratedField::Header),
                            "rollupIds" | "rollup_ids" => Ok(GeneratedField::RollupIds),
                            "rollupTransactionsProof" | "rollup_transactions_proof" => Ok(GeneratedField::RollupTransactionsProof),
                            "rollupIdsProof" | "rollup_ids_proof" => Ok(GeneratedField::RollupIdsProof),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SubmittedMetadata;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1alpha1.SubmittedMetadata")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SubmittedMetadata, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut block_hash__ = None;
                let mut header__ = None;
                let mut rollup_ids__ = None;
                let mut rollup_transactions_proof__ = None;
                let mut rollup_ids_proof__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BlockHash => {
                            if block_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHash"));
                            }
                            block_hash__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Header => {
                            if header__.is_some() {
                                return Err(serde::de::Error::duplicate_field("header"));
                            }
                            header__ = map_.next_value()?;
                        }
                        GeneratedField::RollupIds => {
                            if rollup_ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupIds"));
                            }
                            rollup_ids__ = Some(map_.next_value()?);
                        }
                        GeneratedField::RollupTransactionsProof => {
                            if rollup_transactions_proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupTransactionsProof"));
                            }
                            rollup_transactions_proof__ = map_.next_value()?;
                        }
                        GeneratedField::RollupIdsProof => {
                            if rollup_ids_proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupIdsProof"));
                            }
                            rollup_ids_proof__ = map_.next_value()?;
                        }
                    }
                }
                Ok(SubmittedMetadata {
                    block_hash: block_hash__.unwrap_or_default(),
                    header: header__,
                    rollup_ids: rollup_ids__.unwrap_or_default(),
                    rollup_transactions_proof: rollup_transactions_proof__,
                    rollup_ids_proof: rollup_ids_proof__,
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.SubmittedMetadata", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SubmittedMetadataList {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.entries.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.SubmittedMetadataList", len)?;
        if !self.entries.is_empty() {
            struct_ser.serialize_field("entries", &self.entries)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SubmittedMetadataList {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "entries",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Entries,
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
                            "entries" => Ok(GeneratedField::Entries),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SubmittedMetadataList;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1alpha1.SubmittedMetadataList")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SubmittedMetadataList, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut entries__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Entries => {
                            if entries__.is_some() {
                                return Err(serde::de::Error::duplicate_field("entries"));
                            }
                            entries__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(SubmittedMetadataList {
                    entries: entries__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.SubmittedMetadataList", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SubmittedRollupData {
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
        if self.rollup_id.is_some() {
            len += 1;
        }
        if !self.transactions.is_empty() {
            len += 1;
        }
        if self.proof.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.SubmittedRollupData", len)?;
        if !self.sequencer_block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("sequencerBlockHash", pbjson::private::base64::encode(&self.sequencer_block_hash).as_str())?;
        }
        if let Some(v) = self.rollup_id.as_ref() {
            struct_ser.serialize_field("rollupId", v)?;
        }
        if !self.transactions.is_empty() {
            struct_ser.serialize_field("transactions", &self.transactions.iter().map(pbjson::private::base64::encode).collect::<Vec<_>>())?;
        }
        if let Some(v) = self.proof.as_ref() {
            struct_ser.serialize_field("proof", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SubmittedRollupData {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "sequencer_block_hash",
            "sequencerBlockHash",
            "rollup_id",
            "rollupId",
            "transactions",
            "proof",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SequencerBlockHash,
            RollupId,
            Transactions,
            Proof,
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
                            "rollupId" | "rollup_id" => Ok(GeneratedField::RollupId),
                            "transactions" => Ok(GeneratedField::Transactions),
                            "proof" => Ok(GeneratedField::Proof),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SubmittedRollupData;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1alpha1.SubmittedRollupData")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SubmittedRollupData, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut sequencer_block_hash__ = None;
                let mut rollup_id__ = None;
                let mut transactions__ = None;
                let mut proof__ = None;
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
                        GeneratedField::RollupId => {
                            if rollup_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupId"));
                            }
                            rollup_id__ = map_.next_value()?;
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
                        GeneratedField::Proof => {
                            if proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("proof"));
                            }
                            proof__ = map_.next_value()?;
                        }
                    }
                }
                Ok(SubmittedRollupData {
                    sequencer_block_hash: sequencer_block_hash__.unwrap_or_default(),
                    rollup_id: rollup_id__,
                    transactions: transactions__.unwrap_or_default(),
                    proof: proof__,
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.SubmittedRollupData", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SubmittedRollupDataList {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.entries.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.SubmittedRollupDataList", len)?;
        if !self.entries.is_empty() {
            struct_ser.serialize_field("entries", &self.entries)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SubmittedRollupDataList {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "entries",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Entries,
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
                            "entries" => Ok(GeneratedField::Entries),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SubmittedRollupDataList;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1alpha1.SubmittedRollupDataList")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SubmittedRollupDataList, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut entries__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Entries => {
                            if entries__.is_some() {
                                return Err(serde::de::Error::duplicate_field("entries"));
                            }
                            entries__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(SubmittedRollupDataList {
                    entries: entries__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.SubmittedRollupDataList", FIELDS, GeneratedVisitor)
    }
}
