use std::{
    error::Error,
    fmt::Display,
    str::FromStr,
};

use ibc_types::{
    core::{
        channel::{
            ChannelId,
            PortId,
        },
        client::Height as IbcHeight,
    },
    IdentifierError,
};
use penumbra_ibc::component::packet::{
    IBCPacket,
    Unchecked,
};
use penumbra_proto::penumbra::core::component::ibc::v1alpha1::FungibleTokenPacketData;

use super::{
    asset::{
        IbcAsset,
        IbcAssetError,
    },
    v1alpha1::IncorrectAddressLength,
};
use crate::{
    generated::{
        primitive::v1::Uint128,
        sequencer::v1alpha1 as raw,
    },
    native::sequencer::v1alpha1::Address,
};

#[derive(Debug, Clone)]
pub struct Ics20Withdrawal {
    // a transparent value consisting of an amount and a denom.
    pub amount: u128,
    pub denom: IbcAsset,
    // the address on the destination chain to send the transfer to.
    pub destination_chain_address: String,
    // an Astria address to use to return funds from this withdrawal
    // in the case it fails.
    pub return_address: Address,
    // the height (on Astria) at which this transfer expires (and funds are sent
    // back to the return address?). NOTE: if funds are sent back to the sender,
    // we MUST verify a nonexistence proof before accepting the timeout, to
    // prevent relayer censorship attacks. The core IBC implementation does this
    // in its handling of validation of timeouts.
    pub timeout_height: IbcHeight,
    // the timestamp at which this transfer expires.
    pub timeout_time: u64,
    // the source channel used for the withdrawal
    pub source_channel: ChannelId,
}

impl From<Ics20Withdrawal> for FungibleTokenPacketData {
    fn from(withdrawal: Ics20Withdrawal) -> Self {
        Self {
            amount: withdrawal.amount.to_string(),
            denom: withdrawal.denom.to_string(),
            sender: withdrawal.return_address.to_string(),
            receiver: withdrawal.destination_chain_address,
        }
    }
}

impl From<Ics20Withdrawal> for IBCPacket<Unchecked> {
    fn from(withdrawal: Ics20Withdrawal) -> Self {
        Self::new(
            PortId::transfer(),
            withdrawal.source_channel.clone(),
            withdrawal.timeout_height,
            withdrawal.timeout_time,
            withdrawal.packet_data(),
        )
    }
}

impl Ics20Withdrawal {
    /// Returns the JSON-encoded packet data for this withdrawal.
    ///
    /// # Panics
    ///
    /// If the packet data cannot be serialized as JSON.
    #[must_use]
    pub fn packet_data(&self) -> Vec<u8> {
        let ftpd: FungibleTokenPacketData = self.clone().into();

        // In violation of the ICS20 spec, ibc-go encodes transfer packets as JSON.
        serde_json::to_vec(&ftpd).expect("can serialize FungibleTokenPacketData as JSON")
    }

    #[must_use]
    pub fn to_raw(&self) -> raw::Ics20Withdrawal {
        raw::Ics20Withdrawal {
            amount: Some(self.amount.into()),
            denom: self.denom.to_string(),
            destination_chain_address: self.destination_chain_address.clone(),
            return_address: self.return_address.0.to_vec(),
            timeout_height: Some(self.timeout_height.into()),
            timeout_time: self.timeout_time,
            source_channel: self.source_channel.to_string(),
        }
    }

    #[must_use]
    pub fn into_raw(self) -> raw::Ics20Withdrawal {
        raw::Ics20Withdrawal {
            amount: Some(self.amount.into()),
            denom: self.denom.to_string(),
            destination_chain_address: self.destination_chain_address,
            return_address: self.return_address.0.to_vec(),
            timeout_height: Some(self.timeout_height.into()),
            timeout_time: self.timeout_time,
            source_channel: self.source_channel.to_string(),
        }
    }

    /// Convert from a raw, unchecked protobuf [`raw::Ics20Withdrawal`].
    ///
    /// # Errors
    ///
    /// - if the `amount` field is missing
    /// - if the `denom` field is invalid
    /// - if the `return_address` field is invalid
    /// - if the `timeout_height` field is missing
    /// - if the `source_channel` field is invalid
    pub fn try_from_raw(proto: raw::Ics20Withdrawal) -> Result<Self, Ics20WithdrawalError> {
        let amount: Uint128 = proto.amount.ok_or(Ics20WithdrawalError::MissingAmount)?;
        let return_address = Address::try_from_slice(&proto.return_address)
            .map_err(Ics20WithdrawalError::InvalidReturnAddress)?;
        let timeout_height = proto
            .timeout_height
            .ok_or(Ics20WithdrawalError::MissingTimeoutHeight)?
            .into();

        Ok(Self {
            amount: amount.into(),
            denom: IbcAsset::from_str(&proto.denom).map_err(Ics20WithdrawalError::InvalidDenom)?,
            destination_chain_address: proto.destination_chain_address,
            return_address,
            timeout_height,
            timeout_time: proto.timeout_time,
            source_channel: ChannelId::from_str(&proto.source_channel)
                .map_err(Ics20WithdrawalError::InvalidSourceChannel)?,
        })
    }
}

impl From<raw::IbcHeight> for IbcHeight {
    fn from(h: raw::IbcHeight) -> Self {
        Self {
            revision_number: h.revision_number,
            revision_height: h.revision_height,
        }
    }
}

impl From<IbcHeight> for raw::IbcHeight {
    fn from(h: IbcHeight) -> Self {
        Self {
            revision_number: h.revision_number,
            revision_height: h.revision_height,
        }
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub enum Ics20WithdrawalError {
    MissingAmount,
    InvalidDenom(IbcAssetError),
    InvalidReturnAddress(IncorrectAddressLength),
    MissingTimeoutHeight,
    InvalidSourceChannel(IdentifierError),
}

impl Display for Ics20WithdrawalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingAmount => f.pad("`amount` field was missing"),
            Self::InvalidDenom(_) => f.pad("`denom` field was invalid"),
            Self::InvalidReturnAddress(_) => f.pad("`return_address` field was invalid"),
            Self::MissingTimeoutHeight => f.pad("`timeout_height` field was missing"),
            Self::InvalidSourceChannel(_) => f.pad("`source_channel` field was invalid"),
        }
    }
}

impl Error for Ics20WithdrawalError {}
