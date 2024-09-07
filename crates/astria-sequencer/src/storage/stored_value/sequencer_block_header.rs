use std::borrow::Cow;

use astria_core::sequencerblock::v1alpha1::block::{
    SequencerBlockHeader as DomainSequencerBlockHeader,
    SequencerBlockHeaderParts,
};
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::{
    AddressBytes,
    BlockHeight,
    BlockTimestamp,
    ChainId,
    StoredValue,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct SequencerBlockHeader<'a> {
    chain_id: ChainId<'a>,
    height: BlockHeight,
    time: BlockTimestamp,
    rollup_transactions_root: Cow<'a, [u8; 32]>,
    data_hash: Cow<'a, [u8; 32]>,
    proposer_address: AddressBytes<'a>,
}

impl<'a> From<&'a DomainSequencerBlockHeader> for SequencerBlockHeader<'a> {
    fn from(header: &'a DomainSequencerBlockHeader) -> Self {
        SequencerBlockHeader {
            chain_id: header.chain_id().into(),
            height: header.height().value().into(),
            time: header.time().into(),
            rollup_transactions_root: Cow::Borrowed(header.rollup_transactions_root()),
            data_hash: Cow::Borrowed(header.data_hash()),
            proposer_address: header.proposer_address().into(),
        }
    }
}

impl<'a> From<SequencerBlockHeader<'a>> for DomainSequencerBlockHeader {
    fn from(header: SequencerBlockHeader<'a>) -> Self {
        let height = tendermint::block::Height::try_from(u64::from(header.height))
            .expect("should not be able to store invalid height");
        DomainSequencerBlockHeader::unchecked_from_parts(SequencerBlockHeaderParts {
            chain_id: header.chain_id.into(),
            height,
            time: header.time.into(),
            rollup_transactions_root: header.rollup_transactions_root.into_owned(),
            data_hash: header.data_hash.into_owned(),
            proposer_address: tendermint::account::Id::new(header.proposer_address.into()),
        })
    }
}

impl<'a> TryFrom<StoredValue<'a>> for SequencerBlockHeader<'a> {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::SequencerBlockHeader(block_header) = value else {
            return Err(super::type_mismatch("sequencer block header", &value));
        };
        Ok(block_header)
    }
}
