use astria_core::{
    primitive::v1::RollupId,
    sequencerblock::v1::block::{
        RollupTransactions,
        SequencerBlock,
        SequencerBlockHeader,
        SequencerBlockParts,
    },
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        bail,
        Result,
        WrapErr as _,
    },
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::instrument;

use super::storage::{
    self,
    keys,
};
use crate::storage::StoredValue;

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    async fn get_block_hash_by_height(&self, height: u64) -> Result<[u8; 32]> {
        let Some(bytes) = self
            .nonverifiable_get_raw(keys::block_hash_by_height(height).as_bytes())
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to read block hash by height from state")?
        else {
            bail!("block hash not found for given height");
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::BlockHash::try_from(value).map(<[u8; 32]>::from))
            .wrap_err("invalid block hash bytes")
    }

    #[instrument(skip_all)]
    async fn get_sequencer_block_header_by_hash(
        &self,
        hash: &[u8; 32],
    ) -> Result<SequencerBlockHeader> {
        let Some(bytes) = self
            .nonverifiable_get_raw(keys::sequencer_block_header_by_hash(hash).as_bytes())
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to read raw sequencer block from state")?
        else {
            bail!("header not found for given block hash");
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::SequencerBlockHeader::try_from(value).map(SequencerBlockHeader::from)
            })
            .wrap_err("invalid sequencer block header bytes")
    }

    #[instrument(skip_all)]
    async fn get_rollup_ids_by_block_hash(&self, hash: &[u8; 32]) -> Result<Vec<RollupId>> {
        let Some(bytes) = self
            .nonverifiable_get_raw(keys::rollup_ids_by_hash(hash).as_bytes())
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to read rollup IDs by block hash from state")?
        else {
            bail!("rollup IDs not found for given block hash");
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::RollupIds::try_from(value).map(Vec::<RollupId>::from))
            .wrap_err("invalid rollup ids bytes")
    }

    #[instrument(skip_all)]
    async fn get_sequencer_block_by_height(&self, height: u64) -> Result<SequencerBlock> {
        let hash = self
            .get_block_hash_by_height(height)
            .await
            .wrap_err("failed to get block hash by height")?;
        get_sequencer_block_by_hash(self, &hash)
            .await
            .wrap_err("failed to get sequencer block by hash")
    }

    #[instrument(skip_all)]
    async fn get_rollup_data(
        &self,
        hash: &[u8; 32],
        rollup_id: &RollupId,
    ) -> Result<RollupTransactions> {
        let Some(bytes) = self
            .nonverifiable_get_raw(
                keys::rollup_data_by_hash_and_rollup_id(hash, rollup_id).as_bytes(),
            )
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err(
                "failed to read rollup transactions by block hash and rollup ID from state",
            )?
        else {
            bail!("rollup transactions not found for given block hash and rollup ID");
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::RollupTransactions::try_from(value).map(RollupTransactions::from)
            })
            .wrap_err("invalid rollup transactions bytes")
    }

    #[instrument(skip_all)]
    async fn get_rollup_transactions_proof_by_block_hash(
        &self,
        hash: &[u8; 32],
    ) -> Result<merkle::Proof> {
        let Some(bytes) = self
            .nonverifiable_get_raw(keys::rollup_transactions_proof_by_hash(hash).as_bytes())
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to read rollup transactions proof by block hash from state")?
        else {
            bail!("rollup transactions proof not found for given block hash");
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Proof::try_from(value).map(merkle::Proof::from))
            .wrap_err("invalid rollup transactions proof bytes")
    }

    #[instrument(skip_all)]
    async fn get_rollup_ids_proof_by_block_hash(&self, hash: &[u8; 32]) -> Result<merkle::Proof> {
        let Some(bytes) = self
            .nonverifiable_get_raw(keys::rollup_ids_proof_by_hash(hash).as_bytes())
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to read rollup IDs proof by block hash from state")?
        else {
            bail!("rollup IDs proof not found for given block hash");
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Proof::try_from(value).map(merkle::Proof::from))
            .wrap_err("invalid rollup IDs proof bytes")
    }
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_sequencer_block(&mut self, block: SequencerBlock) -> Result<()> {
        // write the sequencer block to state in the following order:
        // 1. height to block hash
        // 2. block hash to rollup IDs
        // 3. block hash to block header
        // 4. for each rollup ID in the block, map block hash + rollup ID to rollup data
        // 5. block hash to rollup transactions proof
        // 6. block hash to rollup IDs proof

        let SequencerBlockParts {
            block_hash,
            header,
            rollup_transactions,
            rollup_transactions_proof,
            rollup_ids_proof,
        } = block.into_parts();

        put_block_hash(self, header.height(), block_hash)?;
        put_rollup_ids(self, &block_hash, rollup_transactions.keys().copied())?;
        put_block_header(self, &block_hash, header)?;
        put_rollups_transactions(self, &block_hash, rollup_transactions.into_iter())?;
        put_rollups_transactions_proof(self, &block_hash, rollup_transactions_proof)?;
        put_rollup_ids_proof(self, &block_hash, rollup_ids_proof)
    }
}

