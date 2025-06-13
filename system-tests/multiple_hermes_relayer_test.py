"""
This script tests the functionality of connecting multiple Hermes relayers to a
single Astria non-validating full node. It deploys the necessary components and
ensures that the relayers do not stall upon submitting many IBC transfers to Celestia.
"""

import argparse
from termcolor import colored
from helpers.celestia_controller import CelestiaController
from helpers.sequencer_controller import SequencerController
from helpers.hermes_controller import HermesController
from helpers.image_controller import ImageController
from helpers.utils import update_chart_dependencies
from helpers.defaults import IBC_TRANSFER_AMOUNT
from concurrent.futures import (
    FIRST_EXCEPTION,
    ThreadPoolExecutor,
)
import concurrent
from helpers.astria_cli import Cli

SEQUENCER_DESTINATION_ADDRESS = "astria1mg49ywffq0tt7rkunfmd7paxcvrtvqn5yr53rq"
# Abci error code for "Nonce Taken", expected log from Hermes when stalled
ERROR_MSG  = "Error with code 15"
NUM_IBC_TRANSFERS = 10

parser = argparse.ArgumentParser(
    prog="multiple_hermes_relayer_test",
    description="Tests multiple Hermes relayers connected to a non-validating full node."
)
ImageController.add_argument(parser)
args = vars(parser.parse_args())

# Process image tags
image_controller = ImageController(args["image_tag"])

print(colored("################################################################################", "light_blue"))
print(colored("Running Hermes multiple relayers test", "light_blue"))
for component, tag in image_controller.image_tags.items():
    print(colored(f"  * specified {component} image tag: {tag}", "light_blue"))
print(colored("################################################################################", "light_blue"))

# Update chart dependencies
update_chart_dependencies("sequencer")

# Deploy Celestia and Sequencers
print(colored("Deploying Celestia, validating Sequencer, and non-validating Sequencer...", "blue"))
executor = ThreadPoolExecutor(max_workers=3)
deploy_celestia_fn = lambda celestia_node: celestia_node.deploy_celestia()
deploy_sequencer_fn = lambda seq_node: seq_node.deploy_sequencer(
    image_controller,
    enable_price_feed=False
)
celestia = CelestiaController()
sequencer_validator = SequencerController("single")
sequencer_full_node = SequencerController("full-node")
future = [
    executor.submit(deploy_celestia_fn, celestia),
    executor.submit(deploy_sequencer_fn, sequencer_validator),
    executor.submit(deploy_sequencer_fn, sequencer_full_node),
]
done, _ = concurrent.futures.wait(future, return_when=FIRST_EXCEPTION, timeout=600)
for completed_future in done:
    completed_future.result()
print(colored("Celestia and Sequencers successfully deployed", "green"))

# Deploy Hermes relayers, must be done non-concurrently because they will not init at the same time
print(colored("Deploying Hermes relayers... (this may take a few minutes)", "blue"))
hermes_relayer_0 = HermesController("full-node")
hermes_relayer_1 = HermesController("full-node-1")
hermes_relayer_0.deploy_hermes(image_controller)
hermes_relayer_1.deploy_hermes(image_controller)
print(colored("Hermes relayers successfully deployed", "green"))

# Instantiate CLI
cli_image = image_controller.cli_image_tag()
if cli_image is None:
    cli_image = "latest"
cli = Cli(cli_image)

cli.add_utia_asset()

# Check starting sequencer balance
print(colored("Checking starting balance on sequencer...", "blue"))
try:
    cli._try_get_balance(SEQUENCER_DESTINATION_ADDRESS, "full-node")
    raise SystemExit("Balance check should have returned none")
except Exception:
    print(colored("Balance check returned none as expected", "green"))

# Perform IBC transfers
print(colored("Performing IBC transfers...", "blue"))
for i in range(NUM_IBC_TRANSFERS):
    print(colored(f"Performing IBC transfer {i + 1} of {NUM_IBC_TRANSFERS}...", "blue"))
    celestia.do_ibc_transfer(
        SEQUENCER_DESTINATION_ADDRESS,
    )
    cli.wait_until_balance(
        SEQUENCER_DESTINATION_ADDRESS,
        IBC_TRANSFER_AMOUNT * (i + 1),
        60,
        "full-node",
        asset="transfer/channel-0/utia"
    )
    if hermes_relayer_0.check_logs(ERROR_MSG):
        raise SystemExit(colored("Hermes relayer 0 stalled", "red"))
    if hermes_relayer_1.check_logs(ERROR_MSG):
        raise SystemExit(colored("Hermes relayer 1 stalled", "red"))
    print(colored(f"IBC transfer {i + 1} of {NUM_IBC_TRANSFERS} completed successfully", "green"))

print(colored("All IBC transfers completed successfully", "green"))
