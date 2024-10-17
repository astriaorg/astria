use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use frost_ed25519::{
    self,
};

#[derive(Debug, clap::Args)]
pub(super) struct Command {
    /// hex-encoded verifying key
    #[arg(long)]
    verifying_key: String,

    // path to file with message bytes to verify
    #[arg(long)]
    message_path: String,

    // hex-encoded signature
    #[arg(long)]
    signature: String,
}

impl Command {
    pub(super) fn run(self) -> eyre::Result<()> {
        let Self {
            verifying_key,
            message_path,
            signature,
        } = self;

        let message = std::fs::read(&message_path).wrap_err("failed to read message file")?;

        let verifying_key = frost_ed25519::VerifyingKey::deserialize(
            &hex::decode(verifying_key).wrap_err("failed to parse verifying key")?,
        )
        .wrap_err("failed to parse verifying key")?;
        let signature = frost_ed25519::Signature::deserialize(
            &hex::decode(signature).wrap_err("failed to parse signature")?,
        )
        .wrap_err("failed to parse signature")?;

        match verifying_key.verify(&message, &signature) {
            Ok(()) => println!("Signature is valid"),
            Err(e) => println!("Signature is invalid: {e}"),
        }

        Ok(())
    }
}
