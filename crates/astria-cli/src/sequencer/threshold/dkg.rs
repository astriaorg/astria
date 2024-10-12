use std::collections::BTreeMap;

use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use frost_ed25519::{
    Identifier,
    keys::dkg::{
        self,
        round1,
        round2,
    },
};
use rand::thread_rng;

use super::read_line_raw;

#[derive(Debug, clap::Args)]
pub(super) struct Command {
    /// index of the participant of the DKG protocol.
    /// must be 1 <= index <= n, where n is the maximum number of signers.
    #[arg(long)]
    index: u16,

    /// minimum number of signers required to sign a transaction.
    #[arg(long)]
    min_signers: u16,

    /// maximum number of signers that can sign a transaction.
    #[arg(long)]
    max_signers: u16,

    /// path to a file with the secret key package from keygen ceremony
    #[arg(long)]
    secret_key_package_path: String,

    /// path to a file with the public key package from keygen ceremony
    #[arg(long)]
    public_key_package_path: String,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        let rng = thread_rng();

        let Self {
            index,
            min_signers,
            max_signers,
            secret_key_package_path,
            public_key_package_path,
        } = self;

        let id: Identifier = index
            .try_into()
            .wrap_err("failed to convert index to frost identifier")?;
        println!("Our identifier is: {}", hex::encode(id.serialize()));

        // round 1
        let (round1_secret_package, public_package) =
            dkg::part1(id, max_signers, min_signers, rng).wrap_err("failed to run dkg part1")?;
        println!(
            "Send our public package to all other participants: {}",
            hex::encode(public_package.serialize()?)
        );

        let mut round1_public_packages: BTreeMap<Identifier, round1::Package> = BTreeMap::new();
        for i in 1..=max_signers {
            if i == index {
                continue;
            }

            println!("Enter public package for participant {}:", i);
            let public_package = read_line_raw().await?;
            let public_package = dkg::round1::Package::deserialize(&hex::decode(public_package)?)?;
            round1_public_packages.insert(i.try_into()?, public_package);
        }

        // round 2
        let (round2_secret_package, round2_packages) =
            dkg::part2(round1_secret_package, &round1_public_packages)
                .wrap_err("failed to run dkg part2")?;

        let mut round2_public_packages: BTreeMap<Identifier, round2::Package> = BTreeMap::new();
        for (id, round2_package) in round2_packages.iter() {
            println!(
                "Send package to participant with id {}: {}",
                hex::encode(id.serialize()),
                hex::encode(round2_package.serialize()?)
            );
        }

        for i in 1..=max_signers {
            if i == index {
                continue;
            }

            println!("Enter round 2 package received from participant {}:", i);
            let round2_package = read_line_raw().await?;
            let round2_package = dkg::round2::Package::deserialize(&hex::decode(round2_package)?)?;
            round2_public_packages.insert(i.try_into()?, round2_package);
        }

        // round 3 (final)
        let (key_package, pubkey_package) = dkg::part3(
            &round2_secret_package,
            &round1_public_packages,
            &round2_public_packages,
        )
        .wrap_err("failed to run dkg part3")?;

        // store the secret key package and public key package
        std::fs::write(
            secret_key_package_path.clone(),
            serde_json::to_string_pretty(&key_package)
                .wrap_err("failed to serialize secret key package")?,
        )?;
        std::fs::write(
            public_key_package_path.clone(),
            serde_json::to_string_pretty(&pubkey_package)
                .wrap_err("failed to serialize public key package")?,
        )?;

        println!("DKG completed successfully!");
        println!("Secret key package saved to: {}", secret_key_package_path);
        println!("Public key package saved to: {}", public_key_package_path);
        Ok(())
    }
}
