//! This module implements the ICS20 transfer handler, which handles
//! incoming packets.
//!
//! It contains an [`Ics20Transfer`] struct which implements the Penumbra
//! [`AppHandler`] trait, which is passed through the Penumbra IBC implementation
//! during transaction checks and execution. The IBC implementation calls into
//! the ICS20 transfer handler during the IBC transaction lifecycle.
//!
//! [`AppHandler`] consists of two traits: [`AppHandlerCheck`] and [`AppHandlerExecute`].
//! [`AppHandlerCheck`] is used for stateless and stateful checks, while
//! [`AppHandlerExecute`] is used for execution.

use astria_core::{
    primitive::v1::{
        asset::{
            denom,
            Denom,
        },
        Address,
        Bech32,
        Bech32m,
    },
    protocol::memos::v1::{
        Ics20TransferDeposit,
        Ics20WithdrawalFromRollup,
    },
    sequencerblock::v1::block::Deposit,
};
use astria_eyre::{
    anyhow::{
        self,
        Context as _,
    },
    eyre::{
        bail,
        ensure,
        OptionExt as _,
        Result,
        WrapErr as _,
    },
    eyre_to_anyhow,
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use ibc_types::{
    core::channel::{
        channel,
        msgs::{
            MsgAcknowledgement,
            MsgChannelCloseConfirm,
            MsgChannelCloseInit,
            MsgChannelOpenAck,
            MsgChannelOpenConfirm,
            MsgChannelOpenInit,
            MsgChannelOpenTry,
            MsgRecvPacket,
            MsgTimeout,
        },
        ChannelId,
        Packet,
        PortId,
    },
    transfer::acknowledgement::TokenTransferAcknowledgement,
};
use penumbra_ibc::component::app_handler::{
    AppHandler,
    AppHandlerCheck,
    AppHandlerExecute,
};
use penumbra_proto::penumbra::core::component::ibc::v1::FungibleTokenPacketData;
use tokio::try_join;
use tracing::instrument;

use crate::{
    accounts::StateWriteExt as _,
    address::StateReadExt as _,
    assets::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    ibc::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
    utils::create_deposit_event,
};

/// The maximum length of the encoded Ics20 `FungibleTokenPacketData` in bytes.
const MAX_PACKET_DATA_BYTE_LENGTH: usize = 2048;

/// The maximum length of the rollup address in bytes.
const MAX_ROLLUP_ADDRESS_BYTE_LENGTH: usize = 256;

/// Returns if Sequencer originated `asset`, or if `asset` was bridged.
///
/// To use cosmos nomenclature: if Sequencer is a source zone or a sink zone.
///
/// # Idea behind this test
///
/// Assume Sequencer and the counterparty chain established an IBC channel with
/// `(Sp, Sc)` the (port, channel) pair on the sequencer side, and `(Cp, Cc)` the
/// (port, channel) pair on the counterparty side.
///
/// ## Sequencer sends a token that originates on Sequencer
///
/// 1. Sequencer sends `Original` from `(Sp, Sc)` to `(Cp, Cc)`.
/// 2. Counterparty receives `Original`, prefixes it `Cp/Cc/Original`, and stores the prefixed
///    version.
///
/// ## Counterparty sends a token that originates on Sequencer
///
/// 1. Counterparty sends `Cp/Cc/Original` from `(Cp, Cc)` to `(Sp, Sc)`.
/// 2. Sequencer tests if it was the origin of `Cp/Cc/Original` source by checking if it is prefixed
///    by the source (port, channel) pair of this transfer (answer: yes, `source_port` is `Cp`,
///    `source_channel` is `Cc`).
/// 3. Sequencer strips `Cp/Cc/` and credits `Original` to the relevant recipient address.
///
/// ## Counterparty sends a different asset
///
/// If the counterparty sends back `DifferentAsset` that does not carry the
/// prefix `Cp/Cc/`, then the test in the previous section will be negative
/// and Sequencer will add the prefix `Sp/Sc/DifferentAsset` before crediting a
/// receiving Astria address.
///
/// Note that `DifferentAsset` could have any number of prefixes, including `Cp/Cc`,
/// as long as `Cp/Cc` are not the leading prefixes for this test! So Sequencer would
/// consider `DifferentAsset` as not originating on Sequencer in the same way as
/// `Dp/Dc/Ep/Ec/Fp/Fc/Cp/Cc/Asset` did not originate on Sequencer either.
///
/// # Refunds
///
/// Refund logic negates the above result.
///
/// If a Sequencer initiated ICS20 transfer fails, then the original packet it sent is
/// returned unchanged:
///
/// As before, Sequencer sends `Asset` from `(Sp, Sc)` to `(Cp, Cc)`, but the packet is
/// returned as-is. Where for a packet coming on from another chain `source_port = Cp`
/// and `source_channel = Cc`, for a refund `source_port = Sp` and `source_channel = Sc`.
///
/// ## Refunding a token for which Sequencer is the source zone
///
/// If Sequencer is the source zone of a an `<asset>`, then `<asset>` will *not* be prefixed
/// `Sp/Sc/<asset>`. The reason is that any non-originating asset will be prefixed
/// `Sp/Sc/<asset>`, and so when sending `Sp/Sc/<asset>`, Sequencer will set
/// `source_port = Sp` and `source_channel = Sc` always.
///
/// ## Refunding an asset that was sent to Sequencer / that is bridged in
///
/// If `<asset>` was sent to Sequencer and did not originate on Sequencer, then Sequencer will
/// have prefixed it `Sp/Sc/<asset>` upon receipt. And so when sending, the payload asset
/// *will* be prefixed `Sp/Sc/<asset>` with `source_port = Sp` and `source_channel = Sc` set.
fn is_transfer_source_zone(
    asset: &denom::TracePrefixed,
    port: &PortId,
    channel: &ChannelId,
) -> bool {
    asset.has_leading_port(port) && asset.has_leading_channel(channel)
}

/// See [`is_transfer_source_zone`] for what this does.
fn is_refund_source_zone(asset: &denom::TracePrefixed, port: &PortId, channel: &ChannelId) -> bool {
    !is_transfer_source_zone(asset, port, channel)
}

/// The ICS20 transfer handler.
///
/// See [here](https://github.com/cosmos/ibc/blob/main/spec/app/ics-020-fungible-token-transfer/README.md)
/// for the specification which this is based on.
#[derive(Clone)]
pub(crate) struct Ics20Transfer;

#[async_trait::async_trait]
impl AppHandlerCheck for Ics20Transfer {
    async fn chan_open_init_check<S: StateRead>(
        _: S,
        msg: &MsgChannelOpenInit,
    ) -> anyhow::Result<()> {
        if msg.ordering != channel::Order::Unordered {
            anyhow::bail!("channel order must be unordered for Ics20 transfer");
        }

        if msg.version_proposal.as_str() != "ics20-1" {
            anyhow::bail!("channel version must be ics20-1 for Ics20 transfer");
        }

        Ok(())
    }

    async fn chan_open_try_check<S: StateRead>(
        _: S,
        msg: &MsgChannelOpenTry,
    ) -> anyhow::Result<()> {
        if msg.ordering != channel::Order::Unordered {
            anyhow::bail!("channel order must be unordered for Ics20 transfer");
        }

        if msg.version_supported_on_a.as_str() != "ics20-1" {
            anyhow::bail!("counterparty version must be ics20-1 for Ics20 transfer");
        }

        Ok(())
    }

    async fn chan_open_ack_check<S: StateRead>(
        _: S,
        msg: &MsgChannelOpenAck,
    ) -> anyhow::Result<()> {
        if msg.version_on_b.as_str() != "ics20-1" {
            anyhow::bail!("counterparty version must be ics20-1 for Ics20 transfer");
        }

        Ok(())
    }

    async fn chan_open_confirm_check<S: StateRead>(
        _: S,
        _: &MsgChannelOpenConfirm,
    ) -> anyhow::Result<()> {
        // accept channel confirmations, port has already been validated, version has already been
        // validated
        Ok(())
    }

    async fn chan_close_init_check<S: StateRead>(
        _: S,
        _: &MsgChannelCloseInit,
    ) -> anyhow::Result<()> {
        anyhow::bail!("ics20 always aborts on chan_close_init");
    }

    async fn chan_close_confirm_check<S: StateRead>(
        _: S,
        _: &MsgChannelCloseConfirm,
    ) -> anyhow::Result<()> {
        // no action needed
        Ok(())
    }

    async fn recv_packet_check<S: StateRead>(_: S, msg: &MsgRecvPacket) -> anyhow::Result<()> {
        // most checks performed in `execute`
        // perform stateless checks here
        if msg.packet.data.is_empty() {
            anyhow::bail!("packet data is empty");
        }

        if msg.packet.data.len() > MAX_PACKET_DATA_BYTE_LENGTH {
            anyhow::bail!("packet data is too long: exceeds MAX_PACKET_DATA_BYTE_LENGTH");
        }

        Ok(())
    }

    async fn timeout_packet_check<S: StateRead>(state: S, msg: &MsgTimeout) -> anyhow::Result<()> {
        refund_tokens_check(
            state,
            msg.packet.data.as_slice(),
            &msg.packet.port_on_a,
            &msg.packet.chan_on_a,
        )
        .await
        .map_err(eyre_to_anyhow)
    }

    async fn acknowledge_packet_check<S: StateRead>(
        state: S,
        msg: &MsgAcknowledgement,
    ) -> anyhow::Result<()> {
        // see https://github.com/cosmos/ibc-go/blob/3f5b2b6632e0fa37056e5805b289a9307870ac9a/modules/core/04-channel/types/acknowledgement.go
        // and https://github.com/cosmos/ibc-go/blob/3f5b2b6632e0fa37056e5805b289a9307870ac9a/proto/ibc/core/channel/v1/channel.proto#L155
        // for formatting
        let ack: TokenTransferAcknowledgement =
            serde_json::from_slice(msg.acknowledgement.as_slice())?;
        if ack.is_successful() {
            return Ok(());
        }

        refund_tokens_check(
            state,
            msg.packet.data.as_slice(),
            &msg.packet.port_on_a,
            &msg.packet.chan_on_a,
        )
        .await
        .map_err(eyre_to_anyhow)
    }
}

async fn refund_tokens_check<S: StateRead>(
    state: S,
    data: &[u8],
    source_port: &PortId,
    source_channel: &ChannelId,
) -> Result<()> {
    let packet_data: FungibleTokenPacketData = serde_json::from_slice(data)
        .wrap_err("failed to decode fungible token packet data json")?;

    let asset = parse_asset(&state, &packet_data.denom)
        .await
        .wrap_err_with(|| {
            format!(
                "failed to read packet.denom `{}` as trace prefixed asset",
                packet_data.denom
            )
        })?;

    if is_refund_source_zone(&asset, source_port, source_channel) {
        // check if escrow account has enough balance to refund user
        let balance = state
            .get_ibc_channel_balance(source_channel, &asset)
            .await
            .wrap_err("failed to get channel balance in refund_tokens_check")?;

        let packet_amount: u128 = packet_data
            .amount
            .parse()
            .wrap_err("failed to parse packet amount as u128")?;
        ensure!(
            balance >= packet_amount,
            "insufficient balance to refund tokens to sender",
        );
    }

    Ok(())
}

#[async_trait::async_trait]
impl AppHandlerExecute for Ics20Transfer {
    async fn chan_open_init_execute<S: StateWrite>(_: S, _: &MsgChannelOpenInit) {}

    async fn chan_open_try_execute<S: StateWrite>(_: S, _: &MsgChannelOpenTry) {}

    async fn chan_open_ack_execute<S: StateWrite>(_: S, _: &MsgChannelOpenAck) {}

    async fn chan_open_confirm_execute<S: StateWrite>(_: S, _: &MsgChannelOpenConfirm) {}

    async fn chan_close_confirm_execute<S: StateWrite>(_: S, _: &MsgChannelCloseConfirm) {}

    async fn chan_close_init_execute<S: StateWrite>(_: S, _: &MsgChannelCloseInit) {}

    #[instrument(skip_all, err)]
    async fn recv_packet_execute<S: StateWrite>(
        mut state: S,
        msg: &MsgRecvPacket,
    ) -> anyhow::Result<()> {
        use penumbra_ibc::component::packet::WriteAcknowledgement as _;

        let ack = match receive_tokens(&mut state, &msg.packet).await {
            Ok(()) => TokenTransferAcknowledgement::success(),
            Err(e) => {
                tracing::debug!(
                    error = AsRef::<dyn std::error::Error>::as_ref(&e),
                    "failed to execute ics20 transfer"
                );
                TokenTransferAcknowledgement::Error(format!("{e:#}"))
            }
        };

        let ack_bytes: Vec<u8> = ack.into();

        state
            .write_acknowledgement(&msg.packet, &ack_bytes)
            .await
            .context("failed to write acknowledgement")
    }

    #[instrument(skip_all, err)]
    async fn timeout_packet_execute<S: StateWrite>(
        mut state: S,
        msg: &MsgTimeout,
    ) -> anyhow::Result<()> {
        refund_tokens(&mut state, &msg.packet).await.map_err(|err| {
            eyre_to_anyhow(err).context("failed to refund tokens during timeout_packet_execute")
        })
    }

    #[instrument(skip_all, err)]
    async fn acknowledge_packet_execute<S: StateWrite>(
        mut state: S,
        msg: &MsgAcknowledgement,
    ) -> anyhow::Result<()> {
        let ack: TokenTransferAcknowledgement = astria_eyre::anyhow::Context::context(
            serde_json::from_slice(msg.acknowledgement.as_slice()),
            "failed to deserialize token transfer acknowledgement",
        )?;
        if !ack.is_successful() {
            return refund_tokens(&mut state, &msg.packet)
                .await
                .map_err(|err| eyre_to_anyhow(err).context("failed to refund tokens"));
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl AppHandler for Ics20Transfer {}

#[instrument(
    skip_all,
    fields(
        %packet.port_on_a,
        %packet.chan_on_a,
        %packet.port_on_b,
        %packet.chan_on_b,
        %packet.timeout_height_on_b,
        %packet.timeout_timestamp_on_b,
    ),
    err,
)]
async fn receive_tokens<S: StateWrite>(mut state: S, packet: &Packet) -> Result<()> {
    let packet_data: FungibleTokenPacketData = serde_json::from_slice(&packet.data)
        .wrap_err("failed to deserialize fungible token packet data")?;

    let amount: u128 = packet_data
        .amount
        .parse()
        .wrap_err("failed to parse packet data amount to u128")?;

    let recipient = parse_address_on_sequencer(&state, &packet_data.receiver)
        .await
        .with_context(|| {
            format!(
                "failed parsing packet.receiver `{}` as the recipient address",
                packet_data.receiver
            )
        })?;

    let mut asset = parse_asset(&state, &packet_data.denom)
        .await
        .with_context(|| {
            format!(
                "failed reading asset `{}` from packet data",
                packet_data.denom
            )
        })?;

    let is_source = is_transfer_source_zone(&asset, &packet.port_on_a, &packet.chan_on_a);
    if is_source {
        asset.pop_leading_port_and_channel();
    } else {
        // TODO(janis): we should provide a method on `denom::TracePrefixed` to directly
        // prefix it with a (port, channel) pair.
        asset = format!(
            "{destination_port}/{destination_channel}/{asset}",
            destination_port = &packet.port_on_b,
            destination_channel = &packet.chan_on_b,
        )
        .parse()
        .expect(
            "dest port and channel are valid prefix segments, so this concatenation must be a \
             valid denom",
        );
    }

    // If `recipient` is a bridge account then create a deposit event to signal to
    // its associated rollup that funds were received.
    //
    // `recipient` is a bridge account if it has an associated rollup identified by its ID.
    if state
        .get_bridge_account_rollup_id(&recipient)
        .await
        .context("failed to get bridge account rollup ID from state")?
        .is_some()
    {
        emit_bridge_lock_deposit(&mut state, recipient, &asset, amount, &packet_data.memo)
            .await
            .context("failed to execute ics20 transfer to bridge account")?;
    }

    if is_source {
        // the asset being transferred in is an asset that originated on Astria;
        // subtract balance from escrow account.
        state
            .decrease_ibc_channel_balance(&packet.chan_on_b, &asset, amount)
            .await
            .context("failed to deduct funds from IBC escrow account")?;
    } else {
        // register denomination in global ID -> denom map if it's not already there
        if !state
            .has_ibc_asset(&asset)
            .await
            .wrap_err("failed to check if IBC asset exists in state")?
        {
            state
                .put_ibc_asset(asset.clone())
                .wrap_err("failed to write IBC asset to state")?;
        }
    }

    state
        .increase_balance(&recipient, &asset, amount)
        .await
        .context("failed to update user account balance")?;

    Ok(())
}

#[instrument(
    skip_all,
    fields(
        %packet.port_on_a,
        %packet.chan_on_a,
        %packet.port_on_b,
        %packet.chan_on_b,
        %packet.timeout_height_on_b,
        %packet.timeout_timestamp_on_b,
    ),
    err,
)]
async fn refund_tokens<S: StateWrite>(mut state: S, packet: &Packet) -> Result<()> {
    let packet_data: FungibleTokenPacketData = serde_json::from_slice(&packet.data)
        .wrap_err("failed to deserialize fungible token packet data")?;

    let amount: u128 = packet_data
        .amount
        .parse()
        .wrap_err("failed to parse packet data amount to u128")?;

    // Since we are refunding tokens, packet_data.sender is an address on Astria:
    // the packet was not commited on the counter party chain but returned as-is.
    let receiver = parse_address_on_sequencer(&state, &packet_data.sender)
        .await
        .with_context(|| {
            format!(
                "failed parsing packet.sender `{}` as the return address",
                packet_data.sender
            )
        })?;

    let asset = parse_asset(&state, &packet_data.denom)
        .await
        .with_context(|| format!("failed parsing packet.asset `{}`", packet_data.denom))?;

    // Refunding a rollup is the same as refunding an address on sequencer (which would
    // be the bridge account associated with the rollup) plus emitting a deposit.
    if let Some(memo) = does_failed_transfer_come_from_rollup(&packet_data) {
        emit_deposit(
            &mut state,
            &receiver,
            memo.rollup_return_address,
            &asset,
            amount,
        )
        .await
        .context("failed to emit deposit for refunding tokens to rollup")?;
    }
    refund_tokens_to_sequencer_address(
        &mut state,
        &receiver,
        &asset,
        amount,
        &packet.port_on_a,
        &packet.chan_on_a,
    )
    .await
    .context("failed to refund a sequencer address")?;

    Ok(())
}

