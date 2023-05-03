#!/bin/sh

set -o errexit -o nounset

# Start the celestia-app
exec metro start --home "${home_dir}"
