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
                action::Value::TransferAction(v) => {
                    struct_ser.serialize_field("transferAction", v)?;
                }
                action::Value::SequenceAction(v) => {
                    struct_ser.serialize_field("sequenceAction", v)?;
                }
                action::Value::InitBridgeAccountAction(v) => {
                    struct_ser.serialize_field("initBridgeAccountAction", v)?;
                }
                action::Value::BridgeLockAction(v) => {
                    struct_ser.serialize_field("bridgeLockAction", v)?;
                }
                action::Value::BridgeUnlockAction(v) => {
                    struct_ser.serialize_field("bridgeUnlockAction", v)?;
                }
                action::Value::BridgeSudoChangeAction(v) => {
                    struct_ser.serialize_field("bridgeSudoChangeAction", v)?;
                }
                action::Value::IbcAction(v) => {
                    struct_ser.serialize_field("ibcAction", v)?;
                }
                action::Value::Ics20Withdrawal(v) => {
                    struct_ser.serialize_field("ics20Withdrawal", v)?;
                }
                action::Value::SudoAddressChangeAction(v) => {
                    struct_ser.serialize_field("sudoAddressChangeAction", v)?;
                }
                action::Value::ValidatorUpdateAction(v) => {
                    struct_ser.serialize_field("validatorUpdateAction", v)?;
                }
                action::Value::IbcRelayerChangeAction(v) => {
                    struct_ser.serialize_field("ibcRelayerChangeAction", v)?;
                }
                action::Value::FeeAssetChangeAction(v) => {
                    struct_ser.serialize_field("feeAssetChangeAction", v)?;
                }
                action::Value::FeeChangeAction(v) => {
                    struct_ser.serialize_field("feeChangeAction", v)?;
                }
                action::Value::IbcSudoChangeAction(v) => {
                    struct_ser.serialize_field("ibcSudoChangeAction", v)?;
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
            "transfer_action",
            "transferAction",
            "sequence_action",
            "sequenceAction",
            "init_bridge_account_action",
            "initBridgeAccountAction",
            "bridge_lock_action",
            "bridgeLockAction",
            "bridge_unlock_action",
            "bridgeUnlockAction",
            "bridge_sudo_change_action",
            "bridgeSudoChangeAction",
            "ibc_action",
            "ibcAction",
            "ics20_withdrawal",
            "ics20Withdrawal",
            "sudo_address_change_action",
            "sudoAddressChangeAction",
            "validator_update_action",
            "validatorUpdateAction",
            "ibc_relayer_change_action",
            "ibcRelayerChangeAction",
            "fee_asset_change_action",
            "feeAssetChangeAction",
            "fee_change_action",
            "feeChangeAction",
            "ibc_sudo_change_action",
            "ibcSudoChangeAction",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TransferAction,
            SequenceAction,
            InitBridgeAccountAction,
            BridgeLockAction,
            BridgeUnlockAction,
            BridgeSudoChangeAction,
            IbcAction,
            Ics20Withdrawal,
            SudoAddressChangeAction,
            ValidatorUpdateAction,
            IbcRelayerChangeAction,
            FeeAssetChangeAction,
            FeeChangeAction,
            IbcSudoChangeAction,
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
                            "transferAction" | "transfer_action" => Ok(GeneratedField::TransferAction),
                            "sequenceAction" | "sequence_action" => Ok(GeneratedField::SequenceAction),
                            "initBridgeAccountAction" | "init_bridge_account_action" => Ok(GeneratedField::InitBridgeAccountAction),
                            "bridgeLockAction" | "bridge_lock_action" => Ok(GeneratedField::BridgeLockAction),
                            "bridgeUnlockAction" | "bridge_unlock_action" => Ok(GeneratedField::BridgeUnlockAction),
                            "bridgeSudoChangeAction" | "bridge_sudo_change_action" => Ok(GeneratedField::BridgeSudoChangeAction),
                            "ibcAction" | "ibc_action" => Ok(GeneratedField::IbcAction),
                            "ics20Withdrawal" | "ics20_withdrawal" => Ok(GeneratedField::Ics20Withdrawal),
                            "sudoAddressChangeAction" | "sudo_address_change_action" => Ok(GeneratedField::SudoAddressChangeAction),
                            "validatorUpdateAction" | "validator_update_action" => Ok(GeneratedField::ValidatorUpdateAction),
                            "ibcRelayerChangeAction" | "ibc_relayer_change_action" => Ok(GeneratedField::IbcRelayerChangeAction),
                            "feeAssetChangeAction" | "fee_asset_change_action" => Ok(GeneratedField::FeeAssetChangeAction),
                            "feeChangeAction" | "fee_change_action" => Ok(GeneratedField::FeeChangeAction),
                            "ibcSudoChangeAction" | "ibc_sudo_change_action" => Ok(GeneratedField::IbcSudoChangeAction),
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
                        GeneratedField::TransferAction => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transferAction"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::TransferAction)
;
                        }
                        GeneratedField::SequenceAction => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequenceAction"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::SequenceAction)
