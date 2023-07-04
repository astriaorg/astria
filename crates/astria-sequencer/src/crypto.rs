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

    #[track_caller]
    fn generate_account(name: &[u8], message: &[u8]) {
        let bytes: [u8; 32] = hash(name).try_into().unwrap();
        let secret = SigningKey::from(bytes);
        let public = secret.verification_key();

        let sig = secret.sign(message);
        public.verify(&sig, message).unwrap();
        let _address: Address = Address::from_verification_key(&public);
    }

    /// note that this test does not really test anything; it's used to generate
    /// default keys for the test accounts.
    /// we want the test keys to be deterministic so that we can easily
    /// fund them at genesis.
    #[test]
    fn generate_default_keys() {
        let default_accounts: Vec<&[u8]> = vec![b"alice", b"bob", b"carol"];
        let message = b"test";

        for acc in default_accounts {
            generate_account(acc, message);
        }
    }
}
