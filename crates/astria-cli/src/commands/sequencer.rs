use astria_core::sequencer::v1::{
    transaction::action::{
        Action,
        BridgeLockAction,
        InitBridgeAccountAction,
        TransferAction,
    },
    UnsignedTransaction,
};
use astria_sequencer_client::{
    tendermint_rpc::endpoint,
    Address,
    HttpClient,
    SequencerClientExt,
};
use color_eyre::{
    eyre,
    eyre::{
        ensure,
        eyre,
        Context,
    },
};
use ed25519_consensus::SigningKey;
use rand::rngs::OsRng;

use crate::cli::sequencer::{
    BasicAccountArgs,
    BlockHeightGetArgs,
    BridgeLockArgs,
    InitBridgeAccountArgs,
    TransferArgs,
};

/// Generate a new signing key (this is also called a secret key by other implementations)
fn get_new_signing_key() -> SigningKey {
    SigningKey::new(OsRng)
}

/// Get the public key from the signing key
fn get_public_key_pretty(signing_key: &SigningKey) -> String {
    let verifying_key_bytes = signing_key.verification_key().to_bytes();
    hex::encode(verifying_key_bytes)
}

/// Get the private key from the signing key
fn get_private_key_pretty(signing_key: &SigningKey) -> String {
    let secret_key_bytes = signing_key.to_bytes();
    hex::encode(secret_key_bytes)
}

/// Get the address from the signing key
fn get_address_pretty(signing_key: &SigningKey) -> String {
    let address = Address::from_verification_key(signing_key.verification_key());
    hex::encode(address.to_vec())
}

/// Generates a new ED25519 keypair and prints the public key, private key, and address
pub(crate) fn create_account() {
    let signing_key = get_new_signing_key();
    let public_key_pretty = get_public_key_pretty(&signing_key);
    let private_key_pretty = get_private_key_pretty(&signing_key);
    let address_pretty = get_address_pretty(&signing_key);

    println!("Create Sequencer Account");
    println!();
    // TODO: don't print private keys to CLI, prefer writing to file:
    // https://github.com/astriaorg/astria/issues/594
    println!("Private Key: {private_key_pretty:?}");
    println!("Public Key:  {public_key_pretty:?}");
    println!("Address:     {address_pretty:?}");
}

/// Gets the balance of a Sequencer account
///
/// # Arguments
///
/// * `args` - The arguments passed to the command
///
/// # Errors
///
/// * If the http client cannot be created
/// * If the balance cannot be retrieved
pub(crate) async fn get_balance(args: &BasicAccountArgs) -> eyre::Result<()> {
    let address = &args.address;
    let sequencer_client = HttpClient::new(args.sequencer_url.as_str())
        .wrap_err("failed constructing http sequencer client")?;

    let res = sequencer_client
        .get_latest_balance(address.0)
        .await
        .wrap_err("failed to get balance")?;

    println!("Balances for address {}:", address.0);
    for balance in res.balances {
        println!("    asset ID: {}", hex::encode(balance.denom.id()));
        println!("    {} {}", balance.balance, balance.denom);
    }

    Ok(())
}

// Gets the balance of a Sequencer account
/// # Arguments
///
/// * `args` - The arguments passed to the command
///
/// # Errors
///
/// * If the http client cannot be created
/// * If the balance cannot be retrieved
pub(crate) async fn get_nonce(args: &BasicAccountArgs) -> eyre::Result<()> {
    let address = &args.address;
    let sequencer_client = HttpClient::new(args.sequencer_url.as_str())
        .wrap_err("failed constructing http sequencer client")?;

    let res = sequencer_client
        .get_latest_nonce(address.0)
        .await
        .wrap_err("failed to get nonce")?;

    println!("Nonce for address {}:", address.0);
    println!("    {} at height {}", res.nonce, res.height);

    Ok(())
}

/// Gets the latest block height of a Sequencer node
///
/// # Arguments
///
/// * `args` - The arguments passed to the command
///
/// # Errors
///
/// * If the http client cannot be created
/// * If the latest block height cannot be retrieved
pub(crate) async fn get_block_height(args: &BlockHeightGetArgs) -> eyre::Result<()> {
    let sequencer_client = HttpClient::new(args.sequencer_url.as_str())
        .wrap_err("failed constructing http sequencer client")?;

    let res = sequencer_client
        .latest_sequencer_block()
        .await
        .wrap_err("failed to get sequencer block")?;

    println!("Block Height:");
    println!("    {}", res.header().cometbft_header().height);

    Ok(())
}

