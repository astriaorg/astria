impl serde::Serialize for CommitmentWithIdentifier {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.commitment.is_empty() {
            len += 1;
        }
        if !self.participant_identifier.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.signer.v1.CommitmentWithIdentifier", len)?;
        if !self.commitment.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("commitment", pbjson::private::base64::encode(&self.commitment).as_str())?;
        }
        if !self.participant_identifier.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("participantIdentifier", pbjson::private::base64::encode(&self.participant_identifier).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CommitmentWithIdentifier {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "commitment",
            "participant_identifier",
            "participantIdentifier",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Commitment,
            ParticipantIdentifier,
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
                            "commitment" => Ok(GeneratedField::Commitment),
                            "participantIdentifier" | "participant_identifier" => Ok(GeneratedField::ParticipantIdentifier),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CommitmentWithIdentifier;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.signer.v1.CommitmentWithIdentifier")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CommitmentWithIdentifier, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut commitment__ = None;
                let mut participant_identifier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Commitment => {
                            if commitment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("commitment"));
                            }
                            commitment__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ParticipantIdentifier => {
                            if participant_identifier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("participantIdentifier"));
                            }
                            participant_identifier__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(CommitmentWithIdentifier {
                    commitment: commitment__.unwrap_or_default(),
                    participant_identifier: participant_identifier__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.signer.v1.CommitmentWithIdentifier", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetVerifyingShareRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("astria.signer.v1.GetVerifyingShareRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetVerifyingShareRequest {
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
            type Value = GetVerifyingShareRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.signer.v1.GetVerifyingShareRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetVerifyingShareRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetVerifyingShareRequest {
                })
            }
        }
        deserializer.deserialize_struct("astria.signer.v1.GetVerifyingShareRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetVerifyingShareResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.verifying_share.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.signer.v1.GetVerifyingShareResponse", len)?;
        if !self.verifying_share.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("verifyingShare", pbjson::private::base64::encode(&self.verifying_share).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetVerifyingShareResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "verifying_share",
            "verifyingShare",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            VerifyingShare,
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
                            "verifyingShare" | "verifying_share" => Ok(GeneratedField::VerifyingShare),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetVerifyingShareResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.signer.v1.GetVerifyingShareResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetVerifyingShareResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut verifying_share__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::VerifyingShare => {
                            if verifying_share__.is_some() {
                                return Err(serde::de::Error::duplicate_field("verifyingShare"));
                            }
                            verifying_share__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(GetVerifyingShareResponse {
                    verifying_share: verifying_share__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.signer.v1.GetVerifyingShareResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Part1Request {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("astria.signer.v1.Part1Request", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Part1Request {
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
            type Value = Part1Request;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.signer.v1.Part1Request")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Part1Request, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(Part1Request {
                })
            }
        }
        deserializer.deserialize_struct("astria.signer.v1.Part1Request", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Part1Response {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.commitment.is_empty() {
            len += 1;
        }
        if self.request_identifier != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.signer.v1.Part1Response", len)?;
        if !self.commitment.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("commitment", pbjson::private::base64::encode(&self.commitment).as_str())?;
        }
        if self.request_identifier != 0 {
            struct_ser.serialize_field("requestIdentifier", &self.request_identifier)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Part1Response {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "commitment",
            "request_identifier",
            "requestIdentifier",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Commitment,
            RequestIdentifier,
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
                            "commitment" => Ok(GeneratedField::Commitment),
                            "requestIdentifier" | "request_identifier" => Ok(GeneratedField::RequestIdentifier),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Part1Response;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.signer.v1.Part1Response")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Part1Response, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut commitment__ = None;
                let mut request_identifier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Commitment => {
                            if commitment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("commitment"));
                            }
                            commitment__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::RequestIdentifier => {
                            if request_identifier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("requestIdentifier"));
                            }
                            request_identifier__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Part1Response {
                    commitment: commitment__.unwrap_or_default(),
                    request_identifier: request_identifier__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.signer.v1.Part1Response", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Part2Request {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.commitments.is_empty() {
            len += 1;
        }
        if self.transaction_body.is_some() {
            len += 1;
        }
        if self.request_identifier != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.signer.v1.Part2Request", len)?;
        if !self.commitments.is_empty() {
            struct_ser.serialize_field("commitments", &self.commitments)?;
        }
        if let Some(v) = self.transaction_body.as_ref() {
            struct_ser.serialize_field("transactionBody", v)?;
        }
        if self.request_identifier != 0 {
            struct_ser.serialize_field("requestIdentifier", &self.request_identifier)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Part2Request {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "commitments",
            "transaction_body",
            "transactionBody",
            "request_identifier",
            "requestIdentifier",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Commitments,
            TransactionBody,
            RequestIdentifier,
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
                            "commitments" => Ok(GeneratedField::Commitments),
                            "transactionBody" | "transaction_body" => Ok(GeneratedField::TransactionBody),
                            "requestIdentifier" | "request_identifier" => Ok(GeneratedField::RequestIdentifier),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Part2Request;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.signer.v1.Part2Request")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Part2Request, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut commitments__ = None;
                let mut transaction_body__ = None;
                let mut request_identifier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Commitments => {
                            if commitments__.is_some() {
                                return Err(serde::de::Error::duplicate_field("commitments"));
                            }
                            commitments__ = Some(map_.next_value()?);
                        }
                        GeneratedField::TransactionBody => {
                            if transaction_body__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactionBody"));
                            }
                            transaction_body__ = map_.next_value()?;
                        }
                        GeneratedField::RequestIdentifier => {
                            if request_identifier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("requestIdentifier"));
                            }
                            request_identifier__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Part2Request {
                    commitments: commitments__.unwrap_or_default(),
                    transaction_body: transaction_body__,
                    request_identifier: request_identifier__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.signer.v1.Part2Request", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Part2Response {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.signature_share.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.signer.v1.Part2Response", len)?;
        if !self.signature_share.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("signatureShare", pbjson::private::base64::encode(&self.signature_share).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Part2Response {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "signature_share",
            "signatureShare",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SignatureShare,
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
                            "signatureShare" | "signature_share" => Ok(GeneratedField::SignatureShare),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Part2Response;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.signer.v1.Part2Response")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Part2Response, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut signature_share__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::SignatureShare => {
                            if signature_share__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signatureShare"));
                            }
                            signature_share__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Part2Response {
                    signature_share: signature_share__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.signer.v1.Part2Response", FIELDS, GeneratedVisitor)
    }
}
