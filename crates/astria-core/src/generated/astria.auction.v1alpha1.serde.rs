impl serde::Serialize for Allocation {
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
        if self.bid.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.auction.v1alpha1.Allocation", len)?;
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
        if let Some(v) = self.bid.as_ref() {
            struct_ser.serialize_field("bid", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Allocation {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "signature",
            "public_key",
            "publicKey",
            "bid",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Signature,
            PublicKey,
            Bid,
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
                            "bid" => Ok(GeneratedField::Bid),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Allocation;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.auction.v1alpha1.Allocation")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Allocation, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut signature__ = None;
                let mut public_key__ = None;
                let mut bid__ = None;
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
                        GeneratedField::Bid => {
                            if bid__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bid"));
                            }
                            bid__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Allocation {
                    signature: signature__.unwrap_or_default(),
                    public_key: public_key__.unwrap_or_default(),
                    bid: bid__,
                })
            }
        }
        deserializer.deserialize_struct("astria.auction.v1alpha1.Allocation", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Bid {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.rollup_parent_block_hash.is_empty() {
            len += 1;
        }
        if !self.sequencer_parent_block_hash.is_empty() {
            len += 1;
        }
        if self.fee != 0 {
            len += 1;
        }
        if !self.transactions.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.auction.v1alpha1.Bid", len)?;
        if !self.rollup_parent_block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("rollupParentBlockHash", pbjson::private::base64::encode(&self.rollup_parent_block_hash).as_str())?;
        }
        if !self.sequencer_parent_block_hash.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("sequencerParentBlockHash", pbjson::private::base64::encode(&self.sequencer_parent_block_hash).as_str())?;
        }
        if self.fee != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("fee", ToString::to_string(&self.fee).as_str())?;
        }
        if !self.transactions.is_empty() {
            struct_ser.serialize_field("transactions", &self.transactions.iter().map(pbjson::private::base64::encode).collect::<Vec<_>>())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Bid {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "rollup_parent_block_hash",
            "rollupParentBlockHash",
            "sequencer_parent_block_hash",
            "sequencerParentBlockHash",
            "fee",
            "transactions",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RollupParentBlockHash,
            SequencerParentBlockHash,
            Fee,
            Transactions,
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
                            "rollupParentBlockHash" | "rollup_parent_block_hash" => Ok(GeneratedField::RollupParentBlockHash),
                            "sequencerParentBlockHash" | "sequencer_parent_block_hash" => Ok(GeneratedField::SequencerParentBlockHash),
                            "fee" => Ok(GeneratedField::Fee),
                            "transactions" => Ok(GeneratedField::Transactions),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Bid;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.auction.v1alpha1.Bid")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Bid, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut rollup_parent_block_hash__ = None;
                let mut sequencer_parent_block_hash__ = None;
                let mut fee__ = None;
                let mut transactions__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RollupParentBlockHash => {
                            if rollup_parent_block_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupParentBlockHash"));
                            }
                            rollup_parent_block_hash__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::SequencerParentBlockHash => {
                            if sequencer_parent_block_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequencerParentBlockHash"));
                            }
                            sequencer_parent_block_hash__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Fee => {
                            if fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("fee"));
                            }
                            fee__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Transactions => {
                            if transactions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("transactions"));
                            }
                            transactions__ = 
                                Some(map_.next_value::<Vec<::pbjson::private::BytesDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                    }
                }
                Ok(Bid {
                    rollup_parent_block_hash: rollup_parent_block_hash__.unwrap_or_default(),
                    sequencer_parent_block_hash: sequencer_parent_block_hash__.unwrap_or_default(),
                    fee: fee__.unwrap_or_default(),
                    transactions: transactions__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.auction.v1alpha1.Bid", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetBidStreamRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("astria.auction.v1alpha1.GetBidStreamRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetBidStreamRequest {
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
            type Value = GetBidStreamRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.auction.v1alpha1.GetBidStreamRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetBidStreamRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetBidStreamRequest {
                })
            }
        }
        deserializer.deserialize_struct("astria.auction.v1alpha1.GetBidStreamRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetBidStreamResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.bid.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.auction.v1alpha1.GetBidStreamResponse", len)?;
        if let Some(v) = self.bid.as_ref() {
            struct_ser.serialize_field("bid", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetBidStreamResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "bid",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Bid,
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
                            "bid" => Ok(GeneratedField::Bid),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetBidStreamResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.auction.v1alpha1.GetBidStreamResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetBidStreamResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut bid__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Bid => {
                            if bid__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bid"));
                            }
                            bid__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GetBidStreamResponse {
                    bid: bid__,
                })
            }
        }
        deserializer.deserialize_struct("astria.auction.v1alpha1.GetBidStreamResponse", FIELDS, GeneratedVisitor)
    }
}
