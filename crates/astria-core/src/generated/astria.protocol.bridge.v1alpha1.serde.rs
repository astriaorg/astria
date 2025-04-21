impl serde::Serialize for BridgeAccountInfoResponse {
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
        if self.rollup_id.is_some() {
            len += 1;
        }
        if self.asset.is_some() {
            len += 1;
        }
        if self.sudo_address.is_some() {
            len += 1;
        }
        if self.withdrawer_address.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.bridge.v1alpha1.BridgeAccountInfoResponse", len)?;
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        if let Some(v) = self.rollup_id.as_ref() {
            struct_ser.serialize_field("rollupId", v)?;
        }
        if let Some(v) = self.asset.as_ref() {
            struct_ser.serialize_field("asset", v)?;
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
impl<'de> serde::Deserialize<'de> for BridgeAccountInfoResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "rollup_id",
            "rollupId",
            "asset",
            "sudo_address",
            "sudoAddress",
            "withdrawer_address",
            "withdrawerAddress",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            RollupId,
            Asset,
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
                            "height" => Ok(GeneratedField::Height),
                            "rollupId" | "rollup_id" => Ok(GeneratedField::RollupId),
                            "asset" => Ok(GeneratedField::Asset),
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
            type Value = BridgeAccountInfoResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.bridge.v1alpha1.BridgeAccountInfoResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BridgeAccountInfoResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut rollup_id__ = None;
                let mut asset__ = None;
                let mut sudo_address__ = None;
                let mut withdrawer_address__ = None;
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
                            asset__ = map_.next_value()?;
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
                Ok(BridgeAccountInfoResponse {
                    height: height__.unwrap_or_default(),
                    rollup_id: rollup_id__,
                    asset: asset__,
                    sudo_address: sudo_address__,
                    withdrawer_address: withdrawer_address__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.bridge.v1alpha1.BridgeAccountInfoResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BridgeAccountLastTxHashResponse {
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
        if self.tx_hash.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.bridge.v1alpha1.BridgeAccountLastTxHashResponse", len)?;
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        if let Some(v) = self.tx_hash.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("txHash", pbjson::private::base64::encode(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BridgeAccountLastTxHashResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "tx_hash",
            "txHash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            TxHash,
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
                            "txHash" | "tx_hash" => Ok(GeneratedField::TxHash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BridgeAccountLastTxHashResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.bridge.v1alpha1.BridgeAccountLastTxHashResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BridgeAccountLastTxHashResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut tx_hash__ = None;
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
                        GeneratedField::TxHash => {
                            if tx_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("txHash"));
                            }
                            tx_hash__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::BytesDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(BridgeAccountLastTxHashResponse {
                    height: height__.unwrap_or_default(),
                    tx_hash: tx_hash__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.bridge.v1alpha1.BridgeAccountLastTxHashResponse", FIELDS, GeneratedVisitor)
    }
}
