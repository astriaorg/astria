use color_eyre::eyre::{self};
use tokio::sync::mpsc::UnboundedSender;

use super::data_structures::{
    streaming_client::StreamingClient, ActorChannel, RollupChainId, RollupTxExt,
};

pub(super) type Collector = ActorChannel<(), RollupTxExt>;

impl Collector {
    pub(super) fn new(outgoing: UnboundedSender<RollupTxExt>) -> Self {
        ActorChannel {
            incoming: None,
            outgoing: Some(outgoing),
        }
    }

    pub(super) async fn add_provider<P>(
        &self,
        pr: Box<P>,
        chain_id: RollupChainId,
    ) -> Result<(), eyre::Error>
    where
        P: StreamingClient<Error = eyre::Error> + ?Sized,
    {
        let sender_clone = self.outgoing.clone().unwrap();
        let mut receiver = pr.start_stream(chain_id).await?;

        tokio::task::spawn(async move {
            while let Some(tx) = receiver.recv().await {
                sender_clone.send(tx).unwrap();
            }
        });

        Ok(())
    }
}
