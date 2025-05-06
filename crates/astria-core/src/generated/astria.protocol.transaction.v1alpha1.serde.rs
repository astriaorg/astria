impl serde::Serialize for Action {
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1alpha1.Action", len)?;
        if let Some(v) = self.value.as_ref() {
            match v {
                action::Value::Transfer(v) => {
                    struct_ser.serialize_field("transfer", v)?;
                }
                action::Value::RollupDataSubmission(v) => {
                    struct_ser.serialize_field("rollupDataSubmission", v)?;
                }
                action::Value::InitBridgeAccount(v) => {
                    struct_ser.serialize_field("initBridgeAccount", v)?;
                }
                action::Value::BridgeLock(v) => {
                    struct_ser.serialize_field("bridgeLock", v)?;
                }
                action::Value::BridgeUnlock(v) => {
                    struct_ser.serialize_field("bridgeUnlock", v)?;
                }
                action::Value::BridgeSudoChange(v) => {
                    struct_ser.serialize_field("bridgeSudoChange", v)?;
                }
                action::Value::Ibc(v) => {
                    struct_ser.serialize_field("ibc", v)?;
                }
                action::Value::Ics20Withdrawal(v) => {
                    struct_ser.serialize_field("ics20Withdrawal", v)?;
                }
                action::Value::SudoAddressChange(v) => {
                    struct_ser.serialize_field("sudoAddressChange", v)?;
                }
                action::Value::ValidatorUpdate(v) => {
                    struct_ser.serialize_field("validatorUpdate", v)?;
                }
                action::Value::IbcRelayerChange(v) => {
                    struct_ser.serialize_field("ibcRelayerChange", v)?;
                }
                action::Value::FeeAssetChange(v) => {
                    struct_ser.serialize_field("feeAssetChange", v)?;
                }
                action::Value::FeeChange(v) => {
                    struct_ser.serialize_field("feeChange", v)?;
                }
                action::Value::IbcSudoChange(v) => {
                    struct_ser.serialize_field("ibcSudoChange", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Action {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "transfer",
            "rollup_data_submission",
            "rollupDataSubmission",
            "init_bridge_account",
            "initBridgeAccount",
            "bridge_lock",
            "bridgeLock",
            "bridge_unlock",
            "bridgeUnlock",
            "bridge_sudo_change",
            "bridgeSudoChange",
            "ibc",
            "ics20_withdrawal",
            "ics20Withdrawal",
            "sudo_address_change",
            "sudoAddressChange",
            "validator_update",
            "validatorUpdate",
            "ibc_relayer_change",
            "ibcRelayerChange",
            "fee_asset_change",
            "feeAssetChange",
            "fee_change",
            "feeChange",
            "ibc_sudo_change",
            "ibcSudoChange",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Transfer,
            RollupDataSubmission,
            InitBridgeAccount,
            BridgeLock,
            BridgeUnlock,
            BridgeSudoChange,
            Ibc,
            Ics20Withdrawal,
            SudoAddressChange,
            ValidatorUpdate,
            IbcRelayerChange,
            FeeAssetChange,
            FeeChange,
            IbcSudoChange,
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
                            "transfer" => Ok(GeneratedField::Transfer),
                            "rollupDataSubmission" | "rollup_data_submission" => Ok(GeneratedField::RollupDataSubmission),
                            "initBridgeAccount" | "init_bridge_account" => Ok(GeneratedField::InitBridgeAccount),
                            "bridgeLock" | "bridge_lock" => Ok(GeneratedField::BridgeLock),
                            "bridgeUnlock" | "bridge_unlock" => Ok(GeneratedField::BridgeUnlock),
                            "bridgeSudoChange" | "bridge_sudo_change" => Ok(GeneratedField::BridgeSudoChange),
                            "ibc" => Ok(GeneratedField::Ibc),
                            "ics20Withdrawal" | "ics20_withdrawal" => Ok(GeneratedField::Ics20Withdrawal),
                            "sudoAddressChange" | "sudo_address_change" => Ok(GeneratedField::SudoAddressChange),
                            "validatorUpdate" | "validator_update" => Ok(GeneratedField::ValidatorUpdate),
                            "ibcRelayerChange" | "ibc_relayer_change" => Ok(GeneratedField::IbcRelayerChange),
                            "feeAssetChange" | "fee_asset_change" => Ok(GeneratedField::FeeAssetChange),
                            "feeChange" | "fee_change" => Ok(GeneratedField::FeeChange),
                            "ibcSudoChange" | "ibc_sudo_change" => Ok(GeneratedField::IbcSudoChange),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Action;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1alpha1.Action")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Action, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut value__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Transfer => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transfer"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::Transfer)
;
                        }
                        GeneratedField::RollupDataSubmission => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupDataSubmission"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::RollupDataSubmission)
;
                        }
                        GeneratedField::InitBridgeAccount => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("initBridgeAccount"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::InitBridgeAccount)
;
                        }
                        GeneratedField::BridgeLock => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeLock"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::BridgeLock)
;
                        }
                        GeneratedField::BridgeUnlock => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeUnlock"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::BridgeUnlock)
;
                        }
                        GeneratedField::BridgeSudoChange => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeSudoChange"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::BridgeSudoChange)
;
                        }
                        GeneratedField::Ibc => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ibc"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::Ibc)
