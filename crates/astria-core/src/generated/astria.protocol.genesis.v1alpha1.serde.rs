impl serde::Serialize for Account {
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
        if self.balance.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.genesis.v1alpha1.Account", len)?;
        if let Some(v) = self.address.as_ref() {
            struct_ser.serialize_field("address", v)?;
        }
        if let Some(v) = self.balance.as_ref() {
            struct_ser.serialize_field("balance", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Account {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "address",
            "balance",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Address,
            Balance,
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
                            "balance" => Ok(GeneratedField::Balance),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Account;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.genesis.v1alpha1.Account")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Account, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut address__ = None;
                let mut balance__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Address => {
                            if address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("address"));
                            }
                            address__ = map_.next_value()?;
                        }
                        GeneratedField::Balance => {
                            if balance__.is_some() {
                                return Err(serde::de::Error::duplicate_field("balance"));
                            }
                            balance__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Account {
                    address: address__,
                    balance: balance__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.genesis.v1alpha1.Account", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AddressPrefixes {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.base.is_empty() {
            len += 1;
        }
        if !self.ibc_compat.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.genesis.v1alpha1.AddressPrefixes", len)?;
        if !self.base.is_empty() {
            struct_ser.serialize_field("base", &self.base)?;
        }
        if !self.ibc_compat.is_empty() {
            struct_ser.serialize_field("ibcCompat", &self.ibc_compat)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AddressPrefixes {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "base",
            "ibc_compat",
            "ibcCompat",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Base,
            IbcCompat,
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
                            "base" => Ok(GeneratedField::Base),
                            "ibcCompat" | "ibc_compat" => Ok(GeneratedField::IbcCompat),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AddressPrefixes;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.genesis.v1alpha1.AddressPrefixes")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AddressPrefixes, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut base__ = None;
                let mut ibc_compat__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Base => {
                            if base__.is_some() {
                                return Err(serde::de::Error::duplicate_field("base"));
                            }
                            base__ = Some(map_.next_value()?);
                        }
                        GeneratedField::IbcCompat => {
                            if ibc_compat__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ibcCompat"));
                            }
                            ibc_compat__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(AddressPrefixes {
                    base: base__.unwrap_or_default(),
                    ibc_compat: ibc_compat__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.genesis.v1alpha1.AddressPrefixes", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GenesisAppState {
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
        if self.address_prefixes.is_some() {
            len += 1;
        }
        if !self.accounts.is_empty() {
            len += 1;
        }
        if self.authority_sudo_address.is_some() {
            len += 1;
        }
        if self.ibc_sudo_address.is_some() {
            len += 1;
        }
        if !self.ibc_relayer_addresses.is_empty() {
            len += 1;
        }
        if !self.native_asset_base_denomination.is_empty() {
            len += 1;
        }
        if self.ibc_parameters.is_some() {
            len += 1;
        }
        if !self.allowed_fee_assets.is_empty() {
            len += 1;
        }
        if self.genesis_fees.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.genesis.v1alpha1.GenesisAppState", len)?;
        if !self.chain_id.is_empty() {
            struct_ser.serialize_field("chainId", &self.chain_id)?;
        }
        if let Some(v) = self.address_prefixes.as_ref() {
            struct_ser.serialize_field("addressPrefixes", v)?;
        }
        if !self.accounts.is_empty() {
            struct_ser.serialize_field("accounts", &self.accounts)?;
        }
        if let Some(v) = self.authority_sudo_address.as_ref() {
            struct_ser.serialize_field("authoritySudoAddress", v)?;
        }
        if let Some(v) = self.ibc_sudo_address.as_ref() {
            struct_ser.serialize_field("ibcSudoAddress", v)?;
        }
        if !self.ibc_relayer_addresses.is_empty() {
            struct_ser.serialize_field("ibcRelayerAddresses", &self.ibc_relayer_addresses)?;
        }
        if !self.native_asset_base_denomination.is_empty() {
            struct_ser.serialize_field("nativeAssetBaseDenomination", &self.native_asset_base_denomination)?;
        }
        if let Some(v) = self.ibc_parameters.as_ref() {
            struct_ser.serialize_field("ibcParameters", v)?;
        }
        if !self.allowed_fee_assets.is_empty() {
            struct_ser.serialize_field("allowedFeeAssets", &self.allowed_fee_assets)?;
        }
        if let Some(v) = self.genesis_fees.as_ref() {
            struct_ser.serialize_field("genesisFees", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GenesisAppState {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "chain_id",
            "chainId",
            "address_prefixes",
            "addressPrefixes",
            "accounts",
            "authority_sudo_address",
            "authoritySudoAddress",
            "ibc_sudo_address",
            "ibcSudoAddress",
            "ibc_relayer_addresses",
            "ibcRelayerAddresses",
            "native_asset_base_denomination",
            "nativeAssetBaseDenomination",
            "ibc_parameters",
            "ibcParameters",
            "allowed_fee_assets",
            "allowedFeeAssets",
            "genesis_fees",
            "genesisFees",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ChainId,
            AddressPrefixes,
            Accounts,
            AuthoritySudoAddress,
            IbcSudoAddress,
            IbcRelayerAddresses,
            NativeAssetBaseDenomination,
            IbcParameters,
            AllowedFeeAssets,
            GenesisFees,
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
                            "addressPrefixes" | "address_prefixes" => Ok(GeneratedField::AddressPrefixes),
                            "accounts" => Ok(GeneratedField::Accounts),
                            "authoritySudoAddress" | "authority_sudo_address" => Ok(GeneratedField::AuthoritySudoAddress),
                            "ibcSudoAddress" | "ibc_sudo_address" => Ok(GeneratedField::IbcSudoAddress),
                            "ibcRelayerAddresses" | "ibc_relayer_addresses" => Ok(GeneratedField::IbcRelayerAddresses),
                            "nativeAssetBaseDenomination" | "native_asset_base_denomination" => Ok(GeneratedField::NativeAssetBaseDenomination),
                            "ibcParameters" | "ibc_parameters" => Ok(GeneratedField::IbcParameters),
                            "allowedFeeAssets" | "allowed_fee_assets" => Ok(GeneratedField::AllowedFeeAssets),
                            "genesisFees" | "genesis_fees" => Ok(GeneratedField::GenesisFees),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GenesisAppState;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.genesis.v1alpha1.GenesisAppState")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GenesisAppState, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut chain_id__ = None;
                let mut address_prefixes__ = None;
                let mut accounts__ = None;
                let mut authority_sudo_address__ = None;
                let mut ibc_sudo_address__ = None;
                let mut ibc_relayer_addresses__ = None;
                let mut native_asset_base_denomination__ = None;
                let mut ibc_parameters__ = None;
                let mut allowed_fee_assets__ = None;
                let mut genesis_fees__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ChainId => {
                            if chain_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("chainId"));
                            }
                            chain_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::AddressPrefixes => {
                            if address_prefixes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("addressPrefixes"));
                            }
                            address_prefixes__ = map_.next_value()?;
                        }
                        GeneratedField::Accounts => {
                            if accounts__.is_some() {
                                return Err(serde::de::Error::duplicate_field("accounts"));
                            }
                            accounts__ = Some(map_.next_value()?);
                        }
                        GeneratedField::AuthoritySudoAddress => {
                            if authority_sudo_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("authoritySudoAddress"));
                            }
                            authority_sudo_address__ = map_.next_value()?;
                        }
                        GeneratedField::IbcSudoAddress => {
                            if ibc_sudo_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ibcSudoAddress"));
                            }
                            ibc_sudo_address__ = map_.next_value()?;
                        }
                        GeneratedField::IbcRelayerAddresses => {
                            if ibc_relayer_addresses__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ibcRelayerAddresses"));
                            }
                            ibc_relayer_addresses__ = Some(map_.next_value()?);
                        }
                        GeneratedField::NativeAssetBaseDenomination => {
                            if native_asset_base_denomination__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nativeAssetBaseDenomination"));
                            }
                            native_asset_base_denomination__ = Some(map_.next_value()?);
                        }
                        GeneratedField::IbcParameters => {
                            if ibc_parameters__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ibcParameters"));
                            }
                            ibc_parameters__ = map_.next_value()?;
                        }
                        GeneratedField::AllowedFeeAssets => {
                            if allowed_fee_assets__.is_some() {
                                return Err(serde::de::Error::duplicate_field("allowedFeeAssets"));
                            }
                            allowed_fee_assets__ = Some(map_.next_value()?);
                        }
                        GeneratedField::GenesisFees => {
                            if genesis_fees__.is_some() {
                                return Err(serde::de::Error::duplicate_field("genesisFees"));
                            }
                            genesis_fees__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GenesisAppState {
                    chain_id: chain_id__.unwrap_or_default(),
                    address_prefixes: address_prefixes__,
                    accounts: accounts__.unwrap_or_default(),
                    authority_sudo_address: authority_sudo_address__,
                    ibc_sudo_address: ibc_sudo_address__,
                    ibc_relayer_addresses: ibc_relayer_addresses__.unwrap_or_default(),
                    native_asset_base_denomination: native_asset_base_denomination__.unwrap_or_default(),
                    ibc_parameters: ibc_parameters__,
                    allowed_fee_assets: allowed_fee_assets__.unwrap_or_default(),
                    genesis_fees: genesis_fees__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.genesis.v1alpha1.GenesisAppState", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GenesisFees {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.transfer_fees.is_some() {
            len += 1;
        }
        if self.sequence_fees.is_some() {
            len += 1;
        }
        if self.ics20_withdrawal_fees.is_some() {
            len += 1;
        }
        if self.init_bridge_account_fees.is_some() {
            len += 1;
        }
        if self.bridge_lock_fees.is_some() {
            len += 1;
        }
        if self.bridge_unlock_fees.is_some() {
            len += 1;
        }
        if self.bridge_sudo_change_fees.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.genesis.v1alpha1.GenesisFees", len)?;
        if let Some(v) = self.transfer_fees.as_ref() {
            struct_ser.serialize_field("transferFees", v)?;
        }
        if let Some(v) = self.sequence_fees.as_ref() {
            struct_ser.serialize_field("sequenceFees", v)?;
        }
        if let Some(v) = self.ics20_withdrawal_fees.as_ref() {
            struct_ser.serialize_field("ics20WithdrawalFees", v)?;
        }
        if let Some(v) = self.init_bridge_account_fees.as_ref() {
            struct_ser.serialize_field("initBridgeAccountFees", v)?;
        }
        if let Some(v) = self.bridge_lock_fees.as_ref() {
            struct_ser.serialize_field("bridgeLockFees", v)?;
        }
        if let Some(v) = self.bridge_unlock_fees.as_ref() {
            struct_ser.serialize_field("bridgeUnlockFees", v)?;
        }
        if let Some(v) = self.bridge_sudo_change_fees.as_ref() {
            struct_ser.serialize_field("bridgeSudoChangeFees", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GenesisFees {
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
            "ics20_withdrawal_fees",
            "ics20WithdrawalFees",
            "init_bridge_account_fees",
            "initBridgeAccountFees",
            "bridge_lock_fees",
            "bridgeLockFees",
            "bridge_unlock_fees",
            "bridgeUnlockFees",
            "bridge_sudo_change_fees",
            "bridgeSudoChangeFees",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TransferFees,
            SequenceFees,
            Ics20WithdrawalFees,
            InitBridgeAccountFees,
            BridgeLockFees,
            BridgeUnlockFees,
            BridgeSudoChangeFees,
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
                            "ics20WithdrawalFees" | "ics20_withdrawal_fees" => Ok(GeneratedField::Ics20WithdrawalFees),
                            "initBridgeAccountFees" | "init_bridge_account_fees" => Ok(GeneratedField::InitBridgeAccountFees),
                            "bridgeLockFees" | "bridge_lock_fees" => Ok(GeneratedField::BridgeLockFees),
                            "bridgeUnlockFees" | "bridge_unlock_fees" => Ok(GeneratedField::BridgeUnlockFees),
                            "bridgeSudoChangeFees" | "bridge_sudo_change_fees" => Ok(GeneratedField::BridgeSudoChangeFees),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GenesisFees;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.genesis.v1alpha1.GenesisFees")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GenesisFees, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut transfer_fees__ = None;
                let mut sequence_fees__ = None;
                let mut ics20_withdrawal_fees__ = None;
                let mut init_bridge_account_fees__ = None;
                let mut bridge_lock_fees__ = None;
                let mut bridge_unlock_fees__ = None;
                let mut bridge_sudo_change_fees__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TransferFees => {
                            if transfer_fees__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transferFees"));
                            }
                            transfer_fees__ = map_.next_value()?;
                        }
                        GeneratedField::SequenceFees => {
                            if sequence_fees__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequenceFees"));
                            }
                            sequence_fees__ = map_.next_value()?;
                        }
                        GeneratedField::Ics20WithdrawalFees => {
                            if ics20_withdrawal_fees__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ics20WithdrawalFees"));
                            }
                            ics20_withdrawal_fees__ = map_.next_value()?;
                        }
                        GeneratedField::InitBridgeAccountFees => {
                            if init_bridge_account_fees__.is_some() {
                                return Err(serde::de::Error::duplicate_field("initBridgeAccountFees"));
                            }
                            init_bridge_account_fees__ = map_.next_value()?;
                        }
                        GeneratedField::BridgeLockFees => {
                            if bridge_lock_fees__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeLockFees"));
                            }
                            bridge_lock_fees__ = map_.next_value()?;
                        }
                        GeneratedField::BridgeUnlockFees => {
                            if bridge_unlock_fees__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeUnlockFees"));
                            }
                            bridge_unlock_fees__ = map_.next_value()?;
                        }
                        GeneratedField::BridgeSudoChangeFees => {
                            if bridge_sudo_change_fees__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bridgeSudoChangeFees"));
                            }
                            bridge_sudo_change_fees__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GenesisFees {
                    transfer_fees: transfer_fees__,
                    sequence_fees: sequence_fees__,
                    ics20_withdrawal_fees: ics20_withdrawal_fees__,
                    init_bridge_account_fees: init_bridge_account_fees__,
                    bridge_lock_fees: bridge_lock_fees__,
                    bridge_unlock_fees: bridge_unlock_fees__,
                    bridge_sudo_change_fees: bridge_sudo_change_fees__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.genesis.v1alpha1.GenesisFees", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for IbcParameters {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.ibc_enabled {
            len += 1;
        }
        if self.inbound_ics20_transfers_enabled {
            len += 1;
        }
        if self.outbound_ics20_transfers_enabled {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.genesis.v1alpha1.IbcParameters", len)?;
        if self.ibc_enabled {
            struct_ser.serialize_field("ibcEnabled", &self.ibc_enabled)?;
        }
        if self.inbound_ics20_transfers_enabled {
            struct_ser.serialize_field("inboundIcs20TransfersEnabled", &self.inbound_ics20_transfers_enabled)?;
        }
        if self.outbound_ics20_transfers_enabled {
            struct_ser.serialize_field("outboundIcs20TransfersEnabled", &self.outbound_ics20_transfers_enabled)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for IbcParameters {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "ibc_enabled",
            "ibcEnabled",
            "inbound_ics20_transfers_enabled",
            "inboundIcs20TransfersEnabled",
            "outbound_ics20_transfers_enabled",
            "outboundIcs20TransfersEnabled",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            IbcEnabled,
            InboundIcs20TransfersEnabled,
            OutboundIcs20TransfersEnabled,
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
                            "ibcEnabled" | "ibc_enabled" => Ok(GeneratedField::IbcEnabled),
                            "inboundIcs20TransfersEnabled" | "inbound_ics20_transfers_enabled" => Ok(GeneratedField::InboundIcs20TransfersEnabled),
                            "outboundIcs20TransfersEnabled" | "outbound_ics20_transfers_enabled" => Ok(GeneratedField::OutboundIcs20TransfersEnabled),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = IbcParameters;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.genesis.v1alpha1.IbcParameters")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<IbcParameters, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut ibc_enabled__ = None;
                let mut inbound_ics20_transfers_enabled__ = None;
                let mut outbound_ics20_transfers_enabled__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::IbcEnabled => {
                            if ibc_enabled__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ibcEnabled"));
                            }
                            ibc_enabled__ = Some(map_.next_value()?);
                        }
                        GeneratedField::InboundIcs20TransfersEnabled => {
                            if inbound_ics20_transfers_enabled__.is_some() {
                                return Err(serde::de::Error::duplicate_field("inboundIcs20TransfersEnabled"));
                            }
                            inbound_ics20_transfers_enabled__ = Some(map_.next_value()?);
                        }
                        GeneratedField::OutboundIcs20TransfersEnabled => {
                            if outbound_ics20_transfers_enabled__.is_some() {
                                return Err(serde::de::Error::duplicate_field("outboundIcs20TransfersEnabled"));
                            }
                            outbound_ics20_transfers_enabled__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(IbcParameters {
                    ibc_enabled: ibc_enabled__.unwrap_or_default(),
                    inbound_ics20_transfers_enabled: inbound_ics20_transfers_enabled__.unwrap_or_default(),
                    outbound_ics20_transfers_enabled: outbound_ics20_transfers_enabled__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.genesis.v1alpha1.IbcParameters", FIELDS, GeneratedVisitor)
    }
}
