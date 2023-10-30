use std::sync::Arc;

use ethers::{
    prelude::*,
    types::transaction::eip2718::TypedTransaction,
};
use optimism::{
    contract::{
        OptimismPortal,
        TransactionDepositedFilter,
    },
    deposit::convert_deposit_event_to_deposit_tx,
    DepositTransaction,
};

use super::{
    eyre,
    eyre::Result,
};

pub(crate) struct Handler {
    provider: Arc<Provider<Ws>>,
    optimism_portal_contract: OptimismPortal<Provider<Ws>>,
    from_block_height: u64,
    to_block_height: u64,
}

impl Handler {
    pub(crate) async fn new(
        ethereum_provider: Provider<Ws>,
        optimism_portal_contract_address: Address,
        initial_ethereum_l1_block_height: u64,
    ) -> Self {
        let provider = Arc::new(ethereum_provider);
        let optimism_portal_contract = optimism::contract::get_optimism_portal_read_only(
            provider.clone(),
            optimism_portal_contract_address,
        );

        Self {
            provider,
            optimism_portal_contract,
            from_block_height: initial_ethereum_l1_block_height,
            to_block_height: initial_ethereum_l1_block_height,
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
        self.to_block_height = to_block.as_u64();
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

        Ok([deposit_txs, sequenced_transactions].concat())
    }
}
