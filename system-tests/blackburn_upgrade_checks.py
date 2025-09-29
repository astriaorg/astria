import time
from helpers.defaults import (
    SEQUENCER_IBC_TRANSFER_DESTINATION_ADDRESS,
    EVM_DESTINATION_ADDRESS,
    SEQUENCER_FUNDS_SIGNING_KEY,
    TRANSFER_AMOUNT,
    BASE_AMOUNT,
    SEQUENCER_BRIDGE_ADDRESS
)
from termcolor import colored

"""
This module contains checks specific to `Blackburn`.
"""

RETRIES = 10

def assert_pre_upgrade_conditions(cli, nodes):
    _check_balance_post_ics20_transfer(cli, nodes, 53000)

def assert_post_upgrade_conditions(cli, celestia, nodes, upgrade_activation_height):
    _check_balance_post_ics20_transfer(cli, nodes, 53000) # should not change since utia should be blocked from depositing
    print(colored("adding utia asset to sequencer", "blue"))
    cli.add_utia_asset()
    print(colored("utia asset added to sequencer", "green"))

    print(colored("submitting post-upgrade ICS20 transfer of fee-asset to Celestia", "blue"))
    celestia.do_ibc_transfer(SEQUENCER_IBC_TRANSFER_DESTINATION_ADDRESS)

    # Give time for ICS20 transfer to land
    time.sleep(10)

    print(colored("checking balance after post-upgrade ICS20 transfer of fee-asset", "blue"))
    _check_balance_post_ics20_transfer(cli, nodes, 106000)

    _check_bridge_account_deposits_disabled(cli, nodes)


def _check_bridge_account_deposits_disabled(cli, nodes):
    # we can't check the actual error string since stderr isn't captured, so we
    # check that the command succeeds prior to disabling deposits, and fails after
    print(colored("checking bridge account deposits are currently enabled", "blue"))
    cli.bridge_lock(nodes[0].name)
    print(colored("bridge lock succeeded as expected", "green"))
    print(colored("removing funds from bridge account", "blue"))
    print(colored("disabling bridge account deposits", "blue"))
    cli.bridge_sudo_change(nodes[0].name, disable_deposits=True)
    print(colored("checking bridge account deposits are actually disabled", "blue"))

    try:
        cli._try_exec_sequencer_command(
                "bridge-lock",
            SEQUENCER_BRIDGE_ADDRESS,
            f"--amount={TRANSFER_AMOUNT*BASE_AMOUNT}",
            f"--destination-chain-address={EVM_DESTINATION_ADDRESS}",
            f"--private-key={SEQUENCER_FUNDS_SIGNING_KEY}",
            "--sequencer.chain-id=sequencer-test-chain-0",
            "--fee-asset=nria",
            "--asset=nria",
            sequencer_name=node.name
        )
        raise SystemExit(f"Bridge account deposits are not disabled for {node.name}")
    except Exception as e:
        print(colored("bridge lock failed as expected", "green"))

def _check_balance_post_ics20_transfer(cli, nodes, expected_balance):
    delay = 1
    for node in nodes:
        for _ in range(RETRIES):
            try:
                actual = cli._try_get_balance(SEQUENCER_IBC_TRANSFER_DESTINATION_ADDRESS, node.name, "transfer/channel-0/utia")
                if actual != expected_balance:
                    raise Exception(
                        f"{node.name}: balance {actual}, expected {expected_balance} after IBC transfer"
                    )
                else:
                    break
            except Exception as e:
                print(colored(f"Error checking balance for {node.name}: {e}, retrying in {delay} seconds...", "yellow"))
                time.sleep(delay)
                delay = min(delay * 2, 5)
