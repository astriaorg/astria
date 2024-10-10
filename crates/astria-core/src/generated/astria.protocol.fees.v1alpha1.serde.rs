impl serde::Serialize for BridgeLockFeeComponents {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.base_fee.is_some() {
            len += 1;
        }
        if self.computed_cost_multiplier.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.fees.v1alpha1.BridgeLockFeeComponents", len)?;
        if let Some(v) = self.base_fee.as_ref() {
            struct_ser.serialize_field("baseFee", v)?;
        }
        if let Some(v) = self.computed_cost_multiplier.as_ref() {
            struct_ser.serialize_field("computedCostMultiplier", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BridgeLockFeeComponents {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "base_fee",
            "baseFee",
            "computed_cost_multiplier",
            "computedCostMultiplier",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BaseFee,
            ComputedCostMultiplier,
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
                            "baseFee" | "base_fee" => Ok(GeneratedField::BaseFee),
                            "computedCostMultiplier" | "computed_cost_multiplier" => Ok(GeneratedField::ComputedCostMultiplier),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BridgeLockFeeComponents;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.fees.v1alpha1.BridgeLockFeeComponents")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BridgeLockFeeComponents, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut base_fee__ = None;
                let mut computed_cost_multiplier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BaseFee => {
                            if base_fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseFee"));
                            }
                            base_fee__ = map_.next_value()?;
                        }
                        GeneratedField::ComputedCostMultiplier => {
                            if computed_cost_multiplier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("computedCostMultiplier"));
                            }
                            computed_cost_multiplier__ = map_.next_value()?;
                        }
                    }
                }
                Ok(BridgeLockFeeComponents {
                    base_fee: base_fee__,
                    computed_cost_multiplier: computed_cost_multiplier__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.fees.v1alpha1.BridgeLockFeeComponents", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BridgeSudoChangeFeeComponents {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.base_fee.is_some() {
            len += 1;
        }
        if self.computed_cost_multiplier.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.fees.v1alpha1.BridgeSudoChangeFeeComponents", len)?;
        if let Some(v) = self.base_fee.as_ref() {
            struct_ser.serialize_field("baseFee", v)?;
        }
        if let Some(v) = self.computed_cost_multiplier.as_ref() {
            struct_ser.serialize_field("computedCostMultiplier", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BridgeSudoChangeFeeComponents {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "base_fee",
            "baseFee",
            "computed_cost_multiplier",
            "computedCostMultiplier",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BaseFee,
            ComputedCostMultiplier,
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
                            "baseFee" | "base_fee" => Ok(GeneratedField::BaseFee),
                            "computedCostMultiplier" | "computed_cost_multiplier" => Ok(GeneratedField::ComputedCostMultiplier),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BridgeSudoChangeFeeComponents;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.fees.v1alpha1.BridgeSudoChangeFeeComponents")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BridgeSudoChangeFeeComponents, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut base_fee__ = None;
                let mut computed_cost_multiplier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BaseFee => {
                            if base_fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseFee"));
                            }
                            base_fee__ = map_.next_value()?;
                        }
                        GeneratedField::ComputedCostMultiplier => {
                            if computed_cost_multiplier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("computedCostMultiplier"));
                            }
                            computed_cost_multiplier__ = map_.next_value()?;
                        }
                    }
                }
                Ok(BridgeSudoChangeFeeComponents {
                    base_fee: base_fee__,
                    computed_cost_multiplier: computed_cost_multiplier__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.fees.v1alpha1.BridgeSudoChangeFeeComponents", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BridgeUnlockFeeComponents {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.base_fee.is_some() {
            len += 1;
        }
        if self.computed_cost_multiplier.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.fees.v1alpha1.BridgeUnlockFeeComponents", len)?;
        if let Some(v) = self.base_fee.as_ref() {
            struct_ser.serialize_field("baseFee", v)?;
        }
        if let Some(v) = self.computed_cost_multiplier.as_ref() {
            struct_ser.serialize_field("computedCostMultiplier", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BridgeUnlockFeeComponents {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "base_fee",
            "baseFee",
            "computed_cost_multiplier",
            "computedCostMultiplier",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BaseFee,
            ComputedCostMultiplier,
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
                            "baseFee" | "base_fee" => Ok(GeneratedField::BaseFee),
                            "computedCostMultiplier" | "computed_cost_multiplier" => Ok(GeneratedField::ComputedCostMultiplier),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BridgeUnlockFeeComponents;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.fees.v1alpha1.BridgeUnlockFeeComponents")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BridgeUnlockFeeComponents, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut base_fee__ = None;
                let mut computed_cost_multiplier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BaseFee => {
                            if base_fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseFee"));
                            }
                            base_fee__ = map_.next_value()?;
                        }
                        GeneratedField::ComputedCostMultiplier => {
                            if computed_cost_multiplier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("computedCostMultiplier"));
                            }
                            computed_cost_multiplier__ = map_.next_value()?;
                        }
                    }
                }
                Ok(BridgeUnlockFeeComponents {
                    base_fee: base_fee__,
                    computed_cost_multiplier: computed_cost_multiplier__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.fees.v1alpha1.BridgeUnlockFeeComponents", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for FeeAssetChangeFeeComponents {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.base_fee.is_some() {
            len += 1;
        }
        if self.computed_cost_multiplier.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.fees.v1alpha1.FeeAssetChangeFeeComponents", len)?;
        if let Some(v) = self.base_fee.as_ref() {
            struct_ser.serialize_field("baseFee", v)?;
        }
        if let Some(v) = self.computed_cost_multiplier.as_ref() {
            struct_ser.serialize_field("computedCostMultiplier", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FeeAssetChangeFeeComponents {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "base_fee",
            "baseFee",
            "computed_cost_multiplier",
            "computedCostMultiplier",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BaseFee,
            ComputedCostMultiplier,
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
                            "baseFee" | "base_fee" => Ok(GeneratedField::BaseFee),
                            "computedCostMultiplier" | "computed_cost_multiplier" => Ok(GeneratedField::ComputedCostMultiplier),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FeeAssetChangeFeeComponents;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.fees.v1alpha1.FeeAssetChangeFeeComponents")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<FeeAssetChangeFeeComponents, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut base_fee__ = None;
                let mut computed_cost_multiplier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BaseFee => {
                            if base_fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseFee"));
                            }
                            base_fee__ = map_.next_value()?;
                        }
                        GeneratedField::ComputedCostMultiplier => {
                            if computed_cost_multiplier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("computedCostMultiplier"));
                            }
                            computed_cost_multiplier__ = map_.next_value()?;
                        }
                    }
                }
                Ok(FeeAssetChangeFeeComponents {
                    base_fee: base_fee__,
                    computed_cost_multiplier: computed_cost_multiplier__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.fees.v1alpha1.FeeAssetChangeFeeComponents", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for FeeChangeFeeComponents {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.base_fee.is_some() {
            len += 1;
        }
        if self.computed_cost_multiplier.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.fees.v1alpha1.FeeChangeFeeComponents", len)?;
        if let Some(v) = self.base_fee.as_ref() {
            struct_ser.serialize_field("baseFee", v)?;
        }
        if let Some(v) = self.computed_cost_multiplier.as_ref() {
            struct_ser.serialize_field("computedCostMultiplier", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FeeChangeFeeComponents {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "base_fee",
            "baseFee",
            "computed_cost_multiplier",
            "computedCostMultiplier",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BaseFee,
            ComputedCostMultiplier,
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
                            "baseFee" | "base_fee" => Ok(GeneratedField::BaseFee),
                            "computedCostMultiplier" | "computed_cost_multiplier" => Ok(GeneratedField::ComputedCostMultiplier),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FeeChangeFeeComponents;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.fees.v1alpha1.FeeChangeFeeComponents")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<FeeChangeFeeComponents, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut base_fee__ = None;
                let mut computed_cost_multiplier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BaseFee => {
                            if base_fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseFee"));
                            }
                            base_fee__ = map_.next_value()?;
                        }
                        GeneratedField::ComputedCostMultiplier => {
                            if computed_cost_multiplier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("computedCostMultiplier"));
                            }
                            computed_cost_multiplier__ = map_.next_value()?;
                        }
                    }
                }
                Ok(FeeChangeFeeComponents {
                    base_fee: base_fee__,
                    computed_cost_multiplier: computed_cost_multiplier__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.fees.v1alpha1.FeeChangeFeeComponents", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for IbcRelayFeeComponents {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.base_fee.is_some() {
            len += 1;
        }
        if self.computed_cost_multiplier.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.fees.v1alpha1.IbcRelayFeeComponents", len)?;
        if let Some(v) = self.base_fee.as_ref() {
            struct_ser.serialize_field("baseFee", v)?;
        }
        if let Some(v) = self.computed_cost_multiplier.as_ref() {
            struct_ser.serialize_field("computedCostMultiplier", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for IbcRelayFeeComponents {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "base_fee",
            "baseFee",
            "computed_cost_multiplier",
            "computedCostMultiplier",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BaseFee,
            ComputedCostMultiplier,
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
                            "baseFee" | "base_fee" => Ok(GeneratedField::BaseFee),
                            "computedCostMultiplier" | "computed_cost_multiplier" => Ok(GeneratedField::ComputedCostMultiplier),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = IbcRelayFeeComponents;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.fees.v1alpha1.IbcRelayFeeComponents")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<IbcRelayFeeComponents, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut base_fee__ = None;
                let mut computed_cost_multiplier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BaseFee => {
                            if base_fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseFee"));
                            }
                            base_fee__ = map_.next_value()?;
                        }
                        GeneratedField::ComputedCostMultiplier => {
                            if computed_cost_multiplier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("computedCostMultiplier"));
                            }
                            computed_cost_multiplier__ = map_.next_value()?;
                        }
                    }
                }
                Ok(IbcRelayFeeComponents {
                    base_fee: base_fee__,
                    computed_cost_multiplier: computed_cost_multiplier__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.fees.v1alpha1.IbcRelayFeeComponents", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for IbcRelayerChangeFeeComponents {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.base_fee.is_some() {
            len += 1;
        }
        if self.computed_cost_multiplier.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.fees.v1alpha1.IbcRelayerChangeFeeComponents", len)?;
        if let Some(v) = self.base_fee.as_ref() {
            struct_ser.serialize_field("baseFee", v)?;
        }
        if let Some(v) = self.computed_cost_multiplier.as_ref() {
            struct_ser.serialize_field("computedCostMultiplier", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for IbcRelayerChangeFeeComponents {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "base_fee",
            "baseFee",
            "computed_cost_multiplier",
            "computedCostMultiplier",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BaseFee,
            ComputedCostMultiplier,
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
                            "baseFee" | "base_fee" => Ok(GeneratedField::BaseFee),
                            "computedCostMultiplier" | "computed_cost_multiplier" => Ok(GeneratedField::ComputedCostMultiplier),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = IbcRelayerChangeFeeComponents;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.fees.v1alpha1.IbcRelayerChangeFeeComponents")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<IbcRelayerChangeFeeComponents, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut base_fee__ = None;
                let mut computed_cost_multiplier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BaseFee => {
                            if base_fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseFee"));
                            }
                            base_fee__ = map_.next_value()?;
                        }
                        GeneratedField::ComputedCostMultiplier => {
                            if computed_cost_multiplier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("computedCostMultiplier"));
                            }
                            computed_cost_multiplier__ = map_.next_value()?;
                        }
                    }
                }
                Ok(IbcRelayerChangeFeeComponents {
                    base_fee: base_fee__,
                    computed_cost_multiplier: computed_cost_multiplier__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.fees.v1alpha1.IbcRelayerChangeFeeComponents", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for IbcSudoChangeFeeComponents {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.base_fee.is_some() {
            len += 1;
        }
        if self.computed_cost_multiplier.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.fees.v1alpha1.IbcSudoChangeFeeComponents", len)?;
        if let Some(v) = self.base_fee.as_ref() {
            struct_ser.serialize_field("baseFee", v)?;
        }
        if let Some(v) = self.computed_cost_multiplier.as_ref() {
            struct_ser.serialize_field("computedCostMultiplier", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for IbcSudoChangeFeeComponents {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "base_fee",
            "baseFee",
            "computed_cost_multiplier",
            "computedCostMultiplier",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BaseFee,
            ComputedCostMultiplier,
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
                            "baseFee" | "base_fee" => Ok(GeneratedField::BaseFee),
                            "computedCostMultiplier" | "computed_cost_multiplier" => Ok(GeneratedField::ComputedCostMultiplier),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = IbcSudoChangeFeeComponents;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.fees.v1alpha1.IbcSudoChangeFeeComponents")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<IbcSudoChangeFeeComponents, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut base_fee__ = None;
                let mut computed_cost_multiplier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BaseFee => {
                            if base_fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseFee"));
                            }
                            base_fee__ = map_.next_value()?;
                        }
                        GeneratedField::ComputedCostMultiplier => {
                            if computed_cost_multiplier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("computedCostMultiplier"));
                            }
                            computed_cost_multiplier__ = map_.next_value()?;
                        }
                    }
                }
                Ok(IbcSudoChangeFeeComponents {
                    base_fee: base_fee__,
                    computed_cost_multiplier: computed_cost_multiplier__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.fees.v1alpha1.IbcSudoChangeFeeComponents", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Ics20WithdrawalFeeComponents {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.base_fee.is_some() {
            len += 1;
        }
        if self.computed_cost_multiplier.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.fees.v1alpha1.Ics20WithdrawalFeeComponents", len)?;
        if let Some(v) = self.base_fee.as_ref() {
            struct_ser.serialize_field("baseFee", v)?;
        }
        if let Some(v) = self.computed_cost_multiplier.as_ref() {
            struct_ser.serialize_field("computedCostMultiplier", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Ics20WithdrawalFeeComponents {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "base_fee",
            "baseFee",
            "computed_cost_multiplier",
            "computedCostMultiplier",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BaseFee,
            ComputedCostMultiplier,
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
                            "baseFee" | "base_fee" => Ok(GeneratedField::BaseFee),
                            "computedCostMultiplier" | "computed_cost_multiplier" => Ok(GeneratedField::ComputedCostMultiplier),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Ics20WithdrawalFeeComponents;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.fees.v1alpha1.Ics20WithdrawalFeeComponents")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Ics20WithdrawalFeeComponents, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut base_fee__ = None;
                let mut computed_cost_multiplier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BaseFee => {
                            if base_fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseFee"));
                            }
                            base_fee__ = map_.next_value()?;
                        }
                        GeneratedField::ComputedCostMultiplier => {
                            if computed_cost_multiplier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("computedCostMultiplier"));
                            }
                            computed_cost_multiplier__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Ics20WithdrawalFeeComponents {
                    base_fee: base_fee__,
                    computed_cost_multiplier: computed_cost_multiplier__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.fees.v1alpha1.Ics20WithdrawalFeeComponents", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for InitBridgeAccountFeeComponents {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.base_fee.is_some() {
            len += 1;
        }
        if self.computed_cost_multiplier.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.fees.v1alpha1.InitBridgeAccountFeeComponents", len)?;
        if let Some(v) = self.base_fee.as_ref() {
            struct_ser.serialize_field("baseFee", v)?;
        }
        if let Some(v) = self.computed_cost_multiplier.as_ref() {
            struct_ser.serialize_field("computedCostMultiplier", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for InitBridgeAccountFeeComponents {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "base_fee",
            "baseFee",
            "computed_cost_multiplier",
            "computedCostMultiplier",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BaseFee,
            ComputedCostMultiplier,
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
                            "baseFee" | "base_fee" => Ok(GeneratedField::BaseFee),
                            "computedCostMultiplier" | "computed_cost_multiplier" => Ok(GeneratedField::ComputedCostMultiplier),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = InitBridgeAccountFeeComponents;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.fees.v1alpha1.InitBridgeAccountFeeComponents")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<InitBridgeAccountFeeComponents, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut base_fee__ = None;
                let mut computed_cost_multiplier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BaseFee => {
                            if base_fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseFee"));
                            }
                            base_fee__ = map_.next_value()?;
                        }
                        GeneratedField::ComputedCostMultiplier => {
                            if computed_cost_multiplier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("computedCostMultiplier"));
                            }
                            computed_cost_multiplier__ = map_.next_value()?;
                        }
                    }
                }
                Ok(InitBridgeAccountFeeComponents {
                    base_fee: base_fee__,
                    computed_cost_multiplier: computed_cost_multiplier__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.fees.v1alpha1.InitBridgeAccountFeeComponents", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SequenceFeeComponents {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.base_fee.is_some() {
            len += 1;
        }
        if self.computed_cost_multiplier.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.fees.v1alpha1.SequenceFeeComponents", len)?;
        if let Some(v) = self.base_fee.as_ref() {
            struct_ser.serialize_field("baseFee", v)?;
        }
        if let Some(v) = self.computed_cost_multiplier.as_ref() {
            struct_ser.serialize_field("computedCostMultiplier", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SequenceFeeComponents {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "base_fee",
            "baseFee",
            "computed_cost_multiplier",
            "computedCostMultiplier",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BaseFee,
            ComputedCostMultiplier,
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
                            "baseFee" | "base_fee" => Ok(GeneratedField::BaseFee),
                            "computedCostMultiplier" | "computed_cost_multiplier" => Ok(GeneratedField::ComputedCostMultiplier),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SequenceFeeComponents;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.fees.v1alpha1.SequenceFeeComponents")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SequenceFeeComponents, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut base_fee__ = None;
                let mut computed_cost_multiplier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BaseFee => {
                            if base_fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseFee"));
                            }
                            base_fee__ = map_.next_value()?;
                        }
                        GeneratedField::ComputedCostMultiplier => {
                            if computed_cost_multiplier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("computedCostMultiplier"));
                            }
                            computed_cost_multiplier__ = map_.next_value()?;
                        }
                    }
                }
                Ok(SequenceFeeComponents {
                    base_fee: base_fee__,
                    computed_cost_multiplier: computed_cost_multiplier__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.fees.v1alpha1.SequenceFeeComponents", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SudoAddressChangeFeeComponents {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.base_fee.is_some() {
            len += 1;
        }
        if self.computed_cost_multiplier.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.fees.v1alpha1.SudoAddressChangeFeeComponents", len)?;
        if let Some(v) = self.base_fee.as_ref() {
            struct_ser.serialize_field("baseFee", v)?;
        }
        if let Some(v) = self.computed_cost_multiplier.as_ref() {
            struct_ser.serialize_field("computedCostMultiplier", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SudoAddressChangeFeeComponents {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "base_fee",
            "baseFee",
            "computed_cost_multiplier",
            "computedCostMultiplier",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BaseFee,
            ComputedCostMultiplier,
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
                            "baseFee" | "base_fee" => Ok(GeneratedField::BaseFee),
                            "computedCostMultiplier" | "computed_cost_multiplier" => Ok(GeneratedField::ComputedCostMultiplier),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SudoAddressChangeFeeComponents;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.fees.v1alpha1.SudoAddressChangeFeeComponents")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SudoAddressChangeFeeComponents, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut base_fee__ = None;
                let mut computed_cost_multiplier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BaseFee => {
                            if base_fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseFee"));
                            }
                            base_fee__ = map_.next_value()?;
                        }
                        GeneratedField::ComputedCostMultiplier => {
                            if computed_cost_multiplier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("computedCostMultiplier"));
                            }
                            computed_cost_multiplier__ = map_.next_value()?;
                        }
                    }
                }
                Ok(SudoAddressChangeFeeComponents {
                    base_fee: base_fee__,
                    computed_cost_multiplier: computed_cost_multiplier__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.fees.v1alpha1.SudoAddressChangeFeeComponents", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TransactionFee {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.asset.is_empty() {
            len += 1;
        }
        if self.fee.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.fees.v1alpha1.TransactionFee", len)?;
        if !self.asset.is_empty() {
            struct_ser.serialize_field("asset", &self.asset)?;
        }
        if let Some(v) = self.fee.as_ref() {
            struct_ser.serialize_field("fee", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransactionFee {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "asset",
            "fee",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Asset,
            Fee,
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
                            "asset" => Ok(GeneratedField::Asset),
                            "fee" => Ok(GeneratedField::Fee),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransactionFee;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.fees.v1alpha1.TransactionFee")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TransactionFee, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut asset__ = None;
                let mut fee__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Asset => {
                            if asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("asset"));
                            }
                            asset__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Fee => {
                            if fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("fee"));
                            }
                            fee__ = map_.next_value()?;
                        }
                    }
                }
                Ok(TransactionFee {
                    asset: asset__.unwrap_or_default(),
                    fee: fee__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.fees.v1alpha1.TransactionFee", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TransferFeeComponents {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.base_fee.is_some() {
            len += 1;
        }
        if self.computed_cost_multiplier.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.fees.v1alpha1.TransferFeeComponents", len)?;
        if let Some(v) = self.base_fee.as_ref() {
            struct_ser.serialize_field("baseFee", v)?;
        }
        if let Some(v) = self.computed_cost_multiplier.as_ref() {
            struct_ser.serialize_field("computedCostMultiplier", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransferFeeComponents {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "base_fee",
            "baseFee",
            "computed_cost_multiplier",
            "computedCostMultiplier",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BaseFee,
            ComputedCostMultiplier,
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
                            "baseFee" | "base_fee" => Ok(GeneratedField::BaseFee),
                            "computedCostMultiplier" | "computed_cost_multiplier" => Ok(GeneratedField::ComputedCostMultiplier),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransferFeeComponents;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.fees.v1alpha1.TransferFeeComponents")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TransferFeeComponents, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut base_fee__ = None;
                let mut computed_cost_multiplier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BaseFee => {
                            if base_fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseFee"));
                            }
                            base_fee__ = map_.next_value()?;
                        }
                        GeneratedField::ComputedCostMultiplier => {
                            if computed_cost_multiplier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("computedCostMultiplier"));
                            }
                            computed_cost_multiplier__ = map_.next_value()?;
                        }
                    }
                }
                Ok(TransferFeeComponents {
                    base_fee: base_fee__,
                    computed_cost_multiplier: computed_cost_multiplier__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.fees.v1alpha1.TransferFeeComponents", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ValidatorUpdateFeeComponents {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.base_fee.is_some() {
            len += 1;
        }
        if self.computed_cost_multiplier.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.fees.v1alpha1.ValidatorUpdateFeeComponents", len)?;
        if let Some(v) = self.base_fee.as_ref() {
            struct_ser.serialize_field("baseFee", v)?;
        }
        if let Some(v) = self.computed_cost_multiplier.as_ref() {
            struct_ser.serialize_field("computedCostMultiplier", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ValidatorUpdateFeeComponents {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "base_fee",
            "baseFee",
            "computed_cost_multiplier",
            "computedCostMultiplier",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BaseFee,
            ComputedCostMultiplier,
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
                            "baseFee" | "base_fee" => Ok(GeneratedField::BaseFee),
                            "computedCostMultiplier" | "computed_cost_multiplier" => Ok(GeneratedField::ComputedCostMultiplier),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ValidatorUpdateFeeComponents;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.fees.v1alpha1.ValidatorUpdateFeeComponents")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ValidatorUpdateFeeComponents, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut base_fee__ = None;
                let mut computed_cost_multiplier__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BaseFee => {
                            if base_fee__.is_some() {
                                return Err(serde::de::Error::duplicate_field("baseFee"));
                            }
                            base_fee__ = map_.next_value()?;
                        }
                        GeneratedField::ComputedCostMultiplier => {
                            if computed_cost_multiplier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("computedCostMultiplier"));
                            }
                            computed_cost_multiplier__ = map_.next_value()?;
                        }
                    }
                }
                Ok(ValidatorUpdateFeeComponents {
                    base_fee: base_fee__,
                    computed_cost_multiplier: computed_cost_multiplier__,
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.fees.v1alpha1.ValidatorUpdateFeeComponents", FIELDS, GeneratedVisitor)
    }
}