/// Gets the latest block height of a Sequencer node
///
/// # Arguments
///
/// * `args` - The arguments passed to the command
///
/// # Errors
///
/// * If the http client cannot be created
/// * If the latest block height cannot be retrieved
pub(crate) async fn send_transfer(args: &TransferArgs) -> eyre::Result<()> {
    use astria_core::sequencer::v1::asset::default_native_asset_id;

    let res = submit_transaction(
        args.sequencer_url.as_str(),
        args.private_key.as_str(),
        Action::Transfer(TransferAction {
            to: args.to_address.0,
            amount: args.amount,
            asset_id: default_native_asset_id(),
            fee_asset_id: default_native_asset_id(),
        }),
    )
    .await
    .wrap_err("failed to submit transfer transaction")?;

    ensure!(res.tx_result.code.is_ok(), "error with transfer");
    println!("Transfer completed!");
    Ok(())
}

pub(crate) async fn init_bridge_account(args: &InitBridgeAccountArgs) -> eyre::Result<()> {
    use astria_core::sequencer::v1::{
        asset::default_native_asset_id,
        RollupId,
    };

    let rollup_id = RollupId::from_unhashed_bytes(args.rollup_name.as_bytes());
    let res = submit_transaction(
        args.sequencer_url.as_str(),
        args.private_key.as_str(),
        Action::InitBridgeAccount(InitBridgeAccountAction {
            rollup_id,
            asset_ids: vec![default_native_asset_id()],
            fee_asset_id: default_native_asset_id(),
        }),
    )
    .await
    .wrap_err("failed to submit InitBridgeAccount transaction")?;

    ensure!(res.tx_result.code.is_ok(), "error with InitBridgeAccount");
    println!("InitBridgeAccount completed!");
    println!("Rollup name: {}", args.rollup_name);
    println!("Rollup ID: {}", rollup_id);
    Ok(())
}

pub(crate) async fn bridge_lock(args: &BridgeLockArgs) -> eyre::Result<()> {
    use astria_core::sequencer::v1::asset::default_native_asset_id;

    let res = submit_transaction(
        args.sequencer_url.as_str(),
        args.private_key.as_str(),
        Action::BridgeLock(BridgeLockAction {
            to: args.to_address.0,
            asset_id: default_native_asset_id(),
            amount: args.amount,
            fee_asset_id: default_native_asset_id(),
            destination_chain_address: args.destination_chain_address.clone(),
        }),
    )
    .await
    .wrap_err("failed to submit BridgeLock transaction")?;

    ensure!(res.tx_result.code.is_ok(), "error with BridgeLock");
    println!("BridgeLock completed!");
    Ok(())
}

async fn submit_transaction(
    sequencer_url: &str,
    private_key: &str,
    action: Action,
) -> eyre::Result<endpoint::broadcast::tx_commit::Response> {
    println!("private_key: {}", private_key);
    let sequencer_client =
        HttpClient::new(sequencer_url).wrap_err("failed constructing http sequencer client")?;

    let private_key_bytes: [u8; 32] = hex::decode(private_key)
        .wrap_err("failed to decode private key bytes from hex string")?
        .try_into()
        .map_err(|_| eyre!("invalid private key length; must be 32 bytes"))?;
    let sequencer_key = SigningKey::from(private_key_bytes);

    let from_address = Address::from_verification_key(sequencer_key.verification_key());

    let nonce_res = sequencer_client
        .get_latest_nonce(from_address)
        .await
        .wrap_err("failed to get nonce")?;

    let tx = UnsignedTransaction {
        nonce: nonce_res.nonce,
        actions: vec![action],
    }
    .into_signed(&sequencer_key);
    sequencer_client
        .submit_transaction_commit(tx)
        .await
        .wrap_err("failed to submit InitBridgeAccount transaction")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_new_signing_key() {
        // generates seed of 32 bytes
        let key1 = get_new_signing_key();
        assert_eq!(key1.to_bytes().len(), 32, "Signing key is not 32 bytes");

        // generates different values
        let key2 = get_new_signing_key();
        assert_ne!(
            key1.to_bytes(),
            key2.to_bytes(),
            "Two signing key seeds are unexpectedly equal"
        );
    }

    #[test]
    fn test_signing_key_is_valid() {
        let key = get_new_signing_key();
        let msg = "Hello, world!";
        let signature = key.sign(msg.as_bytes());

        let verification_key = key.verification_key();
        assert!(
            verification_key.verify(&signature, msg.as_bytes()).is_ok(),
            "Signature verification failed"
        );
    }

    #[test]
    fn test_get_public_key_pretty() {
        let signing_key = get_new_signing_key();
        let public_key_pretty = get_public_key_pretty(&signing_key);
        assert_eq!(public_key_pretty.len(), 64);
    }

    #[test]
    fn test_get_private_key_pretty() {
        let signing_key = get_new_signing_key();
        let private_key_pretty = get_private_key_pretty(&signing_key);
        assert_eq!(private_key_pretty.len(), 64);
    }

    #[test]
    fn test_get_address_pretty() {
        let signing_key = get_new_signing_key();
        let address_pretty = get_address_pretty(&signing_key);
        assert_eq!(address_pretty.len(), 40);
    }
}
