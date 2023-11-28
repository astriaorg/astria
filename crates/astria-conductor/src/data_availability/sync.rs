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
    Report,
    WrapErr as _,
};
use futures::stream::FuturesOrdered;
use tokio::select;
use tracing::{
    error,
    info,
    instrument,
    warn,
};
use tryhard::RetryFutureConfig;

use crate::{
    block_verifier::BlockVerifier,
    data_availability::{
        send_sequencer_subsets,
        verify_sequencer_blobs_and_assemble_rollups,
        SequencerNamespaceData,
    },
    executor,
};

async fn get_new_sequencer_block_data_with_retry(
    height: Height,
    celestia_client: HttpClient,
    sequencer_namespace: Namespace,
) -> Result<Vec<SequencerNamespaceData>, Report> {
    let retry_config = RetryFutureConfig::new(50)
        .exponential_backoff(Duration::from_secs(5))
        .max_delay(Duration::from_secs(60))
        .on_retry(|attempt, next_delay: Option<Duration>, error: &Report| {
            let wait_duration = next_delay
                .map(humantime::format_duration)
                .map(tracing::field::display);
            let error: &(dyn std::error::Error + 'static) = error.as_ref();
            let error = error.to_string();
            async move {
                warn!(
                    attempt,
                    wait_duration,
                    error,
                    "attempt to get data from DA failed; retrying after backoff",
                );
            }
        });

    tryhard::retry_fn(|| async {
        celestia_client
            .get_sequencer_data(height, sequencer_namespace)
            .await
            .wrap_err("failed to fetch sequencer data from celestia")
            .map(|rsp| rsp.datas)
    })
    .with_config(retry_config)
    .await
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
        FutureExt as _,
        StreamExt as _,
    };

    info!("running DA sync");

    let mut height_stream = futures::stream::iter(start_sync_height..end_sync_height);
    let mut block_stream = FuturesOrdered::new();
    let mut verification_stream = FuturesOrdered::new();

    'sync: loop {
        let client = client.clone();
        let block_verifier = block_verifier.clone();
        select!(
            Some(height) = height_stream.next(), if block_stream.len() <= 20 => {
                let height = Height::from(height);
                block_stream.push_back(async move {
                    get_new_sequencer_block_data_with_retry(height, client.clone(), namespace).await
                }.map(move |res| (height, res)).boxed());
            }

            Some((height, res)) = block_stream.next() => {
                match res {
                    Err(error) => {
                        let error = error.as_ref() as &(dyn std::error::Error + 'static);

                        error!(da_block_height = %height.value(), error, "failed getting da block");
                    }

                    Ok(datas) => {
                        verification_stream.push_back(async move {
                            verify_sequencer_blobs_and_assemble_rollups(
                                height,
                                datas,
                                client.clone(),
                                block_verifier.clone(),
                                namespace,
                            )
                            .await
                        }.map(move |res| (height, res)).boxed());

                    }
                }
            }

            Some((height, res)) = verification_stream.next() => {
                match res {
                    Err(error) => {
                        let error = error.as_ref() as &(dyn std::error::Error + 'static);

                        warn!(da_block_height = %height.value(), error, "verification of sequencer blocs failed; skipping");

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
