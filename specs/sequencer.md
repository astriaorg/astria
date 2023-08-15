# Astria Sequencer

The Astria sequencer is an ABCI application built in Rust, using Tower ABCI. It
creates ordered blocks of transactions (bytes) across many rollups identified by
a `chain-id`, and is driven by the CometBFT consensus engine.
