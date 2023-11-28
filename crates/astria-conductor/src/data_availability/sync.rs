use std::time::Duration;

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
use tryhard::{
    backoff_strategies::ExponentialBackoff,
    OnRetry,
    RetryFutureConfig,
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

fn make_retry_config(
    attempts: u32,
) -> RetryFutureConfig<ExponentialBackoff, impl Copy + OnRetry<eyre::Report>> {
    RetryFutureConfig::new(attempts)
        .exponential_backoff(Duration::from_secs(5))
        .max_delay(Duration::from_secs(60))
        .on_retry(
            |attempt, next_delay: Option<Duration>, error: &eyre::Report| {
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                async move {
                    // let error = &error as &(dyn std::error::Error + 'static);
                    // let error = error.as_ref() as &(dyn std::error::Error + 'static);

                    // let error: &(dyn std::error::Error + 'static) = error;
                    // let error = error.as_dyn_error();
                    let error: &(dyn std::error::Error + 'static) = error.as_ref();
                    warn!(
                        attempt,
                        wait_duration,
                        error,
                        "attempt to get data from DA failed; retrying after backoff",
                    );
                }
            },
        )
}

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
        // future::FusedFuture as _,
        FutureExt as _,
        StreamExt as _,
    };

    info!("running DA sync");

    let mut height_stream = futures::stream::iter(start_sync_height..end_sync_height);
    let mut block_stream = FuturesOrdered::new();

    let retry_config = make_retry_config(1024);

    // let Some(height) = height_stream.next().await;
    // let mut retrieve_data_from_da = tryhard::retry_fn(|| async move {
    //     get_sequencer_data_from_da(
    //         Height::from(height),
    //         client.clone(),
    //         namespace,
    //         block_verifier.clone(),
    //     )
    //     .await
    // })
    // .with_config(retry_config)
    // .boxed()
    // .fuse();
    // info!("we should wait here until the celestia client is ready");
    // panic!("stopping for testing");

    'sync: loop {
        let client = client.clone();
        let block_verifier = block_verifier.clone();
        select!(
            Some(height) = height_stream.next(), if block_stream.len() <= 20 => {
                let height = Height::from(height);
                block_stream.push_back(async move {
                    tryhard::retry_fn(|| async {get_sequencer_data_from_da(height, client.clone(), namespace, block_verifier.clone()).await} )
                    .with_config(retry_config).await
                    // get_sequencer_data_from_da(height, client.clone(), namespace, block_verifier.clone()).await
                }.map(move |res| (height, res)).boxed());
            }

            Some((height, res)) = block_stream.next() => {
                match res {
                    Err(error) => {
                        let error = error.as_ref() as &(dyn std::error::Error + 'static);

                        warn!(da_block_height = %height.value(), error, "failed getting da block; rescheduling");
                        // warn!(da_block_height = %height.value(), error, "failed getting da block; skipping");

                        // block_stream.push_front(async move {
                        //     get_sequencer_data_from_da(height, client.clone(), namespace, block_verifier.clone()).await
                        // }.map(move |res| (height, res)).boxed());
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
