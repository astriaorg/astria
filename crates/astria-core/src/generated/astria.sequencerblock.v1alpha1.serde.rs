impl serde::Serialize for Deposit {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.bridge_address.is_empty() {
            len += 1;
        }
        if !self.rollup_id.is_empty() {
            len += 1;
        }
        if self.amount.is_some() {
            len += 1;
        }
        if !self.asset_id.is_empty() {
            len += 1;
        }
        if !self.destination_chain_address.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.Deposit", len)?;
        if !self.bridge_address.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("bridge_address", pbjson::private::base64::encode(&self.bridge_address).as_str())?;
        }
        if !self.rollup_id.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("rollup_id", pbjson::private::base64::encode(&self.rollup_id).as_str())?;
        }
        if let Some(v) = self.amount.as_ref() {
            struct_ser.serialize_field("amount", v)?;
        }
        if !self.asset_id.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("asset_id", pbjson::private::base64::encode(&self.asset_id).as_str())?;
        }
        if !self.destination_chain_address.is_empty() {
            struct_ser.serialize_field("destination_chain_address", &self.destination_chain_address)?;
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
            "asset_id",
            "assetId",
            "destination_chain_address",
            "destinationChainAddress",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BridgeAddress,
            RollupId,
            Amount,
            AssetId,
            DestinationChainAddress,
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
                            "assetId" | "asset_id" => Ok(GeneratedField::AssetId),
                            "destinationChainAddress" | "destination_chain_address" => Ok(GeneratedField::DestinationChainAddress),
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
                let mut asset_id__ = None;
                let mut destination_chain_address__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BridgeAddress => {
                            if bridge_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeAddress"));
                            }
                            bridge_address__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::RollupId => {
                            if rollup_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupId"));
                            }
                            rollup_id__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Amount => {
                            if amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("amount"));
                            }
                            amount__ = map_.next_value()?;
                        }
                        GeneratedField::AssetId => {
                            if asset_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("assetId"));
                            }
                            asset_id__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::DestinationChainAddress => {
                            if destination_chain_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("destinationChainAddress"));
                            }
                            destination_chain_address__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Deposit {
                    bridge_address: bridge_address__.unwrap_or_default(),
                    rollup_id: rollup_id__.unwrap_or_default(),
                    amount: amount__,
                    asset_id: asset_id__.unwrap_or_default(),
                    destination_chain_address: destination_chain_address__.unwrap_or_default(),
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
            struct_ser.serialize_field("block_hash", pbjson::private::base64::encode(&self.block_hash).as_str())?;
        }
        if let Some(v) = self.header.as_ref() {
            struct_ser.serialize_field("header", v)?;
        }
        if !self.rollup_transactions.is_empty() {
            struct_ser.serialize_field("rollup_transactions", &self.rollup_transactions)?;
        }
        if let Some(v) = self.rollup_transactions_proof.as_ref() {
            struct_ser.serialize_field("rollup_transactions_proof", v)?;
        }
        if !self.all_rollup_ids.is_empty() {
            struct_ser.serialize_field("all_rollup_ids", &self.all_rollup_ids.iter().map(pbjson::private::base64::encode).collect::<Vec<_>>())?;
        }
        if let Some(v) = self.rollup_ids_proof.as_ref() {
            struct_ser.serialize_field("rollup_ids_proof", v)?;
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
                            all_rollup_ids__ = 
                                Some(map_.next_value::<Vec<::pbjson::private::BytesDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
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
            struct_ser.serialize_field("rollup_ids", &self.rollup_ids.iter().map(pbjson::private::base64::encode).collect::<Vec<_>>())?;
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
                            rollup_ids__ = 
                                Some(map_.next_value::<Vec<::pbjson::private::BytesDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
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
impl serde::Serialize for Proof {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.audit_path.is_empty() {
            len += 1;
        }
        if self.leaf_index != 0 {
            len += 1;
        }
        if self.tree_size != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.Proof", len)?;
        if !self.audit_path.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("audit_path", pbjson::private::base64::encode(&self.audit_path).as_str())?;
        }
        if self.leaf_index != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("leaf_index", ToString::to_string(&self.leaf_index).as_str())?;
        }
        if self.tree_size != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("tree_size", ToString::to_string(&self.tree_size).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Proof {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "audit_path",
            "auditPath",
            "leaf_index",
            "leafIndex",
            "tree_size",
            "treeSize",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            AuditPath,
            LeafIndex,
            TreeSize,
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
                            "auditPath" | "audit_path" => Ok(GeneratedField::AuditPath),
                            "leafIndex" | "leaf_index" => Ok(GeneratedField::LeafIndex),
                            "treeSize" | "tree_size" => Ok(GeneratedField::TreeSize),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Proof;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1alpha1.Proof")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Proof, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut audit_path__ = None;
                let mut leaf_index__ = None;
                let mut tree_size__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::AuditPath => {
                            if audit_path__.is_some() {
                                return Err(serde::de::Error::duplicate_field("auditPath"));
                            }
                            audit_path__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::LeafIndex => {
                            if leaf_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("leafIndex"));
                            }
                            leaf_index__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::TreeSize => {
                            if tree_size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("treeSize"));
                            }
                            tree_size__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Proof {
                    audit_path: audit_path__.unwrap_or_default(),
                    leaf_index: leaf_index__.unwrap_or_default(),
                    tree_size: tree_size__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.Proof", FIELDS, GeneratedVisitor)
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
                    struct_ser.serialize_field("sequenced_data", pbjson::private::base64::encode(&v).as_str())?;
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
        if !self.rollup_id.is_empty() {
            len += 1;
        }
        if !self.transactions.is_empty() {
            len += 1;
        }
        if self.proof.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1alpha1.RollupTransactions", len)?;
        if !self.rollup_id.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("rollup_id", pbjson::private::base64::encode(&self.rollup_id).as_str())?;
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
                            rollup_id__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
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
                        GeneratedField::Proof => {
                            if proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("proof"));
                            }
                            proof__ = map_.next_value()?;
                        }
                    }
                }
                Ok(RollupTransactions {
                    rollup_id: rollup_id__.unwrap_or_default(),
                    transactions: transactions__.unwrap_or_default(),
                    proof: proof__,
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1alpha1.RollupTransactions", FIELDS, GeneratedVisitor)
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
            struct_ser.serialize_field("chain_id", &self.chain_id)?;
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
            struct_ser.serialize_field("data_hash", pbjson::private::base64::encode(&self.data_hash).as_str())?;
        }
        if !self.proposer_address.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("proposer_address", pbjson::private::base64::encode(&self.proposer_address).as_str())?;
        }
        if !self.rollup_transactions_root.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("rollup_transactions_root", pbjson::private::base64::encode(&self.rollup_transactions_root).as_str())?;
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
