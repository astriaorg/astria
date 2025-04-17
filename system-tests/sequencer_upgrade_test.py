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
import aspen_upgrade_checks
from concurrent.futures import FIRST_EXCEPTION
from helpers.astria_cli import Cli
from helpers.evm_controller import EvmController
from helpers.sequencer_controller import SequencerController
from helpers.utils import update_chart_dependencies, check_change_infos

# The number of sequencer validator nodes to use in the test.
NUM_NODES = 5
# A map of upgrade name to sequencer, relayer image to use for running BEFORE the given upgrade is executed.
PRE_UPGRADE_IMAGE_TAGS = {
    "aspen": ("2.0.1", "1.0.1"),
}
EVM_DESTINATION_ADDRESS = "0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30"
ACCOUNT = "astria17w0adeg64ky0daxwd2ugyuneellmjgnxl39504"
BRIDGE_TX_HASH = "0x326c3910da4c96c5a40ba1505fc338164b659729f2f975ccb07e8794c96b66f6"

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
    choices=("aspen",),
    required=True
)
args = vars(parser.parse_args())
upgrade_image_tag = args["image_tag"]
upgrade_name = args["upgrade_name"].lower()

print("################################################################################")
print("Running sequencer upgrade test")
print(f"  * upgraded container image tag: {upgrade_image_tag}")
print(f"  * pre-upgrade container image tags: {PRE_UPGRADE_IMAGE_TAGS[upgrade_name]}")
print(f"  * upgrade name: {upgrade_name}")
print("################################################################################")

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
print(f"starting {len(nodes)} sequencers and the evm rollup")
executor = concurrent.futures.ThreadPoolExecutor(NUM_NODES + 1)

deploy_sequencer_fn = lambda seq_node: seq_node.deploy_sequencer(
    PRE_UPGRADE_IMAGE_TAGS[upgrade_name][0],
    PRE_UPGRADE_IMAGE_TAGS[upgrade_name][1],
    # Enabled for all but sequencer 2.
    enable_price_feed=(seq_node.name != "node2"),
    upgrade_name=upgrade_name,
)
futures = [executor.submit(deploy_sequencer_fn, node) for node in nodes]
futures.append(executor.submit(lambda: evm.deploy_rollup(upgrade_image_tag)))
done, _ = concurrent.futures.wait(futures, return_when=FIRST_EXCEPTION)
for completed_future in done:
    completed_future.result()

# Note block 1 and the current app version before attempting the upgrade.
for node in nodes:
    node.wait_until_chain_at_height(1, 60)
