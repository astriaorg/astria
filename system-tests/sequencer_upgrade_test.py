"""
This script provides a general test to ensure logic common to all sequencer upgrades have executed
correctly. It also invokes checks specific to a single upgrade, as set via the `--upgrade-name`
command line arg.

The chosen upgrade name also decides the image tag to use for the start of the test where a
sequencer network is started before the upgrade is staged. It should generally reflect what is being
run in the Astria Mainnet before the given upgrade is applied.

For details on running the test, see the README.md file in `/system-tests`.
"""

import argparse
import concurrent
import blackburn_upgrade_checks
import time
from concurrent.futures import FIRST_EXCEPTION
from helpers.astria_cli import Cli
from helpers.celestia_controller import CelestiaController
from helpers.defaults import (
    EVM_DESTINATION_ADDRESS,
    SEQUENCER_IBC_TRANSFER_DESTINATION_ADDRESS,
    SEQUENCER_WITHDRAWER_ADDRESS,
    BRIDGE_TX_HASH,
    UPGRADE_CHANGES,
)
from helpers.evm_controller import EvmController
from helpers.hermes_controller import HermesController
from helpers.image_controller import ImageController
from helpers.sequencer_controller import SequencerController
from helpers.utils import update_chart_dependencies, check_change_infos
from termcolor import colored

# The number of sequencer validator nodes to use in the test.
NUM_NODES = 5
# A map of upgrade name to image tag to use for running BEFORE the given upgrade is executed.
PRE_UPGRADE_IMAGE_TAGS = ["sequencer=3.0.0"]
pre_upgrade_image_controller = ImageController(PRE_UPGRADE_IMAGE_TAGS)

parser = argparse.ArgumentParser(prog="upgrade_test", description="Runs the sequencer upgrade test.")
parser.add_argument(
    "-t", "--image-tag",
    help=
        "The tag specifying the sequencer image to run to execute the upgrade, e.g. 'latest', \
        'local', 'pr-2000'. NOTE: this is not the image used to run sequencers before the upgrade \
        is staged; that image is chosen based upon the provided --upgrade-name value.",
    metavar="TAG",
    required=True
)
parser.add_argument(
    "-n", "--upgrade-name",
    help="The name of the upgrade to apply.",
    choices=("blackburn"),
    required=True
)
args = vars(parser.parse_args())
upgrade_image_tag = args["image_tag"]
upgrade_name = args["upgrade_name"].lower()

print(colored("################################################################################", "light_blue"))
print(colored("Running sequencer upgrade test", "light_blue"))
print(colored(f"  * upgraded container image tag: {upgrade_image_tag}", "light_blue"))
print(colored(f"  * pre-upgrade container image tags: {PRE_UPGRADE_IMAGE_TAGS}", "light_blue"))
print(colored(f"  * upgrade name: {upgrade_name}", "light_blue"))
print(colored("################################################################################", "light_blue"))

if upgrade_name not in UPGRADE_CHANGES.keys():
    raise SystemExit(
        f"upgrade name {upgrade_name} not supported. Supported upgrades are: "
        f"{', '.join(UPGRADE_CHANGES.keys())}. If you want to run a test for a new upgrade, please "
        "add it to the list of supported upgrades."
    )

# Update chart dependencies.
for chart in ("sequencer", "evm-stack"):
    update_chart_dependencies(chart)

# Start `NUM_NODES - 1` sequencer validators in parallel and start the EVM rollup.
#
# Note that sequencers 0, 1 and 2 have voting power 10 and validators 3 and 4 have voting power 1.
#
# Disable the price feed on sequencer 2 to ensure the oracle still works on all nodes as long as a
# supermajority are participating.
nodes = [SequencerController(f"node{i}") for i in range(NUM_NODES - 1)]
evm = EvmController()
celestia = CelestiaController()
print(colored(f"starting {len(nodes)} sequencers, evm rollup, and local celestia network", "blue"))
executor = concurrent.futures.ThreadPoolExecutor(NUM_NODES + 2)

