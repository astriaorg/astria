impl serde::Serialize for GetNodeInfoRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("cosmos.base.tendermint.v1beta1.GetNodeInfoRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetNodeInfoRequest {
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
            type Value = GetNodeInfoRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct cosmos.base.tendermint.v1beta1.GetNodeInfoRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetNodeInfoRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetNodeInfoRequest {
                })
            }
        }
        deserializer.deserialize_struct("cosmos.base.tendermint.v1beta1.GetNodeInfoRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetNodeInfoResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.default_node_info.is_some() {
            len += 1;
        }
        if self.application_version.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("cosmos.base.tendermint.v1beta1.GetNodeInfoResponse", len)?;
        if let Some(v) = self.default_node_info.as_ref() {
            struct_ser.serialize_field("default_node_info", v)?;
        }
        if let Some(v) = self.application_version.as_ref() {
            struct_ser.serialize_field("application_version", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetNodeInfoResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "default_node_info",
            "defaultNodeInfo",
            "application_version",
            "applicationVersion",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            DefaultNodeInfo,
            ApplicationVersion,
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
                            "defaultNodeInfo" | "default_node_info" => Ok(GeneratedField::DefaultNodeInfo),
                            "applicationVersion" | "application_version" => Ok(GeneratedField::ApplicationVersion),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetNodeInfoResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct cosmos.base.tendermint.v1beta1.GetNodeInfoResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetNodeInfoResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut default_node_info__ = None;
                let mut application_version__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::DefaultNodeInfo => {
                            if default_node_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("defaultNodeInfo"));
                            }
                            default_node_info__ = map_.next_value()?;
                        }
                        GeneratedField::ApplicationVersion => {
                            if application_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("applicationVersion"));
                            }
                            application_version__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GetNodeInfoResponse {
                    default_node_info: default_node_info__,
                    application_version: application_version__,
                })
            }
        }
        deserializer.deserialize_struct("cosmos.base.tendermint.v1beta1.GetNodeInfoResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Module {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.path.is_empty() {
            len += 1;
        }
        if !self.version.is_empty() {
            len += 1;
        }
        if !self.sum.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("cosmos.base.tendermint.v1beta1.Module", len)?;
        if !self.path.is_empty() {
            struct_ser.serialize_field("path", &self.path)?;
        }
        if !self.version.is_empty() {
            struct_ser.serialize_field("version", &self.version)?;
        }
        if !self.sum.is_empty() {
            struct_ser.serialize_field("sum", &self.sum)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Module {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "path",
            "version",
            "sum",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Path,
            Version,
            Sum,
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
                            "path" => Ok(GeneratedField::Path),
                            "version" => Ok(GeneratedField::Version),
                            "sum" => Ok(GeneratedField::Sum),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Module;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct cosmos.base.tendermint.v1beta1.Module")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Module, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut path__ = None;
                let mut version__ = None;
                let mut sum__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Path => {
                            if path__.is_some() {
                                return Err(serde::de::Error::duplicate_field("path"));
                            }
                            path__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Sum => {
                            if sum__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sum"));
                            }
                            sum__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Module {
                    path: path__.unwrap_or_default(),
                    version: version__.unwrap_or_default(),
                    sum: sum__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("cosmos.base.tendermint.v1beta1.Module", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for VersionInfo {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.name.is_empty() {
            len += 1;
        }
        if !self.app_name.is_empty() {
            len += 1;
        }
        if !self.version.is_empty() {
            len += 1;
        }
        if !self.git_commit.is_empty() {
            len += 1;
        }
        if !self.build_tags.is_empty() {
            len += 1;
        }
        if !self.go_version.is_empty() {
            len += 1;
        }
        if !self.build_deps.is_empty() {
            len += 1;
        }
        if !self.cosmos_sdk_version.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("cosmos.base.tendermint.v1beta1.VersionInfo", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.app_name.is_empty() {
            struct_ser.serialize_field("app_name", &self.app_name)?;
        }
        if !self.version.is_empty() {
            struct_ser.serialize_field("version", &self.version)?;
        }
        if !self.git_commit.is_empty() {
            struct_ser.serialize_field("git_commit", &self.git_commit)?;
        }
        if !self.build_tags.is_empty() {
            struct_ser.serialize_field("build_tags", &self.build_tags)?;
        }
        if !self.go_version.is_empty() {
            struct_ser.serialize_field("go_version", &self.go_version)?;
        }
        if !self.build_deps.is_empty() {
            struct_ser.serialize_field("build_deps", &self.build_deps)?;
        }
        if !self.cosmos_sdk_version.is_empty() {
            struct_ser.serialize_field("cosmos_sdk_version", &self.cosmos_sdk_version)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for VersionInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "app_name",
            "appName",
            "version",
            "git_commit",
            "gitCommit",
            "build_tags",
            "buildTags",
            "go_version",
            "goVersion",
            "build_deps",
            "buildDeps",
            "cosmos_sdk_version",
            "cosmosSdkVersion",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            AppName,
            Version,
            GitCommit,
            BuildTags,
            GoVersion,
            BuildDeps,
            CosmosSdkVersion,
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
                            "name" => Ok(GeneratedField::Name),
                            "appName" | "app_name" => Ok(GeneratedField::AppName),
                            "version" => Ok(GeneratedField::Version),
                            "gitCommit" | "git_commit" => Ok(GeneratedField::GitCommit),
                            "buildTags" | "build_tags" => Ok(GeneratedField::BuildTags),
                            "goVersion" | "go_version" => Ok(GeneratedField::GoVersion),
                            "buildDeps" | "build_deps" => Ok(GeneratedField::BuildDeps),
                            "cosmosSdkVersion" | "cosmos_sdk_version" => Ok(GeneratedField::CosmosSdkVersion),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = VersionInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct cosmos.base.tendermint.v1beta1.VersionInfo")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<VersionInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut app_name__ = None;
                let mut version__ = None;
                let mut git_commit__ = None;
                let mut build_tags__ = None;
                let mut go_version__ = None;
                let mut build_deps__ = None;
                let mut cosmos_sdk_version__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::AppName => {
                            if app_name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("appName"));
                            }
                            app_name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = Some(map_.next_value()?);
                        }
                        GeneratedField::GitCommit => {
                            if git_commit__.is_some() {
                                return Err(serde::de::Error::duplicate_field("gitCommit"));
                            }
                            git_commit__ = Some(map_.next_value()?);
                        }
                        GeneratedField::BuildTags => {
                            if build_tags__.is_some() {
                                return Err(serde::de::Error::duplicate_field("buildTags"));
                            }
                            build_tags__ = Some(map_.next_value()?);
                        }
                        GeneratedField::GoVersion => {
                            if go_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("goVersion"));
                            }
                            go_version__ = Some(map_.next_value()?);
                        }
                        GeneratedField::BuildDeps => {
                            if build_deps__.is_some() {
                                return Err(serde::de::Error::duplicate_field("buildDeps"));
                            }
                            build_deps__ = Some(map_.next_value()?);
                        }
                        GeneratedField::CosmosSdkVersion => {
                            if cosmos_sdk_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("cosmosSdkVersion"));
                            }
                            cosmos_sdk_version__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(VersionInfo {
                    name: name__.unwrap_or_default(),
                    app_name: app_name__.unwrap_or_default(),
                    version: version__.unwrap_or_default(),
                    git_commit: git_commit__.unwrap_or_default(),
                    build_tags: build_tags__.unwrap_or_default(),
                    go_version: go_version__.unwrap_or_default(),
                    build_deps: build_deps__.unwrap_or_default(),
                    cosmos_sdk_version: cosmos_sdk_version__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("cosmos.base.tendermint.v1beta1.VersionInfo", FIELDS, GeneratedVisitor)
    }
}
