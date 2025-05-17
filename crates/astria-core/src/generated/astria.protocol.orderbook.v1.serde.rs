impl serde::Serialize for Order {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.id.is_empty() {
            len += 1;
        }
        if self.owner.is_some() {
            len += 1;
        }
        if !self.market.is_empty() {
            len += 1;
        }
        if self.side.is_some() {
            len += 1;
        }
        if self.r#type.is_some() {
            len += 1;
        }
        if self.price.is_some() {
            len += 1;
        }
        if self.quantity.is_some() {
            len += 1;
        }
        if self.remaining_quantity.is_some() {
            len += 1;
        }
        if self.created_at != 0 {
            len += 1;
        }
        if self.time_in_force.is_some() {
            len += 1;
        }
        if !self.fee_asset.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.orderbook.v1.Order", len)?;
        if !self.id.is_empty() {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if let Some(v) = self.owner.as_ref() {
            struct_ser.serialize_field("owner", v)?;
        }
        if !self.market.is_empty() {
            struct_ser.serialize_field("market", &self.market)?;
        }
        if let Some(v) = self.side.as_ref() {
            struct_ser.serialize_field("side", v)?;
        }
        if let Some(v) = self.r#type.as_ref() {
            struct_ser.serialize_field("type", v)?;
        }
        if let Some(v) = self.price.as_ref() {
            struct_ser.serialize_field("price", v)?;
        }
        if let Some(v) = self.quantity.as_ref() {
            struct_ser.serialize_field("quantity", v)?;
        }
        if let Some(v) = self.remaining_quantity.as_ref() {
            struct_ser.serialize_field("remainingQuantity", v)?;
        }
        if self.created_at != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("createdAt", ToString::to_string(&self.created_at).as_str())?;
        }
        if let Some(v) = self.time_in_force.as_ref() {
            struct_ser.serialize_field("timeInForce", v)?;
        }
        if !self.fee_asset.is_empty() {
            struct_ser.serialize_field("feeAsset", &self.fee_asset)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Order {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "owner",
            "market",
            "side",
            "type",
            "price",
            "quantity",
            "remaining_quantity",
            "remainingQuantity",
            "created_at",
            "createdAt",
            "time_in_force",
            "timeInForce",
            "fee_asset",
            "feeAsset",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Owner,
            Market,
            Side,
            Type,
            Price,
            Quantity,
            RemainingQuantity,
            CreatedAt,
            TimeInForce,
            FeeAsset,
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
                            "id" => Ok(GeneratedField::Id),
                            "owner" => Ok(GeneratedField::Owner),
                            "market" => Ok(GeneratedField::Market),
                            "side" => Ok(GeneratedField::Side),
                            "type" => Ok(GeneratedField::Type),
                            "price" => Ok(GeneratedField::Price),
                            "quantity" => Ok(GeneratedField::Quantity),
                            "remainingQuantity" | "remaining_quantity" => Ok(GeneratedField::RemainingQuantity),
                            "createdAt" | "created_at" => Ok(GeneratedField::CreatedAt),
                            "timeInForce" | "time_in_force" => Ok(GeneratedField::TimeInForce),
                            "feeAsset" | "fee_asset" => Ok(GeneratedField::FeeAsset),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Order;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.orderbook.v1.Order")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Order, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut owner__ = None;
                let mut market__ = None;
                let mut side__ = None;
                let mut r#type__ = None;
                let mut price__ = None;
                let mut quantity__ = None;
                let mut remaining_quantity__ = None;
                let mut created_at__ = None;
                let mut time_in_force__ = None;
                let mut fee_asset__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Owner => {
                            if owner__.is_some() {
                                return Err(serde::de::Error::duplicate_field("owner"));
                            }
                            owner__ = map_.next_value()?;
                        }
                        GeneratedField::Market => {
                            if market__.is_some() {
                                return Err(serde::de::Error::duplicate_field("market"));
                            }
                            market__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Side => {
                            if side__.is_some() {
                                return Err(serde::de::Error::duplicate_field("side"));
                            }
                            side__ = map_.next_value()?;
                        }
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = map_.next_value()?;
                        }
                        GeneratedField::Price => {
                            if price__.is_some() {
                                return Err(serde::de::Error::duplicate_field("price"));
                            }
                            price__ = map_.next_value()?;
                        }
                        GeneratedField::Quantity => {
                            if quantity__.is_some() {
                                return Err(serde::de::Error::duplicate_field("quantity"));
                            }
                            quantity__ = map_.next_value()?;
                        }
                        GeneratedField::RemainingQuantity => {
                            if remaining_quantity__.is_some() {
                                return Err(serde::de::Error::duplicate_field("remainingQuantity"));
                            }
                            remaining_quantity__ = map_.next_value()?;
                        }
                        GeneratedField::CreatedAt => {
                            if created_at__.is_some() {
                                return Err(serde::de::Error::duplicate_field("createdAt"));
                            }
                            created_at__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::TimeInForce => {
                            if time_in_force__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timeInForce"));
                            }
                            time_in_force__ = map_.next_value()?;
                        }
                        GeneratedField::FeeAsset => {
                            if fee_asset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("feeAsset"));
                            }
                            fee_asset__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Order {
                    id: id__.unwrap_or_default(),
                    owner: owner__,
                    market: market__.unwrap_or_default(),
                    side: side__,
                    r#type: r#type__,
                    price: price__,
                    quantity: quantity__,
                    remaining_quantity: remaining_quantity__,
                    created_at: created_at__.unwrap_or_default(),
                    time_in_force: time_in_force__,
                    fee_asset: fee_asset__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.orderbook.v1.Order", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for OrderMatch {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.id.is_empty() {
            len += 1;
        }
        if !self.market.is_empty() {
            len += 1;
        }
        if self.price.is_some() {
            len += 1;
        }
        if self.quantity.is_some() {
            len += 1;
        }
        if !self.maker_order_id.is_empty() {
            len += 1;
        }
        if !self.taker_order_id.is_empty() {
            len += 1;
        }
        if self.taker_side.is_some() {
            len += 1;
        }
        if self.timestamp != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.orderbook.v1.OrderMatch", len)?;
        if !self.id.is_empty() {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if !self.market.is_empty() {
            struct_ser.serialize_field("market", &self.market)?;
        }
        if let Some(v) = self.price.as_ref() {
            struct_ser.serialize_field("price", v)?;
        }
        if let Some(v) = self.quantity.as_ref() {
            struct_ser.serialize_field("quantity", v)?;
        }
        if !self.maker_order_id.is_empty() {
            struct_ser.serialize_field("makerOrderId", &self.maker_order_id)?;
        }
        if !self.taker_order_id.is_empty() {
            struct_ser.serialize_field("takerOrderId", &self.taker_order_id)?;
        }
        if let Some(v) = self.taker_side.as_ref() {
            struct_ser.serialize_field("takerSide", v)?;
        }
        if self.timestamp != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("timestamp", ToString::to_string(&self.timestamp).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for OrderMatch {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "market",
            "price",
            "quantity",
            "maker_order_id",
            "makerOrderId",
            "taker_order_id",
            "takerOrderId",
            "taker_side",
            "takerSide",
            "timestamp",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Market,
            Price,
            Quantity,
            MakerOrderId,
            TakerOrderId,
            TakerSide,
            Timestamp,
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
                            "id" => Ok(GeneratedField::Id),
                            "market" => Ok(GeneratedField::Market),
                            "price" => Ok(GeneratedField::Price),
                            "quantity" => Ok(GeneratedField::Quantity),
                            "makerOrderId" | "maker_order_id" => Ok(GeneratedField::MakerOrderId),
                            "takerOrderId" | "taker_order_id" => Ok(GeneratedField::TakerOrderId),
                            "takerSide" | "taker_side" => Ok(GeneratedField::TakerSide),
                            "timestamp" => Ok(GeneratedField::Timestamp),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = OrderMatch;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.orderbook.v1.OrderMatch")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<OrderMatch, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut market__ = None;
                let mut price__ = None;
                let mut quantity__ = None;
                let mut maker_order_id__ = None;
                let mut taker_order_id__ = None;
                let mut taker_side__ = None;
                let mut timestamp__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Market => {
                            if market__.is_some() {
                                return Err(serde::de::Error::duplicate_field("market"));
                            }
                            market__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Price => {
                            if price__.is_some() {
                                return Err(serde::de::Error::duplicate_field("price"));
                            }
                            price__ = map_.next_value()?;
                        }
                        GeneratedField::Quantity => {
                            if quantity__.is_some() {
                                return Err(serde::de::Error::duplicate_field("quantity"));
                            }
                            quantity__ = map_.next_value()?;
                        }
                        GeneratedField::MakerOrderId => {
                            if maker_order_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("makerOrderId"));
                            }
                            maker_order_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::TakerOrderId => {
                            if taker_order_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("takerOrderId"));
                            }
                            taker_order_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::TakerSide => {
                            if taker_side__.is_some() {
                                return Err(serde::de::Error::duplicate_field("takerSide"));
                            }
                            taker_side__ = map_.next_value()?;
                        }
                        GeneratedField::Timestamp => {
                            if timestamp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timestamp"));
                            }
                            timestamp__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(OrderMatch {
                    id: id__.unwrap_or_default(),
                    market: market__.unwrap_or_default(),
                    price: price__,
                    quantity: quantity__,
                    maker_order_id: maker_order_id__.unwrap_or_default(),
                    taker_order_id: taker_order_id__.unwrap_or_default(),
                    taker_side: taker_side__,
                    timestamp: timestamp__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.orderbook.v1.OrderMatch", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for OrderSide {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.inner != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.orderbook.v1.OrderSide", len)?;
        if self.inner != 0 {
            let v = OrderSideInner::try_from(self.inner)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.inner)))?;
            struct_ser.serialize_field("inner", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for OrderSide {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "inner",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Inner,
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
                            "inner" => Ok(GeneratedField::Inner),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = OrderSide;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.orderbook.v1.OrderSide")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<OrderSide, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut inner__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Inner => {
                            if inner__.is_some() {
                                return Err(serde::de::Error::duplicate_field("inner"));
                            }
                            inner__ = Some(map_.next_value::<OrderSideInner>()? as i32);
                        }
                    }
                }
                Ok(OrderSide {
                    inner: inner__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.orderbook.v1.OrderSide", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for OrderSideInner {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "ORDER_SIDE_INNER_UNSPECIFIED",
            Self::Buy => "ORDER_SIDE_INNER_BUY",
            Self::Sell => "ORDER_SIDE_INNER_SELL",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for OrderSideInner {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "ORDER_SIDE_INNER_UNSPECIFIED",
            "ORDER_SIDE_INNER_BUY",
            "ORDER_SIDE_INNER_SELL",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = OrderSideInner;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "ORDER_SIDE_INNER_UNSPECIFIED" => Ok(OrderSideInner::Unspecified),
                    "ORDER_SIDE_INNER_BUY" => Ok(OrderSideInner::Buy),
                    "ORDER_SIDE_INNER_SELL" => Ok(OrderSideInner::Sell),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for OrderTimeInForce {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.inner != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.orderbook.v1.OrderTimeInForce", len)?;
        if self.inner != 0 {
            let v = OrderTimeInForceInner::try_from(self.inner)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.inner)))?;
            struct_ser.serialize_field("inner", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for OrderTimeInForce {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "inner",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Inner,
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
                            "inner" => Ok(GeneratedField::Inner),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = OrderTimeInForce;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.orderbook.v1.OrderTimeInForce")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<OrderTimeInForce, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut inner__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Inner => {
                            if inner__.is_some() {
                                return Err(serde::de::Error::duplicate_field("inner"));
                            }
                            inner__ = Some(map_.next_value::<OrderTimeInForceInner>()? as i32);
                        }
                    }
                }
                Ok(OrderTimeInForce {
                    inner: inner__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.orderbook.v1.OrderTimeInForce", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for OrderTimeInForceInner {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "ORDER_TIME_IN_FORCE_INNER_UNSPECIFIED",
            Self::Gtc => "ORDER_TIME_IN_FORCE_INNER_GTC",
            Self::Ioc => "ORDER_TIME_IN_FORCE_INNER_IOC",
            Self::Fok => "ORDER_TIME_IN_FORCE_INNER_FOK",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for OrderTimeInForceInner {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "ORDER_TIME_IN_FORCE_INNER_UNSPECIFIED",
            "ORDER_TIME_IN_FORCE_INNER_GTC",
            "ORDER_TIME_IN_FORCE_INNER_IOC",
            "ORDER_TIME_IN_FORCE_INNER_FOK",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = OrderTimeInForceInner;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "ORDER_TIME_IN_FORCE_INNER_UNSPECIFIED" => Ok(OrderTimeInForceInner::Unspecified),
                    "ORDER_TIME_IN_FORCE_INNER_GTC" => Ok(OrderTimeInForceInner::Gtc),
                    "ORDER_TIME_IN_FORCE_INNER_IOC" => Ok(OrderTimeInForceInner::Ioc),
                    "ORDER_TIME_IN_FORCE_INNER_FOK" => Ok(OrderTimeInForceInner::Fok),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for OrderType {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.inner != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.orderbook.v1.OrderType", len)?;
        if self.inner != 0 {
            let v = OrderTypeInner::try_from(self.inner)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.inner)))?;
            struct_ser.serialize_field("inner", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for OrderType {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "inner",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Inner,
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
                            "inner" => Ok(GeneratedField::Inner),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = OrderType;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.orderbook.v1.OrderType")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<OrderType, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut inner__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Inner => {
                            if inner__.is_some() {
                                return Err(serde::de::Error::duplicate_field("inner"));
                            }
                            inner__ = Some(map_.next_value::<OrderTypeInner>()? as i32);
                        }
                    }
                }
                Ok(OrderType {
                    inner: inner__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.orderbook.v1.OrderType", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for OrderTypeInner {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "ORDER_TYPE_INNER_UNSPECIFIED",
            Self::Limit => "ORDER_TYPE_INNER_LIMIT",
            Self::Market => "ORDER_TYPE_INNER_MARKET",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for OrderTypeInner {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "ORDER_TYPE_INNER_UNSPECIFIED",
            "ORDER_TYPE_INNER_LIMIT",
            "ORDER_TYPE_INNER_MARKET",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = OrderTypeInner;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "ORDER_TYPE_INNER_UNSPECIFIED" => Ok(OrderTypeInner::Unspecified),
                    "ORDER_TYPE_INNER_LIMIT" => Ok(OrderTypeInner::Limit),
                    "ORDER_TYPE_INNER_MARKET" => Ok(OrderTypeInner::Market),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for Orderbook {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.market.is_empty() {
            len += 1;
        }
        if !self.bids.is_empty() {
            len += 1;
        }
        if !self.asks.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.orderbook.v1.Orderbook", len)?;
        if !self.market.is_empty() {
            struct_ser.serialize_field("market", &self.market)?;
        }
        if !self.bids.is_empty() {
            struct_ser.serialize_field("bids", &self.bids)?;
        }
        if !self.asks.is_empty() {
            struct_ser.serialize_field("asks", &self.asks)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Orderbook {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "market",
            "bids",
            "asks",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Market,
            Bids,
            Asks,
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
                            "market" => Ok(GeneratedField::Market),
                            "bids" => Ok(GeneratedField::Bids),
                            "asks" => Ok(GeneratedField::Asks),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Orderbook;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.orderbook.v1.Orderbook")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Orderbook, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut market__ = None;
                let mut bids__ = None;
                let mut asks__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Market => {
                            if market__.is_some() {
                                return Err(serde::de::Error::duplicate_field("market"));
                            }
                            market__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Bids => {
                            if bids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bids"));
                            }
                            bids__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Asks => {
                            if asks__.is_some() {
                                return Err(serde::de::Error::duplicate_field("asks"));
                            }
                            asks__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Orderbook {
                    market: market__.unwrap_or_default(),
                    bids: bids__.unwrap_or_default(),
                    asks: asks__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.orderbook.v1.Orderbook", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for OrderbookEntry {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.price.is_some() {
            len += 1;
        }
        if self.quantity.is_some() {
            len += 1;
        }
        if self.order_count != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.orderbook.v1.OrderbookEntry", len)?;
        if let Some(v) = self.price.as_ref() {
            struct_ser.serialize_field("price", v)?;
        }
        if let Some(v) = self.quantity.as_ref() {
            struct_ser.serialize_field("quantity", v)?;
        }
        if self.order_count != 0 {
            struct_ser.serialize_field("orderCount", &self.order_count)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for OrderbookEntry {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "price",
            "quantity",
            "order_count",
            "orderCount",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Price,
            Quantity,
            OrderCount,
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
                            "price" => Ok(GeneratedField::Price),
                            "quantity" => Ok(GeneratedField::Quantity),
                            "orderCount" | "order_count" => Ok(GeneratedField::OrderCount),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = OrderbookEntry;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.orderbook.v1.OrderbookEntry")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<OrderbookEntry, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut price__ = None;
                let mut quantity__ = None;
                let mut order_count__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Price => {
                            if price__.is_some() {
                                return Err(serde::de::Error::duplicate_field("price"));
                            }
                            price__ = map_.next_value()?;
                        }
                        GeneratedField::Quantity => {
                            if quantity__.is_some() {
                                return Err(serde::de::Error::duplicate_field("quantity"));
                            }
                            quantity__ = map_.next_value()?;
                        }
                        GeneratedField::OrderCount => {
                            if order_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("orderCount"));
                            }
                            order_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(OrderbookEntry {
                    price: price__,
                    quantity: quantity__,
                    order_count: order_count__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.orderbook.v1.OrderbookEntry", FIELDS, GeneratedVisitor)
    }
}
