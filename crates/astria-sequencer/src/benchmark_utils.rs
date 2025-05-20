use std::{
    collections::HashMap,
    sync::{
        Arc,
        OnceLock,
    },
};

use astria_core::{
    crypto::SigningKey,
    protocol::transaction::v1::action::Transfer,
};

use crate::{
    checked_transaction::CheckedTransaction,
    test_utils::{
        astria_address,
        nria,
        Fixture,
    },
};

/// The number of different signers of transactions.
pub(crate) const SIGNER_COUNT: u8 = 10;
/// The number of transfers per transaction.
///
/// 2866 chosen after experimentation of spamming composer.
pub(crate) const TRANSFERS_PER_TX: usize = 2866;

const ROLLUP_DATA_TX_COUNT: usize = 100_001;
const TRANSFERS_TX_COUNT: usize = 1_000;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) enum TxTypes {
    AllRollupDataSubmissions,
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
pub(crate) fn transactions(tx_types: TxTypes) -> &'static Vec<Arc<CheckedTransaction>> {
    static TXS: OnceLock<HashMap<TxTypes, Vec<Arc<CheckedTransaction>>>> = OnceLock::new();
    TXS.get_or_init(|| {
        let fixture = new_fixture();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();

        let mut map = HashMap::new();
        map.insert(
            TxTypes::AllRollupDataSubmissions,
            runtime.block_on(async { rollup_data_submission_actions(&fixture).await }),
        );
        map.insert(
            TxTypes::AllTransfers,
            runtime.block_on(async { transfers(&fixture).await }),
        );
        map
    })
    .get(&tx_types)
    .unwrap()
}

/// Returns a new test [`Fixture`] where all accounts under [`signing_keys`] have been funded at
/// genesis, and where the first of these is used as the sudo address and IBC sudo address.
pub(crate) fn new_fixture() -> Fixture {
    let accounts = signing_keys()
        .enumerate()
        .take(usize::from(SIGNER_COUNT))
        .map(|(index, signing_key)| {
            let address = astria_address(&signing_key.address_bytes());
            let balance = 10_u128
                .pow(19)
                .saturating_add(u128::try_from(index).unwrap());
            (address, balance)
        });

    let first_address = astria_address(&signing_keys().next().unwrap().address_bytes());

    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    runtime.block_on(async {
        let mut fixture = Fixture::uninitialized(None).await;
        fixture
            .chain_initializer()
            .with_genesis_accounts(accounts)
            .with_authority_sudo_address(first_address)
            .with_ibc_sudo_address(first_address)
            .init()
            .await;
        fixture
    })
}

#[expect(
    clippy::mutable_key_type,
    reason = "false-positive as described in \"Known problems\" of lint"
)]
async fn rollup_data_submission_actions(fixture: &Fixture) -> Vec<Arc<CheckedTransaction>> {
    let mut nonces = HashMap::new();
    let mut txs = Vec::with_capacity(ROLLUP_DATA_TX_COUNT);
    for signing_key in signing_keys().take(ROLLUP_DATA_TX_COUNT) {
        let nonce = nonces
            .entry(signing_key.verification_key())
            .or_insert(0_u32);
        let tx = fixture
            .checked_tx_builder()
            .with_rollup_data_submission(vec![2; 1000])
            .with_signer(signing_key.clone())
            .with_nonce(*nonce)
            .build()
            .await;
        txs.push(tx);
        *nonce = (*nonce).wrapping_add(1);
    }
    txs
}

async fn transfers(fixture: &Fixture) -> Vec<Arc<CheckedTransaction>> {
    let sender = signing_keys().next().unwrap();
    let receiver = signing_keys().nth(1).unwrap();
    let to = astria_address(&receiver.address_bytes());
    let transfer = Transfer {
        to,
        amount: 1,
        asset: nria().into(),
        fee_asset: nria().into(),
    };
    let mut txs = Vec::with_capacity(TRANSFERS_TX_COUNT);
    for nonce in 0..TRANSFERS_TX_COUNT {
        let mut tx_builder = fixture.checked_tx_builder();
        for _ in 0..TRANSFERS_PER_TX {
            tx_builder = tx_builder.with_action(transfer.clone());
        }
        let tx = tx_builder
            .with_nonce(u32::try_from(nonce).unwrap())
            .with_signer(sender.clone())
            .build()
            .await;
        txs.push(tx);
    }
    txs
}
