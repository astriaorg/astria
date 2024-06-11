# Astria rollup bridging protocol

The Astria sequencer implements a native bridging protocol which allows for bridging assets from the sequencer to rollups which decide to use this protocol.

At a high level, the sequencer-to-rollup protocol works as follows:

- a bridge account is initialized on the sequencer which has an associated rollup ID, which is the rollup ID that the rollup reads its block data from
- the rollup enshrines the sequencer bridge account into its consensus, authorizing transfers (locks) into this bridge account of a specific asset to result in mints of the synthetic token on the rollup
- users then send transfers to the sequencer bridge account, which result in `Deposit` events being included as part of the rollup's block data
- when the rollup sees a `Deposit`, it mints the corresponding tokens to the user's account

This is similar to how existing L1-to-rollup bridges work, where some lock event happening on the L1 results in a deposit transaction being derived automatically on the rollup.

The rollup-to-sequencer protocol works as follows:
- the bridge account has an associated private key. this could potentially be backed by the threshold signature scheme/multisig.
- a bridge withdrawer is able to sign transaction which move funds out of the bridge account to other sequencer accounts.
- there is a withdrawal contract deployed on the rollup which emits `Withdrawal` events when funds are locked in it (and effectively burned, as the contract should not be able to transfer its own funds).
- the bridge withdrawer watches for these `Withdrawal` events. when it sees one, it sends a sequencer transaction which transfers funds from the bridge account to the account specified in the withdrawal event.

Since the sequencer can support multiple assets via IBC, the bridging protocol also has support for deposits and withdrawals via IBC, as well as support for various assets to be bridged to the rollup.

The one-step deposit flow from another IBC chain is as follows:
- a user initiates an ICS20 withdrawal from some IBC chain to Astria.
- the user sets the destination chain address as the sequencer bridge address which corresponds to the rollup they wish to deposit to.
- the user sets the withdrawal memo to their rollup address.
- when the ICS20 transfer is executed on Astria, the funds are locked into the bridge address, and a `Deposit` event is created for that rollup's ID with the user's rollup address.

The one-step withdrawal flow from a rollup to another IBC chain is as follows:
- the user withdraws the asset by burning it on the rollup, which emits a `Withdrawal` event that includes the asset, the amount, and a memo.
- the memo contains the needed information for creating an Ics20 withdrawal on the sequencer, such as the recipient destination chain address.
- the bridge withdrawer sees the `Withdrawal` event and submits a sequencer transaction which withdraws the funds to the destination IBC chain.
- in the case that the transfer to the destination IBC chain fails, the sequencer is notified of this, and emits a `Deposit` refunding the funds back to the origin rollup address.
