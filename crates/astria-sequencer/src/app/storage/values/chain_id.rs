use std::borrow::Cow;

use astria_eyre::eyre::bail;
use borsh::{
    io::{
        Read,
        Write,
    },
    BorshDeserialize,
    BorshSerialize,
};

use super::{
    Value,
    ValueImpl,
};

#[derive(Debug)]
pub(in crate::app) struct ChainId<'a>(Cow<'a, tendermint::chain::Id>);

impl<'a> From<&'a tendermint::chain::Id> for ChainId<'a> {
    fn from(chain_id: &'a tendermint::chain::Id) -> Self {
        ChainId(Cow::Borrowed(chain_id))
    }
}

impl<'a> From<ChainId<'a>> for tendermint::chain::Id {
    fn from(chain_id: ChainId<'a>) -> Self {
        chain_id.0.into_owned()
    }
}

impl<'a> BorshSerialize for ChainId<'a> {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.as_str().serialize(writer)
    }
}

impl<'a> BorshDeserialize for ChainId<'a> {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let chain_id_str = String::deserialize_reader(reader)?;
        let chain_id =
            tendermint::chain::Id::try_from(chain_id_str).map_err(std::io::Error::other)?;
        Ok(ChainId(Cow::Owned(chain_id)))
    }
}

impl<'a> From<ChainId<'a>> for crate::storage::StoredValue<'a> {
    fn from(chain_id: ChainId<'a>) -> Self {
        crate::storage::StoredValue::App(Value(ValueImpl::ChainId(chain_id)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for ChainId<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::App(Value(ValueImpl::ChainId(chain_id))) = value else {
            bail!("app stored value type mismatch: expected chain id, found {value:?}");
        };
        Ok(chain_id)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn chain_id_serialization_round_trip() {
        let id = tendermint::chain::Id::from_str("a").unwrap();
        let chain_id = ChainId::from(&id);
        let serialized = borsh::to_vec(&chain_id).unwrap();
        let deserialized: ChainId = borsh::from_slice(&serialized).unwrap();
        assert_eq!(chain_id.0, deserialized.0);
    }
}
