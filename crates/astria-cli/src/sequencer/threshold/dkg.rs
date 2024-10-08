use std::collections::BTreeMap;

use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use frost_ed25519::{
    keys::{
        dkg::{
            self,
            round1,
            round2,
        },
        PublicKeyPackage,
    },
    Identifier,
};
use rand::thread_rng;

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
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        let rng = thread_rng();

        let Self {
            index,
            min_signers,
            max_signers,
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

        println!("Save the following information!");
        println!(
            "Our secret key package: {}",
            hex::encode(key_package.serialize()?)
        );
        println!(
            "Public key package: {}",
            serde_json::to_string_pretty(&pubkey_package)
                .wrap_err("failed to serialize public key packages")?
        );
        println!("DKG completed successfully!");
        Ok(())
    }
}

// from penumbra `ActualTerminal`
async fn read_line_raw() -> eyre::Result<String> {
    use std::io::{
        Read as _,
        Write as _,
    };

    use termion::color;
    // Use raw mode to allow reading more than 1KB/4KB of data at a time
    // See https://unix.stackexchange.com/questions/204815/terminal-does-not-accept-pasted-or-typed-lines-of-more-than-1024-characters
    use termion::raw::IntoRawMode;

    print!("{}", color::Fg(color::Red));
    // In raw mode, the input is not mirrored into the terminal, so we need
    // to read char-by-char and echo it back.
    let mut stdout = std::io::stdout().into_raw_mode()?;

    let mut bytes = Vec::with_capacity(8192);
    for b in std::io::stdin().bytes() {
        let b = b?;
        // In raw mode, we need to handle control characters ourselves
        if b == 3 || b == 4 {
            // Ctrl-C or Ctrl-D
            return Err(eyre::eyre!("aborted"));
        }
        // In raw mode, the enter key might generate \r or \n, check either.
        if b == b'\n' || b == b'\r' {
            break;
        }
        // Store the byte we read and print it back to the terminal.
        bytes.push(b);
        stdout.write_all(&[b]).expect("stdout write failed");
        // Flushing may not be the most efficient but performance isn't critical here.
        stdout.flush()?;
    }
    // Drop _stdout to restore the terminal to normal mode
    std::mem::drop(stdout);
    // We consumed a newline of some kind but didn't echo it, now print
    // one out so subsequent output is guaranteed to be on a new line.
    println!("");
    print!("{}", color::Fg(color::Reset));

    let line = String::from_utf8(bytes)?;
    Ok(line)
}
