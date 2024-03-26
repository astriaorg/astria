#!/bin/bash

set -o errexit -o nounset -o pipefail

# Request Celestia token if connecting to celestia
BEARER_TOKEN=""
if [ "$ASTRIA_CONDUCTOR_EXECUTION_COMMIT_LEVEL" != "SoftOnly" ]; then
    BEARER_TOKEN=$(wget -qO- "$TOKEN_SERVER_URL")

    if [ -z "$BEARER_TOKEN" ]; then
        echo "Failed to fetch the Celestia bearer token."
        exit 1
    fi

    echo "Celestia Bearer token fetched successfully."
fi

export ASTRIA_CONDUCTOR_CELESTIA_BEARER_TOKEN="$BEARER_TOKEN"

exec /usr/local/bin/astria-conductor
