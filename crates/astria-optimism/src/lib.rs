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

/// Returns a new ready-only OptimismPortal contract instance.
pub fn get_optimism_portal_read_only<P: JsonRpcClient>(
    provider: Arc<Provider<P>>,
    contract_address: Address,
) -> OptimismPortal<Provider<P>> {
    OptimismPortal::new(contract_address, provider)
}

/// Returns a new OptimismPortal contract instance with a signer.
pub fn get_optimism_portal_with_signer<P: JsonRpcClient>(
    provider: Provider<P>,
    wallet: Wallet<SigningKey>,
    contract_address: Address,
) -> OptimismPortal<SignerMiddleware<Provider<P>, Wallet<SigningKey>>> {
    let signer = SignerMiddleware::new(provider, wallet.with_chain_id(31337u64));
    let client = std::sync::Arc::new(signer);
    OptimismPortal::new(contract_address, client)
}

/// Calls `depositTransaction` on the OptimismPortal contract, which
/// makes an L2 deposit.
///
/// Set `to` to `None` for contract creation.
///
/// Returns the transaction receipt.
pub async fn make_deposit_transaction<M: Middleware + 'static>(
    contract: OptimismPortal<M>,
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
        let provider = Provider::<Http>::try_from("http://localhost:8545").unwrap();
        let wallet = "2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6"
            .parse::<LocalWallet>()
            .unwrap();
        let contract_addr: [u8; 20] = hex::decode("F87a0abe1b875489CA84ab1E4FE47A2bF52C7C64")
            .unwrap()
            .try_into()
            .unwrap();
        let contract = get_optimism_portal_with_signer(provider, wallet, contract_addr.into());

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