deploy_sequencer_fn = lambda seq_node: seq_node.deploy_sequencer(
    pre_upgrade_image_controller,
    # Enabled for all but sequencer 2.
    enable_price_feed=(seq_node.name != "node2"),
    upgrade_name=upgrade_name,
)
futures = [executor.submit(deploy_sequencer_fn, node) for node in nodes]
futures.append(executor.submit(lambda: celestia.deploy_celestia()))
futures.append(executor.submit(lambda: evm.deploy_rollup(pre_upgrade_image_controller)))
done, _ = concurrent.futures.wait(futures, return_when=FIRST_EXCEPTION)
for completed_future in done:
    completed_future.result()

# Hermes doesn't play well with parallel deployment, so deploy it last.
print(colored("deploying hermes", "blue"))
hermes = HermesController("local")
hermes.deploy_hermes(pre_upgrade_image_controller)

# Instantiate CLI
cli = Cli(upgrade_image_tag)

# Convert node addresses to astria-prefixed bech32m addresses.
for node in nodes:
    address = node.address
    node.bech32m_address = cli.address(node.name, address)

# Note block 1 and the current app version before attempting the upgrade.
for node in nodes:
    node.wait_until_chain_at_height(1, 60)
block_1_before = nodes[0].get_sequencer_block(1)
app_version_before = nodes[0].get_current_app_version()
# Expect app version to be one less than the upgrade we are testing, ensuring all
# previous upgrades have been applied by this point. Aspen (index 0) starts at
# version 2, so version before is defined as (index + 1).
expected_app_version = list(UPGRADE_CHANGES.keys()).index(upgrade_name) + 1
if app_version_before != expected_app_version:
    raise SystemExit(
        f"expected genesis app version {expected_app_version} to be the same as the current app "
        f"version {app_version_before}.\nPossibly this test has already run on this network, or "
        "persistent volume data has not been deleted between attempts?\nTry running `just clean "
        "&& rm -r /tmp/astria` (sudo may be required for `rm -r /tmp/astria`) before re-running "
        "the test."
    )

# Ensure all other sequencers report the same values.
for node in nodes[1:]:
    if block_1_before != node.get_sequencer_block(1):
        raise SystemExit(f"node0 and {node.name} report different values for block 1")
    if app_version_before != node.get_current_app_version():
        raise SystemExit(f"node0 and {node.name} report different values for current app version")

# Run pre-upgrade validator updates to check that the new action still executes correctly.
print(colored("running pre-upgrade validator updates", "blue"))
for node in nodes:
    node.power += 1
    cli.validator_update(node.name, node.pub_key, node.power)

# Submit pre-upgrade ICS20 transfer
print(colored("submitting pre-upgrade ICS20 transfer to Celestia", "blue"))
celestia.do_ibc_transfer(SEQUENCER_IBC_TRANSFER_DESTINATION_ADDRESS)

# Give time for validator updates and ICS20 transfer to land
time.sleep(10)

# Run pre-upgrade checks specific to this upgrade.
print(colored(f"running pre-upgrade checks specific to {upgrade_name}", "blue"))
if upgrade_name == "blackburn":
    blackburn_upgrade_checks.assert_pre_upgrade_conditions(cli, nodes)
print(colored(f"passed {upgrade_name}-specific pre-upgrade checks", "green"))


# Perform a bridge in.
print(colored("testing bridge in", "blue"))
evm_balance = evm.get_balance(EVM_DESTINATION_ADDRESS)
if evm_balance != 0:
    raise SystemExit(f"starting evm balance not 0: balance {evm_balance}")
cli.init_bridge_account(sequencer_name="node1")
cli.bridge_lock(sequencer_name="node2")
expected_evm_balance = 10000000000000000000
evm.wait_until_balance(EVM_DESTINATION_ADDRESS, expected_evm_balance, 30)
print(colored("bridge in succeeded", "green"))

print(colored("app version before upgrade:", "blue"), colored(f"{app_version_before}", "yellow"))

# Get the current block height from the sequencer and set the upgrade to activate soon.
block_height_difference = 10
latest_block_height = nodes[0].get_last_block_height()
upgrade_activation_height = latest_block_height + block_height_difference
print(colored("setting upgrade activation height to", "blue"), colored(f"{upgrade_activation_height}", "yellow"))
# Leave the last sequencer running the old binary through the upgrade to ensure it can catch up
# later. Pop it from the `nodes` list and re-add it later once it's caught up.
missed_upgrade_node = nodes.pop()
print(colored(f"not upgrading {missed_upgrade_node.name} until the rest have executed the upgrade", "blue"))
for node in nodes:
    node.stage_upgrade(
        ImageController([f"sequencer={upgrade_image_tag}"]),
        enable_price_feed=(node.name != "node2"),
        upgrade_name=upgrade_name,
        activation_height=upgrade_activation_height,
    )

