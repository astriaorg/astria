impl serde::Serialize for TxStatusRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.tx_id.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("celestia.core.v1.tx.TxStatusRequest", len)?;
        if !self.tx_id.is_empty() {
            struct_ser.serialize_field("txId", &self.tx_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TxStatusRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "tx_id",
            "txId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TxId,
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
                            "txId" | "tx_id" => Ok(GeneratedField::TxId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TxStatusRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct celestia.core.v1.tx.TxStatusRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TxStatusRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut tx_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TxId => {
                            if tx_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("txId"));
                            }
                            tx_id__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(TxStatusRequest {
                    tx_id: tx_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("celestia.core.v1.tx.TxStatusRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TxStatusResponse {
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
        if self.index != 0 {
            len += 1;
        }
        if self.execution_code != 0 {
            len += 1;
        }
        if !self.error.is_empty() {
            len += 1;
        }
        if !self.status.is_empty() {
            len += 1;
        }
        if !self.codespace.is_empty() {
            len += 1;
        }
        if self.gas_wanted != 0 {
            len += 1;
        }
        if self.gas_used != 0 {
            len += 1;
        }
        if !self.signers.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("celestia.core.v1.tx.TxStatusResponse", len)?;
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        if self.index != 0 {
            struct_ser.serialize_field("index", &self.index)?;
        }
        if self.execution_code != 0 {
            struct_ser.serialize_field("executionCode", &self.execution_code)?;
        }
        if !self.error.is_empty() {
            struct_ser.serialize_field("error", &self.error)?;
        }
        if !self.status.is_empty() {
            struct_ser.serialize_field("status", &self.status)?;
        }
        if !self.codespace.is_empty() {
            struct_ser.serialize_field("codespace", &self.codespace)?;
        }
        if self.gas_wanted != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("gasWanted", ToString::to_string(&self.gas_wanted).as_str())?;
        }
        if self.gas_used != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("gasUsed", ToString::to_string(&self.gas_used).as_str())?;
        }
        if !self.signers.is_empty() {
            struct_ser.serialize_field("signers", &self.signers)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TxStatusResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "index",
            "execution_code",
            "executionCode",
            "error",
            "status",
            "codespace",
            "gas_wanted",
            "gasWanted",
            "gas_used",
            "gasUsed",
            "signers",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            Index,
            ExecutionCode,
            Error,
            Status,
            Codespace,
            GasWanted,
            GasUsed,
            Signers,
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
                            "index" => Ok(GeneratedField::Index),
                            "executionCode" | "execution_code" => Ok(GeneratedField::ExecutionCode),
                            "error" => Ok(GeneratedField::Error),
                            "status" => Ok(GeneratedField::Status),
                            "codespace" => Ok(GeneratedField::Codespace),
                            "gasWanted" | "gas_wanted" => Ok(GeneratedField::GasWanted),
                            "gasUsed" | "gas_used" => Ok(GeneratedField::GasUsed),
                            "signers" => Ok(GeneratedField::Signers),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TxStatusResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct celestia.core.v1.tx.TxStatusResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TxStatusResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut index__ = None;
                let mut execution_code__ = None;
                let mut error__ = None;
                let mut status__ = None;
                let mut codespace__ = None;
                let mut gas_wanted__ = None;
                let mut gas_used__ = None;
                let mut signers__ = None;
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
                        GeneratedField::Index => {
                            if index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("index"));
                            }
                            index__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ExecutionCode => {
                            if execution_code__.is_some() {
                                return Err(serde::de::Error::duplicate_field("executionCode"));
                            }
                            execution_code__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Error => {
                            if error__.is_some() {
                                return Err(serde::de::Error::duplicate_field("error"));
                            }
                            error__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Status => {
                            if status__.is_some() {
                                return Err(serde::de::Error::duplicate_field("status"));
                            }
                            status__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Codespace => {
                            if codespace__.is_some() {
                                return Err(serde::de::Error::duplicate_field("codespace"));
                            }
                            codespace__ = Some(map_.next_value()?);
                        }
                        GeneratedField::GasWanted => {
                            if gas_wanted__.is_some() {
                                return Err(serde::de::Error::duplicate_field("gasWanted"));
                            }
                            gas_wanted__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::GasUsed => {
                            if gas_used__.is_some() {
                                return Err(serde::de::Error::duplicate_field("gasUsed"));
                            }
                            gas_used__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Signers => {
                            if signers__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signers"));
                            }
                            signers__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(TxStatusResponse {
                    height: height__.unwrap_or_default(),
                    index: index__.unwrap_or_default(),
                    execution_code: execution_code__.unwrap_or_default(),
                    error: error__.unwrap_or_default(),
                    status: status__.unwrap_or_default(),
                    codespace: codespace__.unwrap_or_default(),
                    gas_wanted: gas_wanted__.unwrap_or_default(),
                    gas_used: gas_used__.unwrap_or_default(),
                    signers: signers__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("celestia.core.v1.tx.TxStatusResponse", FIELDS, GeneratedVisitor)
    }
}
