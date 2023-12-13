use std::str::FromStr;

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
    native::{
        sequencer::v1alpha1::Address,
        Protobuf,
    },
};

/// Represents an IBC withdrawal of an asset from a source chain to a destination chain.
///
/// The parameters match the arguments to the `sendFungibleTokens` function in the
/// [ICS 20 spec](https://github.com/cosmos/ibc/blob/fe150abb629de5c1a598e8c7896a7568f2083681/spec/app/ics-020-fungible-token-transfer/README.md#packet-relay).
///
/// Note that it does not contain `source_port` as that is implicit (it uses the `transfer`) port.
///
/// It also contains a `return_address` field which may or may not be the same as the signer
/// of the packet. The funds will be returned to the `return_address` in the case of a timeout.
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
    // the unix timestamp (in nanoseconds) at which this transfer expires.
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
            return_address: self.return_address.to_vec(),
            timeout_height: Some(self.timeout_height.into_raw()),
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
            return_address: self.return_address.to_vec(),
            timeout_height: Some(self.timeout_height.into_raw()),
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
        let amount: Uint128 = proto.amount.ok_or(Ics20WithdrawalError::missing_amount())?;
        let return_address = Address::try_from_slice(&proto.return_address)
            .map_err(Ics20WithdrawalError::invalid_return_address)?;
        let timeout_height = proto
            .timeout_height
            .ok_or(Ics20WithdrawalError::missing_timeout_height())?
            .into();

        Ok(Self {
            amount: amount.into(),
            denom: IbcAsset::from_str(&proto.denom).map_err(Ics20WithdrawalError::invalid_denom)?,
            destination_chain_address: proto.destination_chain_address,
            return_address,
            timeout_height,
            timeout_time: proto.timeout_time,
            source_channel: ChannelId::from_str(&proto.source_channel)
                .map_err(Ics20WithdrawalError::invalid_source_channel)?,
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

impl Protobuf for IbcHeight {
    type Error = ::std::convert::Infallible;
    type Raw = raw::IbcHeight;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        Ok(Self {
            revision_number: raw.revision_number,
            revision_height: raw.revision_height,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        Self::Raw {
            revision_number: self.revision_number,
            revision_height: self.revision_height,
        }
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Ics20WithdrawalError(Ics20WithdrawalErrorKind);

impl Ics20WithdrawalError {
    #[must_use]
    pub fn missing_amount() -> Self {
        Self(Ics20WithdrawalErrorKind::MissingAmount)
    }

    #[must_use]
    pub fn invalid_denom(err: IbcAssetError) -> Self {
        Self(Ics20WithdrawalErrorKind::InvalidDenom(err))
    }

    #[must_use]
    pub fn invalid_return_address(err: IncorrectAddressLength) -> Self {
        Self(Ics20WithdrawalErrorKind::InvalidReturnAddress(err))
    }

    #[must_use]
    pub fn missing_timeout_height() -> Self {
        Self(Ics20WithdrawalErrorKind::MissingTimeoutHeight)
    }

    #[must_use]
    pub fn invalid_source_channel(err: IdentifierError) -> Self {
        Self(Ics20WithdrawalErrorKind::InvalidSourceChannel(err))
    }
}

#[derive(Debug, thiserror::Error)]
enum Ics20WithdrawalErrorKind {
    #[error("`amount` field was missing")]
    MissingAmount,
    #[error("`denom` field was invalid")]
    InvalidDenom(IbcAssetError),
    #[error("`return_address` field was invalid")]
    InvalidReturnAddress(IncorrectAddressLength),
    #[error("`timeout_height` field was missing")]
    MissingTimeoutHeight,
    #[error("`source_channel` field was invalid")]
    InvalidSourceChannel(IdentifierError),
}
