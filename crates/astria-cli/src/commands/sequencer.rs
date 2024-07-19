use std::path::Path;

use astria_core::{
    crypto::SigningKey,
    protocol::transaction::v1alpha1::{
        action::{
            Action,
            BridgeLockAction,
            FeeAssetChangeAction,
            IbcRelayerChangeAction,
            InitBridgeAccountAction,
            SudoAddressChangeAction,
            TransferAction,
            ValidatorUpdate,
        },
        TransactionParams,
        UnsignedTransaction,
    },
};
use astria_sequencer_client::{
    tendermint_rpc::endpoint,
    Client,
    HttpClient,
    SequencerClientExt,
};
use color_eyre::{
    eyre,
    eyre::{
        ensure,
        Context,
    },
};
use rand::rngs::OsRng;
use tracing::{
    debug,
    info,
    instrument,
};

use crate::cli::sequencer::{
    BasicAccountArgs,
    Bech32mAddressArgs,
    BlockHeightGetArgs,
    BridgeLockArgs,
    CreateAccount,
    FeeAssetChangeArgs,
    IbcRelayerChangeArgs,
    InitBridgeAccountArgs,
    SudoAddressChangeArgs,
    TransferArgs,
    ValidatorUpdateArgs,
};

/// Generates a new ED25519 keypair and prints the public key, private key, and address
#[instrument(skip_all, fields(output = %args.output.display()))]
pub(crate) fn create_account(args: CreateAccount) -> eyre::Result<()> {
    use std::io::Write as _;
    let output_file = crate::utils::create_file_with_permissions_0o600(&args.output)?;
    info!("created file to write signing key");
    debug!("created file to write signing key");
    let signing_key = SigningKey::new(OsRng);
    (&output_file)
        .write_all(hex::encode(signing_key.as_bytes()).as_bytes())
        .wrap_err_with(|| {
            format!(
                "failed to write signing key to `{}`",
                &args.output.display()
            )
        })?;

    let verification_key = signing_key.verification_key();
    let address = crate::utils::make_address(&args.prefix, &verification_key.address_bytes())
        .wrap_err("failed to construct an address from the generated signing key")?;

    debug!(%verification_key, %address, "wrote signing key");
    Ok(())
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
    let bytes = hex::decode(&args.bytes).wrap_err("failed decoding provided hex bytes")?;
    let address = crate::utils::make_address(&args.prefix, &bytes).wrap_err(
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
        &args.signing_key,
        Action::Transfer(TransferAction {
            to: args.to_address,
            amount: args.amount,
            asset: args.asset.clone(),
            fee_asset: args.fee_asset.clone(),
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
        &args.signing_key,
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
        &args.signing_key,
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
        &args.signing_key,
        Action::InitBridgeAccount(InitBridgeAccountAction {
            rollup_id,
            asset: args.asset.clone(),
            fee_asset: args.fee_asset.clone(),
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
        &args.signing_key,
        Action::BridgeLock(BridgeLockAction {
            to: args.to_address,
            asset: args.asset.clone(),
            amount: args.amount,
            fee_asset: args.fee_asset.clone(),
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
        &args.signing_key,
        Action::FeeAssetChange(FeeAssetChangeAction::Addition(args.asset.clone())),
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
        &args.signing_key,
        Action::FeeAssetChange(FeeAssetChangeAction::Removal(args.asset.clone())),
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
        &args.signing_key,
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
    let verification_key = astria_core::crypto::VerificationKey::try_from(
        &*hex::decode(&args.validator_public_key)
            .wrap_err("failed to decode public key bytes from argument")?,
    )
    .wrap_err("failed to construct public key from bytes")?;
    let validator_update = ValidatorUpdate {
        power: args.power,
        verification_key,
    };

    let res = submit_transaction(
        args.sequencer_url.as_str(),
        args.sequencer_chain_id.clone(),
        &args.prefix,
        &args.signing_key,
        Action::ValidatorUpdate(validator_update),
    )
    .await
    .wrap_err("failed to submit ValidatorUpdate transaction")?;

    println!("ValidatorUpdate completed!");
    println!("Included in block: {}", res.height);
    Ok(())
}

async fn submit_transaction<P: AsRef<Path>>(
    sequencer_url: &str,
    chain_id: String,
    prefix: &str,
    signing_key: P,
    action: Action,
) -> eyre::Result<endpoint::broadcast::tx_commit::Response> {
    let sequencer_client =
        HttpClient::new(sequencer_url).wrap_err("failed constructing http sequencer client")?;

    let sequencer_key = crate::utils::read_signing_key(signing_key)?;

    let from_address =
        crate::utils::make_address(prefix, &sequencer_key.verification_key().address_bytes())
            .wrap_err("failed constructing a valid source address from the provided prefix")?;
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
