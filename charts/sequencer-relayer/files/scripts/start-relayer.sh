#!/bin/bash

set -o errexit -o nounset -o pipefail

echo "Starting the Astria Sequencer Relayer..."

if ! [ -z ${ASTRIA_SEQUENCER_RELAYER_PRE_SUBMIT_PATH+x} ] && ! [ -f "$ASTRIA_SEQUENCER_RELAYER_PRE_SUBMIT_PATH" ]; then
    echo "Pre-submit storage file not found, instantiating with ignore state. Post submit storage file will be created on first submit."
    echo "{\"state\": \"ignore\"}" > $ASTRIA_SEQUENCER_RELAYER_PRE_SUBMIT_PATH
fi

if ! [ -z ${ASTRIA_SEQUENCER_RELAYER_POST_SUBMIT_PATH+x} ] && ! [ -f "$ASTRIA_SEQUENCER_RELAYER_POST_SUBMIT_PATH" ]; then
    echo "Post-submit storage file does not exist, instantiating with fresh state. Will start relaying from first sequencer block."
    echo "{\"state\": \"fresh\"}" > $ASTRIA_SEQUENCER_RELAYER_POST_SUBMIT_PATH
fi

if ! [ -z ${ASTRIA_SEQUENCER_RELAYER_SUBMISSION_STATE_PATH+x} ] && ! [ -f "$ASTRIA_SEQUENCER_RELAYER_SUBMISSION_STATE_PATH" ]; then
    echo "Submission state file does not exist, instantiating with fresh state. Will start relaying from first sequencer block."
    echo "{\"state\": \"fresh\"}" > $ASTRIA_SEQUENCER_RELAYER_SUBMISSION_STATE_PATH
fi

exec /usr/local/bin/astria-sequencer-relayer
