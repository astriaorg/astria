use astria_core::generated::protocol::transaction::v1::TransactionBody;
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

    /// path to file with message bytes to verify
    #[arg(long)]
    message_path: String,

    /// hex-encoded signature
    #[arg(long)]
    signature: String,

    /// Set if the incoming message is plaintext, otherwise it is assumed to be `TransactionBody`
    /// in pbjson format
    #[arg(long)]
    plaintext: bool,
}

impl Command {
    pub(super) fn run(self) -> eyre::Result<()> {
        use prost::Message as _;
        let Self {
            verifying_key,
            message_path,
            signature,
            plaintext,
        } = self;

        let mut message = std::fs::read(&message_path).wrap_err("failed to read message file")?;
        if !plaintext {
            let tx_body = serde_json::from_slice::<TransactionBody>(&message)
                .wrap_err("failed to deserialize message as TransactionBody")?;
            message = tx_body.encode_to_vec();
        }

        let verifying_key = frost_ed25519::VerifyingKey::deserialize(
            &hex::decode(verifying_key).wrap_err("failed to parse verifying key")?,
        )
        .wrap_err("failed to parse verifying key")?;
        let signature = frost_ed25519::Signature::deserialize(
            &hex::decode(signature).wrap_err("failed to parse signature")?,
        )
        .wrap_err("failed to parse signature")?;

        verifying_key
            .verify(&message, &signature)
            .wrap_err("signature is invalid")?;
        println!("Signature is valid");

        Ok(())
    }
}
