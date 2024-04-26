impl serde::Serialize for DefaultNodeInfo {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.protocol_version.is_some() {
            len += 1;
        }
        if !self.default_node_id.is_empty() {
            len += 1;
        }
        if !self.listen_addr.is_empty() {
            len += 1;
        }
        if !self.network.is_empty() {
            len += 1;
        }
        if !self.version.is_empty() {
            len += 1;
        }
        if !self.channels.is_empty() {
            len += 1;
        }
        if !self.moniker.is_empty() {
            len += 1;
        }
        if self.other.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("tendermint.p2p.DefaultNodeInfo", len)?;
        if let Some(v) = self.protocol_version.as_ref() {
            struct_ser.serialize_field("protocol_version", v)?;
        }
        if !self.default_node_id.is_empty() {
            struct_ser.serialize_field("default_node_id", &self.default_node_id)?;
        }
        if !self.listen_addr.is_empty() {
            struct_ser.serialize_field("listen_addr", &self.listen_addr)?;
        }
        if !self.network.is_empty() {
            struct_ser.serialize_field("network", &self.network)?;
        }
        if !self.version.is_empty() {
            struct_ser.serialize_field("version", &self.version)?;
        }
        if !self.channels.is_empty() {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("channels", pbjson::private::base64::encode(&self.channels).as_str())?;
        }
        if !self.moniker.is_empty() {
            struct_ser.serialize_field("moniker", &self.moniker)?;
        }
        if let Some(v) = self.other.as_ref() {
            struct_ser.serialize_field("other", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DefaultNodeInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "protocol_version",
            "protocolVersion",
            "default_node_id",
            "defaultNodeId",
            "listen_addr",
            "listenAddr",
            "network",
            "version",
            "channels",
            "moniker",
            "other",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ProtocolVersion,
            DefaultNodeId,
            ListenAddr,
            Network,
            Version,
            Channels,
            Moniker,
            Other,
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
                            "protocolVersion" | "protocol_version" => Ok(GeneratedField::ProtocolVersion),
                            "defaultNodeId" | "default_node_id" => Ok(GeneratedField::DefaultNodeId),
                            "listenAddr" | "listen_addr" => Ok(GeneratedField::ListenAddr),
                            "network" => Ok(GeneratedField::Network),
                            "version" => Ok(GeneratedField::Version),
                            "channels" => Ok(GeneratedField::Channels),
                            "moniker" => Ok(GeneratedField::Moniker),
                            "other" => Ok(GeneratedField::Other),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DefaultNodeInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct tendermint.p2p.DefaultNodeInfo")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<DefaultNodeInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut protocol_version__ = None;
                let mut default_node_id__ = None;
                let mut listen_addr__ = None;
                let mut network__ = None;
                let mut version__ = None;
                let mut channels__ = None;
                let mut moniker__ = None;
                let mut other__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ProtocolVersion => {
                            if protocol_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("protocolVersion"));
                            }
                            protocol_version__ = map_.next_value()?;
                        }
                        GeneratedField::DefaultNodeId => {
                            if default_node_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("defaultNodeId"));
                            }
                            default_node_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ListenAddr => {
                            if listen_addr__.is_some() {
                                return Err(serde::de::Error::duplicate_field("listenAddr"));
                            }
                            listen_addr__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Network => {
                            if network__.is_some() {
                                return Err(serde::de::Error::duplicate_field("network"));
                            }
                            network__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Channels => {
                            if channels__.is_some() {
                                return Err(serde::de::Error::duplicate_field("channels"));
                            }
                            channels__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Moniker => {
                            if moniker__.is_some() {
                                return Err(serde::de::Error::duplicate_field("moniker"));
                            }
                            moniker__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Other => {
                            if other__.is_some() {
                                return Err(serde::de::Error::duplicate_field("other"));
                            }
                            other__ = map_.next_value()?;
                        }
                    }
                }
                Ok(DefaultNodeInfo {
                    protocol_version: protocol_version__,
                    default_node_id: default_node_id__.unwrap_or_default(),
                    listen_addr: listen_addr__.unwrap_or_default(),
                    network: network__.unwrap_or_default(),
                    version: version__.unwrap_or_default(),
                    channels: channels__.unwrap_or_default(),
                    moniker: moniker__.unwrap_or_default(),
                    other: other__,
                })
            }
        }
        deserializer.deserialize_struct("tendermint.p2p.DefaultNodeInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DefaultNodeInfoOther {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.tx_index.is_empty() {
            len += 1;
        }
        if !self.rpc_address.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("tendermint.p2p.DefaultNodeInfoOther", len)?;
        if !self.tx_index.is_empty() {
            struct_ser.serialize_field("tx_index", &self.tx_index)?;
        }
        if !self.rpc_address.is_empty() {
            struct_ser.serialize_field("rpc_address", &self.rpc_address)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DefaultNodeInfoOther {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "tx_index",
            "txIndex",
            "rpc_address",
            "rpcAddress",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TxIndex,
            RpcAddress,
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
                            "txIndex" | "tx_index" => Ok(GeneratedField::TxIndex),
                            "rpcAddress" | "rpc_address" => Ok(GeneratedField::RpcAddress),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DefaultNodeInfoOther;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct tendermint.p2p.DefaultNodeInfoOther")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<DefaultNodeInfoOther, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut tx_index__ = None;
                let mut rpc_address__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TxIndex => {
                            if tx_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("txIndex"));
                            }
                            tx_index__ = Some(map_.next_value()?);
                        }
                        GeneratedField::RpcAddress => {
                            if rpc_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rpcAddress"));
                            }
                            rpc_address__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(DefaultNodeInfoOther {
                    tx_index: tx_index__.unwrap_or_default(),
                    rpc_address: rpc_address__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("tendermint.p2p.DefaultNodeInfoOther", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ProtocolVersion {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.p2p != 0 {
            len += 1;
        }
        if self.block != 0 {
            len += 1;
        }
        if self.app != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("tendermint.p2p.ProtocolVersion", len)?;
        if self.p2p != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("p2p", ToString::to_string(&self.p2p).as_str())?;
        }
        if self.block != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("block", ToString::to_string(&self.block).as_str())?;
        }
        if self.app != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("app", ToString::to_string(&self.app).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ProtocolVersion {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "p2p",
            "block",
            "app",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            P2p,
            Block,
            App,
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
                            "p2p" => Ok(GeneratedField::P2p),
                            "block" => Ok(GeneratedField::Block),
                            "app" => Ok(GeneratedField::App),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ProtocolVersion;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct tendermint.p2p.ProtocolVersion")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ProtocolVersion, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut p2p__ = None;
                let mut block__ = None;
                let mut app__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::P2p => {
                            if p2p__.is_some() {
                                return Err(serde::de::Error::duplicate_field("p2p"));
                            }
                            p2p__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Block => {
                            if block__.is_some() {
                                return Err(serde::de::Error::duplicate_field("block"));
                            }
                            block__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::App => {
                            if app__.is_some() {
                                return Err(serde::de::Error::duplicate_field("app"));
                            }
                            app__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(ProtocolVersion {
                    p2p: p2p__.unwrap_or_default(),
                    block: block__.unwrap_or_default(),
                    app: app__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("tendermint.p2p.ProtocolVersion", FIELDS, GeneratedVisitor)
    }
}
