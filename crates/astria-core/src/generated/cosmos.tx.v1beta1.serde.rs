impl serde::Serialize for AuthInfo {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.signer_infos.is_empty() {
            len += 1;
        }
        if self.fee.is_some() {
            len += 1;
        }
        if self.tip.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("cosmos.tx.v1beta1.AuthInfo", len)?;
        if !self.signer_infos.is_empty() {
            struct_ser.serialize_field("signer_infos", &self.signer_infos)?;
        }
        if let Some(v) = self.fee.as_ref() {
            struct_ser.serialize_field("fee", v)?;
        }
        if let Some(v) = self.tip.as_ref() {
            struct_ser.serialize_field("tip", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AuthInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "signer_infos",
            "signerInfos",
            "fee",
            "tip",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SignerInfos,
            Fee,
            Tip,
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
                            "signerInfos" | "signer_infos" => Ok(GeneratedField::SignerInfos),
                            "fee" => Ok(GeneratedField::Fee),
                            "tip" => Ok(GeneratedField::Tip),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AuthInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct cosmos.tx.v1beta1.AuthInfo")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AuthInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut signer_infos__ = None;
                let mut fee__ = None;
                let mut tip__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::SignerInfos => {
                            if signer_infos__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signerInfos"));
                            }
                            signer_infos__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Fee => {
                            if fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("fee"));
                            }
                            fee__ = map_.next_value()?;
                        }
                        GeneratedField::Tip => {
                            if tip__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tip"));
                            }
                            tip__ = map_.next_value()?;
                        }
                    }
                }
                Ok(AuthInfo {
                    signer_infos: signer_infos__.unwrap_or_default(),
                    fee: fee__,
                    tip: tip__,
                })
            }
        }
        deserializer.deserialize_struct("cosmos.tx.v1beta1.AuthInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BroadcastMode {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "BROADCAST_MODE_UNSPECIFIED",
            Self::Block => "BROADCAST_MODE_BLOCK",
            Self::Sync => "BROADCAST_MODE_SYNC",
            Self::Async => "BROADCAST_MODE_ASYNC",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for BroadcastMode {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "BROADCAST_MODE_UNSPECIFIED",
            "BROADCAST_MODE_BLOCK",
            "BROADCAST_MODE_SYNC",
            "BROADCAST_MODE_ASYNC",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BroadcastMode;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "BROADCAST_MODE_UNSPECIFIED" => Ok(BroadcastMode::Unspecified),
                    "BROADCAST_MODE_BLOCK" => Ok(BroadcastMode::Block),
                    "BROADCAST_MODE_SYNC" => Ok(BroadcastMode::Sync),
                    "BROADCAST_MODE_ASYNC" => Ok(BroadcastMode::Async),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for BroadcastTxRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.tx_bytes.is_empty() {
            len += 1;
        }
        if self.mode != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("cosmos.tx.v1beta1.BroadcastTxRequest", len)?;
        if !self.tx_bytes.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("tx_bytes", pbjson::private::base64::encode(&self.tx_bytes).as_str())?;
        }
        if self.mode != 0 {
            let v = BroadcastMode::try_from(self.mode)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.mode)))?;
            struct_ser.serialize_field("mode", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BroadcastTxRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "tx_bytes",
            "txBytes",
            "mode",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TxBytes,
            Mode,
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
                            "txBytes" | "tx_bytes" => Ok(GeneratedField::TxBytes),
                            "mode" => Ok(GeneratedField::Mode),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BroadcastTxRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct cosmos.tx.v1beta1.BroadcastTxRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BroadcastTxRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut tx_bytes__ = None;
                let mut mode__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TxBytes => {
                            if tx_bytes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("txBytes"));
                            }
                            tx_bytes__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Mode => {
                            if mode__.is_some() {
                                return Err(serde::de::Error::duplicate_field("mode"));
                            }
                            mode__ = Some(map_.next_value::<BroadcastMode>()? as i32);
                        }
                    }
                }
                Ok(BroadcastTxRequest {
                    tx_bytes: tx_bytes__.unwrap_or_default(),
                    mode: mode__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("cosmos.tx.v1beta1.BroadcastTxRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BroadcastTxResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.tx_response.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("cosmos.tx.v1beta1.BroadcastTxResponse", len)?;
        if let Some(v) = self.tx_response.as_ref() {
            struct_ser.serialize_field("tx_response", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BroadcastTxResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "tx_response",
            "txResponse",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TxResponse,
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
                            "txResponse" | "tx_response" => Ok(GeneratedField::TxResponse),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BroadcastTxResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct cosmos.tx.v1beta1.BroadcastTxResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BroadcastTxResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut tx_response__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TxResponse => {
                            if tx_response__.is_some() {
                                return Err(serde::de::Error::duplicate_field("txResponse"));
                            }
                            tx_response__ = map_.next_value()?;
                        }
                    }
                }
                Ok(BroadcastTxResponse {
                    tx_response: tx_response__,
                })
            }
        }
        deserializer.deserialize_struct("cosmos.tx.v1beta1.BroadcastTxResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Fee {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.amount.is_empty() {
            len += 1;
        }
        if self.gas_limit != 0 {
            len += 1;
        }
        if !self.payer.is_empty() {
            len += 1;
        }
        if !self.granter.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("cosmos.tx.v1beta1.Fee", len)?;
        if !self.amount.is_empty() {
            struct_ser.serialize_field("amount", &self.amount)?;
        }
        if self.gas_limit != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("gas_limit", ToString::to_string(&self.gas_limit).as_str())?;
        }
        if !self.payer.is_empty() {
            struct_ser.serialize_field("payer", &self.payer)?;
        }
        if !self.granter.is_empty() {
            struct_ser.serialize_field("granter", &self.granter)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Fee {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "amount",
            "gas_limit",
            "gasLimit",
            "payer",
            "granter",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Amount,
            GasLimit,
            Payer,
            Granter,
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
                            "gasLimit" | "gas_limit" => Ok(GeneratedField::GasLimit),
                            "payer" => Ok(GeneratedField::Payer),
                            "granter" => Ok(GeneratedField::Granter),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Fee;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct cosmos.tx.v1beta1.Fee")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Fee, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut amount__ = None;
                let mut gas_limit__ = None;
                let mut payer__ = None;
                let mut granter__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Amount => {
                            if amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("amount"));
                            }
                            amount__ = Some(map_.next_value()?);
                        }
                        GeneratedField::GasLimit => {
                            if gas_limit__.is_some() {
                                return Err(serde::de::Error::duplicate_field("gasLimit"));
                            }
                            gas_limit__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Payer => {
                            if payer__.is_some() {
                                return Err(serde::de::Error::duplicate_field("payer"));
                            }
                            payer__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Granter => {
                            if granter__.is_some() {
                                return Err(serde::de::Error::duplicate_field("granter"));
                            }
                            granter__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Fee {
                    amount: amount__.unwrap_or_default(),
                    gas_limit: gas_limit__.unwrap_or_default(),
                    payer: payer__.unwrap_or_default(),
                    granter: granter__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("cosmos.tx.v1beta1.Fee", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetTxRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.hash.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("cosmos.tx.v1beta1.GetTxRequest", len)?;
        if !self.hash.is_empty() {
            struct_ser.serialize_field("hash", &self.hash)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetTxRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "hash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Hash,
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
                            "hash" => Ok(GeneratedField::Hash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetTxRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct cosmos.tx.v1beta1.GetTxRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetTxRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Hash => {
                            if hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("hash"));
                            }
                            hash__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(GetTxRequest {
                    hash: hash__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("cosmos.tx.v1beta1.GetTxRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetTxResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.tx.is_some() {
            len += 1;
        }
        if self.tx_response.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("cosmos.tx.v1beta1.GetTxResponse", len)?;
        if let Some(v) = self.tx.as_ref() {
            struct_ser.serialize_field("tx", v)?;
        }
        if let Some(v) = self.tx_response.as_ref() {
            struct_ser.serialize_field("tx_response", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetTxResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "tx",
            "tx_response",
            "txResponse",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Tx,
            TxResponse,
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
                            "tx" => Ok(GeneratedField::Tx),
                            "txResponse" | "tx_response" => Ok(GeneratedField::TxResponse),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetTxResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct cosmos.tx.v1beta1.GetTxResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetTxResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut tx__ = None;
                let mut tx_response__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Tx => {
                            if tx__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tx"));
                            }
                            tx__ = map_.next_value()?;
                        }
                        GeneratedField::TxResponse => {
                            if tx_response__.is_some() {
                                return Err(serde::de::Error::duplicate_field("txResponse"));
                            }
                            tx_response__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GetTxResponse {
                    tx: tx__,
                    tx_response: tx_response__,
                })
            }
        }
        deserializer.deserialize_struct("cosmos.tx.v1beta1.GetTxResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ModeInfo {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.sum.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("cosmos.tx.v1beta1.ModeInfo", len)?;
        if let Some(v) = self.sum.as_ref() {
            match v {
                mode_info::Sum::Single(v) => {
                    struct_ser.serialize_field("single", v)?;
                }
                mode_info::Sum::Multi(v) => {
                    struct_ser.serialize_field("multi", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ModeInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "single",
            "multi",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Single,
            Multi,
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
                            "single" => Ok(GeneratedField::Single),
                            "multi" => Ok(GeneratedField::Multi),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ModeInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct cosmos.tx.v1beta1.ModeInfo")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ModeInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut sum__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Single => {
                            if sum__.is_some() {
                                return Err(serde::de::Error::duplicate_field("single"));
                            }
                            sum__ = map_.next_value::<::std::option::Option<_>>()?.map(mode_info::Sum::Single)
;
                        }
                        GeneratedField::Multi => {
                            if sum__.is_some() {
                                return Err(serde::de::Error::duplicate_field("multi"));
                            }
                            sum__ = map_.next_value::<::std::option::Option<_>>()?.map(mode_info::Sum::Multi)
;
                        }
                    }
                }
                Ok(ModeInfo {
                    sum: sum__,
                })
            }
        }
        deserializer.deserialize_struct("cosmos.tx.v1beta1.ModeInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for mode_info::Multi {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.bitarray.is_some() {
            len += 1;
        }
        if !self.mode_infos.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("cosmos.tx.v1beta1.ModeInfo.Multi", len)?;
        if let Some(v) = self.bitarray.as_ref() {
            struct_ser.serialize_field("bitarray", v)?;
        }
        if !self.mode_infos.is_empty() {
            struct_ser.serialize_field("mode_infos", &self.mode_infos)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for mode_info::Multi {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "bitarray",
            "mode_infos",
            "modeInfos",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Bitarray,
            ModeInfos,
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
                            "bitarray" => Ok(GeneratedField::Bitarray),
                            "modeInfos" | "mode_infos" => Ok(GeneratedField::ModeInfos),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = mode_info::Multi;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct cosmos.tx.v1beta1.ModeInfo.Multi")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<mode_info::Multi, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut bitarray__ = None;
                let mut mode_infos__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Bitarray => {
                            if bitarray__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bitarray"));
                            }
                            bitarray__ = map_.next_value()?;
                        }
                        GeneratedField::ModeInfos => {
                            if mode_infos__.is_some() {
                                return Err(serde::de::Error::duplicate_field("modeInfos"));
                            }
                            mode_infos__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(mode_info::Multi {
                    bitarray: bitarray__,
                    mode_infos: mode_infos__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("cosmos.tx.v1beta1.ModeInfo.Multi", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for mode_info::Single {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.mode != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("cosmos.tx.v1beta1.ModeInfo.Single", len)?;
        if self.mode != 0 {
            let v = super::signing::v1beta1::SignMode::try_from(self.mode)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.mode)))?;
            struct_ser.serialize_field("mode", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for mode_info::Single {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "mode",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Mode,
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
                            "mode" => Ok(GeneratedField::Mode),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = mode_info::Single;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct cosmos.tx.v1beta1.ModeInfo.Single")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<mode_info::Single, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut mode__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Mode => {
                            if mode__.is_some() {
                                return Err(serde::de::Error::duplicate_field("mode"));
                            }
                            mode__ = Some(map_.next_value::<super::signing::v1beta1::SignMode>()? as i32);
                        }
                    }
                }
                Ok(mode_info::Single {
                    mode: mode__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("cosmos.tx.v1beta1.ModeInfo.Single", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SignDoc {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.body_bytes.is_empty() {
            len += 1;
        }
        if !self.auth_info_bytes.is_empty() {
            len += 1;
        }
        if !self.chain_id.is_empty() {
            len += 1;
        }
        if self.account_number != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("cosmos.tx.v1beta1.SignDoc", len)?;
        if !self.body_bytes.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("body_bytes", pbjson::private::base64::encode(&self.body_bytes).as_str())?;
        }
        if !self.auth_info_bytes.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("auth_info_bytes", pbjson::private::base64::encode(&self.auth_info_bytes).as_str())?;
        }
        if !self.chain_id.is_empty() {
            struct_ser.serialize_field("chain_id", &self.chain_id)?;
        }
        if self.account_number != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("account_number", ToString::to_string(&self.account_number).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SignDoc {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "body_bytes",
            "bodyBytes",
            "auth_info_bytes",
            "authInfoBytes",
            "chain_id",
            "chainId",
            "account_number",
            "accountNumber",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BodyBytes,
            AuthInfoBytes,
            ChainId,
            AccountNumber,
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
                            "bodyBytes" | "body_bytes" => Ok(GeneratedField::BodyBytes),
                            "authInfoBytes" | "auth_info_bytes" => Ok(GeneratedField::AuthInfoBytes),
                            "chainId" | "chain_id" => Ok(GeneratedField::ChainId),
                            "accountNumber" | "account_number" => Ok(GeneratedField::AccountNumber),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SignDoc;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct cosmos.tx.v1beta1.SignDoc")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SignDoc, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut body_bytes__ = None;
                let mut auth_info_bytes__ = None;
                let mut chain_id__ = None;
                let mut account_number__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BodyBytes => {
                            if body_bytes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bodyBytes"));
                            }
                            body_bytes__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::AuthInfoBytes => {
                            if auth_info_bytes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("authInfoBytes"));
                            }
                            auth_info_bytes__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ChainId => {
                            if chain_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("chainId"));
                            }
                            chain_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::AccountNumber => {
                            if account_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("accountNumber"));
                            }
                            account_number__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(SignDoc {
                    body_bytes: body_bytes__.unwrap_or_default(),
                    auth_info_bytes: auth_info_bytes__.unwrap_or_default(),
                    chain_id: chain_id__.unwrap_or_default(),
                    account_number: account_number__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("cosmos.tx.v1beta1.SignDoc", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SignerInfo {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.public_key.is_some() {
            len += 1;
        }
        if self.mode_info.is_some() {
            len += 1;
        }
        if self.sequence != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("cosmos.tx.v1beta1.SignerInfo", len)?;
        if let Some(v) = self.public_key.as_ref() {
            struct_ser.serialize_field("public_key", v)?;
        }
        if let Some(v) = self.mode_info.as_ref() {
            struct_ser.serialize_field("mode_info", v)?;
        }
        if self.sequence != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("sequence", ToString::to_string(&self.sequence).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SignerInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "public_key",
            "publicKey",
            "mode_info",
            "modeInfo",
            "sequence",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            PublicKey,
            ModeInfo,
            Sequence,
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
                            "publicKey" | "public_key" => Ok(GeneratedField::PublicKey),
                            "modeInfo" | "mode_info" => Ok(GeneratedField::ModeInfo),
                            "sequence" => Ok(GeneratedField::Sequence),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SignerInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct cosmos.tx.v1beta1.SignerInfo")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SignerInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut public_key__ = None;
                let mut mode_info__ = None;
                let mut sequence__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::PublicKey => {
                            if public_key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("publicKey"));
                            }
                            public_key__ = map_.next_value()?;
                        }
                        GeneratedField::ModeInfo => {
                            if mode_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("modeInfo"));
                            }
                            mode_info__ = map_.next_value()?;
                        }
                        GeneratedField::Sequence => {
                            if sequence__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequence"));
                            }
                            sequence__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(SignerInfo {
                    public_key: public_key__,
                    mode_info: mode_info__,
                    sequence: sequence__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("cosmos.tx.v1beta1.SignerInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Tip {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.amount.is_empty() {
            len += 1;
        }
        if !self.tipper.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("cosmos.tx.v1beta1.Tip", len)?;
        if !self.amount.is_empty() {
            struct_ser.serialize_field("amount", &self.amount)?;
        }
        if !self.tipper.is_empty() {
            struct_ser.serialize_field("tipper", &self.tipper)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Tip {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "amount",
            "tipper",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Amount,
            Tipper,
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
                            "tipper" => Ok(GeneratedField::Tipper),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Tip;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct cosmos.tx.v1beta1.Tip")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Tip, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut amount__ = None;
                let mut tipper__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Amount => {
                            if amount__.is_some() {
                                return Err(serde::de::Error::duplicate_field("amount"));
                            }
                            amount__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Tipper => {
                            if tipper__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tipper"));
                            }
                            tipper__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Tip {
                    amount: amount__.unwrap_or_default(),
                    tipper: tipper__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("cosmos.tx.v1beta1.Tip", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Tx {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.body.is_some() {
            len += 1;
        }
        if self.auth_info.is_some() {
            len += 1;
        }
        if !self.signatures.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("cosmos.tx.v1beta1.Tx", len)?;
        if let Some(v) = self.body.as_ref() {
            struct_ser.serialize_field("body", v)?;
        }
        if let Some(v) = self.auth_info.as_ref() {
            struct_ser.serialize_field("auth_info", v)?;
        }
        if !self.signatures.is_empty() {
            struct_ser.serialize_field("signatures", &self.signatures.iter().map(pbjson::private::base64::encode).collect::<Vec<_>>())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Tx {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "body",
            "auth_info",
            "authInfo",
            "signatures",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Body,
            AuthInfo,
            Signatures,
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
                            "body" => Ok(GeneratedField::Body),
                            "authInfo" | "auth_info" => Ok(GeneratedField::AuthInfo),
                            "signatures" => Ok(GeneratedField::Signatures),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Tx;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct cosmos.tx.v1beta1.Tx")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Tx, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut body__ = None;
                let mut auth_info__ = None;
                let mut signatures__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Body => {
                            if body__.is_some() {
                                return Err(serde::de::Error::duplicate_field("body"));
                            }
                            body__ = map_.next_value()?;
                        }
                        GeneratedField::AuthInfo => {
                            if auth_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("authInfo"));
                            }
                            auth_info__ = map_.next_value()?;
                        }
                        GeneratedField::Signatures => {
                            if signatures__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signatures"));
                            }
                            signatures__ = 
                                Some(map_.next_value::<Vec<::pbjson::private::BytesDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                    }
                }
                Ok(Tx {
                    body: body__,
                    auth_info: auth_info__,
                    signatures: signatures__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("cosmos.tx.v1beta1.Tx", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TxBody {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.messages.is_empty() {
            len += 1;
        }
        if !self.memo.is_empty() {
            len += 1;
        }
        if self.timeout_height != 0 {
            len += 1;
        }
        if !self.extension_options.is_empty() {
            len += 1;
        }
        if !self.non_critical_extension_options.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("cosmos.tx.v1beta1.TxBody", len)?;
        if !self.messages.is_empty() {
            struct_ser.serialize_field("messages", &self.messages)?;
        }
        if !self.memo.is_empty() {
            struct_ser.serialize_field("memo", &self.memo)?;
        }
        if self.timeout_height != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("timeout_height", ToString::to_string(&self.timeout_height).as_str())?;
        }
        if !self.extension_options.is_empty() {
            struct_ser.serialize_field("extension_options", &self.extension_options)?;
        }
        if !self.non_critical_extension_options.is_empty() {
            struct_ser.serialize_field("non_critical_extension_options", &self.non_critical_extension_options)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TxBody {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "messages",
            "memo",
            "timeout_height",
            "timeoutHeight",
            "extension_options",
            "extensionOptions",
            "non_critical_extension_options",
            "nonCriticalExtensionOptions",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Messages,
            Memo,
            TimeoutHeight,
            ExtensionOptions,
            NonCriticalExtensionOptions,
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
                            "messages" => Ok(GeneratedField::Messages),
                            "memo" => Ok(GeneratedField::Memo),
                            "timeoutHeight" | "timeout_height" => Ok(GeneratedField::TimeoutHeight),
                            "extensionOptions" | "extension_options" => Ok(GeneratedField::ExtensionOptions),
                            "nonCriticalExtensionOptions" | "non_critical_extension_options" => Ok(GeneratedField::NonCriticalExtensionOptions),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TxBody;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct cosmos.tx.v1beta1.TxBody")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TxBody, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut messages__ = None;
                let mut memo__ = None;
                let mut timeout_height__ = None;
                let mut extension_options__ = None;
                let mut non_critical_extension_options__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Messages => {
                            if messages__.is_some() {
                                return Err(serde::de::Error::duplicate_field("messages"));
                            }
                            messages__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Memo => {
                            if memo__.is_some() {
                                return Err(serde::de::Error::duplicate_field("memo"));
                            }
                            memo__ = Some(map_.next_value()?);
                        }
                        GeneratedField::TimeoutHeight => {
                            if timeout_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timeoutHeight"));
                            }
                            timeout_height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ExtensionOptions => {
                            if extension_options__.is_some() {
                                return Err(serde::de::Error::duplicate_field("extensionOptions"));
                            }
                            extension_options__ = Some(map_.next_value()?);
                        }
                        GeneratedField::NonCriticalExtensionOptions => {
                            if non_critical_extension_options__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nonCriticalExtensionOptions"));
                            }
                            non_critical_extension_options__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(TxBody {
                    messages: messages__.unwrap_or_default(),
                    memo: memo__.unwrap_or_default(),
                    timeout_height: timeout_height__.unwrap_or_default(),
                    extension_options: extension_options__.unwrap_or_default(),
                    non_critical_extension_options: non_critical_extension_options__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("cosmos.tx.v1beta1.TxBody", FIELDS, GeneratedVisitor)
    }
}
