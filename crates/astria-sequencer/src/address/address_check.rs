use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1::action::{
        BridgeLock,
        BridgeSudoChange,
        BridgeTransfer,
        BridgeUnlock,
        FeeAssetChange,
        FeeChange,
        IbcRelayerChange,
        IbcSudoChange,
        Ics20Withdrawal,
        InitBridgeAccount,
        RecoverIbcClient,
        RollupDataSubmission,
        SudoAddressChange,
        Transfer,
        ValidatorUpdate,
    },
};
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use cnidarium::StateRead;
use penumbra_ibc::IbcRelay;

use crate::address::StateReadExt as _;

#[async_trait::async_trait]
pub(crate) trait AddressCheck: Addresses {
    /// Checks that all addresses in the action are base prefixed
    async fn check_addresses<S: StateRead>(&self, state: S) -> eyre::Result<()> {
        for (address_name, address) in self.addresses() {
            state
                .ensure_base_prefix(address)
                .await
                .wrap_err(format!("address `{address_name}` is not base prefixed"))?;
        }
        Ok(())
    }
}

impl<A: Addresses> AddressCheck for A {}

pub(crate) trait Addresses {
    /// Returns a list of all addresses in the action and their corresponding field names.
    fn addresses(&self) -> Vec<(&str, &Address)>;
}

impl Addresses for Transfer {
    fn addresses(&self) -> Vec<(&str, &Address)> {
        vec![("to", &self.to)]
    }
}

impl Addresses for SudoAddressChange {
    fn addresses(&self) -> Vec<(&str, &Address)> {
        vec![("new_address", &self.new_address)]
    }
}

impl Addresses for IbcSudoChange {
    fn addresses(&self) -> Vec<(&str, &Address)> {
        vec![("new_address", &self.new_address)]
    }
}

impl Addresses for Ics20Withdrawal {
    fn addresses(&self) -> Vec<(&str, &Address)> {
        let mut addresses = vec![("return_address", &self.return_address)];
        if let Some(bridge_address) = &self.bridge_address {
            addresses.push(("bridge_address", bridge_address));
        }
        addresses
    }
}

impl Addresses for IbcRelayerChange {
    fn addresses(&self) -> Vec<(&str, &Address)> {
        match self {
            IbcRelayerChange::Addition(new_address) | IbcRelayerChange::Removal(new_address) => {
                vec![("new_address", new_address)]
            }
        }
    }
}

impl Addresses for InitBridgeAccount {
    fn addresses(&self) -> Vec<(&str, &Address)> {
        let mut addresses = vec![];
        if let Some(sudo_address) = &self.sudo_address {
            addresses.push(("sudo_address", sudo_address));
        };
        if let Some(withdrawer_address) = &self.withdrawer_address {
            addresses.push(("withdrawer_address", withdrawer_address));
        };
        addresses
    }
}

impl Addresses for BridgeLock {
    fn addresses(&self) -> Vec<(&str, &Address)> {
        vec![("to", &self.to)]
    }
}

impl Addresses for BridgeUnlock {
    fn addresses(&self) -> Vec<(&str, &Address)> {
        vec![("to", &self.to), ("bridge_address", &self.bridge_address)]
    }
}

impl Addresses for BridgeSudoChange {
    fn addresses(&self) -> Vec<(&str, &Address)> {
        let mut addresses = vec![("bridge_address", &self.bridge_address)];
        if let Some(new_sudo_address) = &self.new_sudo_address {
            addresses.push(("new_sudo_address", new_sudo_address));
        }
        if let Some(new_withdrawer_address) = &self.new_withdrawer_address {
            addresses.push(("new_withdrawer_address", new_withdrawer_address));
        }
        addresses
    }
}

impl Addresses for BridgeTransfer {
    fn addresses(&self) -> Vec<(&str, &Address)> {
        vec![("to", &self.to), ("bridge_address", &self.bridge_address)]
    }
}

