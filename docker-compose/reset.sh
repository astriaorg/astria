#!/bin/bash

rm -rf .data
mkdir -p .data/cometbft
mkdir -p .data/sequencer

# Reset the .data/cometbft/priv_validator_state.json file
echo '{
  "height": "0",
  "round": 0,
  "step": 0
}' > .data/cometbft/priv_validator_state.json
