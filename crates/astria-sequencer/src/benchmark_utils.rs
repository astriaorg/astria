use std::{
    collections::HashMap,
    sync::{
        Arc,
        OnceLock,
    },
};

use astria_core::{
    crypto::SigningKey,
    primitive::v1::{
        asset::{
            Denom,
            IbcPrefixed,
        },
        RollupId,
    },
    protocol::transaction::v1::{
        action,
        action::Action,
        Transaction,
        TransactionBody,
    },
};

use crate::test_utils::{
    astria_address,
    nria,
};

/// The number of different signers of transactions, and also the number of different chain IDs.
pub(crate) const SIGNER_COUNT: u8 = 10;
/// The number of transfers per transaction.
///
/// 2866 chosen after experimentation of spamming composer.
pub(crate) const TRANSFERS_PER_TX: usize = 2866;

const SEQUENCE_ACTION_TX_COUNT: usize = 100_001;
const TRANSFERS_TX_COUNT: usize = 10_000;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) enum TxTypes {
    AllSequenceActions,
    AllTransfers,
}

/// Returns an endlessly-repeating iterator over `SIGNER_COUNT` separate signing keys.
pub(crate) fn signing_keys() -> impl Iterator<Item = &'static SigningKey> {
    static SIGNING_KEYS: OnceLock<Vec<SigningKey>> = OnceLock::new();
    SIGNING_KEYS
        .get_or_init(|| {
            (0..SIGNER_COUNT)
                .map(|i| SigningKey::from([i; 32]))
                .collect()
        })
        .iter()
        .cycle()
}

/// Returns a static ref to a collection of `MAX_INITIAL_TXS + 1` transactions.
pub(crate) fn transactions(tx_types: TxTypes) -> &'static Vec<Arc<Transaction>> {
    static TXS: OnceLock<HashMap<TxTypes, Vec<Arc<Transaction>>>> = OnceLock::new();
    TXS.get_or_init(|| {
        let mut map = HashMap::new();
        map.insert(
            TxTypes::AllSequenceActions,
            rollup_data_submission_actions(),
        );
        map.insert(TxTypes::AllTransfers, transfers());
        map
    })
    .get(&tx_types)
    .unwrap()
}

#[expect(
    clippy::mutable_key_type,
    reason = "false-positive as described in \"Known problems\" of lint"
)]
fn rollup_data_submission_actions() -> Vec<Arc<Transaction>> {
    let mut nonces_and_chain_ids = HashMap::new();
    signing_keys()
        .map(move |signing_key| {
            let verification_key = signing_key.verification_key();
            let (nonce, chain_id) = nonces_and_chain_ids
                .entry(verification_key)
                .or_insert_with(|| (0_u32, format!("chain-{}", signing_key.verification_key())));
            let action = action::RollupDataSubmission {
                rollup_id: RollupId::new([1; 32]),
                data: vec![2; 1000].into(),
                fee_asset: Denom::IbcPrefixed(IbcPrefixed::new([3; 32])),
            };
            let tx = TransactionBody::builder()
                .actions(vec![Action::RollupDataSubmission(action)])
                .nonce(*nonce)
                .chain_id(chain_id.as_str())
                .try_build()
                .expect("failed to build transaction from actions")
                .sign(signing_key);
            Arc::new(tx)
        })
        .take(SEQUENCE_ACTION_TX_COUNT)
        .collect()
}

fn transfers() -> Vec<Arc<Transaction>> {
    let sender = signing_keys().next().unwrap();
    let receiver = signing_keys().nth(1).unwrap();
    let to = astria_address(&receiver.address_bytes());
    let action = Action::from(action::Transfer {
        to,
        amount: 1,
        asset: nria().into(),
        fee_asset: nria().into(),
    });
    (0..TRANSFERS_TX_COUNT)
        .map(|nonce| {
            let tx = TransactionBody::builder()
                .actions(
                    std::iter::repeat(action.clone())
                        .take(TRANSFERS_PER_TX)
                        .collect(),
                )
                .nonce(u32::try_from(nonce).unwrap())
                .chain_id("test")
                .try_build()
                .expect("failed to build transaction from actions")
                .sign(sender);
            Arc::new(tx)
        })
        .collect()
}
