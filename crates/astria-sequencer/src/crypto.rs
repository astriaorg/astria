pub(crate) use ed25519_dalek::{
    Keypair,
    PublicKey,
    Signature,
    Signer,
    Verifier,
};

#[cfg(test)]
mod test {
    use ed25519_dalek::SecretKey;

    use super::*;
    use crate::{
        accounts::types::Address,
        hash,
    };

    /// note that this test does not really test anything; it's used to generate
    /// default keys for the test accounts.
    /// we want the test keys to be deterministic so that we can easily
    /// fund them at genesis.
    #[test]
    fn generate_default_keys() {
        let default_accounts: Vec<&[u8]> = vec![b"alice", b"bob", b"carol"];
        let message = b"test";

        for acc in default_accounts {
            let bytes = hash(acc);
            let secret = SecretKey::from_bytes(&bytes).unwrap();
            let public = (&secret).into();
            let keypair = Keypair {
                secret,
                public,
            };

            let sig = keypair.sign(message);
            assert!(keypair.verify(message, &sig).is_ok());
            let address: Address = (&public).try_into().unwrap();
            println!(
                "{}:\n\tsecret key: {}\n\tpublic key: {}\n\taddress: {}",
                String::from_utf8(acc.to_vec()).unwrap(),
                hex::encode(keypair.secret.to_bytes()),
                hex::encode(keypair.public.to_bytes()),
                address,
            );
        }
    }
}
