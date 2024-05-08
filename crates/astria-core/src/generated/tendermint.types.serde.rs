impl serde::Serialize for Blob {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.namespace_id.is_empty() {
            len += 1;
        }
        if !self.data.is_empty() {
            len += 1;
        }
        if self.share_version != 0 {
            len += 1;
        }
        if self.namespace_version != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("tendermint.types.Blob", len)?;
        if !self.namespace_id.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("namespace_id", pbjson::private::base64::encode(&self.namespace_id).as_str())?;
        }
        if !self.data.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("data", pbjson::private::base64::encode(&self.data).as_str())?;
        }
        if self.share_version != 0 {
            struct_ser.serialize_field("share_version", &self.share_version)?;
        }
        if self.namespace_version != 0 {
            struct_ser.serialize_field("namespace_version", &self.namespace_version)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Blob {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "namespace_id",
            "namespaceId",
            "data",
            "share_version",
            "shareVersion",
            "namespace_version",
            "namespaceVersion",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            NamespaceId,
            Data,
            ShareVersion,
            NamespaceVersion,
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
                            "namespaceId" | "namespace_id" => Ok(GeneratedField::NamespaceId),
                            "data" => Ok(GeneratedField::Data),
                            "shareVersion" | "share_version" => Ok(GeneratedField::ShareVersion),
                            "namespaceVersion" | "namespace_version" => Ok(GeneratedField::NamespaceVersion),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Blob;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct tendermint.types.Blob")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Blob, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut namespace_id__ = None;
                let mut data__ = None;
                let mut share_version__ = None;
                let mut namespace_version__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::NamespaceId => {
                            if namespace_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("namespaceId"));
                            }
                            namespace_id__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
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
                        GeneratedField::ShareVersion => {
                            if share_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("shareVersion"));
                            }
                            share_version__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NamespaceVersion => {
                            if namespace_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("namespaceVersion"));
                            }
                            namespace_version__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Blob {
                    namespace_id: namespace_id__.unwrap_or_default(),
                    data: data__.unwrap_or_default(),
                    share_version: share_version__.unwrap_or_default(),
                    namespace_version: namespace_version__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("tendermint.types.Blob", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BlobTx {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.tx.is_empty() {
            len += 1;
        }
        if !self.blobs.is_empty() {
            len += 1;
        }
        if !self.type_id.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("tendermint.types.BlobTx", len)?;
        if !self.tx.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("tx", pbjson::private::base64::encode(&self.tx).as_str())?;
        }
        if !self.blobs.is_empty() {
            struct_ser.serialize_field("blobs", &self.blobs)?;
        }
        if !self.type_id.is_empty() {
            struct_ser.serialize_field("type_id", &self.type_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BlobTx {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "tx",
            "blobs",
            "type_id",
            "typeId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Tx,
            Blobs,
            TypeId,
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
                            "blobs" => Ok(GeneratedField::Blobs),
                            "typeId" | "type_id" => Ok(GeneratedField::TypeId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BlobTx;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct tendermint.types.BlobTx")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BlobTx, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut tx__ = None;
                let mut blobs__ = None;
                let mut type_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Tx => {
                            if tx__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tx"));
                            }
                            tx__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Blobs => {
                            if blobs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blobs"));
                            }
                            blobs__ = Some(map_.next_value()?);
                        }
                        GeneratedField::TypeId => {
                            if type_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("typeId"));
                            }
                            type_id__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(BlobTx {
                    tx: tx__.unwrap_or_default(),
                    blobs: blobs__.unwrap_or_default(),
                    type_id: type_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("tendermint.types.BlobTx", FIELDS, GeneratedVisitor)
    }
}
