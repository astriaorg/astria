use astria_core::{
    crypto::SigningKey,
    primitive::v1::Address,
    protocol::transaction::v1alpha1::{
        Action,
        TransactionBody,
    },
};
use astria_sequencer_client::{
    tendermint_rpc::endpoint::tx::Response,
    HttpClient,
    SequencerClientExt as _,
};
use color_eyre::eyre::{
    self,
    ensure,
    eyre,
    WrapErr as _,
};

pub(crate) async fn submit_transaction(
    sequencer_url: &str,
    chain_id: String,
    prefix: &str,
    private_key: &str,
    action: Action,
) -> eyre::Result<Response> {
    let sequencer_client =
        HttpClient::new(sequencer_url).wrap_err("failed constructing http sequencer client")?;

    let sequencer_key = signing_key_from_private_key(private_key)?;

    let from_address = address_from_signing_key(&sequencer_key, prefix)?;
    println!("sending tx from address: {from_address}");

    let nonce_res = sequencer_client
        .get_latest_nonce(from_address)
        .await
        .wrap_err("failed to get nonce")?;

    let tx = TransactionBody::builder()
        .nonce(nonce_res.nonce)
        .chain_id(chain_id)
        .actions(vec![action])
        .try_build()
        .wrap_err("failed to construct a transaction")?
        .sign(&sequencer_key);
    let res = sequencer_client
        .submit_transaction_sync(tx)
        .await
        .wrap_err("failed to submit transaction")?;

    ensure!(res.code.is_ok(), "failed to check tx: {}", res.log);

    let tx_response = sequencer_client.wait_for_tx_inclusion(res.hash).await;

    ensure!(
        tx_response.tx_result.code.is_ok(),
        "failed to execute tx: {}",
        tx_response.tx_result.log
    );
    Ok(tx_response)
}

pub(crate) fn signing_key_from_private_key(private_key: &str) -> eyre::Result<SigningKey> {
    // Decode the hex string to get the private key bytes
    let private_key_bytes: [u8; 32] = hex::decode(private_key)
        .wrap_err("failed to decode private key bytes from hex string")?
        .try_into()
        .map_err(|_| eyre!("invalid private key length; must be 32 bytes"))?;

    // Create and return a signing key from the private key bytes
    Ok(SigningKey::from(private_key_bytes))
}

pub(crate) fn address_from_signing_key(
    signing_key: &SigningKey,
    prefix: &str,
) -> eyre::Result<Address> {
    // Build the address using the public key from the signing key
    let from_address = Address::builder()
        .array(*signing_key.verification_key().address_bytes())
        .prefix(prefix)
        .try_build()
        .wrap_err("failed constructing a valid from address from the provided prefix")?;

    // Return the generated address
    Ok(from_address)
}