#[instrument(skip_all)]
async fn get_sequencer_block_by_hash<S: StateRead + ?Sized>(
    state: &S,
    hash: &[u8; 32],
) -> Result<SequencerBlock> {
    let header = state
        .get_sequencer_block_header_by_hash(hash)
        .await
        .wrap_err("failed to get sequencer block header by hash")?;
    let rollup_ids = state
        .get_rollup_ids_by_block_hash(hash)
        .await
        .wrap_err("failed to get rollup ids by block hash")?;
    let rollup_transactions_proof = state
        .get_rollup_transactions_proof_by_block_hash(hash)
        .await
        .wrap_err("failed to get rollup transactions proof by block hash")?;
    let rollup_ids_proof = state
        .get_rollup_ids_proof_by_block_hash(hash)
        .await
        .wrap_err("failed to get rollup ids proof by block hash")?;

    #[expect(
        clippy::default_trait_access,
        reason = "want to avoid explicitly importing `index_map` crate to sequencer crate"
    )]
    let mut parts = SequencerBlockParts {
        block_hash: *hash,
        header,
        rollup_transactions: Default::default(),
        rollup_transactions_proof,
        rollup_ids_proof,
    };

    for rollup_id in rollup_ids {
        let rollup_txs = state
            .get_rollup_data(hash, &rollup_id)
            .await
            .wrap_err("failed to get rollup data")?;
        let _ = parts.rollup_transactions.insert(rollup_id, rollup_txs);
    }

    Ok(SequencerBlock::unchecked_from_parts(parts))
}

fn put_block_hash<S: StateWrite + ?Sized>(
    state: &mut S,
    block_height: tendermint::block::Height,
    block_hash: [u8; 32],
) -> Result<()> {
    let bytes = StoredValue::from(storage::BlockHash::from(&block_hash))
        .serialize()
        .context("failed to serialize block hash")?;
    state.nonverifiable_put_raw(
        keys::block_hash_by_height(block_height.into()).into(),
        bytes,
    );
    Ok(())
}

fn put_rollup_ids<S: StateWrite + ?Sized, I: Iterator<Item = RollupId>>(
    state: &mut S,
    block_hash: &[u8; 32],
    rollup_ids: I,
) -> Result<()> {
    let rollup_ids: Vec<_> = rollup_ids.collect();
    let bytes = StoredValue::from(storage::RollupIds::from(rollup_ids.iter()))
        .serialize()
        .context("failed to serialize rollup ids")?;
    state.nonverifiable_put_raw(keys::rollup_ids_by_hash(block_hash).into(), bytes);
    Ok(())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "`block_header` will be consumed in upcoming PR"
)]
fn put_block_header<S: StateWrite + ?Sized>(
    state: &mut S,
    block_hash: &[u8; 32],
    block_header: SequencerBlockHeader,
) -> Result<()> {
    let bytes = StoredValue::from(storage::SequencerBlockHeader::from(&block_header))
        .serialize()
        .context("failed to serialize sequencer block header")?;
    state.nonverifiable_put_raw(
        keys::sequencer_block_header_by_hash(block_hash).into(),
        bytes,
    );
    Ok(())
}

