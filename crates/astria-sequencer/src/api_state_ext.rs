use anyhow::{
    anyhow,
    bail,
    Context as _,
    Result,
};
use astria_core::{
    generated::sequencer::v1 as raw,
    sequencer::v1::{
        block::{
            RollupTransactions,
            SequencerBlock,
            SequencerBlockHeader,
            SequencerBlockParts,
        },
        RollupId,
    },
    Protobuf,
};
use async_trait::async_trait;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use prost::Message;
use tracing::instrument;

fn block_hash_by_height_key(height: u64) -> String {
    format!("blockhash/{height}")
}

fn sequencer_block_header_by_hash_key(hash: &[u8]) -> String {
    format!("blockheader/{}", crate::utils::Hex(hash))
}

fn rollup_data_by_hash_and_rollup_id_key(hash: &[u8], rollup_id: &RollupId) -> String {
    format!("rollupdata/{}/{}", crate::utils::Hex(hash), rollup_id)
}

fn rollup_ids_by_hash_key(hash: &[u8]) -> String {
    format!("rollupids/{}", crate::utils::Hex(hash))
}

fn rollup_transactions_proof_by_hash_key(hash: &[u8]) -> String {
    format!("rolluptxsproof/{}", crate::utils::Hex(hash))
}

fn rollup_ids_proof_by_hash_key(hash: &[u8]) -> String {
    format!("rollupidsproof/{}", crate::utils::Hex(hash))
}

#[derive(BorshSerialize, BorshDeserialize)]
struct RollupIdSeq(
    #[borsh(
        deserialize_with = "rollup_id_impl::deserialize_many",
        serialize_with = "rollup_id_impl::serialize_many"
    )]
    Vec<RollupId>,
);

impl From<Vec<RollupId>> for RollupIdSeq {
    fn from(value: Vec<RollupId>) -> Self {
        RollupIdSeq(value)
    }
}

mod rollup_id_impl {
    use super::{
        RollupId,
        RollupIdSer,
    };

    pub(super) fn deserialize<R: borsh::io::Read>(
        reader: &mut R,
    ) -> ::core::result::Result<RollupId, borsh::io::Error> {
        let inner: [u8; 32] = borsh::BorshDeserialize::deserialize_reader(reader)?;
        Ok(RollupId::from(inner))
    }

    pub(super) fn serialize<W: borsh::io::Write>(
        obj: &RollupId,
        writer: &mut W,
    ) -> ::core::result::Result<(), borsh::io::Error> {
        borsh::BorshSerialize::serialize(&obj.get(), writer)?;
        Ok(())
    }

    pub(super) fn deserialize_many<R: borsh::io::Read>(
        reader: &mut R,
    ) -> ::core::result::Result<Vec<RollupId>, borsh::io::Error> {
        let deser: Vec<RollupIdSer> = borsh::BorshDeserialize::deserialize_reader(reader)?;
        let ids = deser.into_iter().map(RollupIdSer::get).collect();
        Ok(ids)
    }

    pub(super) fn serialize_many<W: borsh::io::Write>(
        obj: &[RollupId],
        writer: &mut W,
    ) -> ::core::result::Result<(), borsh::io::Error> {
        let inner: Vec<_> = obj.iter().copied().map(RollupIdSer::from).collect();
        borsh::BorshSerialize::serialize(&inner, writer)?;
        Ok(())
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
struct RollupIdSer(
    #[borsh(
        deserialize_with = "rollup_id_impl::deserialize",
        serialize_with = "rollup_id_impl::serialize"
    )]
    RollupId,
);

impl RollupIdSer {
    fn get(self) -> RollupId {
        self.0
    }
}

impl From<RollupId> for RollupIdSer {
    fn from(value: RollupId) -> Self {
        Self(value)
    }
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    async fn get_block_hash_by_height(&self, height: u64) -> Result<[u8; 32]> {
        let key = block_hash_by_height_key(height);
        let Some(hash) = self
            .get_raw(&key)
            .await
            .context("failed to read block hash by height from state")?
        else {
            bail!("block hash not found for given height");
        };

        let hash: [u8; 32] = hash.try_into().map_err(|bytes: Vec<_>| {
            anyhow!("expected 32 bytes block hash, but got {}", bytes.len())
        })?;
        Ok(hash)
    }

