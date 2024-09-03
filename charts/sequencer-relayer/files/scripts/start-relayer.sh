#!/bin/bash

set -o errexit -o nounset -o pipefail

echo "Starting the Astria Sequencer Relayer..."

if ! [ -z ${ASTRIA_SEQUENCER_RELAYER_SUBMISSION_STATE_PATH+x} ] && ! [ -f "$ASTRIA_SEQUENCER_RELAYER_SUBMISSION_STATE_PATH" ]; then
    echo "Submission state file does not exist, instantiating with fresh state. Will start relaying from first sequencer block."
    echo "{\"state\": \"fresh\"}" > $ASTRIA_SEQUENCER_RELAYER_SUBMISSION_STATE_PATH
fi

exec /usr/local/bin/astria-sequencer-relayer
