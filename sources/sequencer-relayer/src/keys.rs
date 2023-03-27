use bech32::{self, ToBase32, Variant};
use ed25519_dalek::{Keypair, PublicKey};
use eyre::{Context, Error};

pub fn private_key_bytes_to_keypair(private_key: &[u8]) -> Result<Keypair, Error> {
    let private_key = ed25519_dalek::SecretKey::from_bytes(&private_key[..32])
        .context("failed reading secret key from bytes")?;
    let public_key = PublicKey::from(&private_key);
    Ok(Keypair {
        secret: private_key,
        public: public_key,
    })
}

pub fn validator_hex_to_address(data: &str) -> Result<String, Error> {
    let address_bytes = hex::decode(data)?;
    let address = bech32::encode("metrovalcons", address_bytes.to_base32(), Variant::Bech32)?;
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
}
