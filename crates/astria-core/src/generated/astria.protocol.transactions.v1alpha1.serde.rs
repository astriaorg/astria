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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.Action", len)?;
        if let Some(v) = self.value.as_ref() {
            match v {
                action::Value::Transfer(v) => {
                    struct_ser.serialize_field("transfer", v)?;
                }
                action::Value::Sequence(v) => {
                    struct_ser.serialize_field("sequence", v)?;
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
            "sequence",
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
            Sequence,
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
                            "sequence" => Ok(GeneratedField::Sequence),
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
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.Action")
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
                        GeneratedField::Sequence => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequence"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::Sequence)
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
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.Action", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.BridgeLock", len)?;
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
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.BridgeLock")
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
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.BridgeLock", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.BridgeSudoChange", len)?;
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
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.BridgeSudoChange")
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
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.BridgeSudoChange", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.BridgeUnlock", len)?;
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
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.BridgeUnlock")
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
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.BridgeUnlock", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.FeeAssetChange", len)?;
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
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.FeeAssetChange")
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
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.FeeAssetChange", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.FeeChange", len)?;
        if let Some(v) = self.fee_components.as_ref() {
            match v {
                fee_change::FeeComponents::TransferFees(v) => {
                    struct_ser.serialize_field("transferFees", v)?;
                }
                fee_change::FeeComponents::SequenceFees(v) => {
                    struct_ser.serialize_field("sequenceFees", v)?;
                }
                fee_change::FeeComponents::InitBridgeAccountFees(v) => {
                    struct_ser.serialize_field("initBridgeAccountFees", v)?;
                }
                fee_change::FeeComponents::BridgeLockFees(v) => {
                    struct_ser.serialize_field("bridgeLockFees", v)?;
                }
                fee_change::FeeComponents::BridgeUnlockFees(v) => {
                    struct_ser.serialize_field("bridgeUnlockFees", v)?;
                }
                fee_change::FeeComponents::BridgeSudoChangeFees(v) => {
                    struct_ser.serialize_field("bridgeSudoChangeFees", v)?;
                }
                fee_change::FeeComponents::Ics20WithdrawalFees(v) => {
                    struct_ser.serialize_field("ics20WithdrawalFees", v)?;
                }
                fee_change::FeeComponents::IbcRelayFees(v) => {
                    struct_ser.serialize_field("ibcRelayFees", v)?;
                }
                fee_change::FeeComponents::ValidatorUpdateFees(v) => {
                    struct_ser.serialize_field("validatorUpdateFees", v)?;
                }
                fee_change::FeeComponents::FeeAssetChangeFees(v) => {
                    struct_ser.serialize_field("feeAssetChangeFees", v)?;
                }
                fee_change::FeeComponents::FeeChangeFees(v) => {
                    struct_ser.serialize_field("feeChangeFees", v)?;
                }
                fee_change::FeeComponents::IbcRelayerChangeFees(v) => {
                    struct_ser.serialize_field("ibcRelayerChangeFees", v)?;
                }
                fee_change::FeeComponents::SudoAddressChangeFees(v) => {
                    struct_ser.serialize_field("sudoAddressChangeFees", v)?;
                }
                fee_change::FeeComponents::IbcSudoChangeFees(v) => {
                    struct_ser.serialize_field("ibcSudoChangeFees", v)?;
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
            "transfer_fees",
            "transferFees",
            "sequence_fees",
            "sequenceFees",
            "init_bridge_account_fees",
            "initBridgeAccountFees",
            "bridge_lock_fees",
            "bridgeLockFees",
            "bridge_unlock_fees",
            "bridgeUnlockFees",
            "bridge_sudo_change_fees",
            "bridgeSudoChangeFees",
            "ics20_withdrawal_fees",
            "ics20WithdrawalFees",
            "ibc_relay_fees",
            "ibcRelayFees",
            "validator_update_fees",
            "validatorUpdateFees",
            "fee_asset_change_fees",
            "feeAssetChangeFees",
            "fee_change_fees",
            "feeChangeFees",
            "ibc_relayer_change_fees",
            "ibcRelayerChangeFees",
            "sudo_address_change_fees",
            "sudoAddressChangeFees",
            "ibc_sudo_change_fees",
            "ibcSudoChangeFees",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TransferFees,
            SequenceFees,
            InitBridgeAccountFees,
            BridgeLockFees,
            BridgeUnlockFees,
            BridgeSudoChangeFees,
            Ics20WithdrawalFees,
            IbcRelayFees,
            ValidatorUpdateFees,
            FeeAssetChangeFees,
            FeeChangeFees,
            IbcRelayerChangeFees,
            SudoAddressChangeFees,
            IbcSudoChangeFees,
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
                            "transferFees" | "transfer_fees" => Ok(GeneratedField::TransferFees),
                            "sequenceFees" | "sequence_fees" => Ok(GeneratedField::SequenceFees),
                            "initBridgeAccountFees" | "init_bridge_account_fees" => Ok(GeneratedField::InitBridgeAccountFees),
                            "bridgeLockFees" | "bridge_lock_fees" => Ok(GeneratedField::BridgeLockFees),
                            "bridgeUnlockFees" | "bridge_unlock_fees" => Ok(GeneratedField::BridgeUnlockFees),
                            "bridgeSudoChangeFees" | "bridge_sudo_change_fees" => Ok(GeneratedField::BridgeSudoChangeFees),
                            "ics20WithdrawalFees" | "ics20_withdrawal_fees" => Ok(GeneratedField::Ics20WithdrawalFees),
                            "ibcRelayFees" | "ibc_relay_fees" => Ok(GeneratedField::IbcRelayFees),
                            "validatorUpdateFees" | "validator_update_fees" => Ok(GeneratedField::ValidatorUpdateFees),
                            "feeAssetChangeFees" | "fee_asset_change_fees" => Ok(GeneratedField::FeeAssetChangeFees),
                            "feeChangeFees" | "fee_change_fees" => Ok(GeneratedField::FeeChangeFees),
                            "ibcRelayerChangeFees" | "ibc_relayer_change_fees" => Ok(GeneratedField::IbcRelayerChangeFees),
                            "sudoAddressChangeFees" | "sudo_address_change_fees" => Ok(GeneratedField::SudoAddressChangeFees),
                            "ibcSudoChangeFees" | "ibc_sudo_change_fees" => Ok(GeneratedField::IbcSudoChangeFees),
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
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.FeeChange")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<FeeChange, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut fee_components__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TransferFees => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transferFees"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::TransferFees)
;
                        }
                        GeneratedField::SequenceFees => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequenceFees"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::SequenceFees)
;
                        }
                        GeneratedField::InitBridgeAccountFees => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("initBridgeAccountFees"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::InitBridgeAccountFees)
