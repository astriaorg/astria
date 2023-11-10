/// This module contains functions for interacting with the OptimismPortal.sol contract,
/// which is part of Optimism Bedrock's L1 contracts.
/// See [the contract](https://github.com/ethereum-optimism/optimism/blob/9a13504bb1f302ca9d412589aac18d589c055f16/packages/contracts-bedrock/src/L1/OptimismPortal.sol).
///
/// This function requires an `OptimismPortal.abi` file in the project root, which
/// contains the ABI of the above contract. This is used for generating the Rust
/// contract bindings with abigen.
use std::sync::Arc;

use ethers::prelude::*;
use eyre::WrapErr as _;
use k256::ecdsa::SigningKey;

abigen!(
    OptimismPortal,
    "./OptimismPortal.abi",
    methods {
        l2Oracle() as renamedl2Oracle;
    },
);

/// Returns a new read-only [`OptimismPortal`] contract instance.
pub fn get_optimism_portal_read_only<P: JsonRpcClient>(
    provider: Arc<Provider<P>>,
    contract_address: Address,
) -> OptimismPortal<Provider<P>> {
    OptimismPortal::new(contract_address, provider)
}

/// Returns a new [`OptimismPortal`] contract instance with a signer.
pub fn get_optimism_portal_with_signer<P: JsonRpcClient>(
    provider: Arc<Provider<P>>,
    wallet: Wallet<SigningKey>,
    contract_address: Address,
) -> OptimismPortal<SignerMiddleware<Arc<Provider<P>>, Wallet<SigningKey>>> {
    let signer = SignerMiddleware::new(provider, wallet);
    let client = std::sync::Arc::new(signer);
    OptimismPortal::new(contract_address, client)
}

/// Calls `depositTransaction` on the [`OptimismPortal`] contract, which
/// makes an L2 deposit.
///
/// Set `to` to `None` for contract creation.
///
/// Returns the transaction receipt.
///
/// # Errors
///
/// - if the transaction fails to submit.
/// - if the pending transaction fails to be included.
pub async fn make_deposit_transaction<M: Middleware + 'static>(
    contract: &OptimismPortal<M>,
    to: Option<Address>,
    value: U256,
    data: Option<Bytes>,
) -> eyre::Result<Option<TransactionReceipt>> {
    let to = to.unwrap_or_default();
    let gas_limit = get_minimum_gas_limit(0);
    let data = data.unwrap_or_default();
    let tx = contract
        .deposit_transaction(to, value, gas_limit, false, data)
        .value(value);
    let receipt = tx
        .send()
        .await
        .wrap_err("failed to submit transaction")?
        .await
        .wrap_err("failed to await pending transaction")?;
    Ok(receipt)
}

/// Returns the minimum gas limit for a deposit transaction with the given data length.
fn get_minimum_gas_limit(data_len: usize) -> u64 {
    let base = 21000;
    let per_byte = 16;
    base + (data_len as u64) * per_byte
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_make_deposit_transaction() {
        let (contract_address, provider, wallet, _anvil_instance) =
            crate::test_utils::deploy_mock_optimism_portal().await;

        // get contract object
        let to = wallet.address();
        let contract = get_optimism_portal_with_signer(provider, wallet, contract_address);

        // submit deposit transaction
        let value = 10_000_000_000_000_000u128;
        let receipt: TransactionReceipt =
            make_deposit_transaction(&contract, Some(to), value.into(), None)
                .await
                .unwrap()
                .unwrap();
        assert_eq!(receipt.status.unwrap(), 1.into());
    }
}