    #[instrument(skip_all)]
    async fn get_sequencer_block_header_by_hash(
        &self,
        hash: &[u8],
    ) -> Result<SequencerBlockHeader> {
        let key = sequencer_block_header_by_hash_key(hash);
        let Some(header_bytes) = self
            .get_raw(&key)
            .await
            .context("failed to read raw sequencer block from state")?
        else {
            bail!("header not found for given block hash");
        };

        let raw = raw::SequencerBlockHeader::decode(header_bytes.as_slice())
            .context("failed to decode sequencer block from raw bytes")?;
        let header = SequencerBlockHeader::try_from_raw(raw)
            .context("failed to convert raw sequencer block to sequencer block")?;
        Ok(header)
    }

    #[instrument(skip_all)]
    async fn get_rollup_ids_by_block_hash(&self, hash: &[u8]) -> Result<Vec<RollupId>> {
        let key = rollup_ids_by_hash_key(hash);
        let Some(rollup_ids_bytes) = self
            .get_raw(&key)
            .await
            .context("failed to read rollup IDs by block hash from state")?
        else {
            bail!("rollup IDs not found for given block hash");
        };

        let RollupIdSeq(rollup_ids) = RollupIdSeq::try_from_slice(&rollup_ids_bytes)
            .context("failed to deserialize rollup IDs list")?;
        Ok(rollup_ids)
    }

    #[instrument(skip_all)]
    async fn get_sequencer_block_by_hash(&self, hash: &[u8]) -> Result<SequencerBlock> {
        let Some(header_bytes) = self
            .get_raw(&sequencer_block_header_by_hash_key(hash))
            .await
            .context("failed to read raw sequencer block from state")?
        else {
            bail!("header not found for given block hash");
        };

        let header_raw = raw::SequencerBlockHeader::decode(header_bytes.as_slice())
            .context("failed to decode sequencer block from raw bytes")?;

        let rollup_ids = self
            .get_rollup_ids_by_block_hash(hash)
            .await
            .context("failed to get rollup IDs by block hash")?;

        let mut rollup_transactions = Vec::with_capacity(rollup_ids.len());
        for id in &rollup_ids {
            let key = rollup_data_by_hash_and_rollup_id_key(hash, id);
            let raw = self
                .get_raw(&key)
                .await
                .context("failed to read rollup data by block hash and rollup ID from state")?;
            if let Some(raw) = raw {
                let raw = raw.as_slice();
                let rollup_data = raw::RollupTransactions::decode(raw)
                    .context("failed to decode rollup data from raw bytes")?;
                rollup_transactions.push(rollup_data);
            }
        }

        let Some(rollup_transactions_proof) = self
            .get_raw(&rollup_transactions_proof_by_hash_key(hash))
            .await
            .context("failed to read rollup transactions proof by block hash from state")?
        else {
            bail!("rollup transactions proof not found for given block hash");
        };

        let rollup_transactions_proof = raw::Proof::decode(rollup_transactions_proof.as_slice())
            .context("failed to decode rollup transactions proof from raw bytes")?;

        let Some(rollup_ids_proof) = self
            .get_raw(&rollup_ids_proof_by_hash_key(hash))
            .await
            .context("failed to read rollup IDs proof by block hash from state")?
        else {
            bail!("rollup IDs proof not found for given block hash");
        };

        let rollup_ids_proof = raw::Proof::decode(rollup_ids_proof.as_slice())
            .context("failed to decode rollup IDs proof from raw bytes")?;

        let raw = raw::SequencerBlock {
            header: header_raw.into(),
            rollup_transactions,
            rollup_transactions_proof: rollup_transactions_proof.into(),
            rollup_ids_proof: rollup_ids_proof.into(),
        };

        let block = SequencerBlock::try_from_raw(raw)
            .context("failed to convert raw sequencer block to sequencer block")?;

        Ok(block)
    }

