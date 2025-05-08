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
impl serde::Serialize for transaction_status::executed::Event {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.kind.is_empty() {
            len += 1;
        }
        if !self.attributes.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.mempool.v1.TransactionStatus.Executed.Event", len)?;
        if !self.kind.is_empty() {
            struct_ser.serialize_field("kind", &self.kind)?;
        }
        if !self.attributes.is_empty() {
            struct_ser.serialize_field("attributes", &self.attributes)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for transaction_status::executed::Event {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "kind",
            "attributes",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Kind,
            Attributes,
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
                            "kind" => Ok(GeneratedField::Kind),
                            "attributes" => Ok(GeneratedField::Attributes),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = transaction_status::executed::Event;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.mempool.v1.TransactionStatus.Executed.Event")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<transaction_status::executed::Event, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut kind__ = None;
                let mut attributes__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Kind => {
                            if kind__.is_some() {
                                return Err(serde::de::Error::duplicate_field("kind"));
                            }
                            kind__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Attributes => {
                            if attributes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("attributes"));
                            }
                            attributes__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(transaction_status::executed::Event {
                    kind: kind__.unwrap_or_default(),
                    attributes: attributes__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.mempool.v1.TransactionStatus.Executed.Event", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for transaction_status::executed::EventAttribute {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.key.is_empty() {
            len += 1;
        }
        if !self.value.is_empty() {
            len += 1;
        }
        if self.index {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.mempool.v1.TransactionStatus.Executed.EventAttribute", len)?;
        if !self.key.is_empty() {
            struct_ser.serialize_field("key", &self.key)?;
        }
        if !self.value.is_empty() {
            struct_ser.serialize_field("value", &self.value)?;
        }
        if self.index {
            struct_ser.serialize_field("index", &self.index)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for transaction_status::executed::EventAttribute {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "key",
            "value",
            "index",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Key,
            Value,
            Index,
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
                            "key" => Ok(GeneratedField::Key),
                            "value" => Ok(GeneratedField::Value),
                            "index" => Ok(GeneratedField::Index),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = transaction_status::executed::EventAttribute;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.mempool.v1.TransactionStatus.Executed.EventAttribute")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<transaction_status::executed::EventAttribute, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut key__ = None;
                let mut value__ = None;
                let mut index__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Key => {
                            if key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("key"));
                            }
                            key__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Value => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("value"));
                            }
                            value__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Index => {
                            if index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("index"));
                            }
                            index__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(transaction_status::executed::EventAttribute {
                    key: key__.unwrap_or_default(),
                    value: value__.unwrap_or_default(),
                    index: index__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.mempool.v1.TransactionStatus.Executed.EventAttribute", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for transaction_status::executed::ExecTxResult {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.code != 0 {
            len += 1;
        }
        if !self.data.is_empty() {
            len += 1;
        }
        if !self.log.is_empty() {
            len += 1;
        }
        if !self.info.is_empty() {
            len += 1;
        }
        if self.gas_wanted != 0 {
            len += 1;
        }
        if self.gas_used != 0 {
            len += 1;
        }
        if !self.events.is_empty() {
            len += 1;
        }
        if !self.codespace.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.mempool.v1.TransactionStatus.Executed.ExecTxResult", len)?;
        if self.code != 0 {
            struct_ser.serialize_field("code", &self.code)?;
        }
        if !self.data.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("data", pbjson::private::base64::encode(&self.data).as_str())?;
        }
        if !self.log.is_empty() {
            struct_ser.serialize_field("log", &self.log)?;
        }
        if !self.info.is_empty() {
            struct_ser.serialize_field("info", &self.info)?;
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
        if !self.events.is_empty() {
            struct_ser.serialize_field("events", &self.events)?;
        }
        if !self.codespace.is_empty() {
            struct_ser.serialize_field("codespace", &self.codespace)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for transaction_status::executed::ExecTxResult {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "code",
            "data",
            "log",
            "info",
            "gas_wanted",
            "gasWanted",
            "gas_used",
            "gasUsed",
            "events",
            "codespace",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Code,
            Data,
            Log,
            Info,
            GasWanted,
            GasUsed,
            Events,
            Codespace,
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
                            "code" => Ok(GeneratedField::Code),
                            "data" => Ok(GeneratedField::Data),
                            "log" => Ok(GeneratedField::Log),
                            "info" => Ok(GeneratedField::Info),
                            "gasWanted" | "gas_wanted" => Ok(GeneratedField::GasWanted),
                            "gasUsed" | "gas_used" => Ok(GeneratedField::GasUsed),
                            "events" => Ok(GeneratedField::Events),
                            "codespace" => Ok(GeneratedField::Codespace),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = transaction_status::executed::ExecTxResult;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.mempool.v1.TransactionStatus.Executed.ExecTxResult")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<transaction_status::executed::ExecTxResult, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut code__ = None;
                let mut data__ = None;
                let mut log__ = None;
                let mut info__ = None;
                let mut gas_wanted__ = None;
                let mut gas_used__ = None;
                let mut events__ = None;
                let mut codespace__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Code => {
                            if code__.is_some() {
                                return Err(serde::de::Error::duplicate_field("code"));
                            }
                            code__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Log => {
                            if log__.is_some() {
                                return Err(serde::de::Error::duplicate_field("log"));
                            }
                            log__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Info => {
                            if info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("info"));
                            }
                            info__ = Some(map_.next_value()?);
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
                        GeneratedField::Events => {
                            if events__.is_some() {
                                return Err(serde::de::Error::duplicate_field("events"));
                            }
                            events__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Codespace => {
                            if codespace__.is_some() {
                                return Err(serde::de::Error::duplicate_field("codespace"));
                            }
                            codespace__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(transaction_status::executed::ExecTxResult {
                    code: code__.unwrap_or_default(),
                    data: data__.unwrap_or_default(),
                    log: log__.unwrap_or_default(),
                    info: info__.unwrap_or_default(),
                    gas_wanted: gas_wanted__.unwrap_or_default(),
                    gas_used: gas_used__.unwrap_or_default(),
                    events: events__.unwrap_or_default(),
                    codespace: codespace__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.mempool.v1.TransactionStatus.Executed.ExecTxResult", FIELDS, GeneratedVisitor)
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
