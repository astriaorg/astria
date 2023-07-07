use bech32::{self, ToBase32, Variant};
use ed25519_dalek::{Keypair, PublicKey};
use eyre::WrapErr as _;
use sha2::{Digest, Sha256};

const ADDRESS_LENGTH: usize = 20;

/// converts an encoded private key to a ed25519_dalek::Keypair
pub fn private_key_bytes_to_keypair(private_key: &[u8]) -> eyre::Result<Keypair> {
    let private_key = ed25519_dalek::SecretKey::from_bytes(&private_key[..32])
        .wrap_err("failed reading secret key from bytes")?;
    let public_key = PublicKey::from(&private_key);
    Ok(Keypair {
        secret: private_key,
        public: public_key,
    })
}

/// converts a hex-encoded validator address to a bech32 address
pub fn validator_hex_to_address(data: &str) -> eyre::Result<String> {
    let address_bytes =
        hex::decode(data).wrap_err("failed reading bytes from hex encoded string")?;
    let address = bech32::encode("metrovalcons", address_bytes.to_base32(), Variant::Bech32)
        .wrap_err("failed converting bytes to bech32 address")?;
    Ok(address)
}

/// converts a validator public key to a bech32 address
/// conversion: Sha256(public_key_bytes)[0..20]
pub fn public_key_to_address(public_key: &[u8]) -> eyre::Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(public_key);
    let result = hasher.finalize();
    let address_bytes: [u8; ADDRESS_LENGTH] = result[0..ADDRESS_LENGTH]
        .try_into()
        .expect("should convert slice to array of 20 bytes");
    let address = bech32::encode("metrovalcons", address_bytes.to_base32(), Variant::Bech32)
        .wrap_err("failed converting bytes to bech32 address")?;
    Ok(address)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::base64_string::Base64String;

    #[test]
    fn test_private_to_public() {
        let public_key_str =
            Base64String::from_string("Fj/2NzG404f+CjHJUThMXNS7xJY5GMPuFVlKMKb86MA=".to_string())
                .unwrap();
        let private_key_str = "1hBYYTBKxkMODNTW6Pk//kA023UAkpgSLhM0SjwndSkWP/Y3MbjTh/4KMclROExc1LvEljkYw+4VWUowpvzowA==".to_string();
        let private_key = Base64String::from_string(private_key_str).unwrap();
        let keypair = private_key_bytes_to_keypair(&private_key.0).unwrap();
        assert_eq!(keypair.public.to_bytes().to_vec(), public_key_str.0);
    }

    #[test]
    fn test_validator_hex_to_address() {
        let validator_address_from_bech32 =
            validator_hex_to_address("468646B2BD3E75229B2163F4D7905748FEC7603E").unwrap();
        assert_eq!(
            validator_address_from_bech32,
            "metrovalcons1g6rydv4a8e6j9xepv06d0yzhfrlvwcp724wzdz"
        );
    }

    #[test]
    fn test_public_key_to_address() {
        let address_hex = "D33A4E621629E47C3DE5F8A21B881AAE8A5730FD";
        let public_key =
            Base64String::from_string("E2wUJabp+hXF6kBQNdKvZ/mqZJI3s75nNlAeiehxgFI=".to_string())
                .unwrap();
        let res_public_key = public_key_to_address(&public_key.0).unwrap();
        assert_eq!(
            validator_hex_to_address(address_hex).unwrap(),
            res_public_key
        );
    }
}
