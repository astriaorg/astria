use std::sync::Arc;

use astria_core::{
    generated::astria::protocol::transaction::v1::TransactionBody as RawTransactionBody,
    protocol::transaction::v1::{
        action::{
            BridgeUnlock,
            Ics20Withdrawal,
        },
        Action,
        TransactionBody,
    },
    Protobuf as _,
};
use astria_eyre::{
    eyre,
    eyre::WrapErr as _,
};
use ethers::{
    core::types::Block,
    providers::{
        Http,
        Middleware,
        Provider,
        ProviderError,
    },
    types::H256,
    utils::hex,
};
use prost::Message as _;

pub struct Verifier {
    provider: Arc<Provider<Http>>,
}

impl Verifier {
    pub fn new(rollup_rpc_endpoint: String) -> eyre::Result<Self> {
        let provider = Provider::<Http>::try_from(rollup_rpc_endpoint)
            .wrap_err("failed to create provider")?;

        Ok(Self {
            provider: Arc::new(provider),
        })
    }

    pub async fn verify_message_to_sign(&self, message: Vec<u8>) -> eyre::Result<()> {
        let raw = RawTransactionBody::decode(message.as_slice())
            .wrap_err("failed to decode bytes into raw proto transaction")?;
        let tx_body = TransactionBody::try_from_raw(raw)
            .wrap_err("failed to convert raw transaction body into transaction body")?;
        for action in tx_body.actions() {
            match action {
                Action::BridgeUnlock(act) => self
                    .verify_bridge_unlock(act)
                    .await
                    .wrap_err("failed to verify bridge unlock")?,
                Action::Ics20Withdrawal(act) => self
                    .verify_ics20_withdrawal(act)
                    .await
                    .wrap_err("failed to verify ics20 withdrawal")?,
                _ => return Err(eyre::eyre!("unsupported action")),
            }
        }

        Ok(())
    }

    async fn verify_bridge_unlock(&self, act: &BridgeUnlock) -> eyre::Result<()> {
        let block = self
            .provider
            .get_block(act.rollup_block_number)
            .await
            .wrap_err("failed to get block")?
            .ok_or_else(|| eyre::eyre!("block not found"))?;
        Ok(())
    }

    async fn verify_ics20_withdrawal(&self, act: &Ics20Withdrawal) -> eyre::Result<()> {
        Ok(())
    }
}