/// A failed transfer is said to originate on a rollup if its memo field can be
/// parsed as a `[Ics20WithdrawalFromRollup]`.
fn does_failed_transfer_come_from_rollup(
    packet_data: &FungibleTokenPacketData,
) -> Option<Ics20WithdrawalFromRollup> {
    serde_json::from_str::<Ics20WithdrawalFromRollup>(&packet_data.memo).ok()
}

#[instrument(skip_all, fields(%recipient, %asset, amount), err)]
async fn refund_tokens_to_sequencer_address<S: StateWrite>(
    mut state: S,
    recipient: &Address,
    asset: &denom::TracePrefixed,
    amount: u128,
    source_port: &PortId,
    source_channel: &ChannelId,
) -> Result<()> {
    if is_refund_source_zone(asset, source_port, source_channel) {
        state
            .decrease_ibc_channel_balance(source_channel, asset, amount)
            .await
            .context("failed to withdraw refund amount from escrow account")?;
    }
    state
        .increase_balance(recipient, asset, amount)
        .await
        .wrap_err("failed to update user account balance when refunding")?;

    Ok(())
}

#[instrument(skip_all, fields(input), err)]
async fn parse_asset<S: StateRead>(state: S, input: &str) -> Result<denom::TracePrefixed> {
    let asset = match input
        .parse::<Denom>()
        .wrap_err("failed parsing input as IBC denomination")?
    {
        Denom::TracePrefixed(trace_prefixed) => trace_prefixed,
        Denom::IbcPrefixed(ibc_prefixed) => state
            .map_ibc_to_trace_prefixed_asset(&ibc_prefixed)
            .await
            .wrap_err("failed reading state to map ibc prefixed asset to trace prefixed asset")?
            .ok_or_eyre(
                "could not find trace prefixed counterpart to ibc prefixed asset in state",
            )?,
    };
    Ok(asset)
}