block_1_before = nodes[0].get_sequencer_block(1)
app_version_before = nodes[0].get_current_app_version()
genesis_app_version = nodes[0].get_app_version_at_genesis()
if app_version_before != genesis_app_version:
    raise SystemExit(
        f"expected genesis app version {genesis_app_version} to be the same as the current app "
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
    if genesis_app_version != node.get_app_version_at_genesis():
        raise SystemExit(f"node0 and {node.name} report different values for genesis app version")

# Run pre-upgrade checks specific to this upgrade.
print(f"running pre-upgrade checks specific to {upgrade_name}")
if upgrade_name == "aspen":
    aspen_upgrade_checks.assert_pre_upgrade_conditions(nodes)
print(f"passed {upgrade_name}-specific pre-upgrade checks")

print("app version before upgrade:", app_version_before)

# Perform a bridge in.
print("testing bridge in")
evm_balance = evm.get_balance(EVM_DESTINATION_ADDRESS)
if evm_balance != 0:
    raise SystemExit(f"starting evm balance not 0: balance {evm_balance}")
cli = Cli(upgrade_image_tag)
cli.init_bridge_account(sequencer_name="node1")
cli.bridge_lock(sequencer_name="node2")
expected_evm_balance = 10000000000000000000
evm.wait_until_balance(EVM_DESTINATION_ADDRESS, expected_evm_balance, 30)
print("bridge in succeeded")

# Get the current block height from the sequencer and set the upgrade to activate soon.
block_height_difference = 10
latest_block_height = nodes[0].get_last_block_height()
upgrade_activation_height = latest_block_height + block_height_difference
print("setting upgrade activation height to", upgrade_activation_height)
# Leave the last sequencer running the old binary through the upgrade to ensure it can catch up
# later. Pop it from the `nodes` list and re-add it later once it's caught up.
missed_upgrade_node = nodes.pop()
print(f"not upgrading {missed_upgrade_node.name} until the rest have executed the upgrade")
for node in nodes:
    node.stage_upgrade(
        upgrade_image_tag,
        upgrade_image_tag,
        enable_price_feed=(node.name != "node2"),
        upgrade_name=upgrade_name,
        activation_height=upgrade_activation_height,
    )

# Wait for the rollout to complete.
print("waiting for pods to become ready")
wait_for_upgrade_fn = lambda seq_node: seq_node.wait_for_upgrade(upgrade_activation_height)
futures = [executor.submit(wait_for_upgrade_fn, node) for node in nodes]
done, _ = concurrent.futures.wait(futures, return_when=FIRST_EXCEPTION)
for completed_future in done:
    completed_future.result()

# Ensure the last sequencer has stopped.
try:
    if missed_upgrade_node.try_get_last_block_height() >= upgrade_activation_height:
        raise SystemExit(f"{missed_upgrade_node.name} should be stalled but isn't")
except Exception as error:
    # This is the expected branch - the node should have crashed when it disagreed about the outcome
    # of executing the block at the upgrade activation height.
    pass
print(f"{missed_upgrade_node.name} lagging as expected; now upgrading")

# Now stage the upgrade on this lagging node and ensure it catches up.
missed_upgrade_node.stage_upgrade(
    upgrade_image_tag,
    upgrade_image_tag,
    enable_price_feed=True,
    upgrade_name=upgrade_name,
    activation_height=upgrade_activation_height
)
missed_upgrade_node.wait_for_upgrade(upgrade_activation_height)
latest_block_height = nodes[0].get_last_block_height()
timeout_secs = max((latest_block_height - upgrade_activation_height) * 10, 30)
missed_upgrade_node.wait_until_chain_at_height(latest_block_height, timeout_secs)
print(f"{missed_upgrade_node.name} has caught up")
# Re-add the lagging node to the list.
nodes.append(missed_upgrade_node)

# Start a fifth sequencer validator now that the upgrade has happened.
new_node = SequencerController(f"node{NUM_NODES - 1}")
print(f"starting a new sequencer")
new_node.deploy_sequencer(
    upgrade_image_tag,
    upgrade_image_tag,
    upgrade_name=upgrade_name,
    upgrade_activation_height=upgrade_activation_height
)

# Wait for the new node to catch up and go through the upgrade too.
new_node.wait_until_chain_at_height(upgrade_activation_height + 2, 60)
print(f"new sequencer {new_node.name} has caught up")
# Add the new node to the list.
nodes.append(new_node)

# Check the app version has increased.
app_version_after = nodes[0].get_current_app_version()
for node in nodes[1:]:
    if node.get_current_app_version() <= app_version_before:
        raise SystemExit(f"{node.name} failed to upgrade: app version unchanged")
    if node.get_current_app_version() != app_version_after:
        raise SystemExit(f"node0 and {node.name} report different values for app version")
print("app version changed after upgrade to:", app_version_after)

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
print("fetching block 1 after the upgrade yields the same result as before the upgrade")

# Fetch and check the upgrade change infos. There should be none scheduled and at least one applied.
applied, scheduled = nodes[0].get_upgrades_info()
if len(list(scheduled)) != 0:
    raise SystemExit("node0 upgrade error: should have no scheduled upgrade change infos")
check_change_infos(applied, upgrade_activation_height, app_version_after)
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
print("upgrade change infos reported correctly")

# Run post-upgrade checks specific to this upgrade.
print(f"running post-upgrade checks specific to {upgrade_name}")
if upgrade_name == "aspen":
    aspen_upgrade_checks.assert_post_upgrade_conditions(nodes, upgrade_activation_height)
print(f"passed {upgrade_name}-specific post-upgrade checks")

# Perform a bridge out.
print("testing bridge out")
# bridge_tx_bytes is the tx to the withdraw smart contract on the evm.
# Uses private key for 0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30 to sign tx.
# was created via:
#  `forge script script/AstriaWithdrawer.s.sol:AstriaWithdrawerScript \
#        --rpc-url "http://executor.astria.localdev.me/" \
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
print("bridge out evm success")
expected_balance = 1000000000
cli.wait_until_balance(ACCOUNT, expected_balance, timeout_secs=60, sequencer_name="node3")
print("bridge out sequencer success")
print("testing tx finalization")
tx_block_number = evm.get_tx_block_number(BRIDGE_TX_HASH)
evm.wait_until_chain_at_height(tx_block_number, timeout_secs=60)
print("sequencer network upgraded successfully")
