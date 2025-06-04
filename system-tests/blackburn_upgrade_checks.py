from helpers.defaults import SEQUENCER_IBC_TRANSFER_DESTINATION_ADDRESS

"""
This module contains checks specific to `Blackburn`.
"""

def assert_pre_upgrade_conditions(cli, nodes):
    _check_balance_post_ics20_transfer(cli, nodes, 53000)

def assert_post_upgrade_conditions(cli, nodes, upgrade_activation_height):
    _check_balance_post_ics20_transfer(cli, nodes, 106000)

def _check_balance_post_ics20_transfer(cli, nodes, expected_balance):
    for node in nodes:
        actual = cli._try_get_balance(SEQUENCER_IBC_TRANSFER_DESTINATION_ADDRESS, node.name, "transfer/channel-0/utia")
        if actual != expected_balance:
            raise SystemExit(
                f"{node.name}: balance {actual}, expected {expected_balance} after IBC transfer"
            )
