use std::borrow::Cow;

use borsh::{
    io::{
        Read,
        Write,
    },
    BorshDeserialize,
    BorshSerialize,
};

use super::StoredValue;

#[derive(Debug)]
pub(crate) struct ChainId<'a>(Cow<'a, tendermint::chain::Id>);

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

impl<'a> TryFrom<StoredValue<'a>> for ChainId<'a> {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::ChainId(chain_id) = value else {
            return Err(super::type_mismatch("chain id", &value));
        };
        Ok(chain_id)
    }
}
