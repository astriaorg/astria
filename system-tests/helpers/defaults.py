"""
Default values for system tests.
"""

# 1 RIA is 10^9 nRIA
BASE_AMOUNT = 1000000000
# RIA is 10^9, WEI is 10^18, 10^9 * 10^9 = 10^18
ROLLUP_MULTIPLIER = 1000000000
# 10 RIA
TRANSFER_AMOUNT = 10

# Sequencer Defaults
#####################
# corresponds to destination address in bridge_tx_bytes
SEQUENCER_WITHDRAWER_ADDRESS = "astria17w0adeg64ky0daxwd2ugyuneellmjgnxl39504"
SEQUENCER_IBC_TRANSFER_DESTINATION_ADDRESS = "astria1mg49ywffq0tt7rkunfmd7paxcvrtvqn5yr53rq"

# EVM Defaults
###############
EVM_DESTINATION_ADDRESS = "0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30"
BRIDGE_TX_BYTES = "0xf8f280843ba60f5782a35194a58639fb5458e65e4fa917ff951c390292c24a15880de0b6b3a7640000b884bab916d00000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002d617374726961313777306164656736346b7930646178776432756779756e65656c6c6d6a676e786c333935303400000000000000000000000000000000000000820a95a034652da1bbcad94f6af3db785127dae70f9b4e7d4da3c3f4b36eafe7fce9bf58a0169ed71974bcd74f0cea148148b5f3f8da50cdd055057dd18a599a2a3e14679f"
BRIDGE_TX_HASH = "0x326c3910da4c96c5a40ba1505fc338164b659729f2f975ccb07e8794c96b66f6"

# Upgrade Changes
##################
UPGRADE_CHANGES = {
    "aspen": [
        "price_feed_change",
        "validator_update_action_change",
        "ibc_acknowledgement_failure_change"
    ],
    "blackburn": [
        "ics20_transfer_action_change",
        "allow_ibc_relay_to_fail"
    ]
}

# Celestia Defaults
####################
IBC_TRANSFER_AMOUNT = 53000