    #[instrument(skip_all)]
    async fn get_sequencer_block_by_height(&self, height: u64) -> Result<SequencerBlock> {
        let hash = self
            .get_block_hash_by_height(height)
            .await
            .context("failed to get block hash by height")?;
        self.get_sequencer_block_by_hash(&hash)
            .await
            .context("failed to get sequencer block by hash")
    }

    #[instrument(skip_all)]
    async fn get_rollup_data(
        &self,
        hash: &[u8],
        rollup_id: &RollupId,
    ) -> Result<RollupTransactions> {
        let key = rollup_data_by_hash_and_rollup_id_key(hash, rollup_id);
        let Some(bytes) = self
            .get_raw(&key)
            .await
            .context("failed to read rollup data by block hash and rollup ID from state")?
        else {
            bail!("rollup data not found for given block hash and rollup ID");
        };
        let raw = raw::RollupTransactions::decode(bytes.as_slice())
            .context("failed to decode rollup data from raw bytes")?;

        let rollup_transactions = RollupTransactions::try_from_raw(raw)
            .context("failed to convert raw rollup transaction to rollup transaction")?;

        Ok(rollup_transactions)
    }

    #[instrument(skip_all)]
    async fn get_block_proofs_by_block_hash(
        &self,
        hash: &[u8],
    ) -> Result<(raw::Proof, raw::Proof)> {
        let Some(rollup_transactions_proof) = self
            .get_raw(&rollup_transactions_proof_by_hash_key(hash))
            .await
            .context("failed to read rollup transactions proof by block hash from state")?
        else {
            bail!("rollup transactions proof not found for given block hash");
        };

        let rollup_transactions_proof = raw::Proof::decode(rollup_transactions_proof.as_slice())
            .context("failed to decode rollup transactions proof from raw bytes")?;

        let Some(rollup_ids_proof) = self
            .get_raw(&rollup_ids_proof_by_hash_key(hash))
            .await
            .context("failed to read rollup IDs proof by block hash from state")?
        else {
            bail!("rollup IDs proof not found for given block hash");
        };

        let rollup_ids_proof = raw::Proof::decode(rollup_ids_proof.as_slice())
            .context("failed to decode rollup IDs proof from raw bytes")?;

        Ok((rollup_transactions_proof, rollup_ids_proof))
    }
}

impl<T: StateRead> StateReadExt for T {}

pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_sequencer_block(&mut self, block: SequencerBlock) -> Result<()> {
        // split up and write the sequencer block to state in the following order:
        // 1. height to block hash
        // 2. block hash to rollup IDs
        // 3. block hash to block header
        // 4. for each rollup ID in the block, map block hash + rollup ID to rollup data
        // 5. block hash to rollup transactions proof
        // 6. block hash to rollup IDs proof

        let key = block_hash_by_height_key(block.height().into());
        self.put_raw(key, block.block_hash().to_vec());

        let rollup_ids = block
            .rollup_transactions()
            .keys()
            .copied()
            .map(From::from)
            .collect::<Vec<_>>();

        let key = rollup_ids_by_hash_key(&block.block_hash());

        self.put_raw(
            key,
            borsh::to_vec(&RollupIdSeq(rollup_ids))
                .context("failed to serialize rollup IDs list")?,
        );

        let block_hash = block.block_hash();
        let key = sequencer_block_header_by_hash_key(&block.block_hash());
        let SequencerBlockParts {
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
        } = block.into_parts();
        let header = header.into_raw();
        self.put_raw(key, header.encode_to_vec());

        for (id, rollup_data) in rollup_transactions {
            let key = rollup_data_by_hash_and_rollup_id_key(&block_hash, &id);
            self.put_raw(key, rollup_data.into_raw().encode_to_vec());
        }

        let key = rollup_transactions_proof_by_hash_key(&block_hash);
        self.put_raw(key, rollup_transactions_proof.into_raw().encode_to_vec());

        let key = rollup_ids_proof_by_hash_key(&block_hash);
        self.put_raw(key, rollup_ids_proof.into_raw().encode_to_vec());

        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}
