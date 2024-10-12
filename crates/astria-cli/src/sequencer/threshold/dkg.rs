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
use serde::{
    Deserialize,
    Serialize,
};

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

    /// path to a file with the output secret key package from keygen ceremony.
    #[arg(long)]
    secret_key_package_path: String,

    /// path to a file with the output public key package from keygen ceremony.
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
        let (round1_secret_package, round1_public_package) =
            dkg::part1(id, max_signers, min_signers, rng).wrap_err("failed to run dkg part1")?;
        let round1_public_package_with_id = Round1PackageWithIdentifier {
            identifier: id,
            package: round1_public_package,
        };
        println!(
            "Send our public package to all other participants: {}",
            serde_json::to_string(&round1_public_package_with_id)?
        );

        let mut round1_public_packages: BTreeMap<Identifier, round1::Package> = BTreeMap::new();
        loop {
            // need a package from every other participant
            if round1_public_packages.len() == (max_signers - 1) as usize {
                break;
            }

            println!(
                "Enter round 1 package for participant (received {}/{} total packages)",
                round1_public_packages.len(),
                max_signers - 1
            );
            let input = read_line_raw().await?;
            let Ok(round1_package) = serde_json::from_str::<Round1PackageWithIdentifier>(&input)
            else {
                continue;
            };

            if round1_package.identifier == id {
                continue;
            }

            round1_public_packages.insert(round1_package.identifier, round1_package.package);
        }

        // round 2
        let (round2_secret_package, round2_packages) =
            dkg::part2(round1_secret_package, &round1_public_packages)
                .wrap_err("failed to run dkg part2")?;

        let mut round2_public_packages: BTreeMap<Identifier, round2::Package> = BTreeMap::new();
        for (their_id, round2_package) in round2_packages.into_iter() {
            let round2_package_with_id = Round2PackageWithIdentifier {
                identifier: id,
                package: round2_package,
            };
            println!(
                "Send package to participant with id {}: {}",
                hex::encode(their_id.serialize()),
                serde_json::to_string(&round2_package_with_id)?
            );
        }

        loop {
            if round2_public_packages.len() == (max_signers - 1) as usize {
                break;
            }

            println!(
                "Enter round 2 package for participant (received {}/{} total packages)",
                round2_public_packages.len(),
                max_signers - 1
            );
            let input = read_line_raw().await?;
            let Ok(round2_package) = serde_json::from_str::<Round2PackageWithIdentifier>(&input)
            else {
                continue;
            };

            if round2_package.identifier == id {
                continue;
            }

            round2_public_packages.insert(round2_package.identifier, round2_package.package);
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

#[derive(Debug, Deserialize, Serialize)]
struct Round1PackageWithIdentifier {
    identifier: Identifier,
    package: round1::Package,
}

#[derive(Debug, Deserialize, Serialize)]
struct Round2PackageWithIdentifier {
    identifier: Identifier,
    package: round2::Package,
}
