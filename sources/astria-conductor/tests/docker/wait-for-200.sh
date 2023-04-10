#!/bin/bash

# Use this script to test if an api endpoint is available.
# NOTE - does not currently support authorization, so only works for public endpoints.

EXPECTED_STATUS_CODE=200
TIMEOUT=30
RETRY_INTERVAL=2

if [ $# -lt 2 ]; then
  echo "Usage: $(basename "$0") <API_URL> -- <command>"
  exit 1
fi

API_URL="$1"
shift

if [ "$1" != "--" ]; then
  echo "Expected -- separator between API_URL and command"
  exit 1
fi

shift

COMMAND=$*

end=$((SECONDS+TIMEOUT))

while [ $SECONDS -lt $end ]; do
  status_code=$(curl --silent --output /dev/null --write-out "%{http_code}" --max-time $RETRY_INTERVAL "$API_URL")
  if [ "$status_code" -eq $EXPECTED_STATUS_CODE ]; then
    echo "API endpoint is available"
    # shellcheck disable=SC2086
    exec $COMMAND
  else
    echo "API endpoint not available yet, retrying..."
    sleep $RETRY_INTERVAL
  fi
done

echo "API endpoint check timed out after ${TIMEOUT}s"
exit 1
