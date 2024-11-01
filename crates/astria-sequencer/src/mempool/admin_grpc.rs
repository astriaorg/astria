use std::{
    net::SocketAddr,
    sync::Arc,
};

use astria_core::{
    generated::sequencerblock::v1::{
        admin_mempool_service_server::{
            AdminMempoolService,
            AdminMempoolServiceServer,
        },
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
use tokio::{
    sync::oneshot,
    task::JoinHandle,
};
use tonic::{
    Code,
    Request,
    Response,
    Status,
};
use tracing::{
    info,
    instrument,
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

        let mempool_read_parked = self.mempool.parked.read().await;
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

#[instrument(skip_all)]
pub(crate) fn start_local_admin_grpc_server(
    storage: &cnidarium::Storage,
    mempool: Mempool,
    shutdown_rx: oneshot::Receiver<()>,
) -> JoinHandle<Result<(), tonic::transport::Error>> {
    use futures::TryFutureExt as _;
    use penumbra_tower_trace::remote_addr;

    let grpc_addr = "127.0.0.1:8080"
        .parse::<SocketAddr>()
        .expect("local host should parse to socket address");
    let admin_api = AdminGrpcServer::new(storage.clone(), mempool);

    let grpc_server = tonic::transport::Server::builder()
        .trace_fn(|req| {
            if let Some(remote_addr) = remote_addr(req) {
                let addr = remote_addr.to_string();
                tracing::error_span!("admin_grpc", addr)
            } else {
                tracing::error_span!("admin_grpc")
            }
        })
        .accept_http1(true)
        .add_service(AdminMempoolServiceServer::new(admin_api));

    info!(grpc_addr = grpc_addr.to_string(), "starting grpc server");
    tokio::task::spawn(
        grpc_server.serve_with_shutdown(grpc_addr, shutdown_rx.unwrap_or_else(|_| ())),
    )
}

#[cfg(all(test, feature = "client"))]
mod tests {
    use astria_core::{
        generated::sequencerblock::v1::{
            admin_mempool_service_client::AdminMempoolServiceClient,
            DumpMempoolRequest,
            Mempool as MempoolDump,
        },
        primitive::v1::{
            Address,
            TransactionId,
        },
        Protobuf as _,
    };
    use cnidarium::StateDelta;
    use telemetry::Metrics as _;

    use crate::{
        app::{
            benchmark_and_test_utils::{
                mock_balances,
                mock_tx_cost,
            },
            test_utils::MockTxBuilder,
        },
        authority::StateWriteExt as _,
        benchmark_and_test_utils::astria_address,
        mempool::{
            admin_grpc::{
                start_local_admin_grpc_server,
                TransactionsForAccount as _,
            },
            transactions_container::TransactionsContainer as _,
            Mempool,
        },
        Metrics,
    };

    // Run with `cargo test -p astria-sequencer --features client`
    #[tokio::test]
    async fn test_admin_grpc_service() {
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100);
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);
        let sudo_address = astria_address(&[1; 20]);
        state.put_sudo_address(sudo_address).unwrap();
        storage.commit(state).await.unwrap();

        let initial_balances = mock_balances(1, 0);
        let tx_cost = mock_tx_cost(1, 0, 0);
        let tx1 = MockTxBuilder::new().nonce(1).build();
        let tx2 = MockTxBuilder::new().nonce(2).build();
        let tx3 = MockTxBuilder::new().nonce(3).build();
        let tx4 = MockTxBuilder::new().nonce(4).build();

        mempool
            .insert(tx1.clone(), 1, initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx2.clone(), 1, initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx3.clone(), 1, initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();
        mempool
            .insert(tx4.clone(), 1, initial_balances.clone(), tx_cost.clone())
            .await
            .unwrap();

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let admin_grpc_handle =
            start_local_admin_grpc_server(&storage, mempool.clone(), shutdown_rx);

        // Wait for the server to start
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let mut client = AdminMempoolServiceClient::connect("http://127.0.0.1:8080")
            .await
            .unwrap();

        let request = DumpMempoolRequest {
            sudo_address: Some(sudo_address.into_raw()),
        };
        let MempoolDump {
            pending,
            parked,
            comet_bft_removal_cache,
            contained_txs,
        } = client.dump_mempool(request).await.unwrap().into_inner();

        pending
            .iter()
            .zip(mempool.pending.read().await.txs().iter())
            .for_each(|(a, b)| {
                assert_eq!(
                    a.account.clone().unwrap(),
                    Address::unchecked_from_parts(*b.0, "astria").into_raw()
                );
                a.transactions
                    .iter()
                    .zip(b.1.txs().iter())
                    .for_each(|(l, r)| {
                        assert_eq!(l, &r.1.signed_tx().to_raw());
                    });
            });
        parked
            .iter()
            .zip(mempool.parked.read().await.txs().iter())
            .for_each(|(a, b)| {
                assert_eq!(
                    a.account.clone().unwrap(),
                    Address::unchecked_from_parts(*b.0, "astria").into_raw()
                );
                a.transactions
                    .iter()
                    .zip(b.1.txs().iter())
                    .for_each(|(l, r)| {
                        assert_eq!(l, &r.1.signed_tx().to_raw());
                    });
            });
        comet_bft_removal_cache
            .iter()
            .zip(mempool.comet_bft_removal_cache.read().await.cache.keys())
            .for_each(|(a, b)| {
                assert_eq!(TransactionId::try_from_raw_ref(a).unwrap().get(), *b);
            });
        contained_txs
            .iter()
            .zip(mempool.contained_txs.read().await.iter())
            .for_each(|(a, b)| {
                assert_eq!(TransactionId::try_from_raw_ref(a).unwrap().get(), *b);
            });

        shutdown_tx.send(()).unwrap();
        admin_grpc_handle.await.unwrap().unwrap();
    }
}
