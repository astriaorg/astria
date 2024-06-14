# Astria rollup bridging protocol

The Astria sequencer implements a native bridging protocol which allows for
bridging assets from the sequencer to rollups which decide to use this protocol.

At a high level, the sequencer-to-rollup protocol works as follows:

- a bridge account is initialized on the sequencer which has an associated
rollup ID, which is the rollup ID that the rollup reads its block data from
- the rollup enshrines the sequencer bridge account into its consensus,
authorizing transfers (locks) into this bridge account of a specific asset
to result in mints of the synthetic token on the rollup
- users then send transfers to the sequencer bridge account, which result in
`Deposit` events being included as part of the rollup's block data
- when the rollup sees a `Deposit`, it mints the corresponding tokens to the
user's account

This is similar to how existing L1-to-rollup bridges work, where some lock
event happening on the L1 results in a deposit transaction being derived
automatically on the rollup.

The rollup-to-sequencer protocol works as follows:

- the bridge account has an associated private key. this could potentially be
backed by the threshold signature scheme/multisig.
- a bridge withdrawer is able to sign transaction which move funds out of the
bridge account to other sequencer accounts.
- there is a withdrawal contract deployed on the rollup which emits `Withdrawal`
events when funds are locked in it (and effectively burned, as the contract
should not be able to transfer its own funds).
- the bridge withdrawer watches for these `Withdrawal` events. when it sees
one, it sends a sequencer transaction which transfers funds from the bridge
account to the account specified in the withdrawal event.

Since the sequencer can support multiple assets via IBC, the bridging protocol
also has support for deposits and withdrawals via IBC, as well as support for
various assets to be bridged to the rollup.

The one-step deposit flow from another IBC chain is as follows:

- a user initiates an ICS20 withdrawal from some IBC chain to Astria.
- the user sets the destination chain address as the sequencer bridge address
which corresponds to the rollup they wish to deposit to.
- the user sets the withdrawal memo to their rollup address.
- when the ICS20 transfer is executed on Astria, the funds are locked into the
bridge address, and a `Deposit` event is created for that rollup's ID with the
user's rollup address.

The one-step withdrawal flow from a rollup to another IBC chain is as follows:

- the user withdraws the asset by burning it on the rollup, which emits a
`Withdrawal` event that includes the asset, the amount, and a memo.
- the memo contains the needed information for creating an Ics20 withdrawal on
the sequencer, such as the recipient destination chain address.
- the bridge withdrawer sees the `Withdrawal` event and submits a sequencer
transaction which withdraws the funds to the destination IBC chain.
- in the case that the transfer to the destination IBC chain fails, the
sequencer is notified of this, and emits a `Deposit` refunding the funds back
to the origin rollup address.

## Implementation

Within the sequencer application, the bridging logic is located in the
[`bridge`](https://github.com/astriaorg/astria/tree/main/crates/astria-sequencer/src/bridge)
module, and the IBC-bridging logic is in the
[`ibc`](https://github.com/astriaorg/astria/tree/main/crates/astria-sequencer/src/ibc)
module.

The bridge related actions are:

- [`InitBridgeAccountAction`](https://github.com/astriaorg/astria/blob/6902ef35370e5980a76302fc756e1a9a56af21b5/proto/protocolapis/astria/protocol/transactions/v1alpha1/types.proto#L167):
initializes the signer of the action as a bridge account. The associated rollup
ID and asset ID which this account accepts are provided. Optional `sudo_address`
and `withdrawer_address` fields can be provided, which are set to the action
sender if unset.
  - the account's rollup ID and asset ID cannot be changed once initialized.
  - the account cannot be re-initialized as a bridge account, and it cannot be
    converted back into a non-bridge account.
  - `sudo_address` is authorized to change the rollup account's `sudo_address`
    and `withdrawer_address`.
  - to make withdrawals from the bridge account, the `withdrawer_address` must
    be the transaction signer.
  - withdrawals are allowed using either `BridgeUnlockAction` or `Ics20Withdrawal`.
- [`BridgeLockAction`](https://github.com/astriaorg/astria/blob/6902ef35370e5980a76302fc756e1a9a56af21b5/proto/protocolapis/astria/protocol/transactions/v1alpha1/types.proto#L188):
transfers funds to a bridge account, locking them and emitting a
[`Deposit`](https://github.com/astriaorg/astria/blob/6902ef35370e5980a76302fc756e1a9a56af21b5/proto/sequencerblockapis/astria/sequencerblock/v1alpha1/block.proto#L76).
The `destination_chain_address` is the rollup account funds are minted to.
- [`BridgeUnlockAction`](https://github.com/astriaorg/astria/blob/main/proto/protocolapis/astria/protocol/transactions/v1alpha1/types.proto#L207):
transfers funds from a bridge account to another account. The asset transferred
is the one for the bridge account (ie. the asset ID specified in `InitBridgeAccountAction`).
The signer of this action must be the bridge account's `withdrawer_address`.
- [`BridgeSudoChangeAction`](https://github.com/astriaorg/astria/blob/6902ef35370e5980a76302fc756e1a9a56af21b5/proto/protocolapis/astria/protocol/transactions/v1alpha1/types.proto#L222)
changes the bridge account's sudo and/or withdrawer addresses. The signer of
this action must be the bridge account's `sudo_address`.

The two IBC actions which can also perform bridging actions are an `IbcRelay`
which contains an `Ics20Transfer` packet, and `Ics20Withdrawal`.

- [`Ics20Transfer`](https://github.com/astriaorg/astria/blob/6902ef35370e5980a76302fc756e1a9a56af21b5/crates/astria-sequencer/src/ibc/ics20_transfer.rs#L370)
packets are received when an IBC transfer to Astria from another chain occurs.
If the recipient of the packet is a bridge account and the asset transferred is
valid, the funds are locked in the bridge account and a `Deposit` is created.
- [`Ics20Withdrawal`](https://github.com/astriaorg/astria/blob/6902ef35370e5980a76302fc756e1a9a56af21b5/proto/protocolapis/astria/protocol/transactions/v1alpha1/types.proto#L102)
represents a withdrawal from Astria to another IBC chain. If the `bridge_address`
field is set, the funds are transferred out of a bridge account. Alternatively,
if `bridge_address` is unset but the signer of the action is a bridge address,
and the withdrawer address is the same as the bridge address, the funds are
transferred out of the bridge account.
