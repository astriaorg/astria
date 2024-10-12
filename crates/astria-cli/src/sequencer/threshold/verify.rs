use color_eyre::eyre::{
    self,
    WrapErr as _,
    eyre,
};
use frost_ed25519::{
    self,
};

#[derive(Debug, clap::Args)]
pub(super) struct Command {
    /// hex-encoded verifying key
    #[arg(long)]
    verifying_key: String,

    // message string to verify
    #[arg(long)]
    message: String,

    // hex-encoded signature
    #[arg(long)]
    signature: String,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        let Self {
            verifying_key,
            message,
            signature,
        } = self;

        let verifying_key = frost_ed25519::VerifyingKey::deserialize(
            hex::decode(verifying_key)?
                .try_into()
                .map_err(|_| eyre!("verifying key must be 32 bytes"))?,
        )
        .wrap_err("failed to parse verifying key")?;
        let signature = frost_ed25519::Signature::deserialize(
            hex::decode(signature)?
                .try_into()
                .map_err(|_| eyre!("signature must be 64 bytes"))?,
        )
        .wrap_err("failed to parse signature")?;

        match verifying_key.verify(message.as_bytes(), &signature) {
            Ok(()) => println!("Signature is valid"),
            Err(e) => println!("Signature is invalid: {}", e),
        }

        Ok(())
    }
}