;
                        }
                        GeneratedField::Ics20Withdrawal => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ics20Withdrawal"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::Ics20Withdrawal)
;
                        }
                        GeneratedField::SudoAddressChange => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sudoAddressChange"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::SudoAddressChange)
;
                        }
                        GeneratedField::ValidatorUpdate => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("validatorUpdate"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::ValidatorUpdate)
;
                        }
                        GeneratedField::IbcRelayerChange => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ibcRelayerChange"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::IbcRelayerChange)
;
                        }
                        GeneratedField::FeeAssetChange => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeAssetChange"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::FeeAssetChange)
;
                        }
                        GeneratedField::FeeChange => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeChange"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::FeeChange)
;
                        }
                        GeneratedField::IbcSudoChange => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ibcSudoChange"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::IbcSudoChange)
;
                        }
                    }
                }
                Ok(Action {
                    value: value__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1alpha1.Action", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BridgeLock {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.to.is_some() {
            len += 1;
        }
        if self.amount.is_some() {
            len += 1;
        }
        if !self.asset.is_empty() {
            len += 1;
        }
        if !self.fee_asset.is_empty() {
            len += 1;
        }
        if !self.destination_chain_address.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1alpha1.BridgeLock", len)?;
        if let Some(v) = self.to.as_ref() {
            struct_ser.serialize_field("to", v)?;
        }
        if let Some(v) = self.amount.as_ref() {
            struct_ser.serialize_field("amount", v)?;
        }
        if !self.asset.is_empty() {
            struct_ser.serialize_field("asset", &self.asset)?;
        }
        if !self.fee_asset.is_empty() {
            struct_ser.serialize_field("feeAsset", &self.fee_asset)?;
        }
        if !self.destination_chain_address.is_empty() {
            struct_ser.serialize_field("destinationChainAddress", &self.destination_chain_address)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BridgeLock {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "to",
            "amount",
            "asset",
            "fee_asset",
            "feeAsset",
            "destination_chain_address",
            "destinationChainAddress",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            To,
            Amount,
            Asset,
            FeeAsset,
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
                            "to" => Ok(GeneratedField::To),
                            "amount" => Ok(GeneratedField::Amount),
                            "asset" => Ok(GeneratedField::Asset),
                            "feeAsset" | "fee_asset" => Ok(GeneratedField::FeeAsset),
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
            type Value = BridgeLock;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1alpha1.BridgeLock")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BridgeLock, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut to__ = None;
                let mut amount__ = None;
                let mut asset__ = None;
                let mut fee_asset__ = None;
                let mut destination_chain_address__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::To => {
                            if to__.is_some() {
                                return Err(serde::de::Error::duplicate_field("to"));
                            }
                            to__ = map_.next_value()?;
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
                        GeneratedField::FeeAsset => {
                            if fee_asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeAsset"));
                            }
                            fee_asset__ = Some(map_.next_value()?);
                        }
                        GeneratedField::DestinationChainAddress => {
                            if destination_chain_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("destinationChainAddress"));
                            }
                            destination_chain_address__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(BridgeLock {
                    to: to__,
                    amount: amount__,
                    asset: asset__.unwrap_or_default(),
                    fee_asset: fee_asset__.unwrap_or_default(),
                    destination_chain_address: destination_chain_address__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1alpha1.BridgeLock", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BridgeSudoChange {
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
        if self.new_sudo_address.is_some() {
            len += 1;
        }
        if self.new_withdrawer_address.is_some() {
            len += 1;
        }
        if !self.fee_asset.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1alpha1.BridgeSudoChange", len)?;
        if let Some(v) = self.bridge_address.as_ref() {
            struct_ser.serialize_field("bridgeAddress", v)?;
        }
        if let Some(v) = self.new_sudo_address.as_ref() {
            struct_ser.serialize_field("newSudoAddress", v)?;
        }
        if let Some(v) = self.new_withdrawer_address.as_ref() {
            struct_ser.serialize_field("newWithdrawerAddress", v)?;
        }
        if !self.fee_asset.is_empty() {
            struct_ser.serialize_field("feeAsset", &self.fee_asset)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BridgeSudoChange {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "bridge_address",
            "bridgeAddress",
            "new_sudo_address",
            "newSudoAddress",
            "new_withdrawer_address",
            "newWithdrawerAddress",
            "fee_asset",
            "feeAsset",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BridgeAddress,
            NewSudoAddress,
            NewWithdrawerAddress,
            FeeAsset,
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
                            "newSudoAddress" | "new_sudo_address" => Ok(GeneratedField::NewSudoAddress),
                            "newWithdrawerAddress" | "new_withdrawer_address" => Ok(GeneratedField::NewWithdrawerAddress),
                            "feeAsset" | "fee_asset" => Ok(GeneratedField::FeeAsset),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BridgeSudoChange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1alpha1.BridgeSudoChange")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BridgeSudoChange, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut bridge_address__ = None;
                let mut new_sudo_address__ = None;
                let mut new_withdrawer_address__ = None;
                let mut fee_asset__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BridgeAddress => {
                            if bridge_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeAddress"));
                            }
                            bridge_address__ = map_.next_value()?;
                        }
                        GeneratedField::NewSudoAddress => {
                            if new_sudo_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("newSudoAddress"));
                            }
                            new_sudo_address__ = map_.next_value()?;
                        }
                        GeneratedField::NewWithdrawerAddress => {
                            if new_withdrawer_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("newWithdrawerAddress"));
                            }
                            new_withdrawer_address__ = map_.next_value()?;
                        }
                        GeneratedField::FeeAsset => {
                            if fee_asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeAsset"));
                            }
                            fee_asset__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(BridgeSudoChange {
                    bridge_address: bridge_address__,
                    new_sudo_address: new_sudo_address__,
                    new_withdrawer_address: new_withdrawer_address__,
                    fee_asset: fee_asset__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1alpha1.BridgeSudoChange", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BridgeUnlock {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.to.is_some() {
            len += 1;
        }
        if self.amount.is_some() {
            len += 1;
        }
        if !self.fee_asset.is_empty() {
            len += 1;
        }
        if !self.memo.is_empty() {
            len += 1;
        }
        if self.bridge_address.is_some() {
            len += 1;
        }
        if self.rollup_block_number != 0 {
            len += 1;
        }
        if !self.rollup_withdrawal_event_id.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1alpha1.BridgeUnlock", len)?;
        if let Some(v) = self.to.as_ref() {
            struct_ser.serialize_field("to", v)?;
        }
        if let Some(v) = self.amount.as_ref() {
            struct_ser.serialize_field("amount", v)?;
        }
        if !self.fee_asset.is_empty() {
            struct_ser.serialize_field("feeAsset", &self.fee_asset)?;
        }
        if !self.memo.is_empty() {
            struct_ser.serialize_field("memo", &self.memo)?;
        }
        if let Some(v) = self.bridge_address.as_ref() {
            struct_ser.serialize_field("bridgeAddress", v)?;
        }
        if self.rollup_block_number != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("rollupBlockNumber", ToString::to_string(&self.rollup_block_number).as_str())?;
        }
        if !self.rollup_withdrawal_event_id.is_empty() {
            struct_ser.serialize_field("rollupWithdrawalEventId", &self.rollup_withdrawal_event_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BridgeUnlock {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "to",
            "amount",
            "fee_asset",
            "feeAsset",
            "memo",
            "bridge_address",
            "bridgeAddress",
            "rollup_block_number",
            "rollupBlockNumber",
            "rollup_withdrawal_event_id",
            "rollupWithdrawalEventId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            To,
            Amount,
            FeeAsset,
            Memo,
            BridgeAddress,
            RollupBlockNumber,
            RollupWithdrawalEventId,
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
                            "to" => Ok(GeneratedField::To),
                            "amount" => Ok(GeneratedField::Amount),
                            "feeAsset" | "fee_asset" => Ok(GeneratedField::FeeAsset),
                            "memo" => Ok(GeneratedField::Memo),
                            "bridgeAddress" | "bridge_address" => Ok(GeneratedField::BridgeAddress),
                            "rollupBlockNumber" | "rollup_block_number" => Ok(GeneratedField::RollupBlockNumber),
                            "rollupWithdrawalEventId" | "rollup_withdrawal_event_id" => Ok(GeneratedField::RollupWithdrawalEventId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BridgeUnlock;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1alpha1.BridgeUnlock")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BridgeUnlock, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut to__ = None;
                let mut amount__ = None;
                let mut fee_asset__ = None;
                let mut memo__ = None;
                let mut bridge_address__ = None;
                let mut rollup_block_number__ = None;
                let mut rollup_withdrawal_event_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::To => {
                            if to__.is_some() {
                                return Err(serde::de::Error::duplicate_field("to"));
                            }
                            to__ = map_.next_value()?;
                        }
                        GeneratedField::Amount => {
                            if amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("amount"));
                            }
                            amount__ = map_.next_value()?;
                        }
                        GeneratedField::FeeAsset => {
                            if fee_asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeAsset"));
                            }
                            fee_asset__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Memo => {
                            if memo__.is_some() {
                                return Err(serde::de::Error::duplicate_field("memo"));
                            }
                            memo__ = Some(map_.next_value()?);
                        }
                        GeneratedField::BridgeAddress => {
                            if bridge_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeAddress"));
                            }
                            bridge_address__ = map_.next_value()?;
                        }
                        GeneratedField::RollupBlockNumber => {
                            if rollup_block_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupBlockNumber"));
                            }
                            rollup_block_number__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::RollupWithdrawalEventId => {
                            if rollup_withdrawal_event_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupWithdrawalEventId"));
                            }
                            rollup_withdrawal_event_id__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(BridgeUnlock {
                    to: to__,
                    amount: amount__,
                    fee_asset: fee_asset__.unwrap_or_default(),
                    memo: memo__.unwrap_or_default(),
                    bridge_address: bridge_address__,
                    rollup_block_number: rollup_block_number__.unwrap_or_default(),
                    rollup_withdrawal_event_id: rollup_withdrawal_event_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1alpha1.BridgeUnlock", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for FeeAssetChange {
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1alpha1.FeeAssetChange", len)?;
        if let Some(v) = self.value.as_ref() {
            match v {
                fee_asset_change::Value::Addition(v) => {
                    struct_ser.serialize_field("addition", v)?;
                }
                fee_asset_change::Value::Removal(v) => {
                    struct_ser.serialize_field("removal", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FeeAssetChange {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "addition",
            "removal",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Addition,
            Removal,
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
                            "addition" => Ok(GeneratedField::Addition),
                            "removal" => Ok(GeneratedField::Removal),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FeeAssetChange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1alpha1.FeeAssetChange")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<FeeAssetChange, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut value__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Addition => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("addition"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_asset_change::Value::Addition);
                        }
                        GeneratedField::Removal => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("removal"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_asset_change::Value::Removal);
                        }
                    }
                }
                Ok(FeeAssetChange {
                    value: value__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1alpha1.FeeAssetChange", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for FeeChange {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.fee_components.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1alpha1.FeeChange", len)?;
        if let Some(v) = self.fee_components.as_ref() {
            match v {
                fee_change::FeeComponents::BridgeLock(v) => {
                    struct_ser.serialize_field("bridgeLock", v)?;
                }
                fee_change::FeeComponents::BridgeSudoChange(v) => {
                    struct_ser.serialize_field("bridgeSudoChange", v)?;
                }
                fee_change::FeeComponents::BridgeUnlock(v) => {
                    struct_ser.serialize_field("bridgeUnlock", v)?;
                }
                fee_change::FeeComponents::FeeAssetChange(v) => {
                    struct_ser.serialize_field("feeAssetChange", v)?;
                }
                fee_change::FeeComponents::FeeChange(v) => {
                    struct_ser.serialize_field("feeChange", v)?;
                }
                fee_change::FeeComponents::IbcRelay(v) => {
                    struct_ser.serialize_field("ibcRelay", v)?;
                }
                fee_change::FeeComponents::IbcRelayerChange(v) => {
                    struct_ser.serialize_field("ibcRelayerChange", v)?;
                }
                fee_change::FeeComponents::IbcSudoChange(v) => {
                    struct_ser.serialize_field("ibcSudoChange", v)?;
                }
                fee_change::FeeComponents::Ics20Withdrawal(v) => {
                    struct_ser.serialize_field("ics20Withdrawal", v)?;
                }
                fee_change::FeeComponents::InitBridgeAccount(v) => {
                    struct_ser.serialize_field("initBridgeAccount", v)?;
                }
                fee_change::FeeComponents::RollupDataSubmission(v) => {
                    struct_ser.serialize_field("rollupDataSubmission", v)?;
                }
                fee_change::FeeComponents::SudoAddressChange(v) => {
                    struct_ser.serialize_field("sudoAddressChange", v)?;
                }
                fee_change::FeeComponents::Transfer(v) => {
                    struct_ser.serialize_field("transfer", v)?;
                }
                fee_change::FeeComponents::ValidatorUpdate(v) => {
                    struct_ser.serialize_field("validatorUpdate", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FeeChange {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "bridge_lock",
            "bridgeLock",
            "bridge_sudo_change",
            "bridgeSudoChange",
            "bridge_unlock",
            "bridgeUnlock",
            "fee_asset_change",
            "feeAssetChange",
            "fee_change",
            "feeChange",
            "ibc_relay",
            "ibcRelay",
            "ibc_relayer_change",
            "ibcRelayerChange",
            "ibc_sudo_change",
            "ibcSudoChange",
            "ics20_withdrawal",
            "ics20Withdrawal",
            "init_bridge_account",
            "initBridgeAccount",
            "rollup_data_submission",
            "rollupDataSubmission",
            "sudo_address_change",
            "sudoAddressChange",
            "transfer",
            "validator_update",
            "validatorUpdate",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BridgeLock,
            BridgeSudoChange,
            BridgeUnlock,
            FeeAssetChange,
            FeeChange,
            IbcRelay,
            IbcRelayerChange,
            IbcSudoChange,
            Ics20Withdrawal,
            InitBridgeAccount,
            RollupDataSubmission,
            SudoAddressChange,
            Transfer,
            ValidatorUpdate,
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
                            "bridgeLock" | "bridge_lock" => Ok(GeneratedField::BridgeLock),
                            "bridgeSudoChange" | "bridge_sudo_change" => Ok(GeneratedField::BridgeSudoChange),
                            "bridgeUnlock" | "bridge_unlock" => Ok(GeneratedField::BridgeUnlock),
                            "feeAssetChange" | "fee_asset_change" => Ok(GeneratedField::FeeAssetChange),
                            "feeChange" | "fee_change" => Ok(GeneratedField::FeeChange),
                            "ibcRelay" | "ibc_relay" => Ok(GeneratedField::IbcRelay),
                            "ibcRelayerChange" | "ibc_relayer_change" => Ok(GeneratedField::IbcRelayerChange),
                            "ibcSudoChange" | "ibc_sudo_change" => Ok(GeneratedField::IbcSudoChange),
                            "ics20Withdrawal" | "ics20_withdrawal" => Ok(GeneratedField::Ics20Withdrawal),
                            "initBridgeAccount" | "init_bridge_account" => Ok(GeneratedField::InitBridgeAccount),
                            "rollupDataSubmission" | "rollup_data_submission" => Ok(GeneratedField::RollupDataSubmission),
                            "sudoAddressChange" | "sudo_address_change" => Ok(GeneratedField::SudoAddressChange),
                            "transfer" => Ok(GeneratedField::Transfer),
                            "validatorUpdate" | "validator_update" => Ok(GeneratedField::ValidatorUpdate),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FeeChange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1alpha1.FeeChange")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<FeeChange, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut fee_components__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BridgeLock => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeLock"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::BridgeLock)
;
                        }
                        GeneratedField::BridgeSudoChange => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeSudoChange"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::BridgeSudoChange)
;
                        }
                        GeneratedField::BridgeUnlock => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeUnlock"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::BridgeUnlock)
;
                        }
                        GeneratedField::FeeAssetChange => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeAssetChange"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::FeeAssetChange)
;
                        }
                        GeneratedField::FeeChange => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeChange"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::FeeChange)
;
                        }
                        GeneratedField::IbcRelay => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ibcRelay"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::IbcRelay)
;
                        }
                        GeneratedField::IbcRelayerChange => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ibcRelayerChange"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::IbcRelayerChange)
;
                        }
                        GeneratedField::IbcSudoChange => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ibcSudoChange"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::IbcSudoChange)
;
                        }
                        GeneratedField::Ics20Withdrawal => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ics20Withdrawal"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::Ics20Withdrawal)
;
                        }
                        GeneratedField::InitBridgeAccount => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("initBridgeAccount"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::InitBridgeAccount)
;
                        }
                        GeneratedField::RollupDataSubmission => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupDataSubmission"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::RollupDataSubmission)
