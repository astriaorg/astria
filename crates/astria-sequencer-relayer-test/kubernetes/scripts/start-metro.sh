#!/bin/sh

set -o errexit -o nounset

# Start the celestia-app
exec metro start --log_level="debug" --home "${home_dir}"
