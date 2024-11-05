# Astria Rollup Bridging Protocol

The Astria sequencer implements a native bridging protocol which allows for
bridging assets from the sequencer to rollups which decide to use this protocol.

The protocol is similar to how existing L1-to-rollup bridges work, where a lock
event on the L1 results in a deposit transaction being derived automatically
on the rollup.

Since the sequencer can support multiple assets via IBC, the bridging protocol
also has support for deposits and withdrawals via IBC, as well as support for
various assets to be bridged to the rollup.

## High-Level Overview

### Sequencer-to-Rollup

1. The bridge account is initialized on the sequencer with an associated
rollup ID.
2. The rollup modifies its consensus to register the sequencer-side bridge account
for use in its state transition logic.
3. Users then send sequencer-side transfers to the bridge account (either natively
or using IBC), which are included in the rollup's block data as `Deposit` events.
4. Mints to the user's rollup-side account are derived from the `Deposit`s included
in the rollup block data.

### Rollup-to-Sequencer

1. The bridge withdrawer has an associated withdrawer address which is authorized
to withdraw from the bridge account's escrow balance.
2. The withdrawal contract is deployed on the rollup, emitting `SequencerWithdrawal`
and `Ics20Withdrawal` events when its `withdrawToSequencer` and `withdrawToIbcChain`
functions are called, respectively.
3. The bridge withdrawer watches for these events, converting them to the appropriate
sequencer-side native or IBC withdrawals and batching them by rollup block. Batches
are then submitted to the sequencer sequentially.

## Bridge Account Initialization

1. A bridge account is initialized on the sequencer with an associated rollup
ID, which is the rollup ID that the rollup reads its block data from.

2. The rollup modifies its consensus to register the sequencer bridge account
for use in its state transition logic. `Deposit`s of the correct asset to the
bridge account will be used to derive a mint of the corresponding tokens on the
rollup, according to the metadata provided.

