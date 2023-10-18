use ethers::{
    abi::ParamType,
    contract::EthAbiType,
    prelude::*,
    types::transaction::optimism_deposited::OptimismDepositedTransactionRequest as DepositTransaction,
};

use crate::contract::{
    OptimismPortal,
    TransactionDepositedFilter,
};

/// Listens to [`TransactionDeposited`] events on the [`OptimismPortal`] contract.
///
/// # Errors
///
/// - if the event stream fails to initialize.
pub async fn listen_to_deposit_events(contract: OptimismPortal<Provider<Ws>>) -> eyre::Result<()> {
    let events = contract.event::<TransactionDepositedFilter>().from_block(1);
    let mut stream = events.stream().await?.with_meta().take(1);

    while let Some(Ok((event, meta))) = stream.next().await {
        println!("TransactionDeposited event: {event:?} {meta:?}");
    }

    Ok(())
}

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
pub fn convert_deposit_event_to_deposit_tx(
    event: TransactionDepositedFilter,
    block_hash: H256,
    log_index: U256,
) -> eyre::Result<DepositTransaction> {
    use abi::Detokenize as _;

    let TransactionDepositedFilter {
        from,
        to,
        version: _,
        opaque_data,
    } = event;

    // from OptimismPortal.sol:
    // `bytes memory opaqueData = abi.encodePacked(msg.value, _value, _gasLimit, _isCreation,
    // _data);`
    let opaque_data_param_types = vec![
        ParamType::Uint(256),
        ParamType::Uint(256),
        ParamType::Uint(64),
        ParamType::Bool,
        ParamType::Bytes,
    ];

    // abi-decode the opaque data
    let tokens = abi::decode_whole(&opaque_data_param_types, &opaque_data)?;
    let TransactionDepositedOpaqueData {
        msg_value,
        value,
        gas_limit,
        is_creation,
        data,
    } = TransactionDepositedOpaqueData::from_tokens(tokens)?;
    let mint = if msg_value.is_zero() {
        None
    } else {
        Some(msg_value)
    };
    let to = if is_creation { None } else { Some(to) };

    Ok(DepositTransaction {
        tx: ethers::types::TransactionRequest {
            from: from.into(),
            to: to.map(std::convert::Into::into),
            gas: Some(gas_limit.into()),
            gas_price: None,
            value: Some(value),
            data: Some(data),
            nonce: None,
            chain_id: None,
        },
        source_hash: Some(get_user_deposit_source_hash(block_hash, log_index).into()),
        mint,
        is_system_tx: Some(false),
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
    use std::sync::Arc;

    use super::*;
    use crate::contract::*;

    #[tokio::test]
    async fn test_listen_to_deposit_events() {
        let provider = Arc::new(
            Provider::<Ws>::connect("ws://localhost:8545")
                .await
                .unwrap(),
        );
        let wallet = "2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6"
            .parse::<LocalWallet>()
            .unwrap();
        let contract_address: [u8; 20] = hex::decode("F87a0abe1b875489CA84ab1E4FE47A2bF52C7C64")
            .unwrap()
            .try_into()
            .unwrap();
        let contract =
            get_optimism_portal_with_signer(provider.clone(), wallet, contract_address.into());
        let contract_read_only = get_optimism_portal_read_only(provider, contract_address.into());

        tokio::spawn(async move {
            listen_to_deposit_events(contract_read_only).await.unwrap();
        });

        let to: [u8; 20] = hex::decode("a0Ee7A142d267C1f36714E4a8F75612F20a79720")
            .unwrap()
            .try_into()
            .unwrap();
        let value = 10_000_000_000_000_000u128;

        let receipt = make_deposit_transaction(contract, Some(to.into()), value.into(), None)
            .await
            .unwrap()
            .unwrap();
        println!("{receipt:?}");
        assert_eq!(receipt.status.unwrap(), 1.into());
    }
}
