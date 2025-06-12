import time
from helpers.defaults import SEQUENCER_IBC_TRANSFER_DESTINATION_ADDRESS
from termcolor import colored

"""
This module contains checks specific to `Blackburn`.
"""

RETRIES = 10

def assert_pre_upgrade_conditions(cli, nodes):
    _check_balance_post_ics20_transfer(cli, nodes, 53000)

def assert_post_upgrade_conditions(cli, nodes, upgrade_activation_height):
    _check_balance_post_ics20_transfer(cli, nodes, 106000)

def _check_balance_post_ics20_transfer(cli, nodes, expected_balance):
    delay = 1
    for node in nodes:
        for _ in range(RETRIES):
            try:
                actual = cli._try_get_balance(SEQUENCER_IBC_TRANSFER_DESTINATION_ADDRESS, node.name, "transfer/channel-0/utia")
                if actual != expected_balance:
                    raise SystemExit(
                        f"{node.name}: balance {actual}, expected {expected_balance} after IBC transfer"
                    )
                else:
                    break
            except Exception as e:
                print(colored(f"Error checking balance for {node.name}: {e}, retrying in {delay} seconds...", "yellow"))
                time.sleep(delay)
                delay = min(delay * 2, 5)