- An example for "registering" the bridge account: [`astria-geth`](https://github.com/astriaorg/astria-geth/blob/09c27dc320570d9e1f58ea60325158f36d6a0309/genesis.json#L24)
- An example for the mint derivation logic: [`astria-geth`](https://github.com/astriaorg/astria-geth/blob/09c27dc320570d9e1f58ea60325158f36d6a0309/grpc/execution/validation.go#L27)

The bridge account is initialized with a sequencer-side [`InitBridgeAccount`](https://github.com/astriaorg/astria/blob/d03059977c3a40590d66591c520bfda3a9b9de1c/proto/protocolapis/astria/protocol/transaction/v1/action.proto#L146)
action.

After being initialized, the bridge account's rollup ID and asset ID cannot
be changed. The account also cannot be re-initialized as a bridge account, or
converted back into a non-bridge account.

Only the `sudo_address` is authorized to change the bridge account's `sudo_address`
and `withdrawer_address`.

## Withdrawer Account

The sequencer-side bridge account has an associated withdrawer account. This is a
sequencer-side account, which uses `Ed25519`, and could potentially be backed
by a threshold signature scheme.

The withdrawer account is set with a sequencer-side [`BridgeSudoChange`](https://github.com/astriaorg/astria/blob/main/proto/protocolapis/astria/protocol/transaction/v1/action.proto#L211)
action.

The sequencer-side bridge withdrawer is the only account able to make actions that
transfer funds out of the bridge account to other sequencer accounts, either via
[`BridgeUnlock` actions](https://github.com/astriaorg/astria/blob/d03059977c3a40590d66591c520bfda3a9b9de1c/proto/protocolapis/astria/protocol/transaction/v1/action.proto#L186)
or [`Ics20Withdrawal` actions](https://github.com/astriaorg/astria/blob/d03059977c3a40590d66591c520bfda3a9b9de1c/proto/protocolapis/astria/protocol/transaction/v1/action.proto#L78)

## Sequencer-To-Rollup Deposits

After the bridge account is initialized, the deposit flow works as follows:

1. When the user makes sequencer-side transfers, such as `BridgeLock`, to the
bridge account, `Deposit` events are included by the sequencer as part of the
rollup's block data.

2. When the rollup receives a `Deposit` in an `ExecuteBlockRequest`, it derives
a mint transactions to the user's account according to the metadata provided.

[`BridgeLock`](https://github.com/astriaorg/astria/blob/d03059977c3a40590d66591c520bfda3a9b9de1c/proto/protocolapis/astria/protocol/transaction/v1/action.proto#L167)
actions have the following structure:

```proto
// `BridgeLock` represents a transaction that transfers
// funds from a sequencer account to a bridge account.
//
// It's the same as a `Transfer` but with the added
// `destination_chain_address` field.
message BridgeLock {
  // the address of the bridge account to transfer to
  astria.primitive.v1.Address to = 1;
  // the amount to transfer
  astria.primitive.v1.Uint128 amount = 2;
  // the asset to be transferred
  string asset = 3;
  // the asset used to pay the transaction fee
  string fee_asset = 4;
  // the address on the destination chain which
  // will receive the bridged funds
  string destination_chain_address = 5;
}
```

- [`Ics20Transfer`](https://github.com/astriaorg/astria/blob/d03059977c3a40590d66591c520bfda3a9b9de1c/crates/astria-sequencer/src/ibc/ics20_transfer.rs#L335)
packets are received when an IBC transfer to Astria from another chain occurs.
If the recipient of the packet is a bridge account and the asset transferred is
valid, the funds are locked in the bridge account and a `Deposit` is created.
- `Deposit` events are defined in the `sequencerblockapis`: [`block.proto`](https://github.com/astriaorg/astria/blob/d03059977c3a40590d66591c520bfda3a9b9de1c/proto/sequencerblockapis/astria/sequencerblock/v1/block.proto#L76)
- An example for the derived rollup side transaction: [`astria-geth`'s `DepositTx`](https://github.com/astriaorg/astria-geth/blob/09c27dc320570d9e1f58ea60325158f36d6a0309/core/types/deposit_tx.go#L14)

## Rollup-To-Sequencer Withdrawals

Tokens that have been deposited into the rollup can be withdrawn out by the user
in the following way:

1. There is a withdrawal smart contract deployed on the rollup. Calling its
payable withdrawal function burns the transferred tokens and emits a
`Withdrawal` event.

2. The bridge withdrawer watches for withdrawal events. For each event emitted
by the smart contract, the withdrawer creates a sequencer-side action which
transfers the funds from the bridge account to the destination specified in
the withdrawal event. These actions are batched by rollup block into one sequencer
transaction, and the transaction is submitted to the sequencer.

[`BridgeUnlock` actions](https://github.com/astriaorg/astria/blob/d03059977c3a40590d66591c520bfda3a9b9de1c/proto/protocolapis/astria/protocol/transaction/v1/action.proto#L186)
have the following structure:

```proto
// `BridgeUnlock` represents a transaction that transfers
// funds from a bridge account to a sequencer account.
//
// It's the same as a `Transfer` but without the `asset` field
// and with the `memo` field.
message BridgeUnlock {
  // the to withdraw funds to
  astria.primitive.v1.Address to = 1;
  // the amount to transfer
  astria.primitive.v1.Uint128 amount = 2;
  // the asset used to pay the transaction fee
  string fee_asset = 3;
  // The memo field can be used to provide unique identifying additional
  // information about the bridge unlock transaction.
  string memo = 4;
  // the address of the bridge account to transfer from
  astria.primitive.v1.Address bridge_address = 5;
  // The block number on the rollup that triggered the transaction underlying
  // this bridge unlock memo.
  uint64 rollup_block_number = 6;
  // An identifier of the original rollup event, such as a transaction hash which
  // triggered a bridge unlock and is underlying event that led to this bridge
  // unlock. This can be utilized for tracing from the bridge back to
  // distinct rollup events.
  //
  // This field is of type `string` so that it can be formatted in the preferred
  // format of the rollup when targeting plain text encoding.
  string rollup_withdrawal_event_id = 7;
}
```

- [`Ics20Withdrawal`](https://github.com/astriaorg/astria/blob/d03059977c3a40590d66591c520bfda3a9b9de1c/proto/protocolapis/astria/protocol/transaction/v1/action.proto#L78)
represents a withdrawal from Astria to another IBC chain.

- If the `bridge_address` field is set, the funds are transferred out of a
bridge account. Alternatively, if `bridge_address` is unset but the signer
of the action is a bridge address, and the withdrawer address is the same
as the bridge address, the funds are transferred out of the bridge account.

- Examples for a withdrawal smart contracts: [`astria-bridge-contracts`](https://github.com/astriaorg/astria-bridge-contracts/tree/main)
- The events emitted by the smart contracts are defined in [`IAstriaWithdrawer`](https://github.com/astriaorg/astria-bridge-contracts/blob/038002e156c667419434204f9e5be43460da7995/src/IAstriaWithdrawer.sol#L22)

## One-Step IBC Bridging

### Deposit Into A Rollup

The user initiates an ICS20 withdrawal from some IBC chain to the sequencer.
The following metadata is provided:

- The destination chain address is the sequencer-side bridge address correspodning
to the destination rollup.
- The correct asset denomination.
- The rollup-side destination address is provided in the memo field.

When the ICS20 transfer is executed on the sequencer, the funds are locked into the
bridge address, and a `Deposit` event is created for that rollup's ID with the
user's rollup address.

### Withdrawal From A Rollup

The User initiates a withdrawal from the rollup to an IBC destination using the
`withdrawToIbcChain` function of an `IAstriaWithdrawer`-compatible smart contract.
This contract calls burns the funds, emitting an `Ics20Withdrawal` event for the
withdrawer to process (as described above).

In addition to the metadata in a sequencer-native withdrawal, the `Ics20Withdrawal`
event also provides the needed information for creating an Ics20 withdrawal on
the sequencer, such as the recipient destination chain address.

In the case that the transfer to the destination IBC chain fails, the sequencer
is notified of this via IBC, and emits a `Deposit` refunding the funds back to
the origin rollup address via the mint derivation process described above.

## Implementation

Within the sequencer application, the bridging logic is located in the
[`bridge`](https://github.com/astriaorg/astria/tree/main/crates/astria-sequencer/src/bridge)
module, and the IBC-bridging logic is in the
[`ibc`](https://github.com/astriaorg/astria/tree/main/crates/astria-sequencer/src/ibc)
module.

The rollup-side smart contracts can be found in the
[`astria-bridge-contracts`](https://github.com/astriaorg/astria-bridge-contracts/tree/4580ffc0747f463e304214bb29848e21e4e93e32)
repository.

The bridge-withdrawer service is responsible for watching for `Withdrawal` events
emitted by the rollup and submitting the corresponding sequencer transactions. The
bridge-withdrawer service is implemented in the [`astria-bridge-withdrawer`](https://github.com/astriaorg/astria/tree/main/crates/astria-bridge-withdrawer)
crate.
