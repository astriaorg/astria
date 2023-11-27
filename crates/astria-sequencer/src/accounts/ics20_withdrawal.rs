use anyhow::{
    ensure,
    Context,
    Result,
};
use ibc_types::core::channel::{
    ChannelId,
    PortId,
};
use penumbra_ibc::component::packet::{
    IBCPacket,
    SendPacketRead as _,
    SendPacketWrite as _,
    Unchecked,
};
use proto::native::sequencer::v1alpha1::{
    asset,
    asset::IbcAsset,
    Address,
    Ics20Withdrawal,
};
use tracing::instrument;

use crate::{
    accounts::state_ext::{
        StateReadExt,
        StateWriteExt,
    },
    transaction::action_handler::ActionHandler,
};

#[async_trait::async_trait]
impl ActionHandler for Ics20Withdrawal {
    #[instrument(skip(self))]
    async fn check_stateless(&self) -> Result<()> {
        if self.timeout_time == 0 {
            anyhow::bail!("timeout time must be non-zero");
        }

        // NOTE: we could validate the destination chain address as bech32 to prevent mistyped
        // addresses, but this would preclude sending to chains that don't use bech32 addresses.
        Ok(())
    }

    #[instrument(skip(self, state))]
    async fn check_stateful<S: StateReadExt + 'static>(
        &self,
        state: &S,
        from: Address,
        _fee_asset_id: asset::Id,
    ) -> Result<()> {
        let packet: IBCPacket<Unchecked> = self.clone().into();
        state
            .send_packet_check(packet)
            .await
            .context("packet failed send check")?;

        let from_transfer_balance = state
            .get_account_balance(from, self.denom.id())
            .await
            .context("failed getting `from` account balance for transfer")?;
        ensure!(
            from_transfer_balance > self.amount,
            "insufficient funds for transfer"
        );

        Ok(())
    }

    #[instrument(skip(self, state))]
    async fn execute<S: StateWriteExt>(
        &self,
        state: &mut S,
        from: Address,
        _fee_asset_id: asset::Id,
    ) -> Result<()> {
        let checked_packet = IBCPacket::<Unchecked>::from(self.clone()).assume_checked();

        let from_transfer_balance = state
            .get_account_balance(from, self.denom.id())
            .await
            .context("failed getting `from` account balance for transfer")?;
        ensure!(
            from_transfer_balance > self.amount,
            "insufficient funds for transfer"
        );

        state
            .put_account_balance(from, self.denom.id(), from_transfer_balance - self.amount)
            .context("failed to update sender balance")?;

        // if we're the source, move tokens to the escrow account,
        // otherwise the tokens are just burned
        if is_source(
            checked_packet.source_port(),
            checked_packet.source_channel(),
            &self.denom,
        ) {
            let channel_balance = state
                .get_ibc_channel_balance(&self.source_channel, self.denom.id())
                .await
                .context("failed to get channel balance")?;

            state
                .put_ibc_channel_balance(
                    &self.source_channel,
                    self.denom.id(),
                    channel_balance + self.amount,
                )
                .context("failed to update channel balance")?;
        }

        state.send_packet_execute(checked_packet).await;
        Ok(())
    }
}

fn is_source(source_port: &PortId, source_channel: &ChannelId, asset: &IbcAsset) -> bool {
    let prefix = format!("{source_port}/{source_channel}/");
    !asset.prefix_is(&prefix)
}
