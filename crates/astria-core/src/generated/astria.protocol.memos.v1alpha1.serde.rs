impl serde::Serialize for Ics20TransferDeposit {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.rollup_deposit_address.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.memos.v1alpha1.Ics20TransferDeposit", len)?;
        if !self.rollup_deposit_address.is_empty() {
            struct_ser.serialize_field("rollupDepositAddress", &self.rollup_deposit_address)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Ics20TransferDeposit {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "rollup_deposit_address",
            "rollupDepositAddress",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RollupDepositAddress,
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
                            "rollupDepositAddress" | "rollup_deposit_address" => Ok(GeneratedField::RollupDepositAddress),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Ics20TransferDeposit;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.memos.v1alpha1.Ics20TransferDeposit")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Ics20TransferDeposit, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut rollup_deposit_address__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RollupDepositAddress => {
                            if rollup_deposit_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupDepositAddress"));
                            }
                            rollup_deposit_address__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Ics20TransferDeposit {
                    rollup_deposit_address: rollup_deposit_address__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.memos.v1alpha1.Ics20TransferDeposit", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Ics20WithdrawalFromRollup {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.rollup_block_number != 0 {
            len += 1;
        }
        if !self.rollup_withdrawal_event_id.is_empty() {
            len += 1;
        }
        if !self.rollup_return_address.is_empty() {
            len += 1;
        }
        if !self.memo.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("astria.protocol.memos.v1alpha1.Ics20WithdrawalFromRollup", len)?;
        if self.rollup_block_number != 0 {
            #[allow(clippy::needless_borrow)]
            struct_ser.serialize_field("rollupBlockNumber", ToString::to_string(&self.rollup_block_number).as_str())?;
        }
        if !self.rollup_withdrawal_event_id.is_empty() {
            struct_ser.serialize_field("rollupWithdrawalEventId", &self.rollup_withdrawal_event_id)?;
        }
        if !self.rollup_return_address.is_empty() {
            struct_ser.serialize_field("rollupReturnAddress", &self.rollup_return_address)?;
        }
        if !self.memo.is_empty() {
            struct_ser.serialize_field("memo", &self.memo)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Ics20WithdrawalFromRollup {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "rollup_block_number",
            "rollupBlockNumber",
            "rollup_withdrawal_event_id",
            "rollupWithdrawalEventId",
            "rollup_return_address",
            "rollupReturnAddress",
            "memo",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RollupBlockNumber,
            RollupWithdrawalEventId,
            RollupReturnAddress,
            Memo,
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
                            "rollupBlockNumber" | "rollup_block_number" => Ok(GeneratedField::RollupBlockNumber),
                            "rollupWithdrawalEventId" | "rollup_withdrawal_event_id" => Ok(GeneratedField::RollupWithdrawalEventId),
                            "rollupReturnAddress" | "rollup_return_address" => Ok(GeneratedField::RollupReturnAddress),
                            "memo" => Ok(GeneratedField::Memo),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Ics20WithdrawalFromRollup;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct astria.protocol.memos.v1alpha1.Ics20WithdrawalFromRollup")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Ics20WithdrawalFromRollup, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut rollup_block_number__ = None;
                let mut rollup_withdrawal_event_id__ = None;
                let mut rollup_return_address__ = None;
                let mut memo__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RollupBlockNumber => {
                            if rollup_block_number__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupBlockNumber"));
                            }
                            rollup_block_number__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::RollupWithdrawalEventId => {
                            if rollup_withdrawal_event_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupWithdrawalEventId"));
                            }
                            rollup_withdrawal_event_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::RollupReturnAddress => {
                            if rollup_return_address__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rollupReturnAddress"));
                            }
                            rollup_return_address__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Memo => {
                            if memo__.is_some() {
                                return Err(serde::de::Error::duplicate_field("memo"));
                            }
                            memo__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Ics20WithdrawalFromRollup {
                    rollup_block_number: rollup_block_number__.unwrap_or_default(),
                    rollup_withdrawal_event_id: rollup_withdrawal_event_id__.unwrap_or_default(),
                    rollup_return_address: rollup_return_address__.unwrap_or_default(),
                    memo: memo__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("astria.protocol.memos.v1alpha1.Ics20WithdrawalFromRollup", FIELDS, GeneratedVisitor)
    }
}
