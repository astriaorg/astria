impl serde::Serialize for GetTransactionFeesRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.transaction_body.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.mempool.v1.GetTransactionFeesRequest", len)?;
        if let Some(v) = self.transaction_body.as_ref() {
            struct_ser.serialize_field("transactionBody", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetTransactionFeesRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "transaction_body",
            "transactionBody",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TransactionBody,
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
                            "transactionBody" | "transaction_body" => Ok(GeneratedField::TransactionBody),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetTransactionFeesRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.mempool.v1.GetTransactionFeesRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetTransactionFeesRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut transaction_body__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TransactionBody => {
                            if transaction_body__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactionBody"));
                            }
                            transaction_body__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GetTransactionFeesRequest {
                    transaction_body: transaction_body__,
                })
            }
        }
        deserializer.deserialize_struct("astria.mempool.v1.GetTransactionFeesRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetTransactionFeesResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.block_height != 0 {
            len += 1;
        }
        if !self.fees.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.mempool.v1.GetTransactionFeesResponse", len)?;
        if self.block_height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("blockHeight", ToString::to_string(&self.block_height).as_str())?;
        }
        if !self.fees.is_empty() {
            struct_ser.serialize_field("fees", &self.fees)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetTransactionFeesResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "block_height",
            "blockHeight",
            "fees",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BlockHeight,
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
                            "blockHeight" | "block_height" => Ok(GeneratedField::BlockHeight),
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
            type Value = GetTransactionFeesResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.mempool.v1.GetTransactionFeesResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetTransactionFeesResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut block_height__ = None;
                let mut fees__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BlockHeight => {
                            if block_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockHeight"));
                            }
                            block_height__ = 
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
                Ok(GetTransactionFeesResponse {
                    block_height: block_height__.unwrap_or_default(),
                    fees: fees__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.mempool.v1.GetTransactionFeesResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetTransactionStatusRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.transaction_hash.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.mempool.v1.GetTransactionStatusRequest", len)?;
        if !self.transaction_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("transactionHash", pbjson::private::base64::encode(&self.transaction_hash).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetTransactionStatusRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "transaction_hash",
            "transactionHash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TransactionHash,
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
                            "transactionHash" | "transaction_hash" => Ok(GeneratedField::TransactionHash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetTransactionStatusRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.mempool.v1.GetTransactionStatusRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetTransactionStatusRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut transaction_hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TransactionHash => {
                            if transaction_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactionHash"));
                            }
                            transaction_hash__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(GetTransactionStatusRequest {
                    transaction_hash: transaction_hash__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.mempool.v1.GetTransactionStatusRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SubmitTransactionRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.transaction.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.mempool.v1.SubmitTransactionRequest", len)?;
        if let Some(v) = self.transaction.as_ref() {
            struct_ser.serialize_field("transaction", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SubmitTransactionRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "transaction",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
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
            type Value = SubmitTransactionRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.mempool.v1.SubmitTransactionRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SubmitTransactionRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut transaction__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Transaction => {
                            if transaction__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transaction"));
                            }
                            transaction__ = map_.next_value()?;
                        }
                    }
                }
                Ok(SubmitTransactionRequest {
                    transaction: transaction__,
                })
            }
        }
        deserializer.deserialize_struct("astria.mempool.v1.SubmitTransactionRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SubmitTransactionResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.status.is_some() {
            len += 1;
        }
        if self.duplicate {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.mempool.v1.SubmitTransactionResponse", len)?;
        if let Some(v) = self.status.as_ref() {
            struct_ser.serialize_field("status", v)?;
        }
        if self.duplicate {
            struct_ser.serialize_field("duplicate", &self.duplicate)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SubmitTransactionResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "status",
            "duplicate",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Status,
            Duplicate,
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
                            "status" => Ok(GeneratedField::Status),
                            "duplicate" => Ok(GeneratedField::Duplicate),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SubmitTransactionResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.mempool.v1.SubmitTransactionResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SubmitTransactionResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut status__ = None;
                let mut duplicate__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Status => {
                            if status__.is_some() {
                                return Err(serde::de::Error::duplicate_field("status"));
                            }
                            status__ = map_.next_value()?;
                        }
                        GeneratedField::Duplicate => {
                            if duplicate__.is_some() {
                                return Err(serde::de::Error::duplicate_field("duplicate"));
                            }
                            duplicate__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(SubmitTransactionResponse {
                    status: status__,
                    duplicate: duplicate__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.mempool.v1.SubmitTransactionResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TransactionStatus {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.transaction_hash.is_empty() {
            len += 1;
        }
        if self.status.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.mempool.v1.TransactionStatus", len)?;
        if !self.transaction_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("transactionHash", pbjson::private::base64::encode(&self.transaction_hash).as_str())?;
        }
        if let Some(v) = self.status.as_ref() {
            match v {
                transaction_status::Status::Pending(v) => {
                    struct_ser.serialize_field("pending", v)?;
                }
                transaction_status::Status::Parked(v) => {
                    struct_ser.serialize_field("parked", v)?;
                }
                transaction_status::Status::Removed(v) => {
                    struct_ser.serialize_field("removed", v)?;
                }
                transaction_status::Status::Executed(v) => {
                    struct_ser.serialize_field("executed", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionStatus {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "transaction_hash",
            "transactionHash",
            "pending",
            "parked",
            "removed",
            "executed",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TransactionHash,
            Pending,
            Parked,
            Removed,
            Executed,
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
                            "transactionHash" | "transaction_hash" => Ok(GeneratedField::TransactionHash),
                            "pending" => Ok(GeneratedField::Pending),
                            "parked" => Ok(GeneratedField::Parked),
                            "removed" => Ok(GeneratedField::Removed),
                            "executed" => Ok(GeneratedField::Executed),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionStatus;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.mempool.v1.TransactionStatus")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TransactionStatus, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut transaction_hash__ = None;
                let mut status__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TransactionHash => {
                            if transaction_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactionHash"));
                            }
                            transaction_hash__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Pending => {
                            if status__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pending"));
                            }
                            status__ = map_.next_value::<::std::option::Option<_>>()?.map(transaction_status::Status::Pending)
;
                        }
                        GeneratedField::Parked => {
                            if status__.is_some() {
                                return Err(serde::de::Error::duplicate_field("parked"));
                            }
                            status__ = map_.next_value::<::std::option::Option<_>>()?.map(transaction_status::Status::Parked)
;
                        }
                        GeneratedField::Removed => {
                            if status__.is_some() {
                                return Err(serde::de::Error::duplicate_field("removed"));
                            }
                            status__ = map_.next_value::<::std::option::Option<_>>()?.map(transaction_status::Status::Removed)
;
                        }
                        GeneratedField::Executed => {
                            if status__.is_some() {
                                return Err(serde::de::Error::duplicate_field("executed"));
                            }
                            status__ = map_.next_value::<::std::option::Option<_>>()?.map(transaction_status::Status::Executed)
;
                        }
                    }
                }
                Ok(TransactionStatus {
                    transaction_hash: transaction_hash__.unwrap_or_default(),
                    status: status__,
                })
            }
        }
        deserializer.deserialize_struct("astria.mempool.v1.TransactionStatus", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for transaction_status::Executed {
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
        if self.result.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.mempool.v1.TransactionStatus.Executed", len)?;
        if self.height != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("height", ToString::to_string(&self.height).as_str())?;
        }
        if let Some(v) = self.result.as_ref() {
            struct_ser.serialize_field("result", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for transaction_status::Executed {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "height",
            "result",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Height,
            Result,
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
                            "result" => Ok(GeneratedField::Result),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = transaction_status::Executed;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.mempool.v1.TransactionStatus.Executed")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<transaction_status::Executed, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut height__ = None;
                let mut result__ = None;
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
                        GeneratedField::Result => {
                            if result__.is_some() {
                                return Err(serde::de::Error::duplicate_field("result"));
                            }
                            result__ = map_.next_value()?;
                        }
                    }
                }
                Ok(transaction_status::Executed {
                    height: height__.unwrap_or_default(),
                    result: result__,
                })
            }
        }
        deserializer.deserialize_struct("astria.mempool.v1.TransactionStatus.Executed", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for transaction_status::Parked {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("astria.mempool.v1.TransactionStatus.Parked", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for transaction_status::Parked {
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
            type Value = transaction_status::Parked;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.mempool.v1.TransactionStatus.Parked")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<transaction_status::Parked, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(transaction_status::Parked {
                })
            }
        }
        deserializer.deserialize_struct("astria.mempool.v1.TransactionStatus.Parked", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for transaction_status::Pending {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("astria.mempool.v1.TransactionStatus.Pending", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for transaction_status::Pending {
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
            type Value = transaction_status::Pending;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.mempool.v1.TransactionStatus.Pending")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<transaction_status::Pending, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(transaction_status::Pending {
                })
            }
        }
        deserializer.deserialize_struct("astria.mempool.v1.TransactionStatus.Pending", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for transaction_status::Removed {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.reason.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.mempool.v1.TransactionStatus.Removed", len)?;
        if !self.reason.is_empty() {
            struct_ser.serialize_field("reason", &self.reason)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for transaction_status::Removed {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "reason",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Reason,
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
                            "reason" => Ok(GeneratedField::Reason),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = transaction_status::Removed;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.mempool.v1.TransactionStatus.Removed")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<transaction_status::Removed, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut reason__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Reason => {
                            if reason__.is_some() {
                                return Err(serde::de::Error::duplicate_field("reason"));
                            }
                            reason__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(transaction_status::Removed {
                    reason: reason__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.mempool.v1.TransactionStatus.Removed", FIELDS, GeneratedVisitor)
    }
}
