use astria_core::{
    crypto::SigningKey,
    primitive::v1::{
        asset::{
            self,
            default_native_asset,
        },
        Address,
        ADDRESS_LEN,
    },
    protocol::transaction::v1alpha1::{
        action::{
            Action,
            BridgeLockAction,
            FeeAssetChangeAction,
            IbcRelayerChangeAction,
            InitBridgeAccountAction,
            SudoAddressChangeAction,
            TransferAction,
        },
        TransactionParams,
        UnsignedTransaction,
    },
};
use astria_sequencer_client::{
    tendermint,
    tendermint_rpc::endpoint,
    Client,
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
use rand::rngs::OsRng;

use crate::cli::sequencer::{
    BasicAccountArgs,
    Bech32mAddressArgs,
    BlockHeightGetArgs,
    BridgeLockArgs,
    FeeAssetChangeArgs,
    IbcRelayerChangeArgs,
    InitBridgeAccountArgs,
    SudoAddressChangeArgs,
    TransferArgs,
    ValidatorUpdateArgs,
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
    hex::encode(signing_key.verification_key().address_bytes())
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
    let sequencer_client = HttpClient::new(args.sequencer_url.as_str())
        .wrap_err("failed constructing http sequencer client")?;

    let res = sequencer_client
        .get_latest_balance(args.address)
        .await
        .wrap_err("failed to get balance")?;

    println!("Balances for address: {}", args.address);
    for balance in res.balances {
        println!("    asset ID: {}", balance.denom.id());
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
    let sequencer_client = HttpClient::new(args.sequencer_url.as_str())
        .wrap_err("failed constructing http sequencer client")?;

    let res = sequencer_client
        .get_latest_nonce(args.address)
        .await
        .wrap_err("failed to get nonce")?;

    println!("Nonce for address {}", args.address);
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
        .latest_block()
        .await
        .wrap_err("failed to get cometbft block")?;

    println!("Block Height:");
    println!("    {}", res.block.header.height);

    Ok(())
}

/// Returns a bech32m sequencer address given a prefix and hex-encoded byte slice
pub(crate) fn make_bech32m(args: &Bech32mAddressArgs) -> eyre::Result<()> {
    use hex::FromHex as _;
    let bytes = <[u8; ADDRESS_LEN]>::from_hex(&args.bytes)
        .wrap_err("failed decoding provided hex bytes")?;
    let address = Address::builder()
        .array(bytes)
        .prefix(&args.prefix)
        .try_build()
        .wrap_err(
            "failed constructing a valid bech32m address from the provided hex bytes and prefix",
        )?;
    println!("{address}");
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
    let res = submit_transaction(
        args.sequencer_url.as_str(),
        args.sequencer_chain_id.clone(),
        &args.prefix,
        args.private_key.as_str(),
        Action::Transfer(TransferAction {
            to: args.to_address,
            amount: args.amount,
            asset_id: default_native_asset().id(),
            fee_asset_id: default_native_asset().id(),
        }),
    )
    .await
    .wrap_err("failed to submit transfer transaction")?;

    println!("Transfer completed!");
    println!("Included in block: {}", res.height);
    Ok(())
}

/// Adds an address to the Ibc Relayer set
///
/// # Arguments
///
/// * `args` - The arguments passed to the command
///
/// # Errors
///
/// * If the http client cannot be created
/// * If the transaction failed to be included
pub(crate) async fn ibc_relayer_add(args: &IbcRelayerChangeArgs) -> eyre::Result<()> {
    let res = submit_transaction(
        args.sequencer_url.as_str(),
        args.sequencer_chain_id.clone(),
        &args.prefix,
        args.private_key.as_str(),
        Action::IbcRelayerChange(IbcRelayerChangeAction::Addition(args.address)),
    )
    .await
    .wrap_err("failed to submit IbcRelayerChangeAction::Addition transaction")?;

    println!("IbcRelayerChangeAction::Addition completed!");
    println!("Included in block: {}", res.height);
    Ok(())
}

/// Removes an address to the Ibc Relayer set
///
/// # Arguments
///
/// * `args` - The arguments passed to the command
///
/// # Errors
///
/// * If the http client cannot be created
/// * If the transaction failed to be included
pub(crate) async fn ibc_relayer_remove(args: &IbcRelayerChangeArgs) -> eyre::Result<()> {
    let res = submit_transaction(
        args.sequencer_url.as_str(),
        args.sequencer_chain_id.clone(),
        &args.prefix,
        args.private_key.as_str(),
        Action::IbcRelayerChange(IbcRelayerChangeAction::Removal(args.address)),
    )
    .await
    .wrap_err("failed to submit IbcRelayerChangeAction::Removal transaction")?;

    println!("IbcRelayerChangeAction::Removal completed!");
    println!("Included in block: {}", res.height);
    Ok(())
}

/// Inits a bridge account
///
/// # Arguments
///
/// * `args` - The arguments passed to the command
///
/// # Errors
///
/// * If the http client cannot be created
/// * If the transaction failed to be included
pub(crate) async fn init_bridge_account(args: &InitBridgeAccountArgs) -> eyre::Result<()> {
    use astria_core::primitive::v1::RollupId;

    let rollup_id = RollupId::from_unhashed_bytes(args.rollup_name.as_bytes());
    let res = submit_transaction(
        args.sequencer_url.as_str(),
        args.sequencer_chain_id.clone(),
        &args.prefix,
        args.private_key.as_str(),
        Action::InitBridgeAccount(InitBridgeAccountAction {
            rollup_id,
            asset_id: default_native_asset().id(),
            fee_asset_id: default_native_asset().id(),
            sudo_address: None,
            withdrawer_address: None,
        }),
    )
    .await
    .wrap_err("failed to submit InitBridgeAccount transaction")?;

    println!("InitBridgeAccount completed!");
    println!("Included in block: {}", res.height);
    println!("Rollup name: {}", args.rollup_name);
    println!("Rollup ID: {rollup_id}");
    Ok(())
}

/// Bridge Lock action
///
/// # Arguments
///
/// * `args` - The arguments passed to the command
///
/// # Errors
///
/// * If the http client cannot be created
/// * If the transaction failed to be included
pub(crate) async fn bridge_lock(args: &BridgeLockArgs) -> eyre::Result<()> {
    let res = submit_transaction(
        args.sequencer_url.as_str(),
        args.sequencer_chain_id.clone(),
        &args.prefix,
        args.private_key.as_str(),
        Action::BridgeLock(BridgeLockAction {
            to: args.to_address,
            asset_id: default_native_asset().id(),
            amount: args.amount,
            fee_asset_id: default_native_asset().id(),
            destination_chain_address: args.destination_chain_address.clone(),
        }),
    )
    .await
    .wrap_err("failed to submit BridgeLock transaction")?;

    println!("BridgeLock completed!");
    println!("Included in block: {}", res.height);
    Ok(())
}

/// Adds a fee asset
///
/// # Arguments
///
/// * `args` - The arguments passed to the command
///
/// # Errors
///
/// * If the http client cannot be created
/// * If the transaction failed to be included
pub(crate) async fn fee_asset_add(args: &FeeAssetChangeArgs) -> eyre::Result<()> {
    let res = submit_transaction(
        args.sequencer_url.as_str(),
        args.sequencer_chain_id.clone(),
        &args.prefix,
        args.private_key.as_str(),
        Action::FeeAssetChange(FeeAssetChangeAction::Addition(
            asset::Id::from_str_unchecked(&args.asset),
        )),
    )
    .await
    .wrap_err("failed to submit FeeAssetChangeAction::Addition transaction")?;

    println!("FeeAssetChangeAction::Addition completed!");
    println!("Included in block: {}", res.height);
    Ok(())
}

/// Removes a fee asset
///
/// # Arguments
///
/// * `args` - The arguments passed to the command
///
/// # Errors
///
/// * If the http client cannot be created
/// * If the transaction failed to be included
pub(crate) async fn fee_asset_remove(args: &FeeAssetChangeArgs) -> eyre::Result<()> {
    let res = submit_transaction(
        args.sequencer_url.as_str(),
        args.sequencer_chain_id.clone(),
        &args.prefix,
        args.private_key.as_str(),
        Action::FeeAssetChange(FeeAssetChangeAction::Removal(
            asset::Id::from_str_unchecked(&args.asset),
        )),
    )
    .await
    .wrap_err("failed to submit FeeAssetChangeAction::Removal transaction")?;

    println!("FeeAssetChangeAction::Removal completed!");
    println!("Included in block: {}", res.height);
    Ok(())
}

/// Changes the Sequencer's sudo address to a new address
///
/// # Arguments
///
/// * `args` - The arguments passed to the command
///
/// # Errors
///
/// * If the http client cannot be created
/// * If the sudo address was not changed
pub(crate) async fn sudo_address_change(args: &SudoAddressChangeArgs) -> eyre::Result<()> {
    let res = submit_transaction(
        args.sequencer_url.as_str(),
        args.sequencer_chain_id.clone(),
        &args.prefix,
        args.private_key.as_str(),
        Action::SudoAddressChange(SudoAddressChangeAction {
            new_address: args.address,
        }),
    )
    .await
    .wrap_err("failed to submit SudoAddressChange transaction")?;

    println!("SudoAddressChange completed!");
    println!("Included in block: {}", res.height);
    Ok(())
}

/// Updates a validator
///
/// # Arguments
///
/// * `args` - The arguments passed to the command
///
/// # Errors
///
/// * If the http client cannot be created
/// * If the transaction failed to be submitted
pub(crate) async fn validator_update(args: &ValidatorUpdateArgs) -> eyre::Result<()> {
    let public_key_raw = hex::decode(args.validator_public_key.as_str())
        .wrap_err("failed to decode public key into bytes")?;
    let pub_key = tendermint::PublicKey::from_raw_ed25519(&public_key_raw)
        .expect("failed to parse public key from parsed bytes");
    let validator_update = tendermint::validator::Update {
        pub_key,
        power: args.power.into(),
    };

    let res = submit_transaction(
        args.sequencer_url.as_str(),
        args.sequencer_chain_id.clone(),
        &args.prefix,
        args.private_key.as_str(),
        Action::ValidatorUpdate(validator_update),
    )
    .await
    .wrap_err("failed to submit ValidatorUpdate transaction")?;

    println!("ValidatorUpdate completed!");
    println!("Included in block: {}", res.height);
    Ok(())
}

async fn submit_transaction(
    sequencer_url: &str,
    chain_id: String,
    prefix: &str,
    private_key: &str,
    action: Action,
) -> eyre::Result<endpoint::broadcast::tx_commit::Response> {
    let sequencer_client =
        HttpClient::new(sequencer_url).wrap_err("failed constructing http sequencer client")?;

    let private_key_bytes: [u8; 32] = hex::decode(private_key)
        .wrap_err("failed to decode private key bytes from hex string")?
        .try_into()
        .map_err(|_| eyre!("invalid private key length; must be 32 bytes"))?;
    let sequencer_key = SigningKey::from(private_key_bytes);

    let from_address = Address::builder()
        .array(sequencer_key.verification_key().address_bytes())
        .prefix(prefix)
        .try_build()
        .wrap_err("failed constructing a valid from address from the provided prefix")?;
    println!("sending tx from address: {from_address}");

    let nonce_res = sequencer_client
        .get_latest_nonce(from_address)
        .await
        .wrap_err("failed to get nonce")?;

    let tx = UnsignedTransaction {
        params: TransactionParams::builder()
            .nonce(nonce_res.nonce)
            .chain_id(chain_id)
            .build(),
        actions: vec![action],
    }
    .into_signed(&sequencer_key);
    let res = sequencer_client
        .submit_transaction_commit(tx)
        .await
        .wrap_err("failed to submit transaction")?;
    ensure!(
        res.check_tx.code.is_ok(),
        "failed to check tx: {}",
        res.check_tx.log
    );
    ensure!(
        res.tx_result.code.is_ok(),
        "failed to execute tx: {}",
        res.tx_result.log
    );
    Ok(res)
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
