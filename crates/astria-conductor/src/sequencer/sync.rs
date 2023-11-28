use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use deadpool::managed::{
    Pool,
    PoolError,
};
use futures::stream::FuturesOrdered;
use sequencer_client::tendermint::block::Height;
use sequencer_types::SequencerBlockData;
use tokio::select;
use tracing::{
    error,
    info,
    instrument,
    warn,
};

use crate::client_provider::{
    self,
    ClientProvider,
};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("failed requesting a client from the pool")]
    Pool(#[from] PoolError<client_provider::Error>),
    #[error("getting a block from sequencer failed")]
    Request(#[from] sequencer_client::extension_trait::Error),
}

#[instrument(name = "sync sequencer", skip(client_pool, executor))]
pub(super) async fn run(
    start: Height,
    end: Height,
    client_pool: Pool<ClientProvider>,
    executor: crate::executor::Sender,
) -> eyre::Result<()> {
    use futures::{
        FutureExt as _,
        StreamExt as _,
    };

    let start: u32 = start
        .value()
        .try_into()
        .wrap_err("start cometbft height overflowed u32")?;
    let end: u32 = end
        .value()
        .try_into()
        .wrap_err("end cometbft height overflowed u32")?;
    let mut height_stream = futures::stream::iter(start..end);
    let mut block_stream = FuturesOrdered::new();

    'sync: loop {
        select!(
            // The condition on block_stream relies on the pool size being currently set to 50.
            // This ensures that no more than 20 requests to the sequencer are active at the same time.
            // Leaving some objects in the pool is important so that failed blocks can be rescheduled
            // in the match arm below.
            Some(height) = height_stream.next(), if block_stream.len() <= 20 => {
                let pool = client_pool.clone();
                block_stream.push_back(async move {
                    get_client_then_block(pool, height).await
                }.map(move |res| (height, res)).boxed());
            }

            Some((height, res)) = block_stream.next() => {
                match res {
                    Err(Error::Request(error)) => {
                        let error = &error as &(dyn std::error::Error + 'static);
                        warn!(height, error, "failed getting sequencer block; rescheduling");
                        let pool = client_pool.clone();
                        block_stream.push_front(async move {
                            get_client_then_block(pool, height).await
                        }.map(move |res| (height, res)).boxed());
                    }

                    Err(Error::Pool(e)) => {
                        let error = &e as &(dyn std::error::Error + 'static);
                        error!(height, error, "failed getting a client from the pool; aborting sync");
                        break 'sync Err(e).wrap_err("failed getting a client from the pool");
                    }

                    Ok(block) => {
                        let block = Box::new(block);
                        if let Err(e) = executor.send(crate::executor::ExecutorCommand::FromSequencer { block }) {
                            let error = &e as &(dyn std::error::Error + 'static);
                            error!(height, error, "failed forwarding block to executor; aborting async");
                            break 'sync Err(e).wrap_err("failed forwarding block to executor");
                        }
                    }
                }
            }

            else => {
                info!("sequencer sync finished");
                break 'sync Ok(())
            }
        );
    }
}

async fn get_client_then_block(
    pool: Pool<ClientProvider>,
    height: u32,
) -> Result<SequencerBlockData, Error> {
    use sequencer_client::SequencerClientExt as _;

    let client = pool.get().await?;
    let block = client.sequencer_block(height).await?;
    Ok(block)
}
