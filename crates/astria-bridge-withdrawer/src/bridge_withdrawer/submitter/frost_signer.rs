use astria_core::{
    crypto::VerificationKey,
    primitive::v1::Address,
    protocol::transaction::v1::{
        Transaction,
        TransactionBody,
    },
};
use astria_eyre::eyre::{
    self,
    eyre,
    Context,
};
use frost_ed25519::keys::{
    KeyPackage,
    PublicKeyPackage,
};

use super::Signer;

pub(crate) struct FrostSignerBuilder {
    key_package: Option<KeyPackage>,
    public_key_package: Option<PublicKeyPackage>,
    prefix: Option<String>,
}

impl FrostSignerBuilder {
    pub(crate) fn key_package(self, key_package: KeyPackage) -> Self {
        Self {
            key_package: Some(key_package),
            ..self
        }
    }

    pub(crate) fn public_key_package(self, public_key_package: PublicKeyPackage) -> Self {
        Self {
            public_key_package: Some(public_key_package),
            ..self
        }
    }

    pub(crate) fn prefix(self, prefix: String) -> Self {
        Self {
            prefix: Some(prefix),
            ..self
        }
    }

    pub(crate) fn try_build(self) -> eyre::Result<FrostSigner> {
        let key_package = self
            .key_package
            .ok_or_else(|| eyre!("key package is required"))?;
        let public_key_package = self
            .public_key_package
            .ok_or_else(|| eyre!("public key package is required"))?;
        let verifying_key_bytes: [u8; 32] = public_key_package
            .verifying_key()
            .serialize()
            .wrap_err("failed to serialize verifying key")?
            .try_into()
            .map_err(|_| eyre!("failed to convert verifying key to 32 bytes"))?;
        let verifying_key: VerificationKey = VerificationKey::try_from(verifying_key_bytes)
            .wrap_err("failed to build verification key")?;
        let address = Address::builder()
            .array(*verifying_key.address_bytes())
            .prefix(
                self.prefix
                    .ok_or_else(|| eyre!("astria address prefix is required"))?,
            )
            .try_build()
            .wrap_err("failed to build address")?;

        Ok(FrostSigner {
            key_package,
            public_key_package,
            address,
        })
    }
}

pub(crate) struct FrostSigner {
    key_package: KeyPackage,
    public_key_package: PublicKeyPackage,
    address: Address,
}

impl Signer for FrostSigner {
    fn sign(&self, tx: TransactionBody) -> Transaction {
        todo!()
    }

    fn address(&self) -> &Address {
        &self.address
    }
}
