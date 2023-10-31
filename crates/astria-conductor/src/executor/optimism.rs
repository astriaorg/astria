use std::sync::Arc;

use astria_optimism::{
    contract::{
        OptimismPortal,
        TransactionDepositedFilter,
    },
    deposit::convert_deposit_event_to_deposit_tx,
    DepositTransaction,
};
use ethers::{
    prelude::*,
    types::transaction::eip2718::TypedTransaction,
};
use tracing::debug;

use super::{
    eyre,
    eyre::Result,
};

pub(crate) struct Handler {
    provider: Arc<Provider<Ws>>,
    optimism_portal_contract: OptimismPortal<Provider<Ws>>,
    from_block_height: u64,
}

impl Handler {
    pub(crate) async fn new(
        ethereum_provider: Arc<Provider<Ws>>,
        optimism_portal_contract_address: Address,
        initial_ethereum_l1_block_height: u64,
    ) -> Self {
        let optimism_portal_contract = astria_optimism::contract::get_optimism_portal_read_only(
            ethereum_provider.clone(),
            optimism_portal_contract_address,
        );

        Self {
            provider: ethereum_provider,
            optimism_portal_contract,
            from_block_height: initial_ethereum_l1_block_height,
        }
    }

    async fn query_events(&mut self) -> Result<Vec<(TransactionDepositedFilter, LogMeta)>> {
        let to_block = self.provider.get_block_number().await?;
        let event_filter = self
            .optimism_portal_contract
            .event::<TransactionDepositedFilter>()
            .from_block(self.from_block_height)
            .to_block(to_block);

        let events = event_filter
            .query_with_meta()
            .await
            .map_err(|e| eyre::eyre!(e))?;

        // event filter `from` and `to` blocks are inclusive (ie. we read events from those blocks),
        // so we set the next block height to read from as the highest we read from + 1.
        self.from_block_height = to_block.as_u64() + 1;
        Ok(events)
    }
}

#[async_trait::async_trait]
impl crate::executor::PreExecutionHook for Handler {
    async fn populate_rollup_transactions(
        &mut self,
        sequenced_transactions: Vec<Vec<u8>>,
    ) -> Result<Vec<Vec<u8>>> {
        let deposit_events = self.query_events().await?;
        let deposit_txs = convert_deposit_events_to_encoded_txs(deposit_events)?;

        debug!(
            num_deposit_txs = deposit_txs.len(),
            num_sequenced_txs = sequenced_transactions.len(),
            "populated rollup transactions"
        );

        Ok([deposit_txs, sequenced_transactions].concat())
    }
}

pub(crate) fn convert_deposit_events_to_encoded_txs(
    deposit_events: Vec<(TransactionDepositedFilter, LogMeta)>,
) -> Result<Vec<Vec<u8>>> {
    let deposit_txs = deposit_events
        .into_iter()
        .map(|(event, meta)| {
            convert_deposit_event_to_deposit_tx(event, meta.block_hash, meta.log_index)
        })
        .collect::<Result<Vec<DepositTransaction>>>()?;

    let deposit_txs = deposit_txs
        .into_iter()
        .map(|tx| TypedTransaction::DepositTransaction(tx).rlp().to_vec())
        .collect::<Vec<Vec<u8>>>();
    Ok(deposit_txs)
}