;
                        }
                        GeneratedField::BridgeLockFees => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeLockFees"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::BridgeLockFees)
;
                        }
                        GeneratedField::BridgeUnlockFees => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeUnlockFees"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::BridgeUnlockFees)
;
                        }
                        GeneratedField::BridgeSudoChangeFees => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeSudoChangeFees"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::BridgeSudoChangeFees)
;
                        }
                        GeneratedField::Ics20WithdrawalFees => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ics20WithdrawalFees"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::Ics20WithdrawalFees)
;
                        }
                        GeneratedField::IbcRelayFees => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ibcRelayFees"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::IbcRelayFees)
;
                        }
                        GeneratedField::ValidatorUpdateFees => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("validatorUpdateFees"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::ValidatorUpdateFees)
;
                        }
                        GeneratedField::FeeAssetChangeFees => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeAssetChangeFees"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::FeeAssetChangeFees)
;
                        }
                        GeneratedField::FeeChangeFees => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeChangeFees"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::FeeChangeFees)
;
                        }
                        GeneratedField::IbcRelayerChangeFees => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ibcRelayerChangeFees"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::IbcRelayerChangeFees)
;
                        }
                        GeneratedField::SudoAddressChangeFees => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sudoAddressChangeFees"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::SudoAddressChangeFees)
;
                        }
                        GeneratedField::IbcSudoChangeFees => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ibcSudoChangeFees"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::IbcSudoChangeFees)