;
                        }
                        GeneratedField::SudoAddressChange => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sudoAddressChange"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::SudoAddressChange)
;
                        }
                        GeneratedField::Transfer => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transfer"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::Transfer)
;
                        }
                        GeneratedField::ValidatorUpdate => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("validatorUpdate"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::ValidatorUpdate)
;
                        }
                    }
                }
                Ok(FeeChange {
                    fee_components: fee_components__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1alpha1.FeeChange", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for IbcHeight {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.revision_number != 0 {
            len += 1;
        }
        if self.revision_height != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1alpha1.IbcHeight", len)?;
        if self.revision_number != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("revisionNumber", ToString::to_string(&self.revision_number).as_str())?;
        }
        if self.revision_height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("revisionHeight", ToString::to_string(&self.revision_height).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for IbcHeight {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "revision_number",
            "revisionNumber",
            "revision_height",
            "revisionHeight",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RevisionNumber,
            RevisionHeight,
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
                            "revisionNumber" | "revision_number" => Ok(GeneratedField::RevisionNumber),
                            "revisionHeight" | "revision_height" => Ok(GeneratedField::RevisionHeight),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = IbcHeight;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1alpha1.IbcHeight")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<IbcHeight, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut revision_number__ = None;
                let mut revision_height__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RevisionNumber => {
                            if revision_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("revisionNumber"));
                            }
                            revision_number__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::RevisionHeight => {
                            if revision_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("revisionHeight"));
                            }
                            revision_height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(IbcHeight {
                    revision_number: revision_number__.unwrap_or_default(),
                    revision_height: revision_height__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1alpha1.IbcHeight", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for IbcRelayerChange {
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1alpha1.IbcRelayerChange", len)?;
        if let Some(v) = self.value.as_ref() {
            match v {
                ibc_relayer_change::Value::Addition(v) => {
                    struct_ser.serialize_field("addition", v)?;
                }
                ibc_relayer_change::Value::Removal(v) => {
                    struct_ser.serialize_field("removal", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for IbcRelayerChange {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "addition",
            "removal",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Addition,
            Removal,
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
                            "addition" => Ok(GeneratedField::Addition),
                            "removal" => Ok(GeneratedField::Removal),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = IbcRelayerChange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1alpha1.IbcRelayerChange")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<IbcRelayerChange, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut value__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Addition => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("addition"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(ibc_relayer_change::Value::Addition)
;
                        }
                        GeneratedField::Removal => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("removal"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(ibc_relayer_change::Value::Removal)
;
                        }
                    }
                }
                Ok(IbcRelayerChange {
                    value: value__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1alpha1.IbcRelayerChange", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for IbcSudoChange {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.new_address.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1alpha1.IbcSudoChange", len)?;
        if let Some(v) = self.new_address.as_ref() {
            struct_ser.serialize_field("newAddress", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for IbcSudoChange {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "new_address",
            "newAddress",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            NewAddress,
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
                            "newAddress" | "new_address" => Ok(GeneratedField::NewAddress),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = IbcSudoChange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1alpha1.IbcSudoChange")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<IbcSudoChange, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut new_address__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::NewAddress => {
                            if new_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("newAddress"));
                            }
                            new_address__ = map_.next_value()?;
                        }
                    }
                }
                Ok(IbcSudoChange {
                    new_address: new_address__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1alpha1.IbcSudoChange", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Ics20Withdrawal {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.amount.is_some() {
            len += 1;
        }
        if !self.denom.is_empty() {
            len += 1;
        }
        if !self.destination_chain_address.is_empty() {
            len += 1;
        }
        if self.return_address.is_some() {
            len += 1;
        }
        if self.timeout_height.is_some() {
            len += 1;
        }
        if self.timeout_time != 0 {
            len += 1;
        }
        if !self.source_channel.is_empty() {
            len += 1;
        }
        if !self.fee_asset.is_empty() {
            len += 1;
        }
        if !self.memo.is_empty() {
            len += 1;
        }
        if self.bridge_address.is_some() {
            len += 1;
        }
        if self.use_compat_address {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1alpha1.Ics20Withdrawal", len)?;
        if let Some(v) = self.amount.as_ref() {
            struct_ser.serialize_field("amount", v)?;
        }
        if !self.denom.is_empty() {
            struct_ser.serialize_field("denom", &self.denom)?;
        }
        if !self.destination_chain_address.is_empty() {
            struct_ser.serialize_field("destinationChainAddress", &self.destination_chain_address)?;
        }
        if let Some(v) = self.return_address.as_ref() {
            struct_ser.serialize_field("returnAddress", v)?;
        }
        if let Some(v) = self.timeout_height.as_ref() {
            struct_ser.serialize_field("timeoutHeight", v)?;
        }
        if self.timeout_time != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("timeoutTime", ToString::to_string(&self.timeout_time).as_str())?;
        }
        if !self.source_channel.is_empty() {
            struct_ser.serialize_field("sourceChannel", &self.source_channel)?;
        }
        if !self.fee_asset.is_empty() {
            struct_ser.serialize_field("feeAsset", &self.fee_asset)?;
        }
        if !self.memo.is_empty() {
            struct_ser.serialize_field("memo", &self.memo)?;
        }
        if let Some(v) = self.bridge_address.as_ref() {
            struct_ser.serialize_field("bridgeAddress", v)?;
        }
        if self.use_compat_address {
            struct_ser.serialize_field("useCompatAddress", &self.use_compat_address)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Ics20Withdrawal {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "amount",
            "denom",
            "destination_chain_address",
            "destinationChainAddress",
            "return_address",
            "returnAddress",
            "timeout_height",
            "timeoutHeight",
            "timeout_time",
            "timeoutTime",
            "source_channel",
            "sourceChannel",
            "fee_asset",
            "feeAsset",
            "memo",
            "bridge_address",
            "bridgeAddress",
            "use_compat_address",
            "useCompatAddress",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Amount,
            Denom,
            DestinationChainAddress,
            ReturnAddress,
            TimeoutHeight,
            TimeoutTime,
            SourceChannel,
            FeeAsset,
            Memo,
            BridgeAddress,
            UseCompatAddress,
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
                            "amount" => Ok(GeneratedField::Amount),
                            "denom" => Ok(GeneratedField::Denom),
                            "destinationChainAddress" | "destination_chain_address" => Ok(GeneratedField::DestinationChainAddress),
                            "returnAddress" | "return_address" => Ok(GeneratedField::ReturnAddress),
                            "timeoutHeight" | "timeout_height" => Ok(GeneratedField::TimeoutHeight),
                            "timeoutTime" | "timeout_time" => Ok(GeneratedField::TimeoutTime),
                            "sourceChannel" | "source_channel" => Ok(GeneratedField::SourceChannel),
                            "feeAsset" | "fee_asset" => Ok(GeneratedField::FeeAsset),
                            "memo" => Ok(GeneratedField::Memo),
                            "bridgeAddress" | "bridge_address" => Ok(GeneratedField::BridgeAddress),
                            "useCompatAddress" | "use_compat_address" => Ok(GeneratedField::UseCompatAddress),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Ics20Withdrawal;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1alpha1.Ics20Withdrawal")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Ics20Withdrawal, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut amount__ = None;
                let mut denom__ = None;
                let mut destination_chain_address__ = None;
                let mut return_address__ = None;
                let mut timeout_height__ = None;
                let mut timeout_time__ = None;
                let mut source_channel__ = None;
                let mut fee_asset__ = None;
                let mut memo__ = None;
                let mut bridge_address__ = None;
                let mut use_compat_address__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Amount => {
                            if amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("amount"));
                            }
                            amount__ = map_.next_value()?;
                        }
                        GeneratedField::Denom => {
                            if denom__.is_some() {
                                return Err(serde::de::Error::duplicate_field("denom"));
                            }
                            denom__ = Some(map_.next_value()?);
                        }
                        GeneratedField::DestinationChainAddress => {
                            if destination_chain_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("destinationChainAddress"));
                            }
                            destination_chain_address__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ReturnAddress => {
                            if return_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("returnAddress"));
                            }
                            return_address__ = map_.next_value()?;
                        }
                        GeneratedField::TimeoutHeight => {
                            if timeout_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timeoutHeight"));
                            }
                            timeout_height__ = map_.next_value()?;
                        }
                        GeneratedField::TimeoutTime => {
                            if timeout_time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timeoutTime"));
                            }
                            timeout_time__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::SourceChannel => {
                            if source_channel__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sourceChannel"));
                            }
                            source_channel__ = Some(map_.next_value()?);
                        }
                        GeneratedField::FeeAsset => {
                            if fee_asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeAsset"));
                            }
                            fee_asset__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Memo => {
                            if memo__.is_some() {
                                return Err(serde::de::Error::duplicate_field("memo"));
                            }
                            memo__ = Some(map_.next_value()?);
                        }
                        GeneratedField::BridgeAddress => {
                            if bridge_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeAddress"));
                            }
                            bridge_address__ = map_.next_value()?;
                        }
                        GeneratedField::UseCompatAddress => {
                            if use_compat_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("useCompatAddress"));
                            }
                            use_compat_address__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Ics20Withdrawal {
                    amount: amount__,
                    denom: denom__.unwrap_or_default(),
                    destination_chain_address: destination_chain_address__.unwrap_or_default(),
                    return_address: return_address__,
                    timeout_height: timeout_height__,
                    timeout_time: timeout_time__.unwrap_or_default(),
                    source_channel: source_channel__.unwrap_or_default(),
                    fee_asset: fee_asset__.unwrap_or_default(),
                    memo: memo__.unwrap_or_default(),
                    bridge_address: bridge_address__,
                    use_compat_address: use_compat_address__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1alpha1.Ics20Withdrawal", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for InitBridgeAccount {
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
        if !self.asset.is_empty() {
            len += 1;
        }
        if !self.fee_asset.is_empty() {
            len += 1;
        }
        if self.sudo_address.is_some() {
            len += 1;
        }
        if self.withdrawer_address.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1alpha1.InitBridgeAccount", len)?;
        if let Some(v) = self.rollup_id.as_ref() {
            struct_ser.serialize_field("rollupId", v)?;
        }
        if !self.asset.is_empty() {
            struct_ser.serialize_field("asset", &self.asset)?;
        }
        if !self.fee_asset.is_empty() {
            struct_ser.serialize_field("feeAsset", &self.fee_asset)?;
        }
        if let Some(v) = self.sudo_address.as_ref() {
            struct_ser.serialize_field("sudoAddress", v)?;
        }
        if let Some(v) = self.withdrawer_address.as_ref() {
            struct_ser.serialize_field("withdrawerAddress", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for InitBridgeAccount {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "rollup_id",
            "rollupId",
            "asset",
            "fee_asset",
            "feeAsset",
            "sudo_address",
            "sudoAddress",
            "withdrawer_address",
            "withdrawerAddress",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RollupId,
            Asset,
            FeeAsset,
            SudoAddress,
            WithdrawerAddress,
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
                            "asset" => Ok(GeneratedField::Asset),
                            "feeAsset" | "fee_asset" => Ok(GeneratedField::FeeAsset),
                            "sudoAddress" | "sudo_address" => Ok(GeneratedField::SudoAddress),
                            "withdrawerAddress" | "withdrawer_address" => Ok(GeneratedField::WithdrawerAddress),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = InitBridgeAccount;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1alpha1.InitBridgeAccount")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<InitBridgeAccount, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut rollup_id__ = None;
                let mut asset__ = None;
                let mut fee_asset__ = None;
                let mut sudo_address__ = None;
                let mut withdrawer_address__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RollupId => {
                            if rollup_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupId"));
                            }
                            rollup_id__ = map_.next_value()?;
                        }
                        GeneratedField::Asset => {
                            if asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("asset"));
                            }
                            asset__ = Some(map_.next_value()?);
                        }
                        GeneratedField::FeeAsset => {
                            if fee_asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeAsset"));
                            }
                            fee_asset__ = Some(map_.next_value()?);
                        }
                        GeneratedField::SudoAddress => {
                            if sudo_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sudoAddress"));
                            }
                            sudo_address__ = map_.next_value()?;
                        }
                        GeneratedField::WithdrawerAddress => {
                            if withdrawer_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("withdrawerAddress"));
                            }
                            withdrawer_address__ = map_.next_value()?;
                        }
                    }
                }
                Ok(InitBridgeAccount {
                    rollup_id: rollup_id__,
                    asset: asset__.unwrap_or_default(),
                    fee_asset: fee_asset__.unwrap_or_default(),
                    sudo_address: sudo_address__,
                    withdrawer_address: withdrawer_address__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1alpha1.InitBridgeAccount", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RollupDataSubmission {
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
        if !self.data.is_empty() {
            len += 1;
        }
        if !self.fee_asset.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1alpha1.RollupDataSubmission", len)?;
        if let Some(v) = self.rollup_id.as_ref() {
            struct_ser.serialize_field("rollupId", v)?;
        }
        if !self.data.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("data", pbjson::private::base64::encode(&self.data).as_str())?;
        }
        if !self.fee_asset.is_empty() {
            struct_ser.serialize_field("feeAsset", &self.fee_asset)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RollupDataSubmission {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "rollup_id",
            "rollupId",
            "data",
            "fee_asset",
            "feeAsset",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RollupId,
            Data,
            FeeAsset,
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
                            "data" => Ok(GeneratedField::Data),
                            "feeAsset" | "fee_asset" => Ok(GeneratedField::FeeAsset),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RollupDataSubmission;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1alpha1.RollupDataSubmission")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RollupDataSubmission, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut rollup_id__ = None;
                let mut data__ = None;
                let mut fee_asset__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RollupId => {
                            if rollup_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupId"));
                            }
                            rollup_id__ = map_.next_value()?;
                        }
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::FeeAsset => {
                            if fee_asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeAsset"));
                            }
                            fee_asset__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(RollupDataSubmission {
                    rollup_id: rollup_id__,
                    data: data__.unwrap_or_default(),
                    fee_asset: fee_asset__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1alpha1.RollupDataSubmission", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SudoAddressChange {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.new_address.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1alpha1.SudoAddressChange", len)?;
        if let Some(v) = self.new_address.as_ref() {
            struct_ser.serialize_field("newAddress", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SudoAddressChange {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "new_address",
            "newAddress",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            NewAddress,
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
                            "newAddress" | "new_address" => Ok(GeneratedField::NewAddress),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SudoAddressChange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1alpha1.SudoAddressChange")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SudoAddressChange, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut new_address__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::NewAddress => {
                            if new_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("newAddress"));
                            }
                            new_address__ = map_.next_value()?;
                        }
                    }
                }
                Ok(SudoAddressChange {
                    new_address: new_address__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1alpha1.SudoAddressChange", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Transaction {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.signature.is_empty() {
            len += 1;
        }
        if !self.public_key.is_empty() {
            len += 1;
        }
        if self.body.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1alpha1.Transaction", len)?;
        if !self.signature.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("signature", pbjson::private::base64::encode(&self.signature).as_str())?;
        }
        if !self.public_key.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("publicKey", pbjson::private::base64::encode(&self.public_key).as_str())?;
        }
        if let Some(v) = self.body.as_ref() {
            struct_ser.serialize_field("body", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Transaction {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "signature",
            "public_key",
            "publicKey",
            "body",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Signature,
            PublicKey,
            Body,
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
                            "signature" => Ok(GeneratedField::Signature),
                            "publicKey" | "public_key" => Ok(GeneratedField::PublicKey),
                            "body" => Ok(GeneratedField::Body),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Transaction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1alpha1.Transaction")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Transaction, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut signature__ = None;
                let mut public_key__ = None;
                let mut body__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Signature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signature"));
                            }
                            signature__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PublicKey => {
                            if public_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("publicKey"));
                            }
                            public_key__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Body => {
                            if body__.is_some() {
                                return Err(serde::de::Error::duplicate_field("body"));
                            }
                            body__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Transaction {
                    signature: signature__.unwrap_or_default(),
                    public_key: public_key__.unwrap_or_default(),
                    body: body__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1alpha1.Transaction", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TransactionBody {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.params.is_some() {
            len += 1;
        }
        if !self.actions.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1alpha1.TransactionBody", len)?;
        if let Some(v) = self.params.as_ref() {
            struct_ser.serialize_field("params", v)?;
        }
        if !self.actions.is_empty() {
            struct_ser.serialize_field("actions", &self.actions)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionBody {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "params",
            "actions",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Params,
            Actions,
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
                            "params" => Ok(GeneratedField::Params),
                            "actions" => Ok(GeneratedField::Actions),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionBody;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1alpha1.TransactionBody")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TransactionBody, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut params__ = None;
                let mut actions__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Params => {
                            if params__.is_some() {
                                return Err(serde::de::Error::duplicate_field("params"));
                            }
                            params__ = map_.next_value()?;
                        }
                        GeneratedField::Actions => {
                            if actions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("actions"));
                            }
                            actions__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(TransactionBody {
                    params: params__,
                    actions: actions__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1alpha1.TransactionBody", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TransactionParams {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.nonce != 0 {
            len += 1;
        }
        if !self.chain_id.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1alpha1.TransactionParams", len)?;
        if self.nonce != 0 {
            struct_ser.serialize_field("nonce", &self.nonce)?;
        }
        if !self.chain_id.is_empty() {
            struct_ser.serialize_field("chainId", &self.chain_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionParams {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "nonce",
            "chain_id",
            "chainId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Nonce,
            ChainId,
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
                            "nonce" => Ok(GeneratedField::Nonce),
                            "chainId" | "chain_id" => Ok(GeneratedField::ChainId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionParams;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1alpha1.TransactionParams")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TransactionParams, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut nonce__ = None;
                let mut chain_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Nonce => {
                            if nonce__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nonce"));
                            }
                            nonce__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ChainId => {
                            if chain_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("chainId"));
                            }
                            chain_id__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(TransactionParams {
                    nonce: nonce__.unwrap_or_default(),
                    chain_id: chain_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1alpha1.TransactionParams", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Transfer {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.to.is_some() {
            len += 1;
        }
        if self.amount.is_some() {
            len += 1;
        }
        if !self.asset.is_empty() {
            len += 1;
        }
        if !self.fee_asset.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1alpha1.Transfer", len)?;
        if let Some(v) = self.to.as_ref() {
            struct_ser.serialize_field("to", v)?;
        }
        if let Some(v) = self.amount.as_ref() {
            struct_ser.serialize_field("amount", v)?;
        }
        if !self.asset.is_empty() {
            struct_ser.serialize_field("asset", &self.asset)?;
        }
        if !self.fee_asset.is_empty() {
            struct_ser.serialize_field("feeAsset", &self.fee_asset)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Transfer {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "to",
            "amount",
            "asset",
            "fee_asset",
            "feeAsset",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            To,
            Amount,
            Asset,
            FeeAsset,
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
                            "to" => Ok(GeneratedField::To),
                            "amount" => Ok(GeneratedField::Amount),
                            "asset" => Ok(GeneratedField::Asset),
                            "feeAsset" | "fee_asset" => Ok(GeneratedField::FeeAsset),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Transfer;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1alpha1.Transfer")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Transfer, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut to__ = None;
                let mut amount__ = None;
                let mut asset__ = None;
                let mut fee_asset__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::To => {
                            if to__.is_some() {
                                return Err(serde::de::Error::duplicate_field("to"));
                            }
                            to__ = map_.next_value()?;
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
                        GeneratedField::FeeAsset => {
                            if fee_asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeAsset"));
                            }
                            fee_asset__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Transfer {
                    to: to__,
                    amount: amount__,
                    asset: asset__.unwrap_or_default(),
                    fee_asset: fee_asset__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1alpha1.Transfer", FIELDS, GeneratedVisitor)
    }
}