fn put_rollups_transactions<S, I>(
    state: &mut S,
    block_hash: &[u8; 32],
    all_rollups_txs: I,
) -> Result<()>
where
    S: StateWrite + ?Sized,
    I: Iterator<Item = (RollupId, RollupTransactions)>,
{
    let all_rollups_txs: Vec<_> = all_rollups_txs.collect();
    all_rollups_txs.iter().try_for_each(|(id, rollup_txs)| {
        let bytes = StoredValue::from(storage::RollupTransactions::from(rollup_txs))
            .serialize()
            .context("failed to serialize rollup transactions")?;
        state.nonverifiable_put_raw(
            keys::rollup_data_by_hash_and_rollup_id(block_hash, id).into(),
            bytes,
        );
        Ok(())
    })
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "`proof` will be consumed in upcoming PR"
)]
fn put_rollups_transactions_proof<S: StateWrite + ?Sized>(
    state: &mut S,
    block_hash: &[u8; 32],
    proof: merkle::Proof,
) -> Result<()> {
    let bytes = StoredValue::from(storage::Proof::from(&proof))
        .serialize()
        .context("failed to serialize rollups transactions proof")?;
    state.nonverifiable_put_raw(
        keys::rollup_transactions_proof_by_hash(block_hash).into(),
        bytes,
    );
    Ok(())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "`proof` will be consumed in upcoming PR"
)]
fn put_rollup_ids_proof<S: StateWrite + ?Sized>(
    state: &mut S,
    block_hash: &[u8; 32],
    proof: merkle::Proof,
) -> Result<()> {
    let bytes = StoredValue::from(storage::Proof::from(&proof))
        .serialize()
        .context("failed to serialize rollup ids proof")?;
    state.nonverifiable_put_raw(keys::rollup_ids_proof_by_hash(block_hash).into(), bytes);
    Ok(())
}