// If adding an impl for a new action, make sure to add a test for it below :)

macro_rules! empty_impl_addresses {
    ($ ($type:ty) *) => {
        $(
            impl Addresses for $type {
                fn addresses(&self) -> Vec<(&str, &Address)> {
                    vec![]
                }
            }
        )*
    };
}

empty_impl_addresses! {
    ValidatorUpdate
    FeeAssetChange
    FeeChange
    IbcRelay
    RollupDataSubmission
    RecoverIbcClient
}

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::{
            Address,
            RollupId,
        },
        protocol::transaction::v1::{
            action::{
                BridgeLock,
                BridgeSudoChange,
                BridgeTransfer,
                BridgeUnlock,
                IbcRelayerChange,
                IbcSudoChange,
                Ics20Withdrawal,
                InitBridgeAccount,
                SudoAddressChange,
                Transfer,
            },
            Action,
        },
    };
    use ibc_types::core::client::Height;

    use super::AddressCheck;
    use crate::{
        action_handler::ActionHandler,
        address::StateWriteExt as _,
        app::StateWriteExt,
        benchmark_and_test_utils::{
            assert_eyre_error,
            astria_address,
            nria,
            ASTRIA_PREFIX,
        },
    };

    fn bad_address() -> Address {
        Address::builder()
            .prefix("bad_prefix")
            .slice(&[0; 20])
            .try_build()
            .unwrap()
    }

    /// Tests that `check_address` works correctly for the given actions. Takes a vec of tuples
    /// containing actions and the corresponding field name which is expected to fail the check.
    /// The argument should be structured something similar to:
    /// `[ (action_with_bad_field_1, "field_1"), (action_with_bad_field_2, "field_2"), ... ]`
    async fn address_check_works_as_expected<A>(actions_and_field_names: Vec<(A, &str)>)
    where
        A: ActionHandler + AddressCheck + Clone + Into<Action> + std::marker::Sync,
    {
        let temp_storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = temp_storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state
            .put_chain_id_and_revision_number("test".try_into().unwrap())
            .unwrap();

        for (action, field_name) in actions_and_field_names {
            let res = action.check_addresses(&state).await.unwrap_err();
            assert_eyre_error(
                &res,
                &format!("address `{field_name}` is not base prefixed"),
            );
        }
    }

    #[tokio::test]
    async fn check_addresses_transfer() {
        let action = Transfer {
            to: bad_address(),
            asset: nria().into(),
            fee_asset: nria().into(),
            amount: 0,
        };
        address_check_works_as_expected(vec![(action, "to")]).await;
    }

    #[tokio::test]
    async fn check_addresses_sudo_address_change() {
        let action = SudoAddressChange {
            new_address: bad_address(),
        };
        address_check_works_as_expected(vec![(action, "new_address")]).await;
    }

    #[tokio::test]
    async fn check_addresses_ibc_sudo_change() {
        let action = IbcSudoChange {
            new_address: bad_address(),
        };
        address_check_works_as_expected(vec![(action, "new_address")]).await;
    }

    #[tokio::test]
    async fn check_addresses_ics20_withdrawal() {
        let test_action = || Ics20Withdrawal {
            amount: 0,
            denom: nria().into(),
            destination_chain_address: "destination_chain_address".to_string(),
            return_address: astria_address(&[0; 20]),
            timeout_height: Height::new(1, 1).unwrap(),
            timeout_time: 1,
            source_channel: "channel-0".to_string().parse().unwrap(),
            fee_asset: nria().into(),
            memo: String::new(),
            bridge_address: None,
            use_compat_address: false,
        };
        let actions_and_field_names = vec![
            (
                Ics20Withdrawal {
                    return_address: bad_address(),
                    ..test_action()
                },
                "return_address",
            ),
            (
                Ics20Withdrawal {
                    bridge_address: Some(bad_address()),
                    ..test_action()
                },
                "bridge_address",
            ),
        ];
        address_check_works_as_expected(actions_and_field_names).await;
    }

    #[tokio::test]
    async fn check_addresses_ibc_relayer_change() {
        let action = IbcRelayerChange::Addition(bad_address());
        address_check_works_as_expected(vec![(action, "new_address")]).await;
    }

    #[tokio::test]
    async fn check_addresses_init_bridge_account() {
        let test_action = || InitBridgeAccount {
            rollup_id: RollupId::new([0; 32]),
            asset: nria().into(),
            fee_asset: nria().into(),
            sudo_address: None,
            withdrawer_address: None,
        };
        let actions_and_field_names = vec![
            (
                InitBridgeAccount {
                    sudo_address: Some(bad_address()),
                    ..test_action()
                },
                "sudo_address",
            ),
            (
                InitBridgeAccount {
                    withdrawer_address: Some(bad_address()),
                    ..test_action()
                },
                "withdrawer_address",
            ),
        ];
        address_check_works_as_expected(actions_and_field_names).await;
    }

    #[tokio::test]
    async fn check_addresses_bridge_lock() {
        let action = BridgeLock {
            to: bad_address(),
            asset: nria().into(),
            fee_asset: nria().into(),
            amount: 0,
            destination_chain_address: "destination_chain_address".to_string(),
        };
        address_check_works_as_expected(vec![(action, "to")]).await;
    }

    #[tokio::test]
    async fn check_addresses_bridge_unlock() {
        let test_action = || BridgeUnlock {
            to: astria_address(&[0; 20]),
            amount: 1,
            fee_asset: nria().into(),
            memo: String::new(),
            bridge_address: astria_address(&[1; 20]),
            rollup_block_number: 1,
            rollup_withdrawal_event_id: "a-rollup-defined-hash".to_string(),
        };
        let actions_and_field_names = vec![
            (
                BridgeUnlock {
                    to: bad_address(),
                    ..test_action()
                },
                "to",
            ),
            (
                BridgeUnlock {
                    bridge_address: bad_address(),
                    ..test_action()
                },
                "bridge_address",
            ),
        ];
        address_check_works_as_expected(actions_and_field_names).await;
    }

    #[tokio::test]
    async fn check_addresses_bridge_sudo_change() {
        let test_action = || BridgeSudoChange {
            bridge_address: astria_address(&[0; 20]),
            new_sudo_address: None,
            new_withdrawer_address: None,
            fee_asset: nria().into(),
        };
        let actions_and_field_names = vec![
            (
                BridgeSudoChange {
                    bridge_address: bad_address(),
                    ..test_action()
                },
                "bridge_address",
            ),
            (
                BridgeSudoChange {
                    new_sudo_address: Some(bad_address()),
                    ..test_action()
                },
                "new_sudo_address",
            ),
            (
                BridgeSudoChange {
                    new_withdrawer_address: Some(bad_address()),
                    ..test_action()
                },
                "new_withdrawer_address",
            ),
        ];
        address_check_works_as_expected(actions_and_field_names).await;
    }

    #[tokio::test]
    async fn check_addresses_bridge_transfer() {
        let test_action = || BridgeTransfer {
            to: astria_address(&[0; 20]),
            amount: 1,
            fee_asset: nria().into(),
            destination_chain_address: "destination_chain_address".to_string(),
            bridge_address: astria_address(&[1; 20]),
            rollup_block_number: 1,
            rollup_withdrawal_event_id: "a-rollup-defined-hash".to_string(),
        };
        let actions_and_field_names = vec![
            (
                BridgeTransfer {
                    to: bad_address(),
                    ..test_action()
                },
                "to",
            ),
            (
                BridgeTransfer {
                    bridge_address: bad_address(),
                    ..test_action()
                },
                "bridge_address",
            ),
        ];
        address_check_works_as_expected(actions_and_field_names).await;
    }
}
