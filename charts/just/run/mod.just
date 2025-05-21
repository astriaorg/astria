import '../defaults.just'
import '../evm.just'

[doc("Run recipes for IBC testing. Use 'just --list run ibc-test' for more info.")]
mod ibc-test


# Set the working directory for all recipes to the project's root.
set working-directory := "../../../"


_default_run:
  @just --list run


#################
## Run Recipes ##
#################


# RIA is 10^9, WEI is 10^18, 10^9 * 10^9 = 10^18
rollup_multiplier := "1000000000"
eth_ws_url := "ws://ws-executor.astria.127.0.0.1.nip.io/"
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
bridge_tx_bytes := "0xf8f280843ba60f5782a35194a58639fb5458e65e4fa917ff951c390292c24a15880de0b6b3a7640000b884bab916d00000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002d617374726961313777306164656736346b7930646178776432756779756e65656c6c6d6a676e786c333935303400000000000000000000000000000000000000820a95a034652da1bbcad94f6af3db785127dae70f9b4e7d4da3c3f4b36eafe7fce9bf58a0169ed71974bcd74f0cea148148b5f3f8da50cdd055057dd18a599a2a3e14679f"
bridge_tx_hash := "0x326c3910da4c96c5a40ba1505fc338164b659729f2f975ccb07e8794c96b66f6"


# Run Smoke Test
#################
[doc("
Runs end-to-end test of Astria stack and EVM rollup, including bridge in/out and
TX finalization.
NOTE: Image tags can be specified for the different components by passing the flag
`-i <component=tag>` to the command line. Pass `--help` for more info.
Examples:
  just run smoke-test (uses image tags specified in `values.yaml` files)
  just run smoke-test -i sequencer=local (specifies image tag for the sequencer)
")]
smoke-test *FLAGS:
  #!/usr/bin/env bash
  if [ -d "system-tests/.venv" ]; then
    echo "Found Python virtual environment, activating it..."
    source system-tests/.venv/bin/activate
    uv pip install -r system-tests/requirements.txt
  fi
  python3 system-tests/smoke_test.py {{FLAGS}}


# Run EVM Restart Test
#######################
[doc("
Runs end-to-end test of Astria stack and EVM rollup, including bridge in/out and
TX finalization, with a fork defined on the EVM at height 5.
NOTE: Image tags can be specified for the different components by passing the flag
`-i <component=tag>` to the command line. Pass `--help` for more info.
Examples:
  just run evm-restart-test (uses image tags specified in `values.yaml` files)
  just run evm-restart-test -i sequencer=local (specifies image tag for the sequencer)
")]
evm-restart-test *FLAGS:
  #!/usr/bin/env bash
  if [ -d "system-tests/.venv" ]; then
    echo "Found Python virtual environment, activating it..."
    source system-tests/.venv/bin/activate
    uv pip install -r system-tests/requirements.txt
  fi
  python3 system-tests/smoke_test.py {{FLAGS}} --evm-restart

# Run Smoke Test CLI
#####################
[doc("
Runs end-to-end smoke test of Astria stack (minus Bridge-Withdrawer) and EVM rollup,
including bridge in/out and TX finalization.
NOTE: Image tags can be specified for the different components by passing the flag
`-i <component=tag>` to the command line. Pass `--help` for more info.
Examples:
  just run smoke-cli-test (uses image tags specified in `values.yaml` files)
  just run smoke-cli-test -i sequencer=local (specifies image tag for the sequencer)
")]
smoke-cli-test *FLAGS:
  #!/usr/bin/env bash
  if [ -d "system-tests/.venv" ]; then
    echo "Found Python virtual environment, activating it..."
    source system-tests/.venv/bin/activate
    uv pip install -r system-tests/requirements.txt
  fi
  python3 system-tests/smoke_cli.py {{FLAGS}}


# Run Upgrade Test
###################
export PYTHONPATH := "${PYTHONPATH}:system-tests/helpers/generated"
[doc("
Runs the sequencer upgrade test in the `system-tests` folder.
<TAG> is the tag specifying the sequencer image to run to EXECUTE the upgrade,
not the image to run before the upgrade executes.
Usage:
  just run upgrade-test <TAG> (default: 'local')
")]
upgrade-test tag=defaultDockerTag:
  #!/usr/bin/env bash
  if [ -d "system-tests/.venv" ]; then
    echo "Found Python virtual environment, activating it..."
    source system-tests/.venv/bin/activate
    uv pip install -r system-tests/requirements.txt
  fi
  python3 system-tests/sequencer_upgrade_test.py \
    --image-tag {{ if tag != '' { tag } else { 'latest' } }} \
    --upgrade-name "aspen"