# Wait for the rollout to complete.
print(colored("waiting for pods to become ready", "blue"))
wait_for_upgrade_fn = lambda seq_node: seq_node.wait_for_upgrade(upgrade_activation_height)
futures = [executor.submit(wait_for_upgrade_fn, node) for node in nodes]
done, _ = concurrent.futures.wait(futures, return_when=FIRST_EXCEPTION)
for completed_future in done:
    completed_future.result()

# Ensure the last sequencer has stopped.
try:
    if missed_upgrade_node.try_get_last_block_height() >= upgrade_activation_height:
        raise SystemExit(f"{missed_upgrade_node.name} should be stalled but isn't")
except Exception:
    # This is the expected branch - the node should have crashed when it disagreed about the outcome
    # of executing the block at the upgrade activation height.
    pass
print(colored(f"{missed_upgrade_node.name} lagging as expected; now upgrading", "blue"))

# Now stage the upgrade on this lagging node and ensure it catches up.
missed_upgrade_node.stage_upgrade(
    ImageController([f"sequencer={upgrade_image_tag}"]),
    enable_price_feed=True,
    upgrade_name=upgrade_name,
    activation_height=upgrade_activation_height
)
missed_upgrade_node.wait_for_upgrade(upgrade_activation_height)
latest_block_height = nodes[0].get_last_block_height()
timeout_secs = max((latest_block_height - upgrade_activation_height) * 10, 30)
missed_upgrade_node.wait_until_chain_at_height(latest_block_height, timeout_secs)
print(colored(f"{missed_upgrade_node.name} has caught up", "green"))
# Re-add the lagging node to the list.
nodes.append(missed_upgrade_node)

# Start a fifth sequencer validator now that the upgrade has happened.
new_node = SequencerController(f"node{NUM_NODES - 1}")
print(colored("starting a new sequencer", "blue"))
new_node.deploy_sequencer(
    ImageController([f"sequencer={upgrade_image_tag}"]),
    upgrade_name=upgrade_name,
    upgrade_activation_height=upgrade_activation_height
)
new_node.bech32m_address = cli.address(new_node.name, new_node.address)

# Wait for the new node to catch up and go through the upgrade too.
new_node.wait_until_chain_at_height(upgrade_activation_height + 2, 60)
print(colored(f"new sequencer {new_node.name} has caught up", "green"))
# Add the new node to the list.
nodes.append(new_node)

# Check the app version has increased.
app_version_after = nodes[0].get_current_app_version()
for node in nodes[1:]:
    if node.get_current_app_version() <= app_version_before:
        raise SystemExit(f"{node.name} failed to upgrade: app version unchanged")
    if node.get_current_app_version() != app_version_after:
        raise SystemExit(f"node0 and {node.name} report different values for app version")
print(colored("app version changed after upgrade to:", "blue"), colored(f"{app_version_after}", "yellow"))

# Check that fetching block 1 yields the same result as before the upgrade (ensures test network
# didn't just restart from genesis using the upgraded binary rather than actually performing a
# network upgrade).
block_1_after = nodes[0].get_sequencer_block(1)
if block_1_before != block_1_after:
    raise SystemExit(
        "node0 failed to upgrade. block 1 is different as reported before and after the upgrade"
    )
for node in nodes[1:]:
    if node.get_sequencer_block(1) != block_1_after:
        raise SystemExit(f"node0 and {node.name} report different values for block 1")
print(colored("fetching block 1 after the upgrade yields the same result as before the upgrade", "green"))

# Fetch and check the upgrade change infos. There should be none scheduled and at least one applied.
applied, scheduled = nodes[0].get_upgrades_info()
if len(list(scheduled)) != 0:
    raise SystemExit("node0 upgrade error: should have no scheduled upgrade change infos")
