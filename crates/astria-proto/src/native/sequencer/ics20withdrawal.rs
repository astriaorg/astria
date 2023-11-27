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
    // the address on the destination chain to send the transfer to
    pub destination_chain_address: String,
    // a "sender" Astria address to use to return funds from this withdrawal.
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

// impl Ics20Withdrawal {
//     pub fn packet_data(&self) -> Vec<u8> {
//         let ftpd: FungibleTokenPacketData = self.clone().into();

//         // In violation of the ICS20 spec, ibc-go encodes transfer packets as JSON.
//         serde_json::to_vec(&ftpd).expect("can serialize FungibleTokenPacketData as JSON")
//     }

//     // stateless validation of an Ics20 withdrawal action.
//     pub fn validate(&self) -> anyhow::Result<()> {
//         if self.timeout_time == 0 {
//             anyhow::bail!("timeout time must be non-zero");
//         }

//         // NOTE: we could validate the destination chain address as bech32 to prevent mistyped
//         // addresses, but this would preclude sending to chains that don't use bech32 addresses.

//         Ok(())
//     }
// }

impl Ics20Withdrawal {
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
