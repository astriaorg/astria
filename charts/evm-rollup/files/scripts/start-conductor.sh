#!/bin/bash

set -o errexit -o nounset -o pipefail

# Check if ASTRIA_CONDUCTOR_CELESTIA_BEARER_TOKEN is already defined
if [ -z "${ASTRIA_CONDUCTOR_CELESTIA_BEARER_TOKEN:-}" ]; then
    echo "ASTRIA_CONDUCTOR_CELESTIA_BEARER_TOKEN is not defined. Fetching the token..."
    
    # Request Celestia token if connecting to celestia
    BEARER_TOKEN=""
    if [ "$ASTRIA_CONDUCTOR_EXECUTION_COMMIT_LEVEL" != "SoftOnly" ]; then
        BEARER_TOKEN=$(wget --timeout=10 --tries=1 -O - "$TOKEN_SERVER_URL")

        if [ -z "$BEARER_TOKEN" ]; then
            echo "Failed to fetch the Celestia bearer token."
            exit 1
        fi

        echo "Celestia Bearer token fetched successfully."
    fi

    export ASTRIA_CONDUCTOR_CELESTIA_BEARER_TOKEN="$BEARER_TOKEN"
else
    echo "ASTRIA_CONDUCTOR_CELESTIA_BEARER_TOKEN is already defined. Skipping token fetch."
fi

exec /usr/local/bin/astria-conductor
