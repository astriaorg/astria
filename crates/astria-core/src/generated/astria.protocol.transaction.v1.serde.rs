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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.Action", len)?;
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
                action::Value::BridgeTransfer(v) => {
                    struct_ser.serialize_field("bridgeTransfer", v)?;
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
                action::Value::RecoverIbcClient(v) => {
                    struct_ser.serialize_field("recoverIbcClient", v)?;
                }
                action::Value::CurrencyPairsChange(v) => {
                    struct_ser.serialize_field("currencyPairsChange", v)?;
                }
                action::Value::MarketsChange(v) => {
                    struct_ser.serialize_field("marketsChange", v)?;
                }
                action::Value::CreateOrder(v) => {
                    struct_ser.serialize_field("createOrder", v)?;
                }
                action::Value::CancelOrder(v) => {
                    struct_ser.serialize_field("cancelOrder", v)?;
                }
                action::Value::CreateMarket(v) => {
                    struct_ser.serialize_field("createMarket", v)?;
                }
                action::Value::UpdateMarket(v) => {
                    struct_ser.serialize_field("updateMarket", v)?;
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
            "bridge_transfer",
            "bridgeTransfer",
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
            "recover_ibc_client",
            "recoverIbcClient",
            "currency_pairs_change",
            "currencyPairsChange",
            "markets_change",
            "marketsChange",
            "create_order",
            "createOrder",
            "cancel_order",
            "cancelOrder",
            "create_market",
            "createMarket",
            "update_market",
            "updateMarket",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Transfer,
            RollupDataSubmission,
            InitBridgeAccount,
            BridgeLock,
            BridgeUnlock,
            BridgeSudoChange,
            BridgeTransfer,
            Ibc,
            Ics20Withdrawal,
            SudoAddressChange,
            ValidatorUpdate,
            IbcRelayerChange,
            FeeAssetChange,
            FeeChange,
            IbcSudoChange,
            RecoverIbcClient,
            CurrencyPairsChange,
            MarketsChange,
            CreateOrder,
            CancelOrder,
            CreateMarket,
            UpdateMarket,
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
                            "bridgeTransfer" | "bridge_transfer" => Ok(GeneratedField::BridgeTransfer),
                            "ibc" => Ok(GeneratedField::Ibc),
                            "ics20Withdrawal" | "ics20_withdrawal" => Ok(GeneratedField::Ics20Withdrawal),
                            "sudoAddressChange" | "sudo_address_change" => Ok(GeneratedField::SudoAddressChange),
                            "validatorUpdate" | "validator_update" => Ok(GeneratedField::ValidatorUpdate),
                            "ibcRelayerChange" | "ibc_relayer_change" => Ok(GeneratedField::IbcRelayerChange),
                            "feeAssetChange" | "fee_asset_change" => Ok(GeneratedField::FeeAssetChange),
                            "feeChange" | "fee_change" => Ok(GeneratedField::FeeChange),
                            "ibcSudoChange" | "ibc_sudo_change" => Ok(GeneratedField::IbcSudoChange),
                            "recoverIbcClient" | "recover_ibc_client" => Ok(GeneratedField::RecoverIbcClient),
                            "currencyPairsChange" | "currency_pairs_change" => Ok(GeneratedField::CurrencyPairsChange),
                            "marketsChange" | "markets_change" => Ok(GeneratedField::MarketsChange),
                            "createOrder" | "create_order" => Ok(GeneratedField::CreateOrder),
                            "cancelOrder" | "cancel_order" => Ok(GeneratedField::CancelOrder),
                            "createMarket" | "create_market" => Ok(GeneratedField::CreateMarket),
                            "updateMarket" | "update_market" => Ok(GeneratedField::UpdateMarket),
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
                formatter.write_str("struct astria.protocol.transaction.v1.Action")
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
                        GeneratedField::BridgeTransfer => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeTransfer"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::BridgeTransfer)
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
                        GeneratedField::RecoverIbcClient => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("recoverIbcClient"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::RecoverIbcClient)
;
                        }
                        GeneratedField::CurrencyPairsChange => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("currencyPairsChange"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::CurrencyPairsChange)
;
                        }
                        GeneratedField::MarketsChange => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("marketsChange"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::MarketsChange)
;
                        }
                        GeneratedField::CreateOrder => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("createOrder"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::CreateOrder)
