pub(crate) use ed25519_consensus::{
    Signature,
    SigningKey,
    VerificationKey,
};

#[cfg(test)]
mod test {
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
            let bytes: [u8; 32] = hash(acc).try_into().unwrap();
            let secret = SigningKey::from(bytes);
            let public = secret.verification_key();

            let sig = secret.sign(message);
            assert!(public.verify(&sig, message).is_ok());
            let address: Address = (&public).try_into().unwrap();
            println!(
                "{}:\n\tsecret key: {}\n\tpublic key: {}\n\taddress: {}",
                String::from_utf8(acc.to_vec()).unwrap(),
                hex::encode(secret.to_bytes()),
                hex::encode(public.to_bytes()),
                address,
            );
        }
    }
}
