use astria_bridge_contracts::i_astria_withdrawer::{
    Ics20WithdrawalFilter,
    SequencerWithdrawalFilter,
};
use astria_core::protocol::{
    memos::v1::Ics20WithdrawalFromRollup,
    transaction::v1::action::{
        BridgeUnlock,
        Ics20Withdrawal,
    },
};
use ethers::{
    abi::{
        self,
        AbiEncode,
        Token,
    },
    contract::EthEvent as _,
    types::{
        Block,
        Filter,
        Log,
        TransactionReceipt,
        H160,
        H256,
    },
    utils::hex,
};
use serde_json::json;
use wiremock::{
    matchers::{
        body_partial_json,
        method,
    },
    Mock,
    MockServer,
    ResponseTemplate,
};

pub(crate) struct MockRollup {
    inner: MockServer,
    cur_block_hash_base: u64,
}

impl MockRollup {
    pub(super) async fn new() -> Self {
        let inner = MockServer::builder().start().await;
        Self {
            inner,
            cur_block_hash_base: 0,
        }
    }

    pub(super) fn get_url(&self) -> String {
        self.inner.uri()
    }

    pub(super) async fn mount_ics20_withdrawal_verification(&mut self, act: &Ics20Withdrawal) {
        let memo: Ics20WithdrawalFromRollup = serde_json::from_str(&act.memo)
            .expect("deserializing Ics20Withdrawal memo shouldnot fail");
        let (tx_hash, event_index) =
            parse_rollup_withdrawal_event_id(&memo.rollup_withdrawal_event_id);
        let tx_receipt = make_transaction_receipt(event_index);

        self.mount_tx_receipt(&tx_hash, &tx_receipt).await;
        let block = self.make_block();
        self.mount_get_block(memo.rollup_block_number, &block).await;

        let contract_address = tx_receipt.logs[event_index].address;
        self.mount_base_chain_asset_denomination(contract_address)
            .await;
        self.mount_base_chain_asset_precision(contract_address)
            .await;
        let filter = Filter::new()
            .at_block_hash(block.hash.unwrap())
            .address(tx_receipt.logs[event_index].address)
            .topic0(Ics20WithdrawalFilter::signature());

        self.mount_get_logs(
            &filter,
            vec![ics20_withdrawal_to_log(
                act,
                memo.rollup_return_address,
                memo.rollup_block_number,
                tx_hash,
                event_index,
                memo.memo,
            )],
        )
        .await;
    }

    pub(super) async fn mount_bridge_unlock_verification(&mut self, act: &BridgeUnlock) {
        let (tx_hash, event_index) =
            parse_rollup_withdrawal_event_id(&act.rollup_withdrawal_event_id);
        let tx_receipt = make_transaction_receipt(event_index);

        self.mount_tx_receipt(&tx_hash, &tx_receipt).await;
        let block = self.make_block();
        self.mount_get_block(act.rollup_block_number, &block).await;

        let contract_address = tx_receipt.logs[event_index].address;
        self.mount_base_chain_asset_denomination(contract_address)
            .await;
        self.mount_base_chain_asset_precision(contract_address)
            .await;
        let filter = Filter::new()
            .at_block_hash(block.hash.unwrap())
            .address(tx_receipt.logs[event_index].address)
            .topic0(SequencerWithdrawalFilter::signature());

        self.mount_get_logs(
            &filter,
            vec![bridge_unlock_to_log(
                act,
                act.rollup_block_number,
                tx_hash,
                event_index,
            )],
        )
        .await;
    }

    async fn mount_base_chain_asset_denomination(&self, contract_address: H160) {
        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "eth_call",
                "params": [{
                    "to": contract_address,
                    "data": "0xb6476c7e"
                }, "latest"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                // ABI-encoded "nria"
                "result": hex::encode(AbiEncode::encode([Token::String("nria".to_string())]))
            })))
            .mount(&self.inner)
            .await;
    }

    async fn mount_base_chain_asset_precision(&self, contract_address: H160) {
        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "eth_call",
                "params": [{
                    "to": contract_address,
                    "data": "0x7eb6dec7"
                }, "latest"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": hex::encode(AbiEncode::encode([Token::Int(18.into())]))
            })))
            .mount(&self.inner)
            .await;
    }

    async fn mount_tx_receipt(&self, tx_hash: &H256, receipt: &TransactionReceipt) {
        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "eth_getTransactionReceipt",
                "params": [tx_hash],
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": receipt
            })))
            .expect(1)
            .mount(&self.inner)
            .await;
    }

    async fn mount_get_block(&self, block_number: u64, block: &Block<H256>) {
        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "eth_getBlockByNumber",
                "params": [format!("0x{block_number}"), false],
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": block
            })))
            .expect(1)
            .mount(&self.inner)
            .await;
    }

    async fn mount_get_logs(&self, filter: &Filter, actions: Vec<Log>) {
        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "method": "eth_getLogs",
                "params": [filter],
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": actions
            })))
            .expect(1)
            .mount(&self.inner)
            .await;
    }

    fn make_block(&mut self) -> Block<H256> {
        let block = Block {
            hash: Some(H256::from_low_u64_be(self.cur_block_hash_base)), // only used field
            ..Block::default()
        };
        self.cur_block_hash_base = self.cur_block_hash_base.saturating_add(1);
        block
    }
}