;
                        }
                        GeneratedField::CancelOrder => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("cancelOrder"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::CancelOrder)
;
                        }
                        GeneratedField::CreateMarket => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("createMarket"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::CreateMarket)
;
                        }
                        GeneratedField::UpdateMarket => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("updateMarket"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(action::Value::UpdateMarket)
;
                        }
                    }
                }
                Ok(Action {
                    value: value__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1.Action", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.BridgeLock", len)?;
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
                formatter.write_str("struct astria.protocol.transaction.v1.BridgeLock")
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
        deserializer.deserialize_struct("astria.protocol.transaction.v1.BridgeLock", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.BridgeSudoChange", len)?;
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
                formatter.write_str("struct astria.protocol.transaction.v1.BridgeSudoChange")
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
        deserializer.deserialize_struct("astria.protocol.transaction.v1.BridgeSudoChange", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BridgeTransfer {
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
        if !self.destination_chain_address.is_empty() {
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.BridgeTransfer", len)?;
        if let Some(v) = self.to.as_ref() {
            struct_ser.serialize_field("to", v)?;
        }
        if let Some(v) = self.amount.as_ref() {
            struct_ser.serialize_field("amount", v)?;
        }
        if !self.fee_asset.is_empty() {
            struct_ser.serialize_field("feeAsset", &self.fee_asset)?;
        }
        if !self.destination_chain_address.is_empty() {
            struct_ser.serialize_field("destinationChainAddress", &self.destination_chain_address)?;
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
impl<'de> serde::Deserialize<'de> for BridgeTransfer {
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
            "destination_chain_address",
            "destinationChainAddress",
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
            DestinationChainAddress,
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
                            "destinationChainAddress" | "destination_chain_address" => Ok(GeneratedField::DestinationChainAddress),
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
            type Value = BridgeTransfer;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1.BridgeTransfer")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BridgeTransfer, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut to__ = None;
                let mut amount__ = None;
                let mut fee_asset__ = None;
                let mut destination_chain_address__ = None;
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
                        GeneratedField::DestinationChainAddress => {
                            if destination_chain_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("destinationChainAddress"));
                            }
                            destination_chain_address__ = Some(map_.next_value()?);
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
                Ok(BridgeTransfer {
                    to: to__,
                    amount: amount__,
                    fee_asset: fee_asset__.unwrap_or_default(),
                    destination_chain_address: destination_chain_address__.unwrap_or_default(),
                    bridge_address: bridge_address__,
                    rollup_block_number: rollup_block_number__.unwrap_or_default(),
                    rollup_withdrawal_event_id: rollup_withdrawal_event_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1.BridgeTransfer", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.BridgeUnlock", len)?;
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
                formatter.write_str("struct astria.protocol.transaction.v1.BridgeUnlock")
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
        deserializer.deserialize_struct("astria.protocol.transaction.v1.BridgeUnlock", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CancelOrder {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.order_id.is_empty() {
            len += 1;
        }
        if !self.fee_asset.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.CancelOrder", len)?;
        if !self.order_id.is_empty() {
            struct_ser.serialize_field("orderId", &self.order_id)?;
        }
        if !self.fee_asset.is_empty() {
            struct_ser.serialize_field("feeAsset", &self.fee_asset)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CancelOrder {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "order_id",
            "orderId",
            "fee_asset",
            "feeAsset",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            OrderId,
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
                            "orderId" | "order_id" => Ok(GeneratedField::OrderId),
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
            type Value = CancelOrder;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1.CancelOrder")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CancelOrder, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut order_id__ = None;
                let mut fee_asset__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::OrderId => {
                            if order_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("orderId"));
                            }
                            order_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::FeeAsset => {
                            if fee_asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeAsset"));
                            }
                            fee_asset__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(CancelOrder {
                    order_id: order_id__.unwrap_or_default(),
                    fee_asset: fee_asset__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1.CancelOrder", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CreateMarket {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.market.is_empty() {
            len += 1;
        }
        if !self.base_asset.is_empty() {
            len += 1;
        }
        if !self.quote_asset.is_empty() {
            len += 1;
        }
        if self.tick_size.is_some() {
            len += 1;
        }
        if self.lot_size.is_some() {
            len += 1;
        }
        if !self.fee_asset.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.CreateMarket", len)?;
        if !self.market.is_empty() {
            struct_ser.serialize_field("market", &self.market)?;
        }
        if !self.base_asset.is_empty() {
            struct_ser.serialize_field("baseAsset", &self.base_asset)?;
        }
        if !self.quote_asset.is_empty() {
            struct_ser.serialize_field("quoteAsset", &self.quote_asset)?;
        }
        if let Some(v) = self.tick_size.as_ref() {
            struct_ser.serialize_field("tickSize", v)?;
        }
        if let Some(v) = self.lot_size.as_ref() {
            struct_ser.serialize_field("lotSize", v)?;
        }
        if !self.fee_asset.is_empty() {
            struct_ser.serialize_field("feeAsset", &self.fee_asset)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CreateMarket {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "market",
            "base_asset",
            "baseAsset",
            "quote_asset",
            "quoteAsset",
            "tick_size",
            "tickSize",
            "lot_size",
            "lotSize",
            "fee_asset",
            "feeAsset",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Market,
            BaseAsset,
            QuoteAsset,
            TickSize,
            LotSize,
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
                            "market" => Ok(GeneratedField::Market),
                            "baseAsset" | "base_asset" => Ok(GeneratedField::BaseAsset),
                            "quoteAsset" | "quote_asset" => Ok(GeneratedField::QuoteAsset),
                            "tickSize" | "tick_size" => Ok(GeneratedField::TickSize),
                            "lotSize" | "lot_size" => Ok(GeneratedField::LotSize),
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
            type Value = CreateMarket;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1.CreateMarket")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CreateMarket, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut market__ = None;
                let mut base_asset__ = None;
                let mut quote_asset__ = None;
                let mut tick_size__ = None;
                let mut lot_size__ = None;
                let mut fee_asset__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Market => {
                            if market__.is_some() {
                                return Err(serde::de::Error::duplicate_field("market"));
                            }
                            market__ = Some(map_.next_value()?);
                        }
                        GeneratedField::BaseAsset => {
                            if base_asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseAsset"));
                            }
                            base_asset__ = Some(map_.next_value()?);
                        }
                        GeneratedField::QuoteAsset => {
                            if quote_asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("quoteAsset"));
                            }
                            quote_asset__ = Some(map_.next_value()?);
                        }
                        GeneratedField::TickSize => {
                            if tick_size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tickSize"));
                            }
                            tick_size__ = map_.next_value()?;
                        }
                        GeneratedField::LotSize => {
                            if lot_size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("lotSize"));
                            }
                            lot_size__ = map_.next_value()?;
                        }
                        GeneratedField::FeeAsset => {
                            if fee_asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeAsset"));
                            }
                            fee_asset__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(CreateMarket {
                    market: market__.unwrap_or_default(),
                    base_asset: base_asset__.unwrap_or_default(),
                    quote_asset: quote_asset__.unwrap_or_default(),
                    tick_size: tick_size__,
                    lot_size: lot_size__,
                    fee_asset: fee_asset__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1.CreateMarket", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CreateOrder {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.market.is_empty() {
            len += 1;
        }
        if self.side.is_some() {
            len += 1;
        }
        if self.r#type.is_some() {
            len += 1;
        }
        if self.price.is_some() {
            len += 1;
        }
        if self.quantity.is_some() {
            len += 1;
        }
        if self.time_in_force.is_some() {
            len += 1;
        }
        if !self.fee_asset.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.CreateOrder", len)?;
        if !self.market.is_empty() {
            struct_ser.serialize_field("market", &self.market)?;
        }
        if let Some(v) = self.side.as_ref() {
            struct_ser.serialize_field("side", v)?;
        }
        if let Some(v) = self.r#type.as_ref() {
            struct_ser.serialize_field("type", v)?;
        }
        if let Some(v) = self.price.as_ref() {
            struct_ser.serialize_field("price", v)?;
        }
        if let Some(v) = self.quantity.as_ref() {
            struct_ser.serialize_field("quantity", v)?;
        }
        if let Some(v) = self.time_in_force.as_ref() {
            struct_ser.serialize_field("timeInForce", v)?;
        }
        if !self.fee_asset.is_empty() {
            struct_ser.serialize_field("feeAsset", &self.fee_asset)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CreateOrder {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "market",
            "side",
            "type",
            "price",
            "quantity",
            "time_in_force",
            "timeInForce",
            "fee_asset",
            "feeAsset",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Market,
            Side,
            Type,
            Price,
            Quantity,
            TimeInForce,
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
                            "market" => Ok(GeneratedField::Market),
                            "side" => Ok(GeneratedField::Side),
                            "type" => Ok(GeneratedField::Type),
                            "price" => Ok(GeneratedField::Price),
                            "quantity" => Ok(GeneratedField::Quantity),
                            "timeInForce" | "time_in_force" => Ok(GeneratedField::TimeInForce),
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
            type Value = CreateOrder;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1.CreateOrder")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CreateOrder, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut market__ = None;
                let mut side__ = None;
                let mut r#type__ = None;
                let mut price__ = None;
                let mut quantity__ = None;
                let mut time_in_force__ = None;
                let mut fee_asset__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Market => {
                            if market__.is_some() {
                                return Err(serde::de::Error::duplicate_field("market"));
                            }
                            market__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Side => {
                            if side__.is_some() {
                                return Err(serde::de::Error::duplicate_field("side"));
                            }
                            side__ = map_.next_value()?;
                        }
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = map_.next_value()?;
                        }
                        GeneratedField::Price => {
                            if price__.is_some() {
                                return Err(serde::de::Error::duplicate_field("price"));
                            }
                            price__ = map_.next_value()?;
                        }
                        GeneratedField::Quantity => {
                            if quantity__.is_some() {
                                return Err(serde::de::Error::duplicate_field("quantity"));
                            }
                            quantity__ = map_.next_value()?;
                        }
                        GeneratedField::TimeInForce => {
                            if time_in_force__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timeInForce"));
                            }
                            time_in_force__ = map_.next_value()?;
                        }
                        GeneratedField::FeeAsset => {
                            if fee_asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeAsset"));
                            }
                            fee_asset__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(CreateOrder {
                    market: market__.unwrap_or_default(),
                    side: side__,
                    r#type: r#type__,
                    price: price__,
                    quantity: quantity__,
                    time_in_force: time_in_force__,
                    fee_asset: fee_asset__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1.CreateOrder", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CurrencyPairs {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.pairs.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.CurrencyPairs", len)?;
        if !self.pairs.is_empty() {
            struct_ser.serialize_field("pairs", &self.pairs)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CurrencyPairs {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "pairs",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Pairs,
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
                            "pairs" => Ok(GeneratedField::Pairs),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CurrencyPairs;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1.CurrencyPairs")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CurrencyPairs, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut pairs__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Pairs => {
                            if pairs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pairs"));
                            }
                            pairs__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(CurrencyPairs {
                    pairs: pairs__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1.CurrencyPairs", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CurrencyPairsChange {
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.CurrencyPairsChange", len)?;
        if let Some(v) = self.value.as_ref() {
            match v {
                currency_pairs_change::Value::Addition(v) => {
                    struct_ser.serialize_field("addition", v)?;
                }
                currency_pairs_change::Value::Removal(v) => {
                    struct_ser.serialize_field("removal", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CurrencyPairsChange {
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
            type Value = CurrencyPairsChange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1.CurrencyPairsChange")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CurrencyPairsChange, V::Error>
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
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(currency_pairs_change::Value::Addition)
;
                        }
                        GeneratedField::Removal => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("removal"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(currency_pairs_change::Value::Removal)
;
                        }
                    }
                }
                Ok(CurrencyPairsChange {
                    value: value__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1.CurrencyPairsChange", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.FeeAssetChange", len)?;
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
                formatter.write_str("struct astria.protocol.transaction.v1.FeeAssetChange")
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
        deserializer.deserialize_struct("astria.protocol.transaction.v1.FeeAssetChange", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.FeeChange", len)?;
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
                fee_change::FeeComponents::BridgeTransfer(v) => {
                    struct_ser.serialize_field("bridgeTransfer", v)?;
                }
                fee_change::FeeComponents::RecoverIbcClient(v) => {
                    struct_ser.serialize_field("recoverIbcClient", v)?;
                }
                fee_change::FeeComponents::CurrencyPairsChange(v) => {
                    struct_ser.serialize_field("currencyPairsChange", v)?;
                }
                fee_change::FeeComponents::MarketsChange(v) => {
                    struct_ser.serialize_field("marketsChange", v)?;
                }
                fee_change::FeeComponents::CreateOrder(v) => {
                    struct_ser.serialize_field("createOrder", v)?;
                }
                fee_change::FeeComponents::CancelOrder(v) => {
                    struct_ser.serialize_field("cancelOrder", v)?;
                }
                fee_change::FeeComponents::CreateMarket(v) => {
                    struct_ser.serialize_field("createMarket", v)?;
                }
                fee_change::FeeComponents::UpdateMarket(v) => {
                    struct_ser.serialize_field("updateMarket", v)?;
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
            "bridge_transfer",
            "bridgeTransfer",
            "recover_ibc_client",
            "recoverIbcClient",
            "currency_pairs_change",
            "currencyPairsChange",
            "markets_change",
            "marketsChange",
            "create_order",
            "createOrder",
            "cancel_order",
            "cancelOrder",
            "create_market",
            "createMarket",
            "update_market",
            "updateMarket",
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
            BridgeTransfer,
            RecoverIbcClient,
            CurrencyPairsChange,
            MarketsChange,
            CreateOrder,
            CancelOrder,
            CreateMarket,
            UpdateMarket,
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
                            "bridgeTransfer" | "bridge_transfer" => Ok(GeneratedField::BridgeTransfer),
                            "recoverIbcClient" | "recover_ibc_client" => Ok(GeneratedField::RecoverIbcClient),
                            "currencyPairsChange" | "currency_pairs_change" => Ok(GeneratedField::CurrencyPairsChange),
                            "marketsChange" | "markets_change" => Ok(GeneratedField::MarketsChange),
                            "createOrder" | "create_order" => Ok(GeneratedField::CreateOrder),
                            "cancelOrder" | "cancel_order" => Ok(GeneratedField::CancelOrder),
                            "createMarket" | "create_market" => Ok(GeneratedField::CreateMarket),
                            "updateMarket" | "update_market" => Ok(GeneratedField::UpdateMarket),
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
                formatter.write_str("struct astria.protocol.transaction.v1.FeeChange")
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
                        GeneratedField::BridgeTransfer => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeTransfer"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::BridgeTransfer)
;
                        }
                        GeneratedField::RecoverIbcClient => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("recoverIbcClient"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::RecoverIbcClient)
;
                        }
                        GeneratedField::CurrencyPairsChange => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("currencyPairsChange"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::CurrencyPairsChange)
;
                        }
                        GeneratedField::MarketsChange => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("marketsChange"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::MarketsChange)
;
                        }
                        GeneratedField::CreateOrder => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("createOrder"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::CreateOrder)
;
                        }
                        GeneratedField::CancelOrder => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("cancelOrder"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::CancelOrder)
;
                        }
                        GeneratedField::CreateMarket => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("createMarket"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::CreateMarket)
;
                        }
                        GeneratedField::UpdateMarket => {
                            if fee_components__.is_some() {
                                return Err(serde::de::Error::duplicate_field("updateMarket"));
                            }
                            fee_components__ = map_.next_value::<::std::option::Option<_>>()?.map(fee_change::FeeComponents::UpdateMarket)
;
                        }
                    }
                }
                Ok(FeeChange {
                    fee_components: fee_components__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1.FeeChange", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.IbcHeight", len)?;
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
                formatter.write_str("struct astria.protocol.transaction.v1.IbcHeight")
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
        deserializer.deserialize_struct("astria.protocol.transaction.v1.IbcHeight", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.IbcRelayerChange", len)?;
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
                formatter.write_str("struct astria.protocol.transaction.v1.IbcRelayerChange")
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
        deserializer.deserialize_struct("astria.protocol.transaction.v1.IbcRelayerChange", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.IbcSudoChange", len)?;
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
                formatter.write_str("struct astria.protocol.transaction.v1.IbcSudoChange")
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
        deserializer.deserialize_struct("astria.protocol.transaction.v1.IbcSudoChange", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.Ics20Withdrawal", len)?;
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
                formatter.write_str("struct astria.protocol.transaction.v1.Ics20Withdrawal")
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
        deserializer.deserialize_struct("astria.protocol.transaction.v1.Ics20Withdrawal", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.InitBridgeAccount", len)?;
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
                formatter.write_str("struct astria.protocol.transaction.v1.InitBridgeAccount")
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
        deserializer.deserialize_struct("astria.protocol.transaction.v1.InitBridgeAccount", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Markets {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.markets.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.Markets", len)?;
        if !self.markets.is_empty() {
            struct_ser.serialize_field("markets", &self.markets)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Markets {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "markets",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Markets,
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
                            "markets" => Ok(GeneratedField::Markets),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Markets;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1.Markets")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Markets, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut markets__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Markets => {
                            if markets__.is_some() {
                                return Err(serde::de::Error::duplicate_field("markets"));
                            }
                            markets__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Markets {
                    markets: markets__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1.Markets", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MarketsChange {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.action.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.MarketsChange", len)?;
        if let Some(v) = self.action.as_ref() {
            match v {
                markets_change::Action::Creation(v) => {
                    struct_ser.serialize_field("creation", v)?;
                }
                markets_change::Action::Removal(v) => {
                    struct_ser.serialize_field("removal", v)?;
                }
                markets_change::Action::Update(v) => {
                    struct_ser.serialize_field("update", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MarketsChange {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "creation",
            "removal",
            "update",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Creation,
            Removal,
            Update,
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
                            "creation" => Ok(GeneratedField::Creation),
                            "removal" => Ok(GeneratedField::Removal),
                            "update" => Ok(GeneratedField::Update),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MarketsChange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1.MarketsChange")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<MarketsChange, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut action__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Creation => {
                            if action__.is_some() {
                                return Err(serde::de::Error::duplicate_field("creation"));
                            }
                            action__ = map_.next_value::<::std::option::Option<_>>()?.map(markets_change::Action::Creation)
;
                        }
                        GeneratedField::Removal => {
                            if action__.is_some() {
                                return Err(serde::de::Error::duplicate_field("removal"));
                            }
                            action__ = map_.next_value::<::std::option::Option<_>>()?.map(markets_change::Action::Removal)
;
                        }
                        GeneratedField::Update => {
                            if action__.is_some() {
                                return Err(serde::de::Error::duplicate_field("update"));
                            }
                            action__ = map_.next_value::<::std::option::Option<_>>()?.map(markets_change::Action::Update)
;
                        }
                    }
                }
                Ok(MarketsChange {
                    action: action__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1.MarketsChange", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RecoverIbcClient {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.client_id.is_empty() {
            len += 1;
        }
        if !self.replacement_client_id.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.RecoverIbcClient", len)?;
        if !self.client_id.is_empty() {
            struct_ser.serialize_field("clientId", &self.client_id)?;
        }
        if !self.replacement_client_id.is_empty() {
            struct_ser.serialize_field("replacementClientId", &self.replacement_client_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RecoverIbcClient {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "client_id",
            "clientId",
            "replacement_client_id",
            "replacementClientId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ClientId,
            ReplacementClientId,
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
                            "clientId" | "client_id" => Ok(GeneratedField::ClientId),
                            "replacementClientId" | "replacement_client_id" => Ok(GeneratedField::ReplacementClientId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RecoverIbcClient;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1.RecoverIbcClient")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RecoverIbcClient, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut client_id__ = None;
                let mut replacement_client_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ClientId => {
                            if client_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("clientId"));
                            }
                            client_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ReplacementClientId => {
                            if replacement_client_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("replacementClientId"));
                            }
                            replacement_client_id__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(RecoverIbcClient {
                    client_id: client_id__.unwrap_or_default(),
                    replacement_client_id: replacement_client_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1.RecoverIbcClient", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.RollupDataSubmission", len)?;
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
                formatter.write_str("struct astria.protocol.transaction.v1.RollupDataSubmission")
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
        deserializer.deserialize_struct("astria.protocol.transaction.v1.RollupDataSubmission", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.SudoAddressChange", len)?;
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
                formatter.write_str("struct astria.protocol.transaction.v1.SudoAddressChange")
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
        deserializer.deserialize_struct("astria.protocol.transaction.v1.SudoAddressChange", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.Transaction", len)?;
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
                formatter.write_str("struct astria.protocol.transaction.v1.Transaction")
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
        deserializer.deserialize_struct("astria.protocol.transaction.v1.Transaction", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.TransactionBody", len)?;
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
                formatter.write_str("struct astria.protocol.transaction.v1.TransactionBody")
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
        deserializer.deserialize_struct("astria.protocol.transaction.v1.TransactionBody", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.TransactionParams", len)?;
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
                formatter.write_str("struct astria.protocol.transaction.v1.TransactionParams")
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
        deserializer.deserialize_struct("astria.protocol.transaction.v1.TransactionParams", FIELDS, GeneratedVisitor)
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
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.Transfer", len)?;
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
                formatter.write_str("struct astria.protocol.transaction.v1.Transfer")
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
        deserializer.deserialize_struct("astria.protocol.transaction.v1.Transfer", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for UpdateMarket {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.market.is_empty() {
            len += 1;
        }
        if self.tick_size.is_some() {
            len += 1;
        }
        if self.lot_size.is_some() {
            len += 1;
        }
        if self.paused {
            len += 1;
        }
        if !self.fee_asset.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.UpdateMarket", len)?;
        if !self.market.is_empty() {
            struct_ser.serialize_field("market", &self.market)?;
        }
        if let Some(v) = self.tick_size.as_ref() {
            struct_ser.serialize_field("tickSize", v)?;
        }
        if let Some(v) = self.lot_size.as_ref() {
            struct_ser.serialize_field("lotSize", v)?;
        }
        if self.paused {
            struct_ser.serialize_field("paused", &self.paused)?;
        }
        if !self.fee_asset.is_empty() {
            struct_ser.serialize_field("feeAsset", &self.fee_asset)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for UpdateMarket {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "market",
            "tick_size",
            "tickSize",
            "lot_size",
            "lotSize",
            "paused",
            "fee_asset",
            "feeAsset",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Market,
            TickSize,
            LotSize,
            Paused,
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
                            "market" => Ok(GeneratedField::Market),
                            "tickSize" | "tick_size" => Ok(GeneratedField::TickSize),
                            "lotSize" | "lot_size" => Ok(GeneratedField::LotSize),
                            "paused" => Ok(GeneratedField::Paused),
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
            type Value = UpdateMarket;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1.UpdateMarket")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<UpdateMarket, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut market__ = None;
                let mut tick_size__ = None;
                let mut lot_size__ = None;
                let mut paused__ = None;
                let mut fee_asset__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Market => {
                            if market__.is_some() {
                                return Err(serde::de::Error::duplicate_field("market"));
                            }
                            market__ = Some(map_.next_value()?);
                        }
                        GeneratedField::TickSize => {
                            if tick_size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tickSize"));
                            }
                            tick_size__ = map_.next_value()?;
                        }
                        GeneratedField::LotSize => {
                            if lot_size__.is_some() {
                                return Err(serde::de::Error::duplicate_field("lotSize"));
                            }
                            lot_size__ = map_.next_value()?;
                        }
                        GeneratedField::Paused => {
                            if paused__.is_some() {
                                return Err(serde::de::Error::duplicate_field("paused"));
                            }
                            paused__ = Some(map_.next_value()?);
                        }
                        GeneratedField::FeeAsset => {
                            if fee_asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeAsset"));
                            }
                            fee_asset__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(UpdateMarket {
                    market: market__.unwrap_or_default(),
                    tick_size: tick_size__,
                    lot_size: lot_size__,
                    paused: paused__.unwrap_or_default(),
                    fee_asset: fee_asset__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1.UpdateMarket", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ValidatorUpdate {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.pub_key.is_some() {
            len += 1;
        }
        if self.power != 0 {
            len += 1;
        }
        if !self.name.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.transaction.v1.ValidatorUpdate", len)?;
        if let Some(v) = self.pub_key.as_ref() {
            struct_ser.serialize_field("pubKey", v)?;
        }
        if self.power != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("power", ToString::to_string(&self.power).as_str())?;
        }
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ValidatorUpdate {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "pub_key",
            "pubKey",
            "power",
            "name",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            PubKey,
            Power,
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
                            "pubKey" | "pub_key" => Ok(GeneratedField::PubKey),
                            "power" => Ok(GeneratedField::Power),
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
            type Value = ValidatorUpdate;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.transaction.v1.ValidatorUpdate")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ValidatorUpdate, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut pub_key__ = None;
                let mut power__ = None;
                let mut name__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::PubKey => {
                            if pub_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pubKey"));
                            }
                            pub_key__ = map_.next_value()?;
                        }
                        GeneratedField::Power => {
                            if power__.is_some() {
                                return Err(serde::de::Error::duplicate_field("power"));
                            }
                            power__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(ValidatorUpdate {
                    pub_key: pub_key__,
                    power: power__.unwrap_or_default(),
                    name: name__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.transaction.v1.ValidatorUpdate", FIELDS, GeneratedVisitor)
    }
}
