impl serde::Serialize for Aspen {
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
        if self.ibc_acknowledgement_failure_change.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.upgrades.v1.Aspen", len)?;
        if let Some(v) = self.base_info.as_ref() {
            struct_ser.serialize_field("baseInfo", v)?;
        }
        if let Some(v) = self.price_feed_change.as_ref() {
            struct_ser.serialize_field("priceFeedChange", v)?;
        }
        if let Some(v) = self.validator_update_action_change.as_ref() {
            struct_ser.serialize_field("validatorUpdateActionChange", v)?;
        }
        if let Some(v) = self.ibc_acknowledgement_failure_change.as_ref() {
            struct_ser.serialize_field("ibcAcknowledgementFailureChange", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Aspen {
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
            "ibc_acknowledgement_failure_change",
            "ibcAcknowledgementFailureChange",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BaseInfo,
            PriceFeedChange,
            ValidatorUpdateActionChange,
            IbcAcknowledgementFailureChange,
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
                            "ibcAcknowledgementFailureChange" | "ibc_acknowledgement_failure_change" => Ok(GeneratedField::IbcAcknowledgementFailureChange),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Aspen;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.upgrades.v1.Aspen")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Aspen, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut base_info__ = None;
                let mut price_feed_change__ = None;
                let mut validator_update_action_change__ = None;
                let mut ibc_acknowledgement_failure_change__ = None;
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
                        GeneratedField::IbcAcknowledgementFailureChange => {
                            if ibc_acknowledgement_failure_change__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ibcAcknowledgementFailureChange"));
                            }
                            ibc_acknowledgement_failure_change__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Aspen {
                    base_info: base_info__,
                    price_feed_change: price_feed_change__,
                    validator_update_action_change: validator_update_action_change__,
                    ibc_acknowledgement_failure_change: ibc_acknowledgement_failure_change__,
                })
            }
        }
        deserializer.deserialize_struct("astria.upgrades.v1.Aspen", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for aspen::IbcAcknowledgementFailureChange {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("astria.upgrades.v1.Aspen.IbcAcknowledgementFailureChange", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for aspen::IbcAcknowledgementFailureChange {
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
            type Value = aspen::IbcAcknowledgementFailureChange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.upgrades.v1.Aspen.IbcAcknowledgementFailureChange")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<aspen::IbcAcknowledgementFailureChange, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(aspen::IbcAcknowledgementFailureChange {
                })
            }
        }
        deserializer.deserialize_struct("astria.upgrades.v1.Aspen.IbcAcknowledgementFailureChange", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for aspen::PriceFeedChange {
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
        let mut struct_ser = serializer.serialize_struct("astria.upgrades.v1.Aspen.PriceFeedChange", len)?;
        if let Some(v) = self.genesis.as_ref() {
            struct_ser.serialize_field("genesis", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for aspen::PriceFeedChange {
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
            type Value = aspen::PriceFeedChange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.upgrades.v1.Aspen.PriceFeedChange")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<aspen::PriceFeedChange, V::Error>
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
                Ok(aspen::PriceFeedChange {
                    genesis: genesis__,
                })
            }
        }
        deserializer.deserialize_struct("astria.upgrades.v1.Aspen.PriceFeedChange", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for aspen::ValidatorUpdateActionChange {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("astria.upgrades.v1.Aspen.ValidatorUpdateActionChange", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for aspen::ValidatorUpdateActionChange {
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
            type Value = aspen::ValidatorUpdateActionChange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.upgrades.v1.Aspen.ValidatorUpdateActionChange")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<aspen::ValidatorUpdateActionChange, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(aspen::ValidatorUpdateActionChange {
                })
            }
        }
        deserializer.deserialize_struct("astria.upgrades.v1.Aspen.ValidatorUpdateActionChange", FIELDS, GeneratedVisitor)
    }
}
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
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("activationHeight", ToString::to_string(&self.activation_height).as_str())?;
        }
        if self.app_version != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
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
impl serde::Serialize for Upgrades {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.aspen.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.upgrades.v1.Upgrades", len)?;
        if let Some(v) = self.aspen.as_ref() {
            struct_ser.serialize_field("aspen", v)?;
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
            "aspen",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Aspen,
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
                            "aspen" => Ok(GeneratedField::Aspen),
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
                let mut aspen__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Aspen => {
                            if aspen__.is_some() {
                                return Err(serde::de::Error::duplicate_field("aspen"));
                            }
                            aspen__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Upgrades {
                    aspen: aspen__,
                })
            }
        }
        deserializer.deserialize_struct("astria.upgrades.v1.Upgrades", FIELDS, GeneratedVisitor)
    }
}
