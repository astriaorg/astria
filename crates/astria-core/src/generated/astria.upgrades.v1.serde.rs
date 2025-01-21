impl serde::Serialize for BaseUpgradeInfo {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.activation_height != 0 {
            len += 1;
        }
        if self.app_version != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.upgrades.v1.BaseUpgradeInfo", len)?;
        if self.activation_height != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("activationHeight", ToString::to_string(&self.activation_height).as_str())?;
        }
        if self.app_version != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("appVersion", ToString::to_string(&self.app_version).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BaseUpgradeInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "activation_height",
            "activationHeight",
            "app_version",
            "appVersion",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ActivationHeight,
            AppVersion,
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
                            "activationHeight" | "activation_height" => Ok(GeneratedField::ActivationHeight),
                            "appVersion" | "app_version" => Ok(GeneratedField::AppVersion),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BaseUpgradeInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.upgrades.v1.BaseUpgradeInfo")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BaseUpgradeInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut activation_height__ = None;
                let mut app_version__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ActivationHeight => {
                            if activation_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("activationHeight"));
                            }
                            activation_height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::AppVersion => {
                            if app_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("appVersion"));
                            }
                            app_version__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(BaseUpgradeInfo {
                    activation_height: activation_height__.unwrap_or_default(),
                    app_version: app_version__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.upgrades.v1.BaseUpgradeInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ChangeInfo {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.activation_height != 0 {
            len += 1;
        }
        if !self.change_name.is_empty() {
            len += 1;
        }
        if self.app_version != 0 {
            len += 1;
        }
        if !self.base64_hash.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.upgrades.v1.ChangeInfo", len)?;
        if self.activation_height != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("activationHeight", ToString::to_string(&self.activation_height).as_str())?;
        }
        if !self.change_name.is_empty() {
            struct_ser.serialize_field("changeName", &self.change_name)?;
        }
        if self.app_version != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("appVersion", ToString::to_string(&self.app_version).as_str())?;
        }
        if !self.base64_hash.is_empty() {
            struct_ser.serialize_field("base64Hash", &self.base64_hash)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ChangeInfo {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "activation_height",
            "activationHeight",
            "change_name",
            "changeName",
            "app_version",
            "appVersion",
            "base64_hash",
            "base64Hash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ActivationHeight,
            ChangeName,
            AppVersion,
            Base64Hash,
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
                            "activationHeight" | "activation_height" => Ok(GeneratedField::ActivationHeight),
                            "changeName" | "change_name" => Ok(GeneratedField::ChangeName),
                            "appVersion" | "app_version" => Ok(GeneratedField::AppVersion),
                            "base64Hash" | "base64_hash" => Ok(GeneratedField::Base64Hash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ChangeInfo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.upgrades.v1.ChangeInfo")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ChangeInfo, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut activation_height__ = None;
                let mut change_name__ = None;
                let mut app_version__ = None;
                let mut base64_hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ActivationHeight => {
                            if activation_height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("activationHeight"));
                            }
                            activation_height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ChangeName => {
                            if change_name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("changeName"));
                            }
                            change_name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::AppVersion => {
                            if app_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("appVersion"));
                            }
                            app_version__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Base64Hash => {
                            if base64_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("base64Hash"));
                            }
                            base64_hash__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(ChangeInfo {
                    activation_height: activation_height__.unwrap_or_default(),
                    change_name: change_name__.unwrap_or_default(),
                    app_version: app_version__.unwrap_or_default(),
                    base64_hash: base64_hash__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.upgrades.v1.ChangeInfo", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Upgrade1 {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.base_info.is_some() {
            len += 1;
        }
        if self.price_feed_change.is_some() {
            len += 1;
        }
        if self.validator_update_action_change.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.upgrades.v1.Upgrade1", len)?;
        if let Some(v) = self.base_info.as_ref() {
            struct_ser.serialize_field("baseInfo", v)?;
        }
        if let Some(v) = self.price_feed_change.as_ref() {
            struct_ser.serialize_field("priceFeedChange", v)?;
        }
        if let Some(v) = self.validator_update_action_change.as_ref() {
            struct_ser.serialize_field("validatorUpdateActionChange", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Upgrade1 {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "base_info",
            "baseInfo",
            "price_feed_change",
            "priceFeedChange",
            "validator_update_action_change",
            "validatorUpdateActionChange",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BaseInfo,
            PriceFeedChange,
            ValidatorUpdateActionChange,
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
                            "baseInfo" | "base_info" => Ok(GeneratedField::BaseInfo),
                            "priceFeedChange" | "price_feed_change" => Ok(GeneratedField::PriceFeedChange),
                            "validatorUpdateActionChange" | "validator_update_action_change" => Ok(GeneratedField::ValidatorUpdateActionChange),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Upgrade1;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.upgrades.v1.Upgrade1")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Upgrade1, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut base_info__ = None;
                let mut price_feed_change__ = None;
                let mut validator_update_action_change__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BaseInfo => {
                            if base_info__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseInfo"));
                            }
                            base_info__ = map_.next_value()?;
                        }
                        GeneratedField::PriceFeedChange => {
                            if price_feed_change__.is_some() {
                                return Err(serde::de::Error::duplicate_field("priceFeedChange"));
                            }
                            price_feed_change__ = map_.next_value()?;
                        }
                        GeneratedField::ValidatorUpdateActionChange => {
                            if validator_update_action_change__.is_some() {
                                return Err(serde::de::Error::duplicate_field("validatorUpdateActionChange"));
                            }
                            validator_update_action_change__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Upgrade1 {
                    base_info: base_info__,
                    price_feed_change: price_feed_change__,
                    validator_update_action_change: validator_update_action_change__,
                })
            }
        }
        deserializer.deserialize_struct("astria.upgrades.v1.Upgrade1", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for upgrade1::PriceFeedChange {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.genesis.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.upgrades.v1.Upgrade1.PriceFeedChange", len)?;
        if let Some(v) = self.genesis.as_ref() {
            struct_ser.serialize_field("genesis", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for upgrade1::PriceFeedChange {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "genesis",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Genesis,
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
                            "genesis" => Ok(GeneratedField::Genesis),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = upgrade1::PriceFeedChange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.upgrades.v1.Upgrade1.PriceFeedChange")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<upgrade1::PriceFeedChange, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut genesis__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Genesis => {
                            if genesis__.is_some() {
                                return Err(serde::de::Error::duplicate_field("genesis"));
                            }
                            genesis__ = map_.next_value()?;
                        }
                    }
                }
                Ok(upgrade1::PriceFeedChange {
                    genesis: genesis__,
                })
            }
        }
        deserializer.deserialize_struct("astria.upgrades.v1.Upgrade1.PriceFeedChange", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for upgrade1::ValidatorUpdateActionChange {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("astria.upgrades.v1.Upgrade1.ValidatorUpdateActionChange", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for upgrade1::ValidatorUpdateActionChange {
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
            type Value = upgrade1::ValidatorUpdateActionChange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.upgrades.v1.Upgrade1.ValidatorUpdateActionChange")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<upgrade1::ValidatorUpdateActionChange, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(upgrade1::ValidatorUpdateActionChange {
                })
            }
        }
        deserializer.deserialize_struct("astria.upgrades.v1.Upgrade1.ValidatorUpdateActionChange", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Upgrades {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.upgrade_1.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.upgrades.v1.Upgrades", len)?;
        if let Some(v) = self.upgrade_1.as_ref() {
            struct_ser.serialize_field("upgrade1", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Upgrades {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "upgrade_1",
            "upgrade1",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Upgrade1,
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
                            "upgrade1" | "upgrade_1" => Ok(GeneratedField::Upgrade1),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Upgrades;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.upgrades.v1.Upgrades")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Upgrades, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut upgrade_1__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Upgrade1 => {
                            if upgrade_1__.is_some() {
                                return Err(serde::de::Error::duplicate_field("upgrade1"));
                            }
                            upgrade_1__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Upgrades {
                    upgrade_1: upgrade_1__,
                })
            }
        }
        deserializer.deserialize_struct("astria.upgrades.v1.Upgrades", FIELDS, GeneratedVisitor)
    }
}
