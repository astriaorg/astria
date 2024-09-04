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
    let boxed: Box<dyn std::error::Error + Send + Sync> = anyhow_error.into();
    eyre!(boxed)
}

pub(crate) fn eyre_to_anyhow(eyre_error: astria_eyre::eyre::Report) -> anyhow::Error {
    let boxed: Box<dyn std::error::Error + Send + Sync> = eyre_error.into();
    anyhow::anyhow!(boxed)
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

mod test {
    #[test]
    fn anyhow_to_eyre_preserves_source_chain() {
        let mut errs = ["foo", "bar", "baz", "qux"];
        let anyhow_error = anyhow::anyhow!(errs[0]).context(errs[1]).context(errs[2]);
        let eyre_from_anyhow = super::anyhow_to_eyre(anyhow_error).wrap_err(errs[3]);

        errs.reverse();
        for (i, err) in eyre_from_anyhow.chain().enumerate() {
            assert_eq!(errs[i], &err.to_string());
        }
    }

    #[test]
    fn eyre_to_anyhow_preserves_source_chain() {
        let mut errs = ["foo", "bar", "baz", "qux"];
        let eyre_error = astria_eyre::eyre::eyre!(errs[0]).wrap_err(errs[1]).wrap_err(errs[2]);
        let anyhow_from_eyre = super::eyre_to_anyhow(eyre_error).context(errs[3]);

        errs.reverse();
        for (i, err) in anyhow_from_eyre.chain().enumerate() {
            assert_eq!(errs[i], &err.to_string());
        }
    }
}
