// use std::time::Duration;

use celestia_client::{
    // celestia_rpc::client,
    // celestia_rpc::client,
    // celestia_tendermint::block,
    celestia_types::{
        nmt::Namespace,
        Height,
    },
    jsonrpsee::http_client::HttpClient,
    CelestiaClientExt as _,
    // SequencerNamespaceData,
    SEQUENCER_NAMESPACE,
};
use color_eyre::eyre::{
    self,
    // Error,
    WrapErr as _,
};
use futures::stream::FuturesOrdered;
use tokio::select;
// use tokio_util::task::JoinMap;
use tracing::{
    // debug,
    // error,
    info,
    instrument,
    warn,
    // Instrument,
};

use crate::{
    block_verifier::BlockVerifier,
    data_availability::{
        send_sequencer_subsets,
        verify_sequencer_blobs_and_assemble_rollups,
    },
    executor,
    types::SequencerBlockSubset,
};

#[instrument(name = "sync DA", skip_all)]
pub(crate) async fn run(
    start_sync_height: u32,
    end_sync_height: u32,
    namespace: Namespace,
    executor_tx: executor::Sender,
    client: HttpClient,
    block_verifier: BlockVerifier,
) -> eyre::Result<()> {
    use futures::{
        FutureExt as _,
        StreamExt as _,
    };

    // let client = self.celestia_client.clone();
    // let namespace = self.namespace;
    // let block_verifier = self.block_verifier.clone();
    // let executor_tx = self.executor_tx.clone();

    let mut height_stream = futures::stream::iter(start_sync_height..end_sync_height);
    let mut block_stream = FuturesOrdered::new();

    'sync: loop {
        let client = client.clone();
        let block_verifier = block_verifier.clone();
        select!(
            Some(height) = height_stream.next(), if block_stream.len() <= 20 => {
                block_stream.push_back(async move {
                    get_sequencer_data_from_da(height, client.clone(), namespace, block_verifier.clone()).await
                }.map(move |res| (height, res)).boxed());
            }

            Some((height, res)) = block_stream.next() => {
                match res {
                    Err(error) => {
                        let error = error.as_ref() as &(dyn std::error::Error + 'static);

                        warn!(height, error, "failed getting da block; rescheduling");

                        block_stream.push_front(async move {
                            get_sequencer_data_from_da(height, client.clone(), namespace, block_verifier.clone()).await
                        }.map(move |res| (height, res)).boxed());
                    }

                    Ok(blocks) => {
                        let span = tracing::info_span!("send_sequencer_subsets", %height);
                        span.in_scope(|| send_sequencer_subsets(executor_tx.clone(), Ok(Ok(blocks))))
                            .wrap_err("failed sending sequencer subsets to executor")?;

                    }
                }
            }

            else => {
                info!("DA sync finished");
                break 'sync Ok(())
            }
        )
    }
}

async fn get_sequencer_data_from_da(
    height: u32,
    celestia_client: HttpClient,
    namespace: Namespace,
    block_verifier: BlockVerifier,
) -> eyre::Result<Vec<SequencerBlockSubset>> {
    // let celestia_client = client;
    // let namespace = self.namespace;

    let res = celestia_client
        .get_sequencer_data(height, SEQUENCER_NAMESPACE)
        .await
        .wrap_err("failed to fetch sequencer data from celestia")
        .map(|rsp| rsp.datas);

    let seq_block_data = match res {
        Ok(datas) => {
            verify_sequencer_blobs_and_assemble_rollups(
                Height::from(height),
                datas,
                celestia_client,
                block_verifier.clone(),
                namespace,
            )
            .await
        }
        Err(e) => {
            let error: &(dyn std::error::Error + 'static) = e.as_ref();
            warn!(
                error,
                "task querying celestia for sequencer data returned with an error"
            );
            Err(e)
        }
    };
    seq_block_data
}
