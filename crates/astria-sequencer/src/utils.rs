use astria_core::{
    Protobuf as _,
    generated::astria_vendored::tendermint::abci as raw,
    protocol::transaction::v1alpha1::action::ValidatorUpdate,
    sequencerblock::v1alpha1::block::Deposit,
};
use astria_eyre::eyre::{
    Result,
    WrapErr as _,
};
use tendermint::abci::{
    self,
    EventAttributeIndexExt as _,
};

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

pub(crate) fn create_deposit_event(deposit: &Deposit) -> abci::Event {
    abci::Event::new("tx.deposit", [
        ("bridgeAddress", deposit.bridge_address.to_string()).index(),
        ("rollupId", deposit.rollup_id.to_string()).index(),
        ("amount", deposit.amount.to_string()).index(),
        ("asset", deposit.asset.to_string()).index(),
        (
            "destinationChainAddress",
            deposit.destination_chain_address.to_string(),
        )
            .index(),
        (
            "sourceTransactionId",
            deposit.source_transaction_id.to_string(),
        )
            .index(),
        ("sourceActionIndex", deposit.source_action_index.to_string()).index(),
    ])
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
        PublicKey as AstriaKey,
        public_key::Sum as AstriaSum,
    };
    use tendermint_proto::crypto::{
        PublicKey as CometbftKey,
        public_key::Sum as CometbftSum,
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