;
                        }
                        GeneratedField::InitBridgeAccountAction => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("initBridgeAccountAction"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::InitBridgeAccountAction)
;
                        }
                        GeneratedField::BridgeLockAction => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeLockAction"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::BridgeLockAction)
;
                        }
                        GeneratedField::BridgeUnlockAction => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeUnlockAction"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::BridgeUnlockAction)
;
                        }
                        GeneratedField::BridgeSudoChangeAction => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeSudoChangeAction"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::BridgeSudoChangeAction)
;
                        }
                        GeneratedField::IbcAction => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ibcAction"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::IbcAction)
;
                        }
                        GeneratedField::Ics20Withdrawal => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ics20Withdrawal"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::Ics20Withdrawal)
;
                        }
                        GeneratedField::SudoAddressChangeAction => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sudoAddressChangeAction"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::SudoAddressChangeAction)
;
                        }
                        GeneratedField::ValidatorUpdateAction => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("validatorUpdateAction"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::ValidatorUpdateAction)
;
                        }
                        GeneratedField::IbcRelayerChangeAction => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ibcRelayerChangeAction"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::IbcRelayerChangeAction)
;
                        }
                        GeneratedField::FeeAssetChangeAction => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeAssetChangeAction"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::FeeAssetChangeAction)
;
                        }
                        GeneratedField::FeeChangeAction => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeChangeAction"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::FeeChangeAction)