fn parse_rollup_withdrawal_event_id(event_id: &str) -> (H256, usize) {
    let regex = regex::Regex::new(r"^(0x[0-9a-fA-F]+).(0x[0-9a-fA-F]+)$")
        .expect("regex creation should not fail");
    let captures = regex
        .captures(event_id)
        .expect("capturing event id with regex should not fail");
    let tx_hash_bytes: [u8; 32] = hex::decode(captures.get(1).unwrap().as_str())
        .expect("decoding tx hash as hex should not fail")
        .try_into()
        .expect("tx hash length should be 32 bytes");
    let tx_hash = H256::from(tx_hash_bytes);
    let event_index_bytes: [u8; 4] = hex::decode(captures.get(2).unwrap().as_str())
        .expect("decoding event index as hash should succeed")
        .split_at_checked(28)
        .expect("event index length should be at least 28 bytes")
        .1
        .try_into()
        .expect("event index length should be 32 bytes");
    let event_index = usize::try_from(u32::from_be_bytes(event_index_bytes))
        .expect("convert event index to usize should not fail");
    (tx_hash, event_index)
}

fn make_transaction_receipt(event_index: usize) -> TransactionReceipt {
    let mut logs = vec![];
    logs.insert(
        event_index,
        Log {
            address: H160::zero(), // `contact_address`, only used field
            ..Log::default()
        },
    );
    TransactionReceipt {
        logs,
        ..TransactionReceipt::default()
    }
}

fn ics20_withdrawal_to_log(
    act: &Ics20Withdrawal,
    rollup_return_address: String,
    rollup_block_number: u64,
    tx_hash: H256,
    event_index: usize,
    memo: String,
) -> Log {
    let event_signature_hash = Ics20WithdrawalFilter::signature();

    let sender_topic = {
        let bytes: [u8; 32] = hex::decode(rollup_return_address)
            .unwrap()
            .try_into()
            .unwrap();
        H256::from(bytes)
    };

    let amount_topic = {
        let mut bytes = [0u8; 32];
        // Place the u128 value in the last 16 bytes
        bytes[16..32].copy_from_slice(&act.amount.to_be_bytes());
        H256::from(bytes)
    };

    let data = abi::encode(&[
        Token::String(act.destination_chain_address.clone()),
        Token::String(memo),
    ]);

    Log {
        topics: vec![event_signature_hash, sender_topic, amount_topic],
        data: data.into(),
        block_number: Some(rollup_block_number.into()),
        transaction_hash: Some(tx_hash),
        transaction_index: Some(0.into()),
        log_index: Some(event_index.into()),
        ..Log::default()
    }
}

fn bridge_unlock_to_log(
    act: &BridgeUnlock,
    rollup_block_number: u64,
    tx_hash: H256,
    event_index: usize,
) -> Log {
    let event_signature = SequencerWithdrawalFilter::signature();
    let sender_topic = {
        let mut bytes: [u8; 32] = [0u8; 32];
        bytes[12..32].copy_from_slice(act.bridge_address.as_bytes());
        H256::from(bytes)
    };
    let amount_topic = {
        let mut bytes = [0u8; 32];
        bytes[16..32].copy_from_slice(&act.amount.to_be_bytes());
        H256::from(bytes)
    };
    let data = ethers::abi::encode(&[Token::String(act.to.to_string())]);
    Log {
        topics: vec![event_signature, sender_topic, amount_topic],
        data: data.into(),
        block_number: Some(rollup_block_number.into()),
        transaction_hash: Some(tx_hash),
        log_index: Some(event_index.into()),
        ..Log::default()
    }
}
