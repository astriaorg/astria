use ed25519_consensus::SigningKey;
use rand::rngs::OsRng;
use sha2::{
    Digest,
    Sha256,
};

/// Generate new keypair
fn get_new_signing_key() -> SigningKey {
    let csprng = OsRng;
    let signing_key: SigningKey = SigningKey::new(csprng);
    signing_key
}

/// Get the public key from the signing key
fn get_public_key_pretty(signing_key: &SigningKey) -> String {
    // hex encode public key for printing
    let verifying_key_bytes = signing_key.verification_key().to_bytes();
    hex::encode(verifying_key_bytes)
}

/// Get the private key from the signing key
fn get_private_key_pretty(signing_key: &SigningKey) -> String {
    // get full private key for printing
    let secret_key_bytes = signing_key.to_bytes();
    hex::encode(secret_key_bytes)
}

/// Get the address from the signing key
fn get_address_pretty(signing_key: &SigningKey) -> String {
    // sha256 hash public key and take first 20 bytes to get address for printing
    let verifying_key_bytes = signing_key.verification_key().to_bytes();
    let mut hasher = Sha256::new();
    hasher.update(verifying_key_bytes);
    let address_bytes = hasher.finalize();
    hex::encode(&address_bytes[..20])
}

/// Create a new sequencer account
pub(crate) fn create_account() {
    let signing_key = get_new_signing_key();
    let public_key_pretty = get_public_key_pretty(&signing_key);
    let private_key_pretty = get_private_key_pretty(&signing_key);
    let address_pretty = get_address_pretty(&signing_key);

    println!("Create Sequencer Account");
    println!();
    println!("Private Key: {private_key_pretty:?}");
    println!("Public Key:  {public_key_pretty:?}");
    println!("Address:     {address_pretty:?}");
}

/// Get the balance of a sequencer account
///
/// # Arguments
///
/// * `args` - The arguments passed to the command
///
/// # Errors
// pub(crate) fn get_balance(args: &SequencerBalanceGetArgs) -> eyre::Result<()> {
//     println!("Get Sequencer Balance {:?}", args);
//     Ok(())
// }

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_new_signing_key() {
        let _signing_key = get_new_signing_key();
        // assert!(signing_key.is_valid());
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
