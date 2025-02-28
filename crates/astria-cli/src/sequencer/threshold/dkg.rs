use std::collections::BTreeMap;

use astria_core::primitive::v1::{
    Address,
    Bech32m,
};
use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use frost_ed25519::{
    keys::dkg::{
        self,
        round1,
        round2,
    },
    Identifier,
};
use rand::thread_rng;
use serde::{
    Deserialize,
    Serialize,
};
use termion::color;

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

    /// The address prefix for the generated address.
    #[arg(long, default_value = "astria")]
    prefix: String,
}

impl Command {
    #[expect(
        clippy::too_many_lines,
        reason = "this is an interactive CLI command which consists of several steps"
    )]
    pub(super) async fn run(self) -> eyre::Result<()> {
        let rng = thread_rng();

        let Self {
            index,
            min_signers,
            max_signers,
            secret_key_package_path,
            public_key_package_path,
            prefix,
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
        println!("Send our public package to all other participants:");
        print!("{}", color::Fg(color::Green));
        println!(
            "{}",
            serde_json::to_string(&round1_public_package_with_id)
                .wrap_err("failed to serialize round 1 public package")?
        );

        let mut round1_public_packages: BTreeMap<Identifier, round1::Package> = BTreeMap::new();
        loop {
            // need a package from every other participant
            if round1_public_packages.len() == max_signers.saturating_sub(1) as usize {
                break;
            }

            println!(
                "Enter round 1 package for participant (received {}/{} total packages)",
                round1_public_packages.len(),
                max_signers.saturating_sub(1)
            );
            let input = read_line_raw().await.wrap_err("failed to read line")?;
            let round1_package = match serde_json::from_str::<Round1PackageWithIdentifier>(&input)
                .wrap_err("failed to parse package")
            {
                Ok(package) => package,
                Err(error) => {
                    eprintln!("{error:#}");
                    continue;
                }
            };

            // ignore if we accidentally put our own package
            if round1_package.identifier == id {
                eprintln!("ignoring package that has our own identifier");
                continue;
            }

            if round1_public_packages
                .insert(round1_package.identifier, round1_package.package)
                .is_some()
            {
                eprintln!(
                    "already added package from {}",
                    hex::encode(round1_package.identifier.serialize())
                );
            }
        }

        // round 2
        let (round2_secret_package, round2_packages) =
            dkg::part2(round1_secret_package, &round1_public_packages)
                .wrap_err("failed to run dkg part2")?;

        let mut round2_public_packages: BTreeMap<Identifier, round2::Package> = BTreeMap::new();
        for (their_id, round2_package) in round2_packages {
            let round2_package_with_id = Round2PackageWithIdentifier {
                identifier: id,
                package: round2_package,
            };
            println!(
                "Send package to participant with id {}:",
                hex::encode(their_id.serialize()),
            );
            print!("{}", color::Fg(color::Green));
            println!(
                "{}",
                serde_json::to_string(&round2_package_with_id)
                    .wrap_err("failed to serialize round 2 package")?
            );
        }

        loop {
            if round2_public_packages.len() == (max_signers.saturating_sub(1)) as usize {
                break;
            }

            println!(
                "Enter round 2 package for participant (received {}/{} total packages)",
                round2_public_packages.len(),
                max_signers.saturating_sub(1)
            );
            let input = read_line_raw().await.wrap_err("failed to read line")?;
            let round2_package = match serde_json::from_str::<Round2PackageWithIdentifier>(&input)
                .wrap_err("failed to parse package")
            {
                Ok(package) => package,
                Err(error) => {
                    eprintln!("{error:#}");
                    continue;
                }
            };

            if round2_package.identifier == id {
                eprintln!("ignoring package that has our own identifier");
                continue;
            }

            if round2_public_packages
                .insert(round2_package.identifier, round2_package.package)
                .is_some()
            {
                eprintln!(
                    "already added packaged from {}",
                    hex::encode(round2_package.identifier.serialize())
                );
            }
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
        )
        .wrap_err("failed to write secret key package")?;
        std::fs::write(
            public_key_package_path.clone(),
            serde_json::to_string_pretty(&pubkey_package)
                .wrap_err("failed to serialize public key package")?,
        )
        .wrap_err("failed to write public key package")?;

        let verifying_key_bytes = pubkey_package.verifying_key().serialize()?;
        let astria_verifying_key =
            astria_core::crypto::VerificationKey::try_from(verifying_key_bytes.as_slice())?;
        let address: Address<Bech32m> = Address::builder()
            .prefix(prefix)
            .array(*astria_verifying_key.address_bytes())
            .try_build()?;

        println!("DKG completed successfully!");
        println!("Secret key package saved to: {secret_key_package_path}");
        println!("Public key package saved to: {public_key_package_path}");
        println!("Generated address: {address}");
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