#[instrument(skip_all, fields(input), err)]
async fn parse_address_on_sequencer<S: StateRead>(state: &S, input: &str) -> Result<Address> {
    use futures::TryFutureExt as _;
    let (base_prefix, compat_prefix) = match try_join!(
        state
            .get_base_prefix()
            .map_err(|e| e.wrap_err("failed to read base prefix from state")),
        state
            .get_ibc_compat_prefix()
            .map_err(|e| e.wrap_err("failed to read ibc compat prefix from state"))
    ) {
        Ok(prefixes) => prefixes,
        Err(err) => return Err(err),
    };
    input
        .parse::<Address<Bech32m>>()
        .wrap_err("failed to parse address in bech32m format")
        .and_then(|addr| {
            ensure!(
                addr.prefix() == base_prefix,
                "address prefix is not base prefix stored in state"
            );
            Ok(addr)
        })
        .or_else(|_| {
            input
                .parse::<Address<Bech32>>()
                .wrap_err("failed to parse address in bech32/compat format")
                .and_then(|addr| {
                    ensure!(
                        addr.prefix() == compat_prefix,
                        "address prefix is not base prefix stored in state"
                    );
                    addr.to_prefix(&base_prefix)
                        .wrap_err(
                            "failed to convert ibc compat prefixed address to standard base \
                             prefixed address",
                        )
                        .map(|addr| addr.to_format::<Bech32m>())
                })
        })
    // "sender address was neither base nor ibc-compat prefixed; returning last error",
}

