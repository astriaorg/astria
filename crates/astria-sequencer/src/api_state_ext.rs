use anyhow::{
    anyhow,
    bail,
    Context as _,
    Result,
};
use astria_core::{
    generated::{
        primitive::v1 as primitiveRaw,
        sequencerblock::v1alpha1 as raw,
    },
    primitive::v1::RollupId,
    sequencerblock::v1alpha1::block::{
        RollupTransactions,
        SequencerBlock,
        SequencerBlockHeader,
        SequencerBlockParts,
    },
    Protobuf as _,
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

        let rollup_transactions_proof =
            primitiveRaw::Proof::decode(rollup_transactions_proof.as_slice())
                .context("failed to decode rollup transactions proof from raw bytes")?;

        let Some(rollup_ids_proof) = self
            .get_raw(&rollup_ids_proof_by_hash_key(hash))
            .await
            .context("failed to read rollup IDs proof by block hash from state")?
        else {
            bail!("rollup IDs proof not found for given block hash");
        };

        let rollup_ids_proof = primitiveRaw::Proof::decode(rollup_ids_proof.as_slice())
            .context("failed to decode rollup IDs proof from raw bytes")?;

        let raw = raw::SequencerBlock {
            block_hash: hash.to_vec(),
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
    ) -> Result<(primitiveRaw::Proof, primitiveRaw::Proof)> {
        let Some(rollup_transactions_proof) = self
            .get_raw(&rollup_transactions_proof_by_hash_key(hash))
            .await
            .context("failed to read rollup transactions proof by block hash from state")?
        else {
            bail!("rollup transactions proof not found for given block hash");
        };

        let rollup_transactions_proof =
            primitiveRaw::Proof::decode(rollup_transactions_proof.as_slice())
                .context("failed to decode rollup transactions proof from raw bytes")?;

        let Some(rollup_ids_proof) = self
            .get_raw(&rollup_ids_proof_by_hash_key(hash))
            .await
            .context("failed to read rollup IDs proof by block hash from state")?
        else {
            bail!("rollup IDs proof not found for given block hash");
        };

        let rollup_ids_proof = primitiveRaw::Proof::decode(rollup_ids_proof.as_slice())
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

        let key = sequencer_block_header_by_hash_key(&block.block_hash());
        let SequencerBlockParts {
            block_hash,
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

#[cfg(test)]
mod test {
    use astria_core::{
        primitive::v1::{
            asset::Id,
            Address,
        },
        protocol::test_utils::ConfigureSequencerBlock,
        sequencerblock::v1alpha1::block::Deposit,
    };
    use cnidarium::StateDelta;
    use rand::Rng;

    use super::*;

    // creates new sequencer block, optionally shifting all values except the height by 1
    fn make_test_sequencer_block(height: u32) -> SequencerBlock {
        let mut rng = rand::thread_rng();
        let block_hash: [u8; 32] = rng.gen();

        // create inner rollup id/tx data
        let mut deposits = vec![];
        for _ in 0..2 {
            let rollup_id = RollupId::new(rng.gen());
            let bridge_address = Address::try_from_slice(&[rng.gen(); 20]).unwrap();
            let amount = rng.gen::<u128>();
            let asset_id = Id::from_denom(&rng.gen::<u8>().to_string());
            let destination_chain_address = rng.gen::<u8>().to_string();
            let deposit = Deposit::new(
                bridge_address,
                rollup_id,
                amount,
                asset_id,
                destination_chain_address,
            );
            deposits.push(deposit);
        }

        ConfigureSequencerBlock {
            block_hash: Some(block_hash),
            height: height.into(),
            deposits,
            ..Default::default()
        }
        .make()
    }

    #[tokio::test]
    async fn put_sequencer_block() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // can write one
        let block_0 = make_test_sequencer_block(2u32);
        state
            .put_sequencer_block(block_0.clone())
            .expect("writing block to database should work");

        assert_eq!(
            state
                .get_sequencer_block_by_height(block_0.height().into())
                .await
                .expect("a block was written to the database and should exist"),
            block_0,
            "stored block does not match expected"
        );

        // can write another and both are ok
        let block_1 = make_test_sequencer_block(3u32);
        state
            .put_sequencer_block(block_1.clone())
            .expect("writing another block to database should work");
        assert_eq!(
            state
                .get_sequencer_block_by_height(block_0.height().into())
                .await
                .expect("a block was written to the database and should exist"),
            block_0,
            "original stored block does not match expected"
        );
        assert_eq!(
            state
                .get_sequencer_block_by_height(block_1.height().into())
                .await
                .expect("a block was written to the database and should exist"),
            block_1,
            "additionally stored block does not match expected"
        );
    }

    #[tokio::test]
    async fn put_sequencer_block_update() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // write original block
        let mut block = make_test_sequencer_block(2u32);
        state
            .put_sequencer_block(block.clone())
            .expect("writing block to database should work");
        assert_eq!(
            state
                .get_sequencer_block_by_height(block.height().into())
                .await
                .expect("a block was written to the database and should exist"),
            block,
            "stored block does not match expected"
        );

        // write to same height but with new values
        block = make_test_sequencer_block(2u32);
        state
            .put_sequencer_block(block.clone())
            .expect("writing block update to database should work");

        // block was updates
        assert_eq!(
            state
                .get_sequencer_block_by_height(block.height().into())
                .await
                .expect("a block was written to the database and should exist"),
            block,
            "updated stored block does not match expected"
        );
    }

    #[tokio::test]
    async fn get_block_hash_by_height() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // write block
        let block = make_test_sequencer_block(2u32);
        state
            .put_sequencer_block(block.clone())
            .expect("writing block to database should work");

        // grab block hash by block height
        assert_eq!(
            state
                .get_block_hash_by_height(block.height().into())
                .await
                .expect(
                    "a block was written to the database and we should be able to query its block \
                     hash by height"
                ),
            block.block_hash(),
            "stored block hash does not match expected"
        );
    }

    #[tokio::test]
    async fn get_sequencer_block_header_by_hash() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // write block
        let block = make_test_sequencer_block(2u32);
        state
            .put_sequencer_block(block.clone())
            .expect("writing block to database should work");

        // grab block header by block hash
        assert_eq!(
            state
                .get_sequencer_block_header_by_hash(block.block_hash().as_ref())
                .await
                .expect(
                    "a block was written to the database and we should be able to query its block \
                     header by block hash"
                ),
            block.header().clone(),
            "stored block header does not match expected"
        );
    }

    #[tokio::test]
    async fn get_rollup_ids_by_block_hash() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // write block
        let block = make_test_sequencer_block(2u32);
        state
            .put_sequencer_block(block.clone())
            .expect("writing block to database should work");

        // grab rollup ids by block hash
        let stored_rollup_ids = state
            .get_rollup_ids_by_block_hash(block.block_hash().as_ref())
            .await
            .expect(
                "a block was written to the database and we should be able to query its rollup ids",
            );
        let original_rollup_ids: Vec<RollupId> =
            block.rollup_transactions().keys().copied().collect();
        assert_eq!(
            stored_rollup_ids, original_rollup_ids,
            "stored rollup ids do not match expected"
        );
    }

    #[tokio::test]
    async fn get_sequencer_block_by_hash() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // write block
        let block = make_test_sequencer_block(2u32);
        state
            .put_sequencer_block(block.clone())
            .expect("writing block to database should work");

        // grab block by block hash
        assert_eq!(
            state
                .get_sequencer_block_by_hash(block.block_hash().as_ref())
                .await
                .expect(
                    "a block was written to the database and we should be able to query its block \
                     by block hash"
                ),
            block,
            "stored block does not match expected"
        );
    }

    #[tokio::test]
    async fn get_rollup_data() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // write block
        let block = make_test_sequencer_block(2u32);
        state
            .put_sequencer_block(block.clone())
            .expect("writing block to database should work");

        // get written rollup id and data
        let rollup_id = block
            .rollup_transactions()
            .keys()
            .copied()
            .collect::<Vec<RollupId>>()[0];
        let rollup_data = block.rollup_transactions().get(&rollup_id).unwrap();

        // grab rollup's data by block hash
        let stored_rollup_data = state
            .get_rollup_data(block.block_hash().as_ref(), &rollup_id)
            .await
            .expect(
                "a block was written to the database and we should be able to query the data for \
                 a rollup",
            );
        assert_eq!(
            stored_rollup_data, *rollup_data,
            "stored rollup data does not match expected"
        );
    }

    #[tokio::test]
    async fn get_block_proofs_by_block_hash() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // write block
        let block = make_test_sequencer_block(2u32);
        state
            .put_sequencer_block(block.clone())
            .expect("writing block to database should work");

        // get written proofs
        let transactions_proof = block
            .clone()
            .into_parts()
            .rollup_transactions_proof
            .into_raw();
        let ids_proof = block.clone().into_parts().rollup_ids_proof.into_raw();

        // grab rollup's stored proofs
        let stored_proofs = state
            .get_block_proofs_by_block_hash(block.block_hash().as_ref())
            .await
            .expect(
                "a block was written to the database and we should be able to query its proof data",
            );
        assert_eq!(
            (transactions_proof, ids_proof),
            stored_proofs,
            "stored proofs do not match expected"
        );
    }
}
