use std::{
    str::FromStr as _,
    sync::Arc,
};

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
use astria_eyre::eyre::{
    self,
    ensure,
    OptionExt,
    WrapErr as _,
};
use ethers::{
    providers::{
        Http,
        Middleware,
        Provider,
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

    pub async fn verify_message_to_sign(&self, message: &[u8]) -> eyre::Result<()> {
        let raw = RawTransactionBody::decode(message)
            .wrap_err("failed to decode bytes into raw proto transaction")?;
        let tx_body = TransactionBody::try_from_raw(raw)
            .wrap_err("failed to convert raw transaction body into transaction body")?;
        let expected_actions = match tx_body
            .actions()
            .first()
            .ok_or_eyre("transaction is empty")?
        {
            Action::BridgeUnlock(act) => self
                .get_expected_actions_from_bridge_unlock(act)
                .await
                .wrap_err("failed to verify bridge unlock")?,
            Action::Ics20Withdrawal(act) => self
                .get_expected_actions_from_ics20_withdrawal(act)
                .await
                .wrap_err("failed to verify ics20 withdrawal")?,
            _ => return Err(eyre::eyre!("unsupported action")),
        };
        ensure!(
            tx_body.actions().len() == expected_actions.len(),
            "number of actions does not match expected"
        );

        for (actual, expected) in tx_body.actions().iter().zip(expected_actions.iter()) {
            match (actual, expected) {
                (Action::BridgeUnlock(actual), Action::BridgeUnlock(expected)) => {
                    ensure!(
                        actual == expected,
                        "actual bridge unlock action does not match expected"
                    );
                }
                (Action::Ics20Withdrawal(actual), Action::Ics20Withdrawal(expected)) => {
                    ensure!(
                        actual == expected,
                        "actual ics20 withdrawal action does not match expected"
                    );
                }
                _ => return Err(eyre::eyre!("action type does not match expected type")),
            }
        }

        Ok(())
    }

    async fn get_expected_actions_from_bridge_unlock(
        &self,
        act: &BridgeUnlock,
    ) -> eyre::Result<Vec<Action>> {
        let (tx_hash, event_index) =
            parse_rollup_withdrawal_event_id(&act.rollup_withdrawal_event_id)?;
        let tx = self
            .provider
            .get_transaction_receipt(tx_hash)
            .await
            .wrap_err("failed to get transaction")?
            .ok_or_eyre("transaction not found")?;
        let log = tx
            .logs
            .get(event_index as usize)
            .ok_or_eyre("log not found")?;
        let contract_address = log.address;
        let contract = astria_bridge_contracts::i_astria_withdrawer::IAstriaWithdrawer::new(
            contract_address,
            self.provider.clone(),
        );
        let sequencer_asset_str = contract
            .base_chain_asset_denomination()
            .await
            .wrap_err("failed to get base chain asset denomination")?;
        let sequencer_asset = Denom::from_str(&sequencer_asset_str)
            .wrap_err("failed to parse base chain asset denomination")?;
        use astria_core::primitive::v1::asset::Denom;

        let getter = astria_bridge_contracts::GetWithdrawalActionsBuilder::new()
            .provider(self.provider.clone())
            .contract_address(contract_address)
            .bridge_address(act.bridge_address)
            .fee_asset(act.fee_asset.clone())
            .sequencer_asset_to_withdraw(sequencer_asset)
            .use_compat_address(false)
            .try_build()
            .await
            .wrap_err("failed to build getter")?;
        let block = self
            .provider
            .get_block(act.rollup_block_number)
            .await
            .wrap_err("failed to get block")?
            .ok_or_eyre("block not found")?;
        let actions: Vec<Action> = getter
            .get_for_block_hash(block.hash.ok_or_eyre("block hash is None")?)
            .await
            .wrap_err("failed to get withdraw actions for block hash")?
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();
        Ok(actions)
    }

    async fn get_expected_actions_from_ics20_withdrawal(
        &self,
        act: &Ics20Withdrawal,
    ) -> eyre::Result<Vec<Action>> {
        Ok(vec![])
    }
}

fn parse_rollup_withdrawal_event_id(event_id: &str) -> eyre::Result<(H256, u64)> {
    let regex = regex::Regex::new(r"^(0x[0-9a-fA-F]{64}).(0x[0-9a-fA-F]{64})$")
        .wrap_err("failed to create regex")?;
    let captures = regex
        .captures(event_id)
        .ok_or_else(|| eyre::eyre!("failed to capture"))?;
    let tx_hash = H256::from_slice(&hex::decode(captures.get(1).unwrap().as_str())?);
    let event_index = u64::from_str_radix(captures.get(2).unwrap().as_str(), 16)
        .wrap_err("failed to parse event index")?;
    Ok((tx_hash, event_index))
}