/// Emits a deposit event signaling to the rollup that funds
/// were added to `bridge_address`.
#[instrument(skip_all, fields(%bridge_address, %asset, amount, memo), err)]
async fn emit_bridge_lock_deposit<S: StateWrite>(
    mut state: S,
    bridge_address: Address,
    asset: &denom::TracePrefixed,
    amount: u128,
    memo: &str,
) -> Result<()> {
    let deposit_memo: Ics20TransferDeposit =
        serde_json::from_str(memo).wrap_err("failed to parse memo as Ics20TransferDepositMemo")?;

    ensure!(
        !deposit_memo.rollup_deposit_address.is_empty(),
        "rollup deposit address must be set to bridge funds from sequencer to rollup",
    );

    ensure!(
        deposit_memo.rollup_deposit_address.len() <= MAX_ROLLUP_ADDRESS_BYTE_LENGTH,
        "rollup address is too long: exceeds MAX_ROLLUP_ADDRESS_BYTE_LENGTH",
    );

    emit_deposit(
        &mut state,
        &bridge_address,
        deposit_memo.rollup_deposit_address,
        asset,
        amount,
    )
    .await
}

#[instrument(skip_all, fields(%bridge_address, destination_chain_address, %asset, amount), err)]
async fn emit_deposit<S: StateWrite>(
    mut state: S,
    bridge_address: &Address,
    destination_chain_address: String,
    asset: &denom::TracePrefixed,
    amount: u128,
) -> Result<()> {
    // check if the recipient is a bridge account and
    // ensure that the asset ID being transferred
    // to it is allowed.
    let Some(rollup_id) = state
        .get_bridge_account_rollup_id(bridge_address)
        .await
        .wrap_err("failed to get bridge account rollup ID from state")?
    else {
        bail!("bridge account rollup ID not found in state; invalid bridge address?")
    };

    let allowed_asset = state
        .get_bridge_account_ibc_asset(bridge_address)
        .await
        .wrap_err("failed to get bridge account asset ID")?;
    ensure!(
        allowed_asset == asset.to_ibc_prefixed(),
        "asset `{asset}` with ID `{}` is not authorized for transfer to bridge account",
        asset.to_ibc_prefixed(),
    );

    let transaction_context = state
        .get_transaction_context()
        .ok_or_eyre("transaction source should be present in state when executing an action")?;
    let source_transaction_id = transaction_context.transaction_id;
    let source_action_index = transaction_context.source_action_index;

    let deposit = Deposit {
        bridge_address: *bridge_address,
        rollup_id,
        amount,
        asset: asset.into(),
        destination_chain_address,
        source_transaction_id,
        source_action_index,
    };
    let deposit_abci_event = create_deposit_event(&deposit);
    state.record(deposit_abci_event);
    state.cache_deposit_event(deposit);
    Ok(())
}

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::{
            asset::Denom,
            RollupId,
            TransactionId,
        },
        protocol::memos::v1::{
            Ics20TransferDeposit,
            Ics20WithdrawalFromRollup,
        },
        sequencerblock::v1::block::Deposit,
    };
    use ibc_types::{
        core::channel::{
            packet::Sequence,
            ChannelId,
            Packet,
            PortId,
            TimeoutHeight,
        },
        timestamp::Timestamp,
    };
    use penumbra_proto::core::component::ibc::v1::FungibleTokenPacketData;

    use super::{
        receive_tokens,
        refund_tokens,
    };
    use crate::{
        accounts::StateReadExt as _,
        address::StateWriteExt as _,
        assets::StateReadExt as _,
        benchmark_and_test_utils::{
            astria_address,
            nria,
            ASTRIA_COMPAT_PREFIX,
            ASTRIA_PREFIX,
        },
        bridge::{
            StateReadExt as _,
            StateWriteExt as _,
        },
        ibc::{
            StateReadExt as _,
            StateWriteExt,
        },
        storage::Storage,
        test_utils::astria_compat_address,
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    fn packet() -> Packet {
        Packet {
            sequence: Sequence(0),
            port_on_a: PortId("transfer".into()),
            chan_on_a: ChannelId("achan".into()),
            port_on_b: PortId("transfer".into()),
            chan_on_b: ChannelId("bchan".into()),
            data: Vec::new(),
            timeout_height_on_b: TimeoutHeight::Never,
            timeout_timestamp_on_b: Timestamp {
                time: None,
            },
        }
    }

    fn source_asset() -> Denom {
        format!("{}/{}/{}", packet().port_on_a, packet().chan_on_a, nria())
            .parse::<Denom>()
            .unwrap()
    }

    fn sink_asset() -> Denom {
        format!("{}/{}/{}", packet().port_on_b, packet().chan_on_b, nria())
            .parse::<Denom>()
            .unwrap()
    }

    #[tokio::test]
    async fn receive_source_zone_asset_on_sequencer_account() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let recipient_address = astria_address(&[1; 20]);
        let amount = 100;

        state_delta
            .put_base_prefix(ASTRIA_PREFIX.to_string())
            .unwrap();
        state_delta
            .put_ibc_compat_prefix(ASTRIA_COMPAT_PREFIX.to_string())
            .unwrap();
        state_delta
            .put_ibc_channel_balance(&packet().chan_on_b, &nria(), amount)
            .unwrap();

        let packet_data = FungibleTokenPacketData {
            denom: source_asset().to_string(),
            sender: String::new(),
            amount: amount.to_string(),
            receiver: recipient_address.to_string(),
            memo: String::new(),
        };

        receive_tokens(
            &mut state_delta,
            &Packet {
                data: serde_json::to_vec(&packet_data).unwrap(),
                ..packet()
            },
        )
        .await
        .unwrap();

        let user_balance = state_delta
            .get_account_balance(&recipient_address, &nria())
            .await
            .expect("ics20 transfer to user account should succeed");
        assert_eq!(user_balance, amount);
        let escrow_balance = state_delta
            .get_ibc_channel_balance(&packet().chan_on_b, &nria())
            .await
            .expect("ics20 transfer to user account from escrow account should succeed");
        assert_eq!(escrow_balance, 0);
    }

    #[tokio::test]
    async fn receive_sink_zone_asset_on_sequencer_account() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        state_delta
            .put_base_prefix(ASTRIA_PREFIX.to_string())
            .unwrap();
        state_delta
            .put_ibc_compat_prefix(ASTRIA_COMPAT_PREFIX.to_string())
            .unwrap();

        let recipient_address = astria_address(&[1; 20]);
        let amount = 100;

        // "nria" being received by sequencer will be a foreign asset
        // because it is not prefixed by sequencer's (port, channel) pair.
        let packet_data = FungibleTokenPacketData {
            denom: nria().to_string(),
            sender: String::new(),
            amount: amount.to_string(),
            receiver: recipient_address.to_string(),
            memo: String::new(),
        };

        receive_tokens(
            &mut state_delta,
            &Packet {
                data: serde_json::to_vec(&packet_data).unwrap(),
                ..packet()
            },
        )
        .await
        .unwrap();

        assert!(state_delta.has_ibc_asset(&sink_asset()).await.expect(
            "a new asset with <sequencer_port>/<sequencer_channel>/<asset> should be registered \
             in the state"
        ));
        let user_balance = state_delta
            .get_account_balance(&recipient_address, &sink_asset())
            .await
            .expect(
                "a successful transfer should be reflected in the account balance of the new asset",
            );
        assert_eq!(user_balance, amount);
    }

    #[tokio::test]
    async fn receive_source_zone_asset_on_bridge_account_and_emit_to_rollup() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let bridge_address = astria_address(&[99; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");

        state_delta
            .put_base_prefix(ASTRIA_PREFIX.to_string())
            .unwrap();
        state_delta
            .put_ibc_compat_prefix(ASTRIA_COMPAT_PREFIX.to_string())
            .unwrap();
        state_delta.put_transaction_context(TransactionContext {
            address_bytes: bridge_address.bytes(),
            transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        });

        let rollup_deposit_address = "rollupaddress";
        let amount = 100;

        state_delta
            .put_bridge_account_rollup_id(&bridge_address, rollup_id)
            .unwrap();
        state_delta
            .put_bridge_account_ibc_asset(&bridge_address, nria())
            .unwrap();
        state_delta
            .put_ibc_channel_balance(&packet().chan_on_b, &nria(), amount)
            .unwrap();

        let packet_data = FungibleTokenPacketData {
            denom: source_asset().to_string(),
            sender: String::new(),
            amount: amount.to_string(),
            receiver: bridge_address.to_string(),
            memo: serde_json::to_string(&Ics20TransferDeposit {
                rollup_deposit_address: rollup_deposit_address.to_string(),
            })
            .unwrap(),
        };
        receive_tokens(
            &mut state_delta,
            &Packet {
                data: serde_json::to_vec(&packet_data).unwrap(),
                ..packet()
            },
        )
        .await
        .unwrap();

        let balance = state_delta
            .get_account_balance(&bridge_address, &nria())
            .await
            .expect(
                "ics20 transfer from sender to bridge account should have updated funds in the \
                 bridge address",
            );
        assert_eq!(balance, 100);

        let deposits = state_delta.get_cached_block_deposits();
        assert_eq!(deposits.len(), 1);

        let expected_deposit = Deposit {
            bridge_address,
            rollup_id,
            amount,
            asset: nria().into(),
            destination_chain_address: rollup_deposit_address.to_string(),
            source_transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        };

        let actual_deposit = deposits
            .get(&rollup_id)
            .expect("a depsit fo the given rollup ID should exist as result of the refund")
            .first()
            .unwrap();
        assert_eq!(&expected_deposit, actual_deposit);
    }

    #[tokio::test]
    async fn receive_sink_zone_asset_on_bridge_account_and_emit_to_rollup() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let bridge_address = astria_address(&[99; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");

        state_delta
            .put_base_prefix(ASTRIA_PREFIX.to_string())
            .unwrap();
        state_delta
            .put_ibc_compat_prefix(ASTRIA_COMPAT_PREFIX.to_string())
            .unwrap();
        state_delta.put_transaction_context(TransactionContext {
            address_bytes: bridge_address.bytes(),
            transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        });

        let rollup_deposit_address = "rollupaddress";
        let amount = 100;

        let remote_asset = "foreignasset".parse::<Denom>().unwrap();
        let remote_asset_on_sequencer = format!(
            "{}/{}/{remote_asset}",
            packet().port_on_b,
            packet().chan_on_b
        )
        .parse::<Denom>()
        .unwrap();
        state_delta
            .put_bridge_account_rollup_id(&bridge_address, rollup_id)
            .unwrap();
        state_delta
            .put_bridge_account_ibc_asset(&bridge_address, &remote_asset_on_sequencer)
            .unwrap();

        let packet_data = FungibleTokenPacketData {
            denom: remote_asset.to_string(),
            sender: String::new(),
            amount: amount.to_string(),
            receiver: bridge_address.to_string(),
            memo: serde_json::to_string(&Ics20TransferDeposit {
                rollup_deposit_address: rollup_deposit_address.to_string(),
            })
            .unwrap(),
        };
        receive_tokens(
            &mut state_delta,
            &Packet {
                data: serde_json::to_vec(&packet_data).unwrap(),
                ..packet()
            },
        )
        .await
        .unwrap();

        let balance = state_delta
            .get_account_balance(&bridge_address, &remote_asset_on_sequencer)
            .await
            .expect("receipt of funds to a rollup should have updated funds in the bridge account");
        assert_eq!(balance, amount);

        let deposits = state_delta.get_cached_block_deposits();
        assert_eq!(deposits.len(), 1);

        let expected_deposit = Deposit {
            bridge_address,
            rollup_id,
            amount,
            asset: remote_asset_on_sequencer,
            destination_chain_address: rollup_deposit_address.to_string(),
            source_transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        };

        assert_eq!(&expected_deposit, &deposits[&rollup_id][0]);
    }

    #[tokio::test]
    async fn transfer_to_bridge_is_rejected_due_to_invalid_memo() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let bridge_address = astria_address(&[99; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");

        state_delta
            .put_bridge_account_rollup_id(&bridge_address, rollup_id)
            .unwrap();
        state_delta
            .put_bridge_account_ibc_asset(&bridge_address, sink_asset())
            .unwrap();

        let packet_data = FungibleTokenPacketData {
            denom: nria().to_string(),
            sender: String::new(),
            amount: "100".to_string(),
            receiver: bridge_address.to_string(),
            memo: "invalid".to_string(),
        };
        // FIXME(janis): assert that the failure is actually due to the malformed memo
        // and not becase of some other input.
        let _ = receive_tokens(
            &mut state_delta,
            &Packet {
                data: serde_json::to_vec(&packet_data).unwrap(),
                ..packet()
            },
        )
        .await
        .expect_err("malformed packet memo field during transfer to bridge account should fail");
    }

    #[tokio::test]
    async fn transfer_to_bridge_account_is_rejected_due_to_not_permitted_token() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        let bridge_address = astria_address(&[99; 20]);
        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");

        state_delta
            .put_bridge_account_rollup_id(&bridge_address, rollup_id)
            .unwrap();
        state_delta
            .put_bridge_account_ibc_asset(&bridge_address, sink_asset())
            .unwrap();

        let packet_data = FungibleTokenPacketData {
            denom: "unknown".to_string(),
            sender: String::new(),
            amount: "100".to_string(),
            receiver: bridge_address.to_string(),
            memo: serde_json::to_string(&Ics20TransferDeposit {
                rollup_deposit_address: "rollupaddress".to_string(),
            })
            .unwrap(),
        };
        // FIXME(janis): assert that the failure is actually due to the not permitted asset
        // and not because of some other input.
        let _ = receive_tokens(
            &mut state_delta,
            &Packet {
                data: serde_json::to_vec(&packet_data).unwrap(),
                ..packet()
            },
        )
        .await
        .expect_err("unknown asset during transfer to bridge account should fail");
    }

    #[tokio::test]
    async fn refund_sequencer_account_with_source_zone_asset() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        state_delta
            .put_base_prefix(ASTRIA_PREFIX.to_string())
            .unwrap();
        state_delta
            .put_ibc_compat_prefix(ASTRIA_COMPAT_PREFIX.to_string())
            .unwrap();

        let recipient_address = astria_address(&[1; 20]);
        let amount = 100;
        state_delta
            .put_ibc_channel_balance(&packet().chan_on_a, &nria(), amount)
            .unwrap();

        let packet_data = FungibleTokenPacketData {
            denom: nria().to_string(),
            sender: recipient_address.to_string(),
            amount: amount.to_string(),
            receiver: recipient_address.to_string(),
            memo: String::new(),
        };

        refund_tokens(
            &mut state_delta,
            &Packet {
                data: serde_json::to_vec(&packet_data).unwrap(),
                ..packet()
            },
        )
        .await
        .expect("valid ics20 refund to user account; recipient, memo, and asset ID are valid");

        let balance = state_delta
            .get_account_balance(&recipient_address, &nria())
            .await
            .expect("ics20 refund to user account should succeed");
        assert_eq!(balance, amount);
        let balance = state_delta
            .get_ibc_channel_balance(&packet().chan_on_a, &nria())
            .await
            .expect("ics20 refund to user account from escrow account should succeed");
        assert_eq!(balance, 0);
    }

    #[tokio::test]
    async fn refund_sequencer_account_with_sink_zone_asset() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        state_delta
            .put_base_prefix(ASTRIA_PREFIX.to_string())
            .unwrap();
        state_delta
            .put_ibc_compat_prefix(ASTRIA_COMPAT_PREFIX.to_string())
            .unwrap();

        let recipient_address = astria_address(&[1; 20]);
        let amount = 100;
        state_delta
            .put_ibc_channel_balance(&packet().chan_on_a, &sink_asset(), amount)
            .unwrap();

        let packet_data = FungibleTokenPacketData {
            denom: sink_asset().to_string(),
            sender: recipient_address.to_string(),
            amount: amount.to_string(),
            receiver: recipient_address.to_string(),
            memo: String::new(),
        };

        refund_tokens(
            &mut state_delta,
            &Packet {
                data: serde_json::to_vec(&packet_data).unwrap(),
                ..packet()
            },
        )
        .await
        .expect("valid ics20 refund to user account; recipient, memo, and asset ID are valid");

        let balance = state_delta
            .get_account_balance(&recipient_address, &sink_asset())
            .await
            .expect("ics20 refund to user account should succeed");
        assert_eq!(balance, amount);
        let balance = state_delta
            .get_ibc_channel_balance(&packet().chan_on_a, &sink_asset())
            .await
            .expect("ics20 refund to user account from escrow account should succeed");
        assert_eq!(balance, 0);
    }

    #[tokio::test]
    async fn refund_rollup_with_sink_zone_asset() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        state_delta
            .put_base_prefix(ASTRIA_PREFIX.to_string())
            .unwrap();
        state_delta
            .put_ibc_compat_prefix(ASTRIA_COMPAT_PREFIX.to_string())
            .unwrap();

        let bridge_address = astria_address(&[99u8; 20]);

        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");

        state_delta.put_transaction_context(TransactionContext {
            address_bytes: bridge_address.bytes(),
            transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        });

        state_delta
            .put_bridge_account_rollup_id(&bridge_address, rollup_id)
            .unwrap();
        state_delta
            .put_bridge_account_ibc_asset(&bridge_address, sink_asset())
            .unwrap();

        let amount = 100;
        state_delta
            .put_ibc_channel_balance(&packet().chan_on_a, &sink_asset(), amount)
            .unwrap();

        let address_on_rollup = "address_on_rollup".to_string();

        let rollup_return_address = "rollup-defined-return-address";
        let packet_data = FungibleTokenPacketData {
            denom: sink_asset().to_string(),
            sender: bridge_address.to_string(),
            amount: amount.to_string(),
            receiver: address_on_rollup.to_string(),
            memo: serde_json::to_string(&Ics20WithdrawalFromRollup {
                rollup_block_number: 42,
                rollup_withdrawal_event_id: "rollup-defined-id".to_string(),
                rollup_return_address: rollup_return_address.to_string(),
                memo: String::new(),
            })
            .unwrap(),
        };
        refund_tokens(
            &mut state_delta,
            &Packet {
                data: serde_json::to_vec(&packet_data).unwrap(),
                ..packet()
            },
        )
        .await
        .expect("valid rollup withdrawal refund");

        let balance = state_delta
            .get_account_balance(&bridge_address, &sink_asset())
            .await
            .expect("rollup withdrawal refund should have updated funds in the bridge address");
        assert_eq!(balance, amount);

        let deposit = state_delta.get_cached_block_deposits();

        let expected_deposit = Deposit {
            bridge_address,
            rollup_id,
            amount,
            asset: sink_asset(),
            destination_chain_address: rollup_return_address.to_string(),
            source_transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        };
        assert_eq!(expected_deposit, deposit[&rollup_id][0],);
    }

    #[tokio::test]
    async fn refund_rollup_with_source_zone_asset() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        state_delta
            .put_base_prefix(ASTRIA_PREFIX.to_string())
            .unwrap();
        state_delta
            .put_ibc_compat_prefix(ASTRIA_COMPAT_PREFIX.to_string())
            .unwrap();

        let amount = 100;
        state_delta
            .put_ibc_channel_balance(&packet().chan_on_a, &nria(), amount)
            .unwrap();

        let bridge_address = astria_address(&[99u8; 20]);
        let destination_chain_address = "rollup-defined";
        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");

        state_delta.put_transaction_context(TransactionContext {
            address_bytes: bridge_address.bytes(),
            transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        });

        state_delta
            .put_bridge_account_rollup_id(&bridge_address, rollup_id)
            .unwrap();
        state_delta
            .put_bridge_account_ibc_asset(&bridge_address, nria())
            .unwrap();

        let packet_denom = FungibleTokenPacketData {
            denom: nria().to_string(),
            sender: bridge_address.to_string(),
            amount: amount.to_string(),
            receiver: "other-chain-address".to_string(),
            memo: serde_json::to_string(&Ics20WithdrawalFromRollup {
                memo: String::new(),
                rollup_block_number: 1,
                rollup_return_address: destination_chain_address.to_string(),
                rollup_withdrawal_event_id: "a-rollup-defined-id".into(),
            })
            .unwrap(),
        };

        refund_tokens(
            &mut state_delta,
            &Packet {
                data: serde_json::to_vec(&packet_denom).unwrap(),
                ..packet()
            },
        )
        .await
        .unwrap();

        let balance = state_delta
            .get_account_balance(&bridge_address, &nria())
            .await
            .expect("refunds of rollup withdrawals should be credited to the bridge account");
        assert_eq!(balance, amount);

        let deposits = state_delta.get_cached_block_deposits();

        let deposit = deposits
            .get(&rollup_id)
            .expect("a depsit fo the given rollup ID should exist as result of the refund")
            .first()
            .unwrap();
        let expected_deposit = Deposit {
            bridge_address,
            rollup_id,
            amount,
            asset: nria().into(),
            destination_chain_address: destination_chain_address.to_string(),
            source_transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        };
        assert_eq!(deposit, &expected_deposit);
    }

    #[tokio::test]
    async fn refund_rollup_with_source_zone_asset_compat_prefix() {
        let storage = Storage::new_temp().await;
        let mut state_delta = storage.new_delta_of_latest_snapshot();

        state_delta
            .put_base_prefix(ASTRIA_PREFIX.to_string())
            .unwrap();
        state_delta
            .put_ibc_compat_prefix(ASTRIA_COMPAT_PREFIX.to_string())
            .unwrap();

        let bridge_address = astria_address(&[99u8; 20]);
        let bridge_address_compat = astria_compat_address(&[99u8; 20]);
        let destination_chain_address = "rollup-defined-address".to_string();

        let amount = 100;
        state_delta
            .put_ibc_channel_balance(&packet().chan_on_a, &nria(), amount)
            .unwrap();

        let rollup_id = RollupId::from_unhashed_bytes(b"testchainid");

        state_delta
            .put_bridge_account_rollup_id(&bridge_address, rollup_id)
            .unwrap();
        state_delta
            .put_bridge_account_ibc_asset(&bridge_address, nria())
            .unwrap();

        let packet_data = FungibleTokenPacketData {
            denom: nria().to_string(),
            sender: bridge_address_compat.to_string(),
            amount: amount.to_string(),
            receiver: "other-chain-address".to_string(),
            memo: serde_json::to_string(&Ics20WithdrawalFromRollup {
                memo: String::new(),
                rollup_block_number: 1,
                rollup_return_address: destination_chain_address.clone(),
                rollup_withdrawal_event_id: "rollup-defined-id".to_string(),
            })
            .unwrap(),
        };

        let transaction_context = TransactionContext {
            address_bytes: bridge_address.bytes(),
            transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        };
        state_delta.put_transaction_context(transaction_context);

        refund_tokens(
            &mut state_delta,
            &Packet {
                data: serde_json::to_vec(&packet_data).unwrap(),
                ..packet()
            },
        )
        .await
        .unwrap();

        let balance = state_delta
            .get_account_balance(&bridge_address, &nria())
            .await
            .expect("refunding a rollup should add the tokens to its bridge address");
        assert_eq!(balance, amount);

        let deposits = state_delta.get_cached_block_deposits();
        assert_eq!(deposits.len(), 1);

        let deposit = deposits.get(&rollup_id).unwrap().first().unwrap();
        let expected_deposit = Deposit {
            bridge_address,
            rollup_id,
            amount,
            asset: nria().into(),
            destination_chain_address: destination_chain_address.clone(),
            source_transaction_id: TransactionId::new([0; 32]),
            source_action_index: 0,
        };
        assert_eq!(deposit, &expected_deposit);
    }
}
