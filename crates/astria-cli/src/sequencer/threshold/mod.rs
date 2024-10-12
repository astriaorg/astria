use color_eyre::eyre;

mod dkg;
mod sign;
mod verify;

#[derive(Debug, clap::Args)]
pub(super) struct Command {
    #[command(subcommand)]
    command: SubCommand,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        match self.command {
            SubCommand::Dkg(dkg) => dkg.run().await,
            SubCommand::Sign(sign) => sign.run().await,
            SubCommand::Verify(verify) => verify.run(),
        }
    }
}

#[derive(Debug, clap::Subcommand)]
enum SubCommand {
    /// distributed key generation command.
    ///
    /// generates a secret key share for each participant in the DKG protocol,
    /// along with the aggregate public key.
    Dkg(dkg::Command),

    /// threshold signing command. requires `min_signers` as specified
    /// during DKG to produce a signature.
    Sign(sign::Command),

    /// verify an ed25519 signature.
    Verify(verify::Command),
}

// from penumbra `ActualTerminal`
pub(crate) async fn read_line_raw() -> eyre::Result<String> {
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
    println!();
    print!("{}", color::Fg(color::Reset));

    let line = String::from_utf8(bytes)?;
    Ok(line)
}
