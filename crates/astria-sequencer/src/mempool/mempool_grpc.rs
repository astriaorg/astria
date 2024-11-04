use std::{
    net::SocketAddr,
    sync::Arc,
};

use astria_core::{
    generated::sequencerblock::v1::{
        mempool_info_service_server::{
            MempoolInfoService,
            MempoolInfoServiceServer,
        },
        AccountTransactions,
        DumpMempoolRequest,
        DumpMempoolResponse,
    },
    primitive::v1::{
        Address,
        TransactionId,
    },
    Protobuf,
};
use tokio::{
    sync::oneshot,
    task::JoinHandle,
};
use tonic::{
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

pub(crate) struct MempoolGrpcServer {
    mempool: Mempool,
}

impl MempoolGrpcServer {
    pub(crate) fn new(mempool: Mempool) -> Self {
        Self {
            mempool,
        }
    }
}

#[async_trait::async_trait]
impl MempoolInfoService for MempoolGrpcServer {
    async fn dump_mempool(
        self: Arc<Self>,
        _request: Request<DumpMempoolRequest>,
    ) -> Result<Response<DumpMempoolResponse>, Status> {
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

        Ok(Response::new(DumpMempoolResponse {
            pending,
            parked,
            comet_bft_removal_cache,
            contained_txs,
        }))
    }
}

#[instrument(skip_all)]
pub(crate) fn start_local_mempool_info_grpc_server(
    mempool: Mempool,
    grpc_addr: SocketAddr,
    shutdown_rx: oneshot::Receiver<()>,
) -> JoinHandle<Result<(), tonic::transport::Error>> {
    use futures::TryFutureExt as _;
    use penumbra_tower_trace::remote_addr;

    let mempool_info_api = MempoolGrpcServer::new(mempool);

    let grpc_server = tonic::transport::Server::builder()
        .trace_fn(|req| {
            if let Some(remote_addr) = remote_addr(req) {
                let addr = remote_addr.to_string();
                tracing::error_span!("mempool_grpc", addr)
            } else {
                tracing::error_span!("mempool_grpc")
            }
        })
        .accept_http1(true)
        .add_service(MempoolInfoServiceServer::new(mempool_info_api));

    info!(grpc_addr = grpc_addr.to_string(), "starting grpc server");
    tokio::task::spawn(
        grpc_server.serve_with_shutdown(grpc_addr, shutdown_rx.unwrap_or_else(|_| ())),
    )
}

#[cfg(all(test, feature = "client"))]
mod tests {
    use astria_core::{
        generated::sequencerblock::v1::{
            mempool_info_service_client::MempoolInfoServiceClient,
            DumpMempoolRequest,
            DumpMempoolResponse,
        },
        primitive::v1::{
            Address,
            TransactionId,
        },
        Protobuf as _,
    };
    use telemetry::Metrics as _;

    use crate::{
        app::{
            benchmark_and_test_utils::{
                mock_balances,
                mock_tx_cost,
            },
            test_utils::MockTxBuilder,
        },
        mempool::{
            mempool_grpc::{
                start_local_mempool_info_grpc_server,
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

        let grpc_addr = "127.0.0.1:8080";
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let mempool_grpc_handle = start_local_mempool_info_grpc_server(
            mempool.clone(),
            grpc_addr.parse().unwrap(),
            shutdown_rx,
        );

        // Wait for the server to start
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let mut client = MempoolInfoServiceClient::connect(format!("http://{grpc_addr}"))
            .await
            .unwrap();

        let request = DumpMempoolRequest {};
        let DumpMempoolResponse {
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
        mempool_grpc_handle.await.unwrap().unwrap();
    }
}