check_change_infos(applied, nodes[0].upgrade_heights, app_version_after)
for node in nodes[1:]:
    this_applied, this_scheduled = node.get_upgrades_info()
    if this_applied != applied:
        raise SystemExit(
            f"node0 and {node.name} report different values for applied upgrade changes"
        )
    if this_scheduled != scheduled:
        raise SystemExit(
            f"node0 and {node.name} report different values for scheduled upgrade changes"
        )
print(colored("upgrade change infos reported correctly", "green"))

# Submit validator updates with names for all validators
print(colored("running post-upgrade validator updates", "blue"))
for node in nodes:
    cli.validator_update(node.name, node.pub_key, node.power)

# Submit post-upgrade ICS20 transfer
print(colored("submitting post-upgrade ICS20 transfer to Celestia", "blue"))
celestia.do_ibc_transfer(SEQUENCER_IBC_TRANSFER_DESTINATION_ADDRESS)

# Give time for validator updates and ICS20 transfer to land
time.sleep(10)

# Run post-upgrade checks specific to this upgrade.
print(colored(f"running post-upgrade checks specific to {upgrade_name}", "blue"))
if upgrade_name == "blackburn":
    # non fee asset ICS20 transfer should have failed post blackburn.
    blackburn_upgrade_checks.assert_post_upgrade_conditions(cli, nodes, 53000)

    print(colored("adding utia asset to sequencer", "blue"))
    cli.add_utia_asset()
    print(colored("utia asset added to sequencer", "green"))

    print(colored("submitting post-upgrade ICS20 transfer of fee-asset to Celestia", "blue"))
    celestia.do_ibc_transfer(SEQUENCER_IBC_TRANSFER_DESTINATION_ADDRESS)

    # Give time for ICS20 transfer to land
    time.sleep(10)

    blackburn_upgrade_checks.assert_post_upgrade_conditions(cli, nodes, 106000)

print(colored(f"passed {upgrade_name}-specific post-upgrade checks", "green"))

# Perform a bridge out.
print(colored("testing bridge out", "blue"))
# bridge_tx_bytes is the tx to the withdraw smart contract on the evm.
# Uses private key for 0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30 to sign tx.
# was created via:
#  `forge script script/AstriaWithdrawer.s.sol:AstriaWithdrawerScript \
#        --rpc-url "http://executor.astria.127.0.0.1.nip.io/" \
#        --legacy \
#        --broadcast \
#        --sig "withdrawToSequencer()" -vvvv`
# w/ values:
#  PRIVATE_KEY=0x8b3a7999072c9c9314c084044fe705db11714c6c4ed7cddb64da18ea270dd203
#  ASTRIA_WITHDRAWER=0xA58639fB5458e65E4fA917FF951C390292C24A15
#  SEQUENCER_DESTINATION_CHAIN_ADDRESS="astria17w0adeg64ky0daxwd2ugyuneellmjgnxl39504"
#  AMOUNT=1000000000000000000
evm.send_raw_tx(
    "0xf8f280843ba60f5782a35194a58639fb5458e65e4fa917ff951c390292c24a15880de0b6b3a7640000b884bab916"
    "d000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000"
    "00000000000000000000000000000000002d617374726961313777306164656736346b793064617877643275677975"
    "6e65656c6c6d6a676e786c333935303400000000000000000000000000000000000000820a95a034652da1bbcad94f"
    "6af3db785127dae70f9b4e7d4da3c3f4b36eafe7fce9bf58a0169ed71974bcd74f0cea148148b5f3f8da50cdd05505"
    "7dd18a599a2a3e14679f"
)
expected_evm_balance = 9000000000000000000
evm.wait_until_balance(EVM_DESTINATION_ADDRESS, expected_evm_balance, timeout_secs=60)
print(colored("bridge out evm success", "green"))
expected_balance = 1000000000
cli.wait_until_balance(SEQUENCER_WITHDRAWER_ADDRESS, expected_balance, timeout_secs=60, sequencer_name="node3")
print(colored("bridge out sequencer success", "green"))
print(colored("testing tx finalization", "blue"))
tx_block_number = evm.get_tx_block_number(BRIDGE_TX_HASH)
evm.wait_until_chain_at_height(tx_block_number, timeout_secs=60)
print(colored("sequencer network upgraded successfully", "green"))