impl<T: StateWrite> StateWriteExt for T {}

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::TransactionId,
        protocol::test_utils::ConfigureSequencerBlock,
        sequencerblock::v1::block::Deposit,
    };
    use rand::Rng;

    use super::*;
    use crate::{
        benchmark_and_test_utils::astria_address,
        storage::Storage,
    };

    // creates new sequencer block, optionally shifting all values except the height by 1
    fn make_test_sequencer_block(height: u32) -> SequencerBlock {
        let mut rng = rand::thread_rng();
        let block_hash: [u8; 32] = rng.gen();

        // create inner rollup id/tx data
        let mut deposits = vec![];
        for _ in 0..2 {
            let rollup_id = RollupId::new(rng.gen());
            let bridge_address = astria_address(&[rng.gen(); 20]);
            let amount = rng.gen::<u128>();
            let asset = "testasset".parse().unwrap();
            let destination_chain_address = rng.gen::<u8>().to_string();
            let deposit = Deposit {
                bridge_address,
                rollup_id,
                amount,
                asset,
                destination_chain_address,
                source_transaction_id: TransactionId::new([0; 32]),
                source_action_index: 9,
            };
            deposits.push(deposit);
        }

        ConfigureSequencerBlock {
            block_hash: Some(block_hash),
            height,
            deposits,
            ..Default::default()
        }
        .make()
    }

    #[tokio::test]
    async fn put_sequencer_block() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // can write one
        let block_0 = make_test_sequencer_block(2u32);
        state_delta
            .put_sequencer_block(block_0.clone())
            .expect("writing block to database should work");

        assert_eq!(
            state_delta
                .get_sequencer_block_by_height(block_0.height().into())
                .await
                .expect("a block was written to the database and should exist"),
            block_0,
            "stored block does not match expected"
        );

        // can write another and both are ok
        let block_1 = make_test_sequencer_block(3u32);
        state_delta
            .put_sequencer_block(block_1.clone())
            .expect("writing another block to database should work");
        assert_eq!(
            state_delta
                .get_sequencer_block_by_height(block_0.height().into())
                .await
                .expect("a block was written to the database and should exist"),
            block_0,
            "original stored block does not match expected"
        );
        assert_eq!(
            state_delta
                .get_sequencer_block_by_height(block_1.height().into())
                .await
                .expect("a block was written to the database and should exist"),
            block_1,
            "additionally stored block does not match expected"
        );
    }

    #[tokio::test]
    async fn put_sequencer_block_update() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // write original block
        let mut block = make_test_sequencer_block(2u32);
        state_delta
            .put_sequencer_block(block.clone())
            .expect("writing block to database should work");
        assert_eq!(
            state_delta
                .get_sequencer_block_by_height(block.height().into())
                .await
                .expect("a block was written to the database and should exist"),
            block,
            "stored block does not match expected"
        );

        // write to same height but with new values
        block = make_test_sequencer_block(2u32);
        state_delta
            .put_sequencer_block(block.clone())
            .expect("writing block update to database should work");

        // block was updates
        assert_eq!(
            state_delta
                .get_sequencer_block_by_height(block.height().into())
                .await
                .expect("a block was written to the database and should exist"),
            block,
            "updated stored block does not match expected"
        );
    }

    #[tokio::test]
    async fn get_block_hash_by_height() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // write block
        let block = make_test_sequencer_block(2u32);
        state_delta
            .put_sequencer_block(block.clone())
            .expect("writing block to database should work");

        // grab block hash by block height
        assert_eq!(
            state_delta
                .get_block_hash_by_height(block.height().into())
                .await
                .expect(
                    "a block was written to the database and we should be able to query its block \
                     hash by height"
                ),
            *block.block_hash(),
            "stored block hash does not match expected"
        );
    }

    #[tokio::test]
    async fn get_sequencer_block_header_by_hash() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // write block
        let block = make_test_sequencer_block(2u32);
        state_delta
            .put_sequencer_block(block.clone())
            .expect("writing block to database should work");

        // grab block header by block hash
        assert_eq!(
            state_delta
                .get_sequencer_block_header_by_hash(block.block_hash())
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
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // write block
        let block = make_test_sequencer_block(2u32);
        state_delta
            .put_sequencer_block(block.clone())
            .expect("writing block to database should work");

        // grab rollup ids by block hash
        let stored_rollup_ids = state_delta
            .get_rollup_ids_by_block_hash(block.block_hash())
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
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // write block
        let block = make_test_sequencer_block(2u32);
        state_delta
            .put_sequencer_block(block.clone())
            .expect("writing block to database should work");

        // grab block by block hash
        assert_eq!(
            super::get_sequencer_block_by_hash(&state_delta, block.block_hash())
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
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        // write block
        let block = make_test_sequencer_block(2u32);
        state_delta
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
        let stored_rollup_data = state_delta
            .get_rollup_data(block.block_hash(), &rollup_id)
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
    async fn get_rollup_transactions_proof_by_block_hash() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let block = make_test_sequencer_block(2u32);
        state_delta
            .put_sequencer_block(block.clone())
            .expect("writing block to database should work");

        let transactions_proof = state_delta
            .get_rollup_transactions_proof_by_block_hash(block.block_hash())
            .await
            .expect("should have txs proof in state");
        assert_eq!(*block.rollup_transactions_proof(), transactions_proof);
    }

    #[tokio::test]
    async fn get_rollup_ids_proof_by_block_hash() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let block = make_test_sequencer_block(2u32);
        state_delta
            .put_sequencer_block(block.clone())
            .expect("writing block to database should work");

        let ids_proof = state_delta
            .get_rollup_ids_proof_by_block_hash(block.block_hash())
            .await
            .expect("should have ids proof in state");
        assert_eq!(*block.rollup_ids_proof(), ids_proof);
    }
}
