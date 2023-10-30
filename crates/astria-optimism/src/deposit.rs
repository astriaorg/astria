use ethers::{
    contract::EthAbiType,
    prelude::*,
    types::transaction::optimism::DepositTransaction,
};
use eyre::WrapErr as _;

use crate::contract::TransactionDepositedFilter;

#[derive(Clone, EthAbiType)]
struct TransactionDepositedOpaqueData {
    msg_value: U256,
    value: U256,
    gas_limit: u64,
    is_creation: bool,
    data: Bytes,
}

/// Returns an L2 deposit transaction given a [`TransactionDeposited`] event and associated
/// metadata.
///
/// See the [Go type definition](https://github.com/ethereum-optimism/op-geth/blob/63125bd85c8083ff4c4a7ae3541738cb97b08ed3/core/types/deposit_tx.go#L29).
///
/// See also [the deposit spec](https://github.com/ethereum-optimism/optimism/blob/develop/specs/deposits.md#the-deposited-transaction-type).
///
/// # Errors
///
/// - if the opaque data in the event cannot be decoded.
/// - if the decoded event data cannot be converted into the correct types.
pub fn convert_deposit_event_to_deposit_tx(
    event: TransactionDepositedFilter,
    block_hash: H256,
    log_index: U256,
) -> eyre::Result<DepositTransaction> {
    let TransactionDepositedFilter {
        from,
        to,
        version: _,
        opaque_data,
    } = event;

    // abi-decode the opaque data
    let TransactionDepositedOpaqueData {
        msg_value,
        value,
        gas_limit,
        is_creation,
        data,
    } = decode_packed_opaque_data(&opaque_data).wrap_err("failed to decode opaque data")?;
    let mint = if msg_value.is_zero() {
        None
    } else {
        Some(msg_value)
    };
    let to = if is_creation { None } else { Some(to) };
    let data = if data.len() == 0 { None } else { Some(data) };

    Ok(DepositTransaction {
        tx: ethers::types::TransactionRequest {
            from: Some(from),
            to: to.map(std::convert::Into::into),
            gas: Some(gas_limit.into()),
            gas_price: None,
            value: Some(value),
            data,
            nonce: None,
            chain_id: None,
        },
        source_hash: get_user_deposit_source_hash(block_hash, log_index).into(),
        mint,
        is_system_tx: false,
    })
}

// from OptimismPortal.sol:
// `bytes memory opaqueData = abi.encodePacked(msg.value, _value, _gasLimit, _isCreation,
// _data);`
//
// ethers-rs has no decode_packed :/
// see [this issue](https://github.com/gakonst/ethers-rs/issues/2643)
fn decode_packed_opaque_data(data: &Bytes) -> eyre::Result<TransactionDepositedOpaqueData> {
    const MIN_LEN: usize = 73;
    if data.len() < MIN_LEN {
        return Err(eyre::eyre!(
            "data is too short to be packed opaque data: {} < {}",
            data.len(),
            MIN_LEN
        ));
    }

    let msg_value = U256::from_big_endian(&data[..32]);
    let value = U256::from_big_endian(&data[32..64]);
    let gas_limit = u64::from_be_bytes(data[64..72].try_into().unwrap());
    let is_creation = data[72] != 0;
    let data = data[73..].to_vec().into();
    Ok(TransactionDepositedOpaqueData {
        msg_value,
        value,
        gas_limit,
        is_creation,
        data,
    })
}

// see https://github.com/ethereum-optimism/optimism/blob/develop/specs/deposits.md#source-hash-computation
fn get_user_deposit_source_hash(block_hash: H256, log_index: U256) -> [u8; 32] {
    let mut log_index_bytes = [0u8; 32];
    log_index.to_big_endian(&mut log_index_bytes);
    let inner = ethers::utils::keccak256([block_hash.as_bytes(), &log_index_bytes].concat());
    ethers::utils::keccak256([[0u8; 32].as_ref(), &inner].concat())
}

#[cfg(test)]
mod test {
    use tokio::sync::oneshot;

    use super::*;
    use crate::contract::*;

    #[test]
    fn rlp_encode_deposit_transaction_go() {
        // result from Go
        let expected = hex::decode("f860a07113be8bbb6ff4bb99fae05639cf76cdecf5a1afbc033b9a01d8bb16b00b9a8094a0ee7a142d267c1f36714e4a8f75612f20a7972094a0ee7a142d267c1f36714e4a8f75612f20a79720872386f26fc10000872386f26fc100008252088080").unwrap();

        let from: Address = TryInto::<[u8; 20]>::try_into(
            hex::decode("a0Ee7A142d267C1f36714E4a8F75612F20a79720").unwrap(),
        )
        .unwrap()
        .try_into()
        .unwrap();
        let to: Option<Address> = Some(from);
        let value: U256 = 10_000_000_000_000_000u64.into();
        let source_hash: [u8; 32] =
            hex::decode("7113be8bbb6ff4bb99fae05639cf76cdecf5a1afbc033b9a01d8bb16b00b9a80")
                .unwrap()
                .try_into()
                .unwrap();
        let tx = DepositTransaction {
            tx: ethers::types::TransactionRequest {
                from: Some(from),
                to: to.map(std::convert::Into::into),
                gas: Some(21000.into()),
                gas_price: None,
                value: Some(value),
                data: None,
                nonce: None,
                chain_id: None,
            },
            source_hash: source_hash.into(),
            mint: Some(value),
            is_system_tx: false,
        };

        let encoded = tx.rlp();
        assert_eq!(encoded.as_ref(), &expected);
    }

    /// Listens to [`TransactionDeposited`] events on the [`OptimismPortal`] contract.
    ///
    /// # Errors
    ///
    /// - if the event stream fails to initialize.
    async fn listen_to_deposit_events(
        contract: OptimismPortal<Provider<Ws>>,
        tx: oneshot::Sender<(TransactionDepositedFilter, LogMeta)>,
    ) {
        let events = contract.event::<TransactionDepositedFilter>().from_block(1);
        let mut stream = events.stream().await.unwrap().with_meta().take(1);

        if let Some(Ok((event, meta))) = stream.next().await {
            tx.send((event, meta)).unwrap();
        } else {
            panic!("listening to TransactionDeposited event stream failed");
        }
    }

    #[tokio::test]
    async fn test_listen_to_deposit_events() {
        let (contract_address, provider, wallet, _anvil_instance) =
            crate::test_utils::deploy_mock_optimism_portal().await;

        // get contract objects
        let to = wallet.address();
        let contract = get_optimism_portal_with_signer(provider.clone(), wallet, contract_address);
        let contract_read_only = get_optimism_portal_read_only(provider, contract_address);

        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            listen_to_deposit_events(contract_read_only, tx).await;
        });

        let value = 10_000_000_000_000_000u128;

        let receipt = make_deposit_transaction(contract, Some(to), value.into(), None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(receipt.status.unwrap(), 1.into());

        let (event, meta) = rx.await.expect("expected TransactionDeposited event");
        let deposit_tx =
            convert_deposit_event_to_deposit_tx(event, meta.block_hash, meta.log_index).unwrap();
        assert_eq!(deposit_tx.tx.value.unwrap(), value.into());
    }
}
