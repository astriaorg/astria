use std::{
    error::Error,
    fmt::Display,
    str::FromStr,
};

use ibc_types::{
    core::{
        channel::ChannelId,
        client::Height as IbcHeight,
    },
    IdentifierError,
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
    amount: u128,
    denom: IbcAsset,
    // the address on the destination chain to send the transfer to.
    destination_chain_address: String,
    // an Astria address to use to return funds from this withdrawal
    // in the case it fails.
    return_address: Address,
    // the height (on Astria) at which this transfer expires.
    timeout_height: IbcHeight,
    // the timestamp at which this transfer expires.
    timeout_time: u64,
    // the source channel used for the withdrawal.
    source_channel: ChannelId,
}

impl Ics20Withdrawal {
    #[must_use]
    pub fn amount(&self) -> u128 {
        self.amount
    }

    #[must_use]
    pub fn denom(&self) -> &IbcAsset {
        &self.denom
    }

    #[must_use]
    pub fn destination_chain_address(&self) -> &str {
        &self.destination_chain_address
    }

    #[must_use]
    pub fn return_address(&self) -> &Address {
        &self.return_address
    }

    #[must_use]
    pub fn timeout_height(&self) -> &IbcHeight {
        &self.timeout_height
    }

    #[must_use]
    pub fn timeout_time(&self) -> u64 {
        self.timeout_time
    }

    #[must_use]
    pub fn source_channel(&self) -> &ChannelId {
        &self.source_channel
    }

    #[must_use]
    pub fn to_fungible_token_packet_data(&self) -> FungibleTokenPacketData {
        FungibleTokenPacketData {
            amount: self.amount.to_string(),
            denom: self.denom.to_string(),
            sender: self.return_address.to_string(),
            receiver: self.destination_chain_address.clone(),
        }
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

impl crate::native::Protobuf for IbcHeight {
    type Error = ::std::convert::Infallible;
    type Raw = raw::IbcHeight;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        Ok(Self {
            revision_number: raw.revision_number,
            revision_height: raw.revision_height,
        })
    }

    fn try_from_raw(h: Self::Raw) -> Result<Self, Self::Error> {
        Ok(Self {
            revision_number: h.revision_number,
            revision_height: h.revision_height,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        Self::Raw {
            revision_number: self.revision_number,
            revision_height: self.revision_height,
        }
    }

    fn into_raw(self) -> Self::Raw {
        Self::Raw {
            revision_number: self.revision_number,
            revision_height: self.revision_height,
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

impl Error for Ics20WithdrawalError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::MissingAmount | Self::MissingTimeoutHeight => None,
            Self::InvalidDenom(e) => Some(e),
            Self::InvalidReturnAddress(e) => Some(e),
            Self::InvalidSourceChannel(e) => Some(e),
        }
    }
}
