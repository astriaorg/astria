"""
This script provides an end-to-end smoke test of the Astria Sequencer and EVM rollup.

The test first deploys the EVM Rollup and Astria Sequencer and awaits their readiness.
It then creates and funds a Bridge account corresponding to the rollup on the Sequencer,
followed by a check that the funds have correctly landed in the bridge account
and on the rollup. It then submits a bridge withdrawal transaction to the rollup
and checks that the funds have been correctly withdrawn, followed by a check that
the funds have been withdrawn to the correct Sequencer account.

For details on running the test, see the README.md file in `/system-tests`.
"""

import argparse
import concurrent
from concurrent.futures import FIRST_EXCEPTION
from helpers.astria_cli import Cli
from helpers.defaults import (
    BASE_AMOUNT,
    BRIDGE_TX_BYTES,
    BRIDGE_TX_HASH,
    EVM_DESTINATION_ADDRESS,
    ROLLUP_MULTIPLIER,
    SEQUENCER_WITHDRAWER_ADDRESS,
    TRANSFER_AMOUNT,
)
from helpers.evm_controller import EvmController
from helpers.sequencer_controller import SequencerController
from helpers.utils import update_chart_dependencies
from termcolor import colored

parser = argparse.ArgumentParser(prog="smoke_test", description="Runs the smoke test.")
parser.add_argument(
    "-t", "--image-tag",
    help=
        "The image tag to use, e.g. 'latest', 'local', 'pr-2000'.",
    metavar="TAG",
    required=True
)
args = vars(parser.parse_args())
image_tag = args["image_tag"]

print(colored("################################################################################", "light_blue"))
print(colored("Running Astria Stack smoke test", "light_blue"))
print(colored(f"  * image tag: {image_tag}", "light_blue"))
print(colored("################################################################################", "light_blue"))

# Update chart dependencies.
for chart in ("sequencer", "evm-stack"):
    update_chart_dependencies(chart)


# Deploy Sequencer and EVM Rollup
print(colored("Deploying Sequencer and EVM Rollup...", "blue"))
executor = concurrent.futures.ThreadPoolExecutor(max_workers=2)
sequencer_node = SequencerController("single")
evm = EvmController()
deploy_sequencer_fn = lambda seq_node: seq_node.deploy_sequencer(image_tag, image_tag, enable_price_feed=False)
deploy_evm_fn = lambda evm_node: evm_node.deploy_rollup(image_tag)
futures = [executor.submit(deploy_sequencer_fn, sequencer_node),
           executor.submit(deploy_evm_fn, evm)]
done, _ = concurrent.futures.wait(futures, return_when=FIRST_EXCEPTION, timeout=600)
for completed_future in done:
    completed_future.result()

sequencer_node.wait_until_chain_at_height(1, 60)

# Instantiate CLI
cli = Cli(image_tag)

# Check starting balance
print(colored("Checking starting balance...", "blue"))
balance = evm.get_balance()
if balance != 0:
    raise SystemExit(colored(f"rollup: expected balance to be 0, but got {balance}", "red"))
print(colored("Starting balance OK", "green"))

# Initialize the bridge account
print(colored("Initializing bridge account on Sequencer...", "blue"))
cli.init_bridge_account(sequencer_node.name)
print(colored("Bridge account initialized", "green"))

# Bridge deposit
print(colored("Executing Bridge Lock on Sequencer...", "blue"))
cli.bridge_lock(sequencer_node.name)
print(colored("Bridge Lock executed", "green"))

# Wait for funds to land
print(colored("Waiting for deposit on EVM Rollup...", "blue"))
expected_balance = TRANSFER_AMOUNT * BASE_AMOUNT * ROLLUP_MULTIPLIER
evm.wait_until_balance(EVM_DESTINATION_ADDRESS, TRANSFER_AMOUNT*BASE_AMOUNT*ROLLUP_MULTIPLIER, 30)
print(colored("Bridge in flow successful", "green"))

# Bridge withdrawal
print(colored("Executing Bridge Withdrawal on EVM Rollup...", "blue"))
evm.send_raw_tx(BRIDGE_TX_BYTES)
print(colored("Bridge Withdrawal executed", "green"))
print(colored("Waiting for funds to be withdrawn from EVM Rollup...", "blue"))
transfer_balance = BASE_AMOUNT * ROLLUP_MULTIPLIER
evm.wait_until_balance(EVM_DESTINATION_ADDRESS, expected_balance - transfer_balance, 30)
print(colored("Bridge Withdrawal successful", "green"))
print(colored("Waiting for funds on Sequencer...", "blue"))
cli.wait_until_balance(SEQUENCER_WITHDRAWER_ADDRESS, BASE_AMOUNT, 30, sequencer_node.name)
print(colored("Funds seen on Sequencer", "green"))

# TX Finalization
print(colored("Waiting for transaction to be finalized...", "blue"))
finalized_block_height = evm.get_tx_block_number(BRIDGE_TX_HASH)
evm.wait_until_chain_at_height(finalized_block_height, 30)
print(colored("Transaction finalized", "green"))
print(colored("Smoke test successful", "green"))