;
                        }
                        GeneratedField::IbcSudoChangeAction => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ibcSudoChangeAction"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::IbcSudoChangeAction)
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
impl serde::Serialize for BridgeLockAction {
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.BridgeLockAction", len)?;
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
impl<'de> serde::Deserialize<'de> for BridgeLockAction {
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
            type Value = BridgeLockAction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.BridgeLockAction")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BridgeLockAction, V::Error>
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
                Ok(BridgeLockAction {
                    to: to__,
                    amount: amount__,
                    asset: asset__.unwrap_or_default(),
                    fee_asset: fee_asset__.unwrap_or_default(),
                    destination_chain_address: destination_chain_address__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.BridgeLockAction", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BridgeSudoChangeAction {
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.BridgeSudoChangeAction", len)?;
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
impl<'de> serde::Deserialize<'de> for BridgeSudoChangeAction {
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
            type Value = BridgeSudoChangeAction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.BridgeSudoChangeAction")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BridgeSudoChangeAction, V::Error>
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
                Ok(BridgeSudoChangeAction {
                    bridge_address: bridge_address__,
                    new_sudo_address: new_sudo_address__,
                    new_withdrawer_address: new_withdrawer_address__,
                    fee_asset: fee_asset__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.BridgeSudoChangeAction", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BridgeUnlockAction {
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.BridgeUnlockAction", len)?;
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
impl<'de> serde::Deserialize<'de> for BridgeUnlockAction {
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
            type Value = BridgeUnlockAction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.BridgeUnlockAction")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BridgeUnlockAction, V::Error>
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
                Ok(BridgeUnlockAction {
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
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.BridgeUnlockAction", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for FeeAssetChangeAction {
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.FeeAssetChangeAction", len)?;
        if let Some(v) = self.value.as_ref() {
            match v {
                fee_asset_change_action::Value::Addition(v) => {
                    struct_ser.serialize_field("addition", v)?;
                }
                fee_asset_change_action::Value::Removal(v) => {
                    struct_ser.serialize_field("removal", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FeeAssetChangeAction {
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
            type Value = FeeAssetChangeAction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.FeeAssetChangeAction")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<FeeAssetChangeAction, V::Error>
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
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_asset_change_action::Value::Addition);
                        }
                        GeneratedField::Removal => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("removal"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_asset_change_action::Value::Removal);
                        }
                    }
                }
                Ok(FeeAssetChangeAction {
                    value: value__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.FeeAssetChangeAction", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for FeeChangeAction {
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.FeeChangeAction", len)?;
        if let Some(v) = self.value.as_ref() {
            match v {
                fee_change_action::Value::TransferBaseFee(v) => {
                    struct_ser.serialize_field("transferBaseFee", v)?;
                }
                fee_change_action::Value::SequenceBaseFee(v) => {
                    struct_ser.serialize_field("sequenceBaseFee", v)?;
                }
                fee_change_action::Value::SequenceByteCostMultiplier(v) => {
                    struct_ser.serialize_field("sequenceByteCostMultiplier", v)?;
                }
                fee_change_action::Value::InitBridgeAccountBaseFee(v) => {
                    struct_ser.serialize_field("initBridgeAccountBaseFee", v)?;
                }
                fee_change_action::Value::BridgeLockByteCostMultiplier(v) => {
                    struct_ser.serialize_field("bridgeLockByteCostMultiplier", v)?;
                }
                fee_change_action::Value::BridgeSudoChangeBaseFee(v) => {
                    struct_ser.serialize_field("bridgeSudoChangeBaseFee", v)?;
                }
                fee_change_action::Value::Ics20WithdrawalBaseFee(v) => {
                    struct_ser.serialize_field("ics20WithdrawalBaseFee", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FeeChangeAction {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "transfer_base_fee",
            "transferBaseFee",
            "sequence_base_fee",
            "sequenceBaseFee",
            "sequence_byte_cost_multiplier",
            "sequenceByteCostMultiplier",
            "init_bridge_account_base_fee",
            "initBridgeAccountBaseFee",
            "bridge_lock_byte_cost_multiplier",
            "bridgeLockByteCostMultiplier",
            "bridge_sudo_change_base_fee",
            "bridgeSudoChangeBaseFee",
            "ics20_withdrawal_base_fee",
            "ics20WithdrawalBaseFee",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TransferBaseFee,
            SequenceBaseFee,
            SequenceByteCostMultiplier,
            InitBridgeAccountBaseFee,
            BridgeLockByteCostMultiplier,
            BridgeSudoChangeBaseFee,
            Ics20WithdrawalBaseFee,
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
                            "transferBaseFee" | "transfer_base_fee" => Ok(GeneratedField::TransferBaseFee),
                            "sequenceBaseFee" | "sequence_base_fee" => Ok(GeneratedField::SequenceBaseFee),
                            "sequenceByteCostMultiplier" | "sequence_byte_cost_multiplier" => Ok(GeneratedField::SequenceByteCostMultiplier),
                            "initBridgeAccountBaseFee" | "init_bridge_account_base_fee" => Ok(GeneratedField::InitBridgeAccountBaseFee),
                            "bridgeLockByteCostMultiplier" | "bridge_lock_byte_cost_multiplier" => Ok(GeneratedField::BridgeLockByteCostMultiplier),
                            "bridgeSudoChangeBaseFee" | "bridge_sudo_change_base_fee" => Ok(GeneratedField::BridgeSudoChangeBaseFee),
                            "ics20WithdrawalBaseFee" | "ics20_withdrawal_base_fee" => Ok(GeneratedField::Ics20WithdrawalBaseFee),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FeeChangeAction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.FeeChangeAction")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<FeeChangeAction, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut value__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TransferBaseFee => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transferBaseFee"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change_action::Value::TransferBaseFee)
;
                        }
                        GeneratedField::SequenceBaseFee => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequenceBaseFee"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change_action::Value::SequenceBaseFee)
;
                        }
                        GeneratedField::SequenceByteCostMultiplier => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequenceByteCostMultiplier"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change_action::Value::SequenceByteCostMultiplier)
;
                        }
                        GeneratedField::InitBridgeAccountBaseFee => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("initBridgeAccountBaseFee"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change_action::Value::InitBridgeAccountBaseFee)
;
                        }
                        GeneratedField::BridgeLockByteCostMultiplier => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeLockByteCostMultiplier"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change_action::Value::BridgeLockByteCostMultiplier)
;
                        }
                        GeneratedField::BridgeSudoChangeBaseFee => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeSudoChangeBaseFee"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change_action::Value::BridgeSudoChangeBaseFee)
;
                        }
                        GeneratedField::Ics20WithdrawalBaseFee => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ics20WithdrawalBaseFee"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change_action::Value::Ics20WithdrawalBaseFee)
;
                        }
                    }
                }
                Ok(FeeChangeAction {
                    value: value__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.FeeChangeAction", FIELDS, GeneratedVisitor)
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
impl serde::Serialize for IbcRelayerChangeAction {
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.IbcRelayerChangeAction", len)?;
        if let Some(v) = self.value.as_ref() {
            match v {
                ibc_relayer_change_action::Value::Addition(v) => {
                    struct_ser.serialize_field("addition", v)?;
                }
                ibc_relayer_change_action::Value::Removal(v) => {
                    struct_ser.serialize_field("removal", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for IbcRelayerChangeAction {
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
            type Value = IbcRelayerChangeAction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.IbcRelayerChangeAction")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<IbcRelayerChangeAction, V::Error>
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
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(ibc_relayer_change_action::Value::Addition)
;
                        }
                        GeneratedField::Removal => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("removal"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(ibc_relayer_change_action::Value::Removal)
;
                        }
                    }
                }
                Ok(IbcRelayerChangeAction {
                    value: value__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.IbcRelayerChangeAction", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for IbcSudoChangeAction {
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.IbcSudoChangeAction", len)?;
        if let Some(v) = self.new_address.as_ref() {
            struct_ser.serialize_field("newAddress", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for IbcSudoChangeAction {
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
            type Value = IbcSudoChangeAction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.IbcSudoChangeAction")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<IbcSudoChangeAction, V::Error>
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
                Ok(IbcSudoChangeAction {
                    new_address: new_address__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.IbcSudoChangeAction", FIELDS, GeneratedVisitor)
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
impl serde::Serialize for InitBridgeAccountAction {
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.InitBridgeAccountAction", len)?;
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
impl<'de> serde::Deserialize<'de> for InitBridgeAccountAction {
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
            type Value = InitBridgeAccountAction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.InitBridgeAccountAction")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<InitBridgeAccountAction, V::Error>
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
                Ok(InitBridgeAccountAction {
                    rollup_id: rollup_id__,
                    asset: asset__.unwrap_or_default(),
                    fee_asset: fee_asset__.unwrap_or_default(),
                    sudo_address: sudo_address__,
                    withdrawer_address: withdrawer_address__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.InitBridgeAccountAction", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SequenceAction {
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.SequenceAction", len)?;
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
impl<'de> serde::Deserialize<'de> for SequenceAction {
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
            type Value = SequenceAction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.SequenceAction")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SequenceAction, V::Error>
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
                Ok(SequenceAction {
                    rollup_id: rollup_id__,
                    data: data__.unwrap_or_default(),
                    fee_asset: fee_asset__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.SequenceAction", FIELDS, GeneratedVisitor)
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
impl serde::Serialize for SudoAddressChangeAction {
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.SudoAddressChangeAction", len)?;
        if let Some(v) = self.new_address.as_ref() {
            struct_ser.serialize_field("newAddress", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SudoAddressChangeAction {
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
            type Value = SudoAddressChangeAction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.SudoAddressChangeAction")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SudoAddressChangeAction, V::Error>
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
                Ok(SudoAddressChangeAction {
                    new_address: new_address__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.SudoAddressChangeAction", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TransactionFee {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.asset.is_empty() {
            len += 1;
        }
        if self.fee.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.TransactionFee", len)?;
        if !self.asset.is_empty() {
            struct_ser.serialize_field("asset", &self.asset)?;
        }
        if let Some(v) = self.fee.as_ref() {
            struct_ser.serialize_field("fee", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionFee {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "asset",
            "fee",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Asset,
            Fee,
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
                            "asset" => Ok(GeneratedField::Asset),
                            "fee" => Ok(GeneratedField::Fee),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionFee;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.TransactionFee")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TransactionFee, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut asset__ = None;
                let mut fee__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Asset => {
                            if asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("asset"));
                            }
                            asset__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Fee => {
                            if fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("fee"));
                            }
                            fee__ = map_.next_value()?;
                        }
                    }
                }
                Ok(TransactionFee {
                    asset: asset__.unwrap_or_default(),
                    fee: fee__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.TransactionFee", FIELDS, GeneratedVisitor)
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
impl serde::Serialize for TransferAction {
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transactions.v1alpha1.TransferAction", len)?;
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
impl<'de> serde::Deserialize<'de> for TransferAction {
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
            type Value = TransferAction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transactions.v1alpha1.TransferAction")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TransferAction, V::Error>
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
                Ok(TransferAction {
                    to: to__,
                    amount: amount__,
                    asset: asset__.unwrap_or_default(),
                    fee_asset: fee_asset__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transactions.v1alpha1.TransferAction", FIELDS, GeneratedVisitor)
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
