use color_eyre::eyre;
use ed25519_consensus::SigningKey;
use rand::rngs::OsRng;
use sha2::{
    Digest,
    Sha256,
};

pub(crate) fn create_sequencer_account() -> eyre::Result<()> {
    // generate new key
    let csprng = OsRng;
    let signing_key: SigningKey = SigningKey::new(csprng);

    // hex encode public key for printing
    let verifying_key_bytes = signing_key.verification_key().to_bytes();
    let public_key = hex::encode(verifying_key_bytes);

    // get full private key for printing
    let secret_key_bytes = signing_key.to_bytes();
    let private_key = {
        let mut complete_key = [0u8; 64];
        complete_key[..32].copy_from_slice(&secret_key_bytes);
        complete_key[32..].copy_from_slice(&verifying_key_bytes);
        complete_key
    };

    // sha256 hash public key and take first 20 bytes to get address for printing
    let address = {
        let mut hasher = Sha256::new();
        hasher.update(verifying_key_bytes);
        let address_bytes = hasher.finalize();
        hex::encode(&address_bytes[..20])
    };

    println!("Create Sequencer Account");
    println!();
    println!("Private Key: {:?}", hex::encode(private_key));
    println!("Public Key:  {:?}", public_key);
    println!("Address:     {:?}", address);

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_create_sequencer_account() {
        assert!(create_sequencer_account().is_ok());
    }
}
