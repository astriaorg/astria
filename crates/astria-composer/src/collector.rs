use std::error::Error;
use std::fmt::Debug;
use std::marker::{Send, Sync};

use crate::ds::{RollupTx, StreamingClient};
use color_eyre::eyre::{self, Context};
use tokio::sync::mpsc as tokio_mpsc;

pub(crate) struct Collector {
    digestion_channel: tokio_mpsc::UnboundedSender<RollupTx>,
}

impl Collector {
    pub(crate) fn new() -> (Self, tokio_mpsc::UnboundedReceiver<RollupTx>) {
        let (stream, sink) = tokio_mpsc::unbounded_channel::<RollupTx>();
        let new_collector = Collector {
            digestion_channel: stream,
        };

        (new_collector, sink)
    }

    pub(crate) async fn add_provider<P>(&self, pr: Box<P>) -> Result<(), eyre::Error>
    where
        P: StreamingClient<Error = eyre::Error>,
    {
        let sender_clone = self.digestion_channel.clone();
        let mut receiver = pr.start_stream().await?;

        while let Some(tx) = receiver.recv().await {
            sender_clone.send(tx)?;
        }
        Ok(())
    }
}
