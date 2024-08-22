use astria_core::{
    generated::astria_vendored::tendermint::abci as raw,
    protocol::transaction::v1alpha1::action::ValidatorUpdate,
    Protobuf as _,
};
use astria_eyre::eyre::{
    eyre,
    Result,
    WrapErr as _,
};

pub(crate) struct Hex<'a>(pub(crate) &'a [u8]);

impl<'a> std::fmt::Display for Hex<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            f.write_fmt(format_args!("{byte:02x}"))?;
        }
        Ok(())
    }
}

pub(crate) fn anyhow_to_eyre(anyhow_error: anyhow::Error) -> astria_eyre::eyre::Report {
    let anyhow_result = Err::<(), _>(anyhow_error);
    let boxed: Result<(), Box<dyn std::error::Error + Send + Sync>> =
        anyhow_result.map_err(std::convert::Into::into);
    let Err(err) = boxed else {
        panic!("anyhow_to_eyre called on `Ok`")
    };
    eyre!(err)
}

pub(crate) fn eyre_to_anyhow(eyre_error: astria_eyre::eyre::Report) -> anyhow::Error {
    let eyre_result = Err::<(), _>(eyre_error);
    let boxed: Result<(), Box<dyn std::error::Error + Send + Sync>> =
        eyre_result.map_err(std::convert::Into::into);
    let Err(err) = boxed else {
        panic!("eyre_to_anyhow called on `Ok`")
    };
    anyhow::anyhow!(err)
}

pub(crate) fn cometbft_to_sequencer_validator(
    value: tendermint::validator::Update,
) -> Result<ValidatorUpdate> {
    let tendermint_proto::abci::ValidatorUpdate {
        pub_key,
        power,
    } = value.into();
    ValidatorUpdate::try_from_raw(raw::ValidatorUpdate {
        power,
        pub_key: pub_key.map(pubkey::cometbft_to_astria),
    })
    .wrap_err("failed converting cometbft validator update to astria validator update")
}

pub(crate) fn sequencer_to_cometbft_validator(
    value: ValidatorUpdate,
) -> Result<tendermint::validator::Update> {
    let astria_core::generated::astria_vendored::tendermint::abci::ValidatorUpdate {
        pub_key,
        power,
    } = value.into_raw();
    tendermint_proto::abci::ValidatorUpdate {
        pub_key: pub_key.map(pubkey::astria_to_cometbft),
        power,
    }
    .try_into()
    .wrap_err("failed converting astria validator update to cometbft validator update")
}

mod pubkey {
    use astria_core::generated::astria_vendored::tendermint::crypto::{
        public_key::Sum as AstriaSum,
        PublicKey as AstriaKey,
    };
    use tendermint_proto::crypto::{
        public_key::Sum as CometbftSum,
        PublicKey as CometbftKey,
    };

    pub(super) fn astria_to_cometbft(key: AstriaKey) -> CometbftKey {
        let AstriaKey {
            sum,
        } = key;
        let sum = match sum {
            Some(AstriaSum::Ed25519(bytes)) => Some(CometbftSum::Ed25519(bytes)),
            Some(AstriaSum::Secp256k1(bytes)) => Some(CometbftSum::Secp256k1(bytes)),
            None => None,
        };
        CometbftKey {
            sum,
        }
    }

    pub(super) fn cometbft_to_astria(key: CometbftKey) -> AstriaKey {
        let CometbftKey {
            sum,
        } = key;
        let sum = match sum {
            Some(CometbftSum::Ed25519(bytes)) => Some(AstriaSum::Ed25519(bytes)),
            Some(CometbftSum::Secp256k1(bytes)) => Some(AstriaSum::Secp256k1(bytes)),
            None => None,
        };
        AstriaKey {
            sum,
        }
    }
}
