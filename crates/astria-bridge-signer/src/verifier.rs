use std::{
    str::FromStr as _,
    sync::Arc,
};

use astria_core::{
    generated::astria::protocol::transaction::v1::TransactionBody as RawTransactionBody,
    primitive::v1::{
        asset::Denom,
        Address,
    },
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
    bail,
    ensure,
    eyre,
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
    /// Creates a new `Verifier` with the given rollup RPC endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider cannot be created.
    pub fn new(rollup_rpc_endpoint: String) -> eyre::Result<Self> {
        let provider = Provider::<Http>::try_from(rollup_rpc_endpoint)
            .wrap_err("failed to create provider")?;

        Ok(Self {
            provider: Arc::new(provider),
        })
    }

    pub(crate) async fn verify_message_to_sign(&self, message: &[u8]) -> eyre::Result<()> {
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
        let actions = get_expected_actions_from_tx_info(
            self.provider.clone(),
            tx_hash,
            event_index,
            act.bridge_address,
            act.fee_asset.clone(),
            act.rollup_block_number,
        )
        .await
        .wrap_err("failed to get expected actions from bridge unlock tx info")?;
        Ok(actions)
    }

    async fn get_expected_actions_from_ics20_withdrawal(
        &self,
        act: &Ics20Withdrawal,
    ) -> eyre::Result<Vec<Action>> {
        let memo: astria_core::protocol::memos::v1::Ics20WithdrawalFromRollup =
            serde_json::from_str(&act.memo).wrap_err("failed to deserialize memo")?;
        let (tx_hash, event_index) =
            parse_rollup_withdrawal_event_id(&memo.rollup_withdrawal_event_id)?;
        let Some(bridge_address) = act.bridge_address else {
            bail!("ic20 withdrawal bridge address must be set")
        };
        let actions = get_expected_actions_from_tx_info(
            self.provider.clone(),
            tx_hash,
            event_index,
            bridge_address,
            act.fee_asset.clone(),
            memo.rollup_block_number,
        )
        .await
        .wrap_err("failed to get expected actions from ics20 withdrawal tx info")?;
        Ok(actions)
    }
}

async fn get_expected_actions_from_tx_info(
    provider: Arc<Provider<Http>>,
    tx_hash: H256,
    event_index: usize,
    bridge_address: Address,
    fee_asset: Denom,
    rollup_block_number: u64,
) -> eyre::Result<Vec<Action>> {
    let tx = provider
        .get_transaction_receipt(tx_hash)
        .await
        .wrap_err("failed to get transaction")?
        .ok_or_eyre("transaction not found")?;
    let log = tx
        .logs
        .get(event_index)
        .ok_or_eyre(format!("log at {event_index} not found"))?;
    let contract_address = log.address;
    let contract = astria_bridge_contracts::i_astria_withdrawer::IAstriaWithdrawer::new(
        contract_address,
        provider.clone(),
    );
    let sequencer_asset_str = contract
        .base_chain_asset_denomination()
        .await
        .wrap_err("failed to get base chain asset denomination")?;
    let sequencer_asset = Denom::from_str(&sequencer_asset_str)
        .wrap_err("failed to parse base chain asset denomination")?;

    let getter = astria_bridge_contracts::GetWithdrawalActionsBuilder::new()
        .provider(provider.clone())
        .contract_address(contract_address)
        .bridge_address(bridge_address)
        .fee_asset(fee_asset)
        .sequencer_asset_to_withdraw(sequencer_asset)
        .use_compat_address(false)
        .try_build()
        .await
        .wrap_err("failed to build getter")?;
    let block = provider
        .get_block(rollup_block_number)
        .await
        .wrap_err("failed to get block")?
        .ok_or_eyre("block not found")?;
    let actions: Vec<Action> = getter
        .get_for_block_hash(block.hash.ok_or_eyre("block hash is None")?)
        .await
        .wrap_err("failed to get withdraw actions for block hash")?
        .into_iter()
        .filter_map(std::result::Result::ok)
        .collect();
    Ok(actions)
}

fn parse_rollup_withdrawal_event_id(event_id: &str) -> eyre::Result<(H256, usize)> {
    let regex = regex::Regex::new(r"^(0x[0-9a-fA-F]+).(0x[0-9a-fA-F]+)$")
        .wrap_err("failed to create regex")?;
    let captures = regex
        .captures(event_id)
        .ok_or_else(|| eyre::eyre!("failed to capture event id with regex"))?;
    let tx_hash_bytes: [u8; 32] = hex::decode(captures.get(1).unwrap().as_str())
        .wrap_err("failed to decode tx hash as hex")?
        .try_into()
        .map_err(|_| eyre!("invalid tx hash length; expected 32 bytes"))?;
    let tx_hash = H256::from(tx_hash_bytes);
    let event_index_bytes: [u8; 4] = hex::decode(captures.get(2).unwrap().as_str())
        .wrap_err("failed to decode event index as hex")?
        .split_at_checked(28)
        .ok_or_eyre("invalid event index length; less than 28 bytes")?
        .1
        .try_into()
        .map_err(|_| eyre!("invalid event index length; expected 32 bytes"))?;
    let event_index = usize::try_from(u32::from_be_bytes(event_index_bytes))
        .wrap_err("failed to convert event index to usize")?;
    Ok((tx_hash, event_index))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_rollup_withdrawal_event_id_ok() {
        let event_id = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef.\
                        0x0000000000000000000000000000000000000000000000000000000000000033";
        let (tx_hash, event_index) = parse_rollup_withdrawal_event_id(event_id).unwrap();
        assert_eq!(
            tx_hash,
            H256::from_slice(
                &hex::decode("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
                    .unwrap()
            )
        );
        assert_eq!(event_index, 0x33);
    }

    #[test]
    fn parse_rollup_withdrawal_event_id_invalid_tx_hash() {
        let event_id = "0x22.0x0000000000000000000000000000000000000000000000000000000000000033";
        assert!(parse_rollup_withdrawal_event_id(event_id)
            .unwrap_err()
            .to_string()
            .contains("invalid tx hash length; expected 32 bytes"));

        let event_id = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef.\
                        0x0000000000000000000000000000000000000000000000000000000000000033";
        assert!(parse_rollup_withdrawal_event_id(event_id)
            .unwrap_err()
            .to_string()
            .contains("failed to capture event id with regex"));
    }

    #[test]
    fn parse_rollup_withdrawal_event_id_invalid_event_index() {
        let event_id = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef.0x22";
        assert!(parse_rollup_withdrawal_event_id(event_id)
            .unwrap_err()
            .to_string()
            .contains("invalid event index length; less than 28 bytes"));

        let event_id = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef.\
                        0x00000000000000000000000000000000000000000000000000000000000033";
        assert!(parse_rollup_withdrawal_event_id(event_id)
            .unwrap_err()
            .to_string()
            .contains("invalid event index length; expected 32 bytes"));

        let event_id = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef.\
                        0000000000000000000000000000000000000000000000000000000000000033";
        assert!(parse_rollup_withdrawal_event_id(event_id)
            .unwrap_err()
            .to_string()
            .contains("failed to capture event id with regex"));
    }
}
