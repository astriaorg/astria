use std::sync::Arc;

use astria_core::{
    generated::sequencerblock::v1::{
        admin_mempool_service_server::AdminMempoolService,
        AccountTransactions,
        DumpMempoolRequest,
        Mempool as MempoolDump,
    },
    primitive::v1::{
        Address,
        TransactionId,
    },
    Protobuf,
};
use cnidarium::Storage;
use tonic::{
    Code,
    Request,
    Response,
    Status,
};

use super::{
    transactions_container::{
        TransactionsContainer as _,
        TransactionsForAccount,
    },
    Mempool,
};
use crate::authority::StateReadExt;

pub(crate) struct AdminGrpcServer {
    storage: Storage,
    mempool: Mempool,
}

impl AdminGrpcServer {
    pub(crate) fn new(storage: Storage, mempool: Mempool) -> Self {
        Self {
            storage,
            mempool,
        }
    }
}

#[async_trait::async_trait]
impl AdminMempoolService for AdminGrpcServer {
    async fn dump_mempool(
        self: Arc<Self>,
        request: Request<DumpMempoolRequest>,
    ) -> Result<Response<MempoolDump>, Status> {
        let state_sudo_address = self
            .storage
            .latest_snapshot()
            .get_sudo_address()
            .await
            .map_err(|err| {
                Status::new(
                    Code::Internal,
                    format!("failed to get sudo address from state: {err:?}"),
                )
            })?;
        let req_sudo_address =
            Address::try_from_raw(&request.get_ref().sudo_address.clone().ok_or(Status::new(
                Code::InvalidArgument,
                "request sudo address is required",
            ))?)
            .map_err(|err| {
                Status::new(
                    Code::Internal,
                    format!("failed to convert sudo address to domain type: {err:?}"),
                )
            })?;
        if state_sudo_address != *req_sudo_address.as_bytes() {
            return Err(Status::new(
                Code::PermissionDenied,
                "sudo address does not match the one in the state",
            ));
        }

        let mempool_read_pending = self.mempool.pending.read().await;
        let pending = mempool_read_pending
            .txs()
            .iter()
            .map(|(address, txs)| AccountTransactions {
                account: Some(Address::unchecked_from_parts(*address, "astria").into_raw()),
                transactions: txs
                    .txs()
                    .iter()
                    .map(|(_, tx)| tx.signed_tx().to_raw())
                    .collect(),
            })
            .collect();

        let mempool_read_parked = self.mempool.pending.read().await;
        let parked = mempool_read_parked
            .txs()
            .iter()
            .map(|(address, txs)| AccountTransactions {
                account: Some(Address::unchecked_from_parts(*address, "astria").into_raw()),
                transactions: txs
                    .txs()
                    .iter()
                    .map(|(_, tx)| tx.signed_tx().to_raw())
                    .collect(),
            })
            .collect();

        let comet_bft_removal_cache = self
            .mempool
            .comet_bft_removal_cache
            .read()
            .await
            .cache
            .keys()
            .map(|slice| TransactionId::new(*slice).into_raw())
            .collect();
        let contained_txs = self
            .mempool
            .contained_txs
            .read()
            .await
            .iter()
            .map(|slice| TransactionId::new(*slice).into_raw())
            .collect();

        Ok(Response::new(MempoolDump {
            pending,
            parked,
            comet_bft_removal_cache,
            contained_txs,
        }))
    }
}
