impl serde::Serialize for DataItem {
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
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.DataItem", len)?;
        if let Some(v) = self.value.as_ref() {
            match v {
                data_item::Value::RollupTransactionsRoot(v) => {
                    #[allow(clippy::needless_borrow)]
                    #[allow(clippy::needless_borrows_for_generic_args)]
                    struct_ser.serialize_field("rollupTransactionsRoot", pbjson::private::base64::encode(&v).as_str())?;
                }
                data_item::Value::RollupIdsRoot(v) => {
                    #[allow(clippy::needless_borrow)]
                    #[allow(clippy::needless_borrows_for_generic_args)]
                    struct_ser.serialize_field("rollupIdsRoot", pbjson::private::base64::encode(&v).as_str())?;
                }
                data_item::Value::UpgradeChangeHashes(v) => {
                    struct_ser.serialize_field("upgradeChangeHashes", v)?;
                }
                data_item::Value::ExtendedCommitInfo(v) => {
                    #[allow(clippy::needless_borrow)]
                    #[allow(clippy::needless_borrows_for_generic_args)]
                    struct_ser.serialize_field("extendedCommitInfo", pbjson::private::base64::encode(&v).as_str())?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DataItem {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "rollup_transactions_root",
            "rollupTransactionsRoot",
            "rollup_ids_root",
            "rollupIdsRoot",
            "upgrade_change_hashes",
            "upgradeChangeHashes",
            "extended_commit_info",
            "extendedCommitInfo",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RollupTransactionsRoot,
            RollupIdsRoot,
            UpgradeChangeHashes,
            ExtendedCommitInfo,
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
                            "rollupTransactionsRoot" | "rollup_transactions_root" => Ok(GeneratedField::RollupTransactionsRoot),
                            "rollupIdsRoot" | "rollup_ids_root" => Ok(GeneratedField::RollupIdsRoot),
                            "upgradeChangeHashes" | "upgrade_change_hashes" => Ok(GeneratedField::UpgradeChangeHashes),
                            "extendedCommitInfo" | "extended_commit_info" => Ok(GeneratedField::ExtendedCommitInfo),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DataItem;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1.DataItem")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<DataItem, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut value__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RollupTransactionsRoot => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupTransactionsRoot"));
                            }
                            value__ = map_.next_value::<::std::option::Option<::pbjson::private::BytesDeserialize<_>>>()?.map(|x| data_item::Value::RollupTransactionsRoot(x.0));
                        }
                        GeneratedField::RollupIdsRoot => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupIdsRoot"));
                            }
                            value__ = map_.next_value::<::std::option::Option<::pbjson::private::BytesDeserialize<_>>>()?.map(|x| data_item::Value::RollupIdsRoot(x.0));
                        }
                        GeneratedField::UpgradeChangeHashes => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("upgradeChangeHashes"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(data_item::Value::UpgradeChangeHashes)
;
                        }
                        GeneratedField::ExtendedCommitInfo => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("extendedCommitInfo"));
                            }
                            value__ = map_.next_value::<::std::option::Option<::pbjson::private::BytesDeserialize<_>>>()?.map(|x| data_item::Value::ExtendedCommitInfo(x.0));
                        }
                    }
                }
                Ok(DataItem {
                    value: value__,
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1.DataItem", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for data_item::UpgradeChangeHashes {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.hashes.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.DataItem.UpgradeChangeHashes", len)?;
        if !self.hashes.is_empty() {
            struct_ser.serialize_field("hashes", &self.hashes.iter().map(pbjson::private::base64::encode).collect::<Vec<_>>())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for data_item::UpgradeChangeHashes {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "hashes",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Hashes,
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
                            "hashes" => Ok(GeneratedField::Hashes),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = data_item::UpgradeChangeHashes;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1.DataItem.UpgradeChangeHashes")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<data_item::UpgradeChangeHashes, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut hashes__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Hashes => {
                            if hashes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hashes"));
                            }
                            hashes__ = 
                                Some(map_.next_value::<Vec<::pbjson::private::BytesDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                    }
                }
                Ok(data_item::UpgradeChangeHashes {
                    hashes: hashes__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1.DataItem.UpgradeChangeHashes", FIELDS, GeneratedVisitor)
    }
}
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
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.Deposit", len)?;
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
            #[allow(clippy::needless_borrows_for_generic_args)]
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
                formatter.write_str("struct astria.sequencerblock.v1.Deposit")
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
        deserializer.deserialize_struct("astria.sequencerblock.v1.Deposit", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ExtendedCommitInfoWithProof {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.extended_commit_info.is_empty() {
            len += 1;
        }
        if self.proof.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.ExtendedCommitInfoWithProof", len)?;
        if !self.extended_commit_info.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("extendedCommitInfo", pbjson::private::base64::encode(&self.extended_commit_info).as_str())?;
        }
        if let Some(v) = self.proof.as_ref() {
            struct_ser.serialize_field("proof", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ExtendedCommitInfoWithProof {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "extended_commit_info",
            "extendedCommitInfo",
            "proof",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ExtendedCommitInfo,
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
                            "extendedCommitInfo" | "extended_commit_info" => Ok(GeneratedField::ExtendedCommitInfo),
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
            type Value = ExtendedCommitInfoWithProof;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1.ExtendedCommitInfoWithProof")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ExtendedCommitInfoWithProof, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut extended_commit_info__ = None;
                let mut proof__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ExtendedCommitInfo => {
                            if extended_commit_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("extendedCommitInfo"));
                            }
                            extended_commit_info__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
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
                Ok(ExtendedCommitInfoWithProof {
                    extended_commit_info: extended_commit_info__.unwrap_or_default(),
                    proof: proof__,
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1.ExtendedCommitInfoWithProof", FIELDS, GeneratedVisitor)
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
        if !self.upgrade_change_hashes.is_empty() {
            len += 1;
        }
        if self.extended_commit_info_with_proof.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.FilteredSequencerBlock", len)?;
        if !self.block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
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
        if !self.upgrade_change_hashes.is_empty() {
            struct_ser.serialize_field("upgradeChangeHashes", &self.upgrade_change_hashes.iter().map(pbjson::private::base64::encode).collect::<Vec<_>>())?;
        }
        if let Some(v) = self.extended_commit_info_with_proof.as_ref() {
            struct_ser.serialize_field("extendedCommitInfoWithProof", v)?;
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
            "upgrade_change_hashes",
            "upgradeChangeHashes",
            "extended_commit_info_with_proof",
            "extendedCommitInfoWithProof",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BlockHash,
            Header,
            RollupTransactions,
            RollupTransactionsProof,
            AllRollupIds,
            RollupIdsProof,
            UpgradeChangeHashes,
            ExtendedCommitInfoWithProof,
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
                            "upgradeChangeHashes" | "upgrade_change_hashes" => Ok(GeneratedField::UpgradeChangeHashes),
                            "extendedCommitInfoWithProof" | "extended_commit_info_with_proof" => Ok(GeneratedField::ExtendedCommitInfoWithProof),
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
                formatter.write_str("struct astria.sequencerblock.v1.FilteredSequencerBlock")
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
                let mut upgrade_change_hashes__ = None;
                let mut extended_commit_info_with_proof__ = None;
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
                        GeneratedField::UpgradeChangeHashes => {
                            if upgrade_change_hashes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("upgradeChangeHashes"));
                            }
                            upgrade_change_hashes__ = 
                                Some(map_.next_value::<Vec<::pbjson::private::BytesDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                        GeneratedField::ExtendedCommitInfoWithProof => {
                            if extended_commit_info_with_proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("extendedCommitInfoWithProof"));
                            }
                            extended_commit_info_with_proof__ = map_.next_value()?;
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
                    upgrade_change_hashes: upgrade_change_hashes__.unwrap_or_default(),
                    extended_commit_info_with_proof: extended_commit_info_with_proof__,
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1.FilteredSequencerBlock", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.GetFilteredSequencerBlockRequest", len)?;
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
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
                formatter.write_str("struct astria.sequencerblock.v1.GetFilteredSequencerBlockRequest")
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
        deserializer.deserialize_struct("astria.sequencerblock.v1.GetFilteredSequencerBlockRequest", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.GetPendingNonceRequest", len)?;
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
                formatter.write_str("struct astria.sequencerblock.v1.GetPendingNonceRequest")
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
        deserializer.deserialize_struct("astria.sequencerblock.v1.GetPendingNonceRequest", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.GetPendingNonceResponse", len)?;
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
                formatter.write_str("struct astria.sequencerblock.v1.GetPendingNonceResponse")
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
        deserializer.deserialize_struct("astria.sequencerblock.v1.GetPendingNonceResponse", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.GetSequencerBlockRequest", len)?;
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
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
                formatter.write_str("struct astria.sequencerblock.v1.GetSequencerBlockRequest")
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
        deserializer.deserialize_struct("astria.sequencerblock.v1.GetSequencerBlockRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetUpgradesInfoRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.GetUpgradesInfoRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetUpgradesInfoRequest {
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
            type Value = GetUpgradesInfoRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1.GetUpgradesInfoRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetUpgradesInfoRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetUpgradesInfoRequest {
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1.GetUpgradesInfoRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetUpgradesInfoResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.applied.is_empty() {
            len += 1;
        }
        if !self.scheduled.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.GetUpgradesInfoResponse", len)?;
        if !self.applied.is_empty() {
            struct_ser.serialize_field("applied", &self.applied)?;
        }
        if !self.scheduled.is_empty() {
            struct_ser.serialize_field("scheduled", &self.scheduled)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetUpgradesInfoResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "applied",
            "scheduled",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Applied,
            Scheduled,
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
                            "applied" => Ok(GeneratedField::Applied),
                            "scheduled" => Ok(GeneratedField::Scheduled),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetUpgradesInfoResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1.GetUpgradesInfoResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetUpgradesInfoResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut applied__ = None;
                let mut scheduled__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Applied => {
                            if applied__.is_some() {
                                return Err(serde::de::Error::duplicate_field("applied"));
                            }
                            applied__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Scheduled => {
                            if scheduled__.is_some() {
                                return Err(serde::de::Error::duplicate_field("scheduled"));
                            }
                            scheduled__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(GetUpgradesInfoResponse {
                    applied: applied__.unwrap_or_default(),
                    scheduled: scheduled__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1.GetUpgradesInfoResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for get_upgrades_info_response::ChangeInfo {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.activation_height != 0 {
            len += 1;
        }
        if !self.change_name.is_empty() {
            len += 1;
        }
        if self.app_version != 0 {
            len += 1;
        }
        if !self.base64_hash.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.GetUpgradesInfoResponse.ChangeInfo", len)?;
        if self.activation_height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("activationHeight", ToString::to_string(&self.activation_height).as_str())?;
        }
        if !self.change_name.is_empty() {
            struct_ser.serialize_field("changeName", &self.change_name)?;
        }
        if self.app_version != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("appVersion", ToString::to_string(&self.app_version).as_str())?;
        }
        if !self.base64_hash.is_empty() {
            struct_ser.serialize_field("base64Hash", &self.base64_hash)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for get_upgrades_info_response::ChangeInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "activation_height",
            "activationHeight",
            "change_name",
            "changeName",
            "app_version",
            "appVersion",
            "base64_hash",
            "base64Hash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ActivationHeight,
            ChangeName,
            AppVersion,
            Base64Hash,
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
                            "activationHeight" | "activation_height" => Ok(GeneratedField::ActivationHeight),
                            "changeName" | "change_name" => Ok(GeneratedField::ChangeName),
                            "appVersion" | "app_version" => Ok(GeneratedField::AppVersion),
                            "base64Hash" | "base64_hash" => Ok(GeneratedField::Base64Hash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = get_upgrades_info_response::ChangeInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1.GetUpgradesInfoResponse.ChangeInfo")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<get_upgrades_info_response::ChangeInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut activation_height__ = None;
                let mut change_name__ = None;
                let mut app_version__ = None;
                let mut base64_hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ActivationHeight => {
                            if activation_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("activationHeight"));
                            }
                            activation_height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ChangeName => {
                            if change_name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("changeName"));
                            }
                            change_name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::AppVersion => {
                            if app_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("appVersion"));
                            }
                            app_version__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Base64Hash => {
                            if base64_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("base64Hash"));
                            }
                            base64_hash__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(get_upgrades_info_response::ChangeInfo {
                    activation_height: activation_height__.unwrap_or_default(),
                    change_name: change_name__.unwrap_or_default(),
                    app_version: app_version__.unwrap_or_default(),
                    base64_hash: base64_hash__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1.GetUpgradesInfoResponse.ChangeInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetValidatorNameRequest {
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
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.GetValidatorNameRequest", len)?;
        if let Some(v) = self.address.as_ref() {
            struct_ser.serialize_field("address", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetValidatorNameRequest {
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
            type Value = GetValidatorNameRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1.GetValidatorNameRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetValidatorNameRequest, V::Error>
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
                Ok(GetValidatorNameRequest {
                    address: address__,
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1.GetValidatorNameRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetValidatorNameResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.name.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.GetValidatorNameResponse", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetValidatorNameResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
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
                            "name" => Ok(GeneratedField::Name),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetValidatorNameResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1.GetValidatorNameResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetValidatorNameResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(GetValidatorNameResponse {
                    name: name__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1.GetValidatorNameResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Price {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.currency_pair.is_some() {
            len += 1;
        }
        if self.price.is_some() {
            len += 1;
        }
        if self.decimals != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.Price", len)?;
        if let Some(v) = self.currency_pair.as_ref() {
            struct_ser.serialize_field("currencyPair", v)?;
        }
        if let Some(v) = self.price.as_ref() {
            struct_ser.serialize_field("price", v)?;
        }
        if self.decimals != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("decimals", ToString::to_string(&self.decimals).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Price {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "currency_pair",
            "currencyPair",
            "price",
            "decimals",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CurrencyPair,
            Price,
            Decimals,
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
                            "currencyPair" | "currency_pair" => Ok(GeneratedField::CurrencyPair),
                            "price" => Ok(GeneratedField::Price),
                            "decimals" => Ok(GeneratedField::Decimals),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Price;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1.Price")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Price, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut currency_pair__ = None;
                let mut price__ = None;
                let mut decimals__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::CurrencyPair => {
                            if currency_pair__.is_some() {
                                return Err(serde::de::Error::duplicate_field("currencyPair"));
                            }
                            currency_pair__ = map_.next_value()?;
                        }
                        GeneratedField::Price => {
                            if price__.is_some() {
                                return Err(serde::de::Error::duplicate_field("price"));
                            }
                            price__ = map_.next_value()?;
                        }
                        GeneratedField::Decimals => {
                            if decimals__.is_some() {
                                return Err(serde::de::Error::duplicate_field("decimals"));
                            }
                            decimals__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Price {
                    currency_pair: currency_pair__,
                    price: price__,
                    decimals: decimals__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1.Price", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PriceFeedData {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.prices.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.PriceFeedData", len)?;
        if !self.prices.is_empty() {
            struct_ser.serialize_field("prices", &self.prices)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PriceFeedData {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "prices",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Prices,
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
                            "prices" => Ok(GeneratedField::Prices),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PriceFeedData;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.sequencerblock.v1.PriceFeedData")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PriceFeedData, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut prices__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Prices => {
                            if prices__.is_some() {
                                return Err(serde::de::Error::duplicate_field("prices"));
                            }
                            prices__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(PriceFeedData {
                    prices: prices__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1.PriceFeedData", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.RollupData", len)?;
        if let Some(v) = self.value.as_ref() {
            match v {
                rollup_data::Value::SequencedData(v) => {
                    #[allow(clippy::needless_borrow)]
                    #[allow(clippy::needless_borrows_for_generic_args)]
                    struct_ser.serialize_field("sequencedData", pbjson::private::base64::encode(&v).as_str())?;
                }
                rollup_data::Value::Deposit(v) => {
                    struct_ser.serialize_field("deposit", v)?;
                }
                rollup_data::Value::PriceFeedData(v) => {
                    struct_ser.serialize_field("priceFeedData", v)?;
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
            "price_feed_data",
            "priceFeedData",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SequencedData,
            Deposit,
            PriceFeedData,
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
                            "priceFeedData" | "price_feed_data" => Ok(GeneratedField::PriceFeedData),
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
                formatter.write_str("struct astria.sequencerblock.v1.RollupData")
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
                        GeneratedField::PriceFeedData => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("priceFeedData"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(rollup_data::Value::PriceFeedData)
;
                        }
                    }
                }
                Ok(RollupData {
                    value: value__,
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1.RollupData", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.RollupTransactions", len)?;
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
                formatter.write_str("struct astria.sequencerblock.v1.RollupTransactions")
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
        deserializer.deserialize_struct("astria.sequencerblock.v1.RollupTransactions", FIELDS, GeneratedVisitor)
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
        if !self.upgrade_change_hashes.is_empty() {
            len += 1;
        }
        if self.extended_commit_info_with_proof.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.SequencerBlock", len)?;
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
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("blockHash", pbjson::private::base64::encode(&self.block_hash).as_str())?;
        }
        if !self.upgrade_change_hashes.is_empty() {
            struct_ser.serialize_field("upgradeChangeHashes", &self.upgrade_change_hashes.iter().map(pbjson::private::base64::encode).collect::<Vec<_>>())?;
        }
        if let Some(v) = self.extended_commit_info_with_proof.as_ref() {
            struct_ser.serialize_field("extendedCommitInfoWithProof", v)?;
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
            "upgrade_change_hashes",
            "upgradeChangeHashes",
            "extended_commit_info_with_proof",
            "extendedCommitInfoWithProof",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Header,
            RollupTransactions,
            RollupTransactionsProof,
            RollupIdsProof,
            BlockHash,
            UpgradeChangeHashes,
            ExtendedCommitInfoWithProof,
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
                            "upgradeChangeHashes" | "upgrade_change_hashes" => Ok(GeneratedField::UpgradeChangeHashes),
                            "extendedCommitInfoWithProof" | "extended_commit_info_with_proof" => Ok(GeneratedField::ExtendedCommitInfoWithProof),
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
                formatter.write_str("struct astria.sequencerblock.v1.SequencerBlock")
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
                let mut upgrade_change_hashes__ = None;
                let mut extended_commit_info_with_proof__ = None;
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
                        GeneratedField::UpgradeChangeHashes => {
                            if upgrade_change_hashes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("upgradeChangeHashes"));
                            }
                            upgrade_change_hashes__ = 
                                Some(map_.next_value::<Vec<::pbjson::private::BytesDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                        GeneratedField::ExtendedCommitInfoWithProof => {
                            if extended_commit_info_with_proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("extendedCommitInfoWithProof"));
                            }
                            extended_commit_info_with_proof__ = map_.next_value()?;
                        }
                    }
                }
                Ok(SequencerBlock {
                    header: header__,
                    rollup_transactions: rollup_transactions__.unwrap_or_default(),
                    rollup_transactions_proof: rollup_transactions_proof__,
                    rollup_ids_proof: rollup_ids_proof__,
                    block_hash: block_hash__.unwrap_or_default(),
                    upgrade_change_hashes: upgrade_change_hashes__.unwrap_or_default(),
                    extended_commit_info_with_proof: extended_commit_info_with_proof__,
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1.SequencerBlock", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.SequencerBlockHeader", len)?;
        if !self.chain_id.is_empty() {
            struct_ser.serialize_field("chainId", &self.chain_id)?;
        }
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        if let Some(v) = self.time.as_ref() {
            struct_ser.serialize_field("time", v)?;
        }
        if !self.data_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("dataHash", pbjson::private::base64::encode(&self.data_hash).as_str())?;
        }
        if !self.proposer_address.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("proposerAddress", pbjson::private::base64::encode(&self.proposer_address).as_str())?;
        }
        if !self.rollup_transactions_root.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
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
                formatter.write_str("struct astria.sequencerblock.v1.SequencerBlockHeader")
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
        deserializer.deserialize_struct("astria.sequencerblock.v1.SequencerBlockHeader", FIELDS, GeneratedVisitor)
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
        if !self.upgrade_change_hashes.is_empty() {
            len += 1;
        }
        if self.extended_commit_info_with_proof.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.SubmittedMetadata", len)?;
        if !self.block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
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
        if !self.upgrade_change_hashes.is_empty() {
            struct_ser.serialize_field("upgradeChangeHashes", &self.upgrade_change_hashes.iter().map(pbjson::private::base64::encode).collect::<Vec<_>>())?;
        }
        if let Some(v) = self.extended_commit_info_with_proof.as_ref() {
            struct_ser.serialize_field("extendedCommitInfoWithProof", v)?;
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
            "upgrade_change_hashes",
            "upgradeChangeHashes",
            "extended_commit_info_with_proof",
            "extendedCommitInfoWithProof",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BlockHash,
            Header,
            RollupIds,
            RollupTransactionsProof,
            RollupIdsProof,
            UpgradeChangeHashes,
            ExtendedCommitInfoWithProof,
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
                            "upgradeChangeHashes" | "upgrade_change_hashes" => Ok(GeneratedField::UpgradeChangeHashes),
                            "extendedCommitInfoWithProof" | "extended_commit_info_with_proof" => Ok(GeneratedField::ExtendedCommitInfoWithProof),
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
                formatter.write_str("struct astria.sequencerblock.v1.SubmittedMetadata")
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
                let mut upgrade_change_hashes__ = None;
                let mut extended_commit_info_with_proof__ = None;
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
                        GeneratedField::UpgradeChangeHashes => {
                            if upgrade_change_hashes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("upgradeChangeHashes"));
                            }
                            upgrade_change_hashes__ = 
                                Some(map_.next_value::<Vec<::pbjson::private::BytesDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                        GeneratedField::ExtendedCommitInfoWithProof => {
                            if extended_commit_info_with_proof__.is_some() {
                                return Err(serde::de::Error::duplicate_field("extendedCommitInfoWithProof"));
                            }
                            extended_commit_info_with_proof__ = map_.next_value()?;
                        }
                    }
                }
                Ok(SubmittedMetadata {
                    block_hash: block_hash__.unwrap_or_default(),
                    header: header__,
                    rollup_ids: rollup_ids__.unwrap_or_default(),
                    rollup_transactions_proof: rollup_transactions_proof__,
                    rollup_ids_proof: rollup_ids_proof__,
                    upgrade_change_hashes: upgrade_change_hashes__.unwrap_or_default(),
                    extended_commit_info_with_proof: extended_commit_info_with_proof__,
                })
            }
        }
        deserializer.deserialize_struct("astria.sequencerblock.v1.SubmittedMetadata", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.SubmittedMetadataList", len)?;
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
                formatter.write_str("struct astria.sequencerblock.v1.SubmittedMetadataList")
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
        deserializer.deserialize_struct("astria.sequencerblock.v1.SubmittedMetadataList", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.SubmittedRollupData", len)?;
        if !self.sequencer_block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
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
                formatter.write_str("struct astria.sequencerblock.v1.SubmittedRollupData")
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
        deserializer.deserialize_struct("astria.sequencerblock.v1.SubmittedRollupData", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.sequencerblock.v1.SubmittedRollupDataList", len)?;
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
                formatter.write_str("struct astria.sequencerblock.v1.SubmittedRollupDataList")
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
        deserializer.deserialize_struct("astria.sequencerblock.v1.SubmittedRollupDataList", FIELDS, GeneratedVisitor)
    }
}
