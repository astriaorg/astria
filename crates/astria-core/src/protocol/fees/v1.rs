use std::{
    fmt::{
        self,
        Debug,
        Formatter,
    },
    marker::PhantomData,
};

use penumbra_ibc::IbcRelay;
use prost::Name as _;

use crate::{
    generated::astria::protocol::fees::v1 as raw,
    primitive::v1::asset,
    protocol::transaction::v1::action::{
        BridgeLock,
        BridgeSudoChange,
        BridgeTransfer,
        BridgeUnlock,
        FeeAssetChange,
        FeeChange,
        IbcRelayerChange,
        IbcSudoChange,
        Ics20Withdrawal,
        InitBridgeAccount,
        RecoverIbcClient,
        RollupDataSubmission,
        SudoAddressChange,
        Transfer,
        ValidatorUpdate,
    },
    Protobuf,
};

#[derive(Debug, thiserror::Error)]
#[error("failed validating on-wire type `{on_wire}` as domain type")]
pub struct FeeComponentError {
    on_wire: String,
    source: FeeComponentErrorKind,
}

impl FeeComponentError {
    fn missing_field(on_wire: String, field: &'static str) -> Self {
        Self {
            on_wire,
            source: FeeComponentErrorKind::MissingField {
                field,
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum FeeComponentErrorKind {
    #[error("field `{field}` was not set")]
    MissingField { field: &'static str },
}

macro_rules! impl_protobuf_for_fee_components {
    ( $( $domain_ty:ty => $raw_ty:ty ),* $(,)?) => {
        $(
            impl Protobuf for $domain_ty {
                type Error = FeeComponentError;
                type Raw = $raw_ty;

                fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
                    let Self::Raw {
                        base,
                        multiplier,
                    } = raw;
                    Ok(Self {
                        base: base
                            .ok_or_else(|| Self::Error::missing_field(Self::Raw::full_name(), "base"))?
                            .into(),
                        multiplier: multiplier
                            .ok_or_else(|| Self::Error::missing_field(Self::Raw::full_name(), "multiplier"))?
                            .into(),
                        _phantom: PhantomData,
                    })
                }

                fn to_raw(&self) -> Self::Raw {
                    let Self {
                        base,
                        multiplier,
                        _phantom,
                    } = self;
                    Self::Raw {
                        base: Some(base.into()),
                        multiplier: Some(multiplier.into()),
                    }
                }
            }
        )*
    };
}
impl_protobuf_for_fee_components!(
    FeeComponents<Transfer> => raw::TransferFeeComponents,
    FeeComponents<RollupDataSubmission> => raw::RollupDataSubmissionFeeComponents,
    FeeComponents<Ics20Withdrawal> => raw::Ics20WithdrawalFeeComponents,
    FeeComponents<InitBridgeAccount> => raw::InitBridgeAccountFeeComponents,
    FeeComponents<BridgeLock> => raw::BridgeLockFeeComponents,
    FeeComponents<BridgeUnlock> => raw::BridgeUnlockFeeComponents,
    FeeComponents<BridgeTransfer> => raw::BridgeTransferFeeComponents,
    FeeComponents<BridgeSudoChange> => raw::BridgeSudoChangeFeeComponents,
    FeeComponents<ValidatorUpdate> => raw::ValidatorUpdateFeeComponents,
    FeeComponents<IbcRelayerChange> => raw::IbcRelayerChangeFeeComponents,
    FeeComponents<IbcRelay> => raw::IbcRelayFeeComponents,
    FeeComponents<FeeAssetChange> => raw::FeeAssetChangeFeeComponents,
    FeeComponents<FeeChange> => raw::FeeChangeFeeComponents,
    FeeComponents<SudoAddressChange> => raw::SudoAddressChangeFeeComponents,
    FeeComponents<IbcSudoChange> => raw::IbcSudoChangeFeeComponents,
    FeeComponents<RecoverIbcClient> => raw::RecoverIbcClientFeeComponents,
);

pub struct FeeComponents<T: ?Sized> {
    base: u128,
    multiplier: u128,
    _phantom: PhantomData<T>,
}

impl<T: ?Sized> FeeComponents<T> {
    #[must_use]
    pub fn new(base: u128, multiplier: u128) -> Self {
        Self {
            base,
            multiplier,
            _phantom: PhantomData,
        }
    }

    #[must_use]
    pub fn base(&self) -> u128 {
        self.base
    }

    #[must_use]
    pub fn multiplier(&self) -> u128 {
        self.multiplier
    }
}

impl<T: Protobuf> Debug for FeeComponents<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct(&format!("FeeComponents<{}>", T::Raw::NAME))
            .field("base", &self.base)
            .field("multiplier", &self.multiplier)
            .finish()
    }
}

impl Debug for FeeComponents<IbcRelay> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("FeeComponents<IbcRelay>")
            .field("base", &self.base)
            .field("multiplier", &self.multiplier)
            .finish()
    }
}

impl<T: ?Sized> Clone for FeeComponents<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for FeeComponents<T> {}

impl<T: ?Sized> PartialEq for FeeComponents<T> {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base && self.multiplier == other.multiplier
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecoverIbcClientFeeComponents {
    pub base: u128,
    pub multiplier: u128,
}

#[derive(Debug, Clone)]
pub struct TransactionFeeResponse {
    pub height: u64,
    pub fees: Vec<(asset::Denom, u128)>,
}

impl TransactionFeeResponse {
    #[must_use]
    pub fn into_raw(self) -> raw::TransactionFeeResponse {
        raw::TransactionFeeResponse {
            height: self.height,
            fees: self
                .fees
                .into_iter()
                .map(|(asset, fee)| raw::TransactionFee {
                    asset: asset.to_string(),
                    fee: Some(fee.into()),
                })
                .collect(),
        }
    }

    /// Attempt to convert from a raw protobuf [`raw::TransactionFeeResponse`].
    ///
    /// # Errors
    ///
    /// - if the asset ID could not be converted from bytes
    /// - if the fee was unset
    pub fn try_from_raw(
        proto: raw::TransactionFeeResponse,
    ) -> Result<Self, TransactionFeeResponseError> {
        let raw::TransactionFeeResponse {
            height,
            fees,
        } = proto;
        let fees = fees
            .into_iter()
            .map(
                |raw::TransactionFee {
                     asset,
                     fee,
                 }| {
                    let asset = asset.parse().map_err(TransactionFeeResponseError::asset)?;
                    let fee = fee.ok_or(TransactionFeeResponseError::unset_fee())?;
                    Ok((asset, fee.into()))
                },
            )
            .collect::<Result<_, _>>()?;
        Ok(Self {
            height,
            fees,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct TransactionFeeResponseError(TransactionFeeResponseErrorKind);

impl TransactionFeeResponseError {
    fn unset_fee() -> Self {
        Self(TransactionFeeResponseErrorKind::UnsetFee)
    }

    fn asset(inner: asset::ParseDenomError) -> Self {
        Self(TransactionFeeResponseErrorKind::Asset(inner))
    }
}

#[derive(Debug, thiserror::Error)]
enum TransactionFeeResponseErrorKind {
    #[error("`fee` field is unset")]
    UnsetFee,
    #[error("failed to parse asset denom in the `assets` field")]
    Asset(#[source] asset::ParseDenomError),
}
