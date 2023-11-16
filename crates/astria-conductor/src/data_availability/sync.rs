use celestia_client::{
    celestia_types::{
        nmt::Namespace,
        Height,
    },
    jsonrpsee::http_client::HttpClient,
    CelestiaClientExt as _,
};
use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use futures::stream::FuturesOrdered;
use tokio::select;
use tracing::{
    info,
    instrument,
    warn,
};

use crate::{
    block_verifier::BlockVerifier,
    data_availability::{
        send_sequencer_subsets,
        verify_sequencer_blobs_and_assemble_rollups,
        SequencerBlockSubset,
    },
    executor,
};

#[instrument(name = "sync DA", skip_all)]
pub(crate) async fn run(
    start_sync_height: u32,
    _end_sync_height: u32,
    namespace: Namespace,
    executor_tx: executor::Sender,
    client: HttpClient,
    block_verifier: BlockVerifier,
) -> eyre::Result<()> {
    use futures::{
        FutureExt as _,
        StreamExt as _,
    };

    info!("starting DA sync");

    let mut height_stream = futures::stream::iter(start_sync_height..);
    let mut block_stream = FuturesOrdered::new();

    'sync: loop {
        let client = client.clone();
        let block_verifier = block_verifier.clone();
        select!(
            Some(height) = height_stream.next(), if block_stream.len() <= 20 => {
                let height = Height::from(height);
                block_stream.push_back(async move {
                    get_sequencer_data_from_da(height, client.clone(), namespace, block_verifier.clone()).await
                }.map(move |res| (height, res)).boxed());
            }

            Some((height, res)) = block_stream.next() => {
                match res {
                    Err(error) => {
                        let error = error.as_ref() as &(dyn std::error::Error + 'static);

                        warn!(da_block_height = %height.value(), error, "failed getting da block; rescheduling");
                        // warn!(da_block_height = %height.value(), error, "failed getting da block; skipping");

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
        );
    }
}

pub(crate) async fn get_sequencer_data_from_da(
    height: Height,
    celestia_client: HttpClient,
    sequencer_namespace: Namespace,
    block_verifier: BlockVerifier,
) -> eyre::Result<Vec<SequencerBlockSubset>> {
    let res = celestia_client
        .get_sequencer_data(height, sequencer_namespace)
        .await
        .wrap_err("failed to fetch sequencer data from celestia")
        .map(|rsp| rsp.datas);

    let seq_block_data = match res {
        Ok(datas) => {
            verify_sequencer_blobs_and_assemble_rollups(
                height,
                datas,
                celestia_client,
                block_verifier.clone(),
                sequencer_namespace,
            )
            .await
        }
        Err(e) => {
            let error: &(dyn std::error::Error + 'static) = e.as_ref();
            warn!(
                da_block_height = %height.value(),
                error,
                "task querying celestia for sequencer data returned with an error"
            );
            Err(e)
        }
    };
    seq_block_data
}