;
                        }
                    }
                }
                Ok(FeeChange {
                    fee_components: fee_components__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.FeeChange", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.IbcHeight", len)?;
        if self.revision_number != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("revisionNumber", ToString::to_string(&self.revision_number).as_str())?;
        }
        if self.revision_height != 0 {
            #[allow(clippy::needless_borrow)]
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
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.IbcHeight")
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
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.IbcHeight", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.IbcRelayerChange", len)?;
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
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.IbcRelayerChange")
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
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.IbcRelayerChange", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.IbcSudoChange", len)?;
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
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.IbcSudoChange")
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
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.IbcSudoChange", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.Ics20Withdrawal", len)?;
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
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.Ics20Withdrawal")
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
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.Ics20Withdrawal", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.InitBridgeAccount", len)?;
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
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.InitBridgeAccount")
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
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.InitBridgeAccount", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Sequence {
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.Sequence", len)?;
        if let Some(v) = self.rollup_id.as_ref() {
            struct_ser.serialize_field("rollupId", v)?;
        }
        if !self.data.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("data", pbjson::private::base64::encode(&self.data).as_str())?;
        }
        if !self.fee_asset.is_empty() {
            struct_ser.serialize_field("feeAsset", &self.fee_asset)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Sequence {
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
            type Value = Sequence;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.Sequence")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Sequence, V::Error>
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
                Ok(Sequence {
                    rollup_id: rollup_id__,
                    data: data__.unwrap_or_default(),
                    fee_asset: fee_asset__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.Sequence", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SignedTransaction {
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
        if self.transaction.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.SignedTransaction", len)?;
        if !self.signature.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("signature", pbjson::private::base64::encode(&self.signature).as_str())?;
        }
        if !self.public_key.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("publicKey", pbjson::private::base64::encode(&self.public_key).as_str())?;
        }
        if let Some(v) = self.transaction.as_ref() {
            struct_ser.serialize_field("transaction", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SignedTransaction {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "signature",
            "public_key",
            "publicKey",
            "transaction",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Signature,
            PublicKey,
            Transaction,
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
                            "transaction" => Ok(GeneratedField::Transaction),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SignedTransaction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.SignedTransaction")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SignedTransaction, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut signature__ = None;
                let mut public_key__ = None;
                let mut transaction__ = None;
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
                        GeneratedField::Transaction => {
                            if transaction__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transaction"));
                            }
                            transaction__ = map_.next_value()?;
                        }
                    }
                }
                Ok(SignedTransaction {
                    signature: signature__.unwrap_or_default(),
                    public_key: public_key__.unwrap_or_default(),
                    transaction: transaction__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.SignedTransaction", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.SudoAddressChange", len)?;
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
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.SudoAddressChange")
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
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.SudoAddressChange", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TransactionFeeResponse {
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
        if !self.fees.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.TransactionFeeResponse", len)?;
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        if !self.fees.is_empty() {
            struct_ser.serialize_field("fees", &self.fees)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionFeeResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "fees",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            Fees,
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
                            "fees" => Ok(GeneratedField::Fees),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionFeeResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.TransactionFeeResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TransactionFeeResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut fees__ = None;
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
                        GeneratedField::Fees => {
                            if fees__.is_some() {
                                return Err(serde::de::Error::duplicate_field("fees"));
                            }
                            fees__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(TransactionFeeResponse {
                    height: height__.unwrap_or_default(),
                    fees: fees__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.TransactionFeeResponse", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.TransactionParams", len)?;
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
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.TransactionParams")
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
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.TransactionParams", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.Transfer", len)?;
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
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.Transfer")
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
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.Transfer", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for UnsignedTransaction {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.actions.is_empty() {
            len += 1;
        }
        if self.params.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.UnsignedTransaction", len)?;
        if !self.actions.is_empty() {
            struct_ser.serialize_field("actions", &self.actions)?;
        }
        if let Some(v) = self.params.as_ref() {
            struct_ser.serialize_field("params", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for UnsignedTransaction {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "actions",
            "params",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Actions,
            Params,
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
                            "actions" => Ok(GeneratedField::Actions),
                            "params" => Ok(GeneratedField::Params),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = UnsignedTransaction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.UnsignedTransaction")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<UnsignedTransaction, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut actions__ = None;
                let mut params__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Actions => {
                            if actions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("actions"));
                            }
                            actions__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Params => {
                            if params__.is_some() {
                                return Err(serde::de::Error::duplicate_field("params"));
                            }
                            params__ = map_.next_value()?;
                        }
                    }
                }
                Ok(UnsignedTransaction {
                    actions: actions__.unwrap_or_default(),
                    params: params__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.UnsignedTransaction", FIELDS, GeneratedVisitor)
    }
}
