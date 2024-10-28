use astria_core::{
    primitive::v1::TransactionId,
    protocol::transaction::v1::action::FeeChange,
};

use crate::{
    action_handler::ActionHandler as _,
    authority::StateWriteExt as _,
    fees::StateReadExt as _,
    transaction::{
        StateWriteExt as _,
        TransactionContext,
    },
};

/// This macro generates a test named e.g. `transfer_fee_change_action_executes` which asserts
/// that executing a `FeeChange` tx for the given action results in the fees being stored for
/// the given action.
macro_rules! test_fee_change_action {
    ( $( $fee_name:tt => $fee_ty:tt ),* $(,)?) => {
        $(
            paste::item! {
                #[tokio::test]
                async fn [< $fee_name _fee_change_action_executes >] () {
                    use astria_core::protocol::fees::v1:: [< $fee_ty FeeComponents >] as Fees;

                    let storage = cnidarium::TempStorage::new().await.unwrap();
                    let snapshot = storage.latest_snapshot();
                    let mut state = cnidarium::StateDelta::new(snapshot);

                    // Put the context to enable the txs to execute.
                    state.put_transaction_context(TransactionContext {
                        address_bytes: [1; 20],
                        transaction_id: TransactionId::new([0; 32]),
                        source_action_index: 0,
                    });
                    state.put_sudo_address([1; 20]).unwrap();

                    assert!(state
                        .[< get_ $fee_name _fees >] ()
                        .await
                        .expect(stringify!(should not error fetching unstored $fee_name fees))
                        .is_none());

                    // Execute an initial fee change tx to store the first version of the fees.
                    let initial_fees = Fees {
                        base: 1,
                        multiplier: 2,
                    };
                    let fee_change = FeeChange:: $fee_ty (initial_fees);
                    fee_change.check_and_execute(&mut state).await.unwrap();

                    let retrieved_fees = state
                        .[< get_ $fee_name _fees >] ()
                        .await
                        .expect(stringify!(should not error fetching initial $fee_name fees))
                        .expect(stringify!(initial $fee_name fees should be stored));
                    assert_eq!(initial_fees, retrieved_fees);

                    // Execute a second fee change tx to overwrite the fees.
                    let new_fees = Fees {
                        base: 3,
                        multiplier: 4,
                    };
                    let fee_change = FeeChange:: $fee_ty (new_fees);
                    fee_change.check_and_execute(&mut state).await.unwrap();

                    let retrieved_fees = state
                        .[< get_ $fee_name _fees >] ()
                        .await
                        .expect(stringify!(should not error fetching new $fee_name fees))
                        .expect(stringify!(new $fee_name fees should be stored));
                    assert_ne!(initial_fees, retrieved_fees);
                    assert_eq!(new_fees, retrieved_fees);
                }
           }
        )*
    };
}

test_fee_change_action!(
transfer => Transfer,
rollup_data_submission => RollupDataSubmission,
ics20_withdrawal => Ics20Withdrawal,
init_bridge_account => InitBridgeAccount,
bridge_lock => BridgeLock,
bridge_unlock => BridgeUnlock,
bridge_sudo_change => BridgeSudoChange,
validator_update => ValidatorUpdate,
ibc_relayer_change => IbcRelayerChange,
ibc_relay => IbcRelay,
fee_asset_change => FeeAssetChange,
fee_change => FeeChange,
sudo_address_change => SudoAddressChange,
ibc_sudo_change => IbcSudoChange,
);
