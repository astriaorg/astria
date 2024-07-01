use astria_bridge_withdrawer::bridge_withdrawer;
use astria_core::{
    bridge::Ics20WithdrawalFromRollupMemo,
    generated::protocol::transaction::v1alpha1::{
        IbcHeight,
        SignedTransaction,
    },
    primitive::v1::asset::default_native_asset,
    protocol::transaction::v1alpha1::{
        action::{
            BridgeUnlockAction,
            Ics20Withdrawal,
        },
        Action,
    },
};
use tendermint_rpc::{
    endpoint::broadcast::tx_sync,
    request,
};
use tracing::debug;
use wiremock::Request;

const DEFAULT_LAST_ROLLUP_HEIGHT: u64 = 1;
const DEFAULT_IBC_DENOM: &str = "transfer/channel-0/utia";

// TODO:
// 1. add sequencer grpc mock server
// 2. add mock for pending nonce
