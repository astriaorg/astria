#!/bin/bash

set -o errexit -o nounset

geth --datadir $home_dir/.astriageth/ init $home_dir/genesis.json
