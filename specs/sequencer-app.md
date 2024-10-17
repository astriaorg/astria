# Sequencer application

A sequencer blockchain node consists of two components:
[CometBFT](https://github.com/cometbft/cometbft) (formerly known as tendermint)
and the [sequencer
application](https://github.com/astriaorg/astria/tree/main/crates/astria-sequencer).
This splits the node logic into two separate components that communicate over
[ABCI](https://docs.cometbft.com/v0.37/spec/abci/). CometBFT contains the logic
for consensus, including the required p2p networking, while the sequencer
application contains the state transition (application) logic of the blockchain.
CometBFT drives the formation of new blocks and finalization of blocks, calling
into the application when necessary to execute the state transition logic.

This document aims to specify the application logic of the sequencer chain.

## Background and transaction types

The sequencer chain's primary purpose is to sequence (order) data. This data is
not executed on the sequencer chain, as it's destined for other chains (i.e.
rollups).

Additionally, the sequencer chain has a native token used to pay fees for
sequencing. The sequencer is account-based, so every account has an associated
balance.

### Accounts and keys

Currently, the sequencer supports [Ed25519](https://ed25519.cr.yp.to/) keys for
accounts and signing.

An address is specified by the first 20 bytes of the sha256 hash of the encoded
public key. Similarly to Ethereum, every account implicitly exists on the chain,
thus funds can be owned by, and transferred to, any 20-byte address. This is
unlike Cosmos-based chains where accounts need to be initialized explicitly.

### Transactions

Transactions are submitted by users to modify the chain state. Transactions can
consist of multiple `Action`s, where an `Action` acts on one specific component
of the state (for example, account balance). This is analogous to the cosmos-sdk
idea of "messages", where every transaction can contain multiple messages. The
benefit of this is that it guarantees that multiple actions can be executed
atomically.

The structure of a transaction body is as follows:

```rust
pub struct TransactionBody {
    params: TransactionParams,
    actions: Actions,
}
```

Transaction parameters and actions are defined as the following:

```rust
pub struct TransactionParams {
    nonce: u32,
    chain_id: String,
}

pub(crate) struct Actions {
    group: Group, // actions are grouped by "bundleability" and authority level
    inner: Vec<Action>,
}
```

`Nonce` is an incrementing value that represents how many transactions have been
previously submitted by the currently submitting account; thus it starts at 0,
and must strictly increase by 1 for each following transaction.

The structure of a (signed) transaction is as follows:

```rust
pub struct Transaction {
    /// transaction signature
    signature: ed25519_consensus::Signature,
    /// the verification key of the signer
    verification_key: ed25519_consensus::VerificationKey,
    /// the transaction body which is signed
    body: TransactionBody,
    /// a bytes representation of the transaction body
    body_bytes: Bytes,
}
```

The address corresponding to the signer is derived from the
`ed25519_consensus::VerificationKey` (i.e. the public key).

### Actions

TBD

## ABCI block lifecycle

CometBFT makes progress through successive consensus rounds. During each round,
a new block is proposed and voted on by validators. If >2/3 of validator
staking power votes on a block, it will be committed (finalized). During each
round, CometBFT calls into the sequencer app to execute the state transition
logic via ABCI (application blockchain interface).

As of CometBFT v0.38, The ABCI methods called during a one-round period are as
follows:

1. Prepare/Process Proposal
    * [PrepareProposal](https://docs.cometbft.com/v0.38/spec/abci/abci++_methods#prepareproposal)
   (if the node is a proposer),
    * [ProcessProposal](https://docs.cometbft.com/v0.38/spec/abci/abci++_methods#processproposal)
   (if the node is a validator but not a proposer)
2. [FinalizeBlock](https://docs.cometbft.com/v0.38/spec/abci/abci++_methods#finalizeblock)
3. [Commit](https://docs.cometbft.com/v0.38/spec/abci/abci++_methods#commit)

### PrepareProposal

If the given node is a validator as well as the proposer for this round,
`PrepareProposal` is called. `PrepareProposal` allows the list of transactions
suggested by CometBFT to be modified. Currently, the only modification we make is
adding commitments to the rollup data for each block. See the [related spec](./sequencer-inclusion-proofs.md)
for more details.

### ProcessProposal

If the given node is a validator, but not the proposer for this round,
`ProcessProposal` is called. This checks if the commitment to the rollup data is
correct. If it is not correct, the validator rejects the block.

### FinalizeBlock

This is executed by all sequencer nodes. It is equivalent to the following series
of steps in CometBFT v0.37:

1. [BeginBlock](https://docs.cometbft.com/v0.37/spec/abci/abci++_methods#beginblock)
    * Updates the current block height and timestamp in the app state, then removes
    byzantine validators from the app state's current validator set.
2. [DeliverTx](https://docs.cometbft.com/v0.37/spec/abci/abci++_methods#delivertx)
    * Performs stateless and stateful checks, then executes each transaction's
    state changes for every transaction in the block. After each transaction's
    execution, its fees (if any) are deducted from the signer's balance. Fees
    are paid to receiver in `EndBlock`. See [fees](#fees) for more details.
3. [EndBlock](https://docs.cometbft.com/v0.37/spec/abci/abci++_methods#endblock)
    * Stores any updates to the current validator set, then pays fees to the receiver.

### Commit

This is executed by all sequencer nodes to write the state changes to disk.

## Transaction lifecycle

The lifecycle of a sequencer transaction is as follows:

* A user/application constructs a `TransactionBody`, which they sign,
  converting it into a signed `Transaction`.
* This transaction is serialized and submitted to a sequencer node via
  CometBFT's RPC endpoints `broadcast_tx_..`
* CometBFT calls into the sequencer application to validate the transaction
  using the ABCI method `CheckTx`. `CheckTx` returns either success or an error.
  If it is successful, the transaction is added to the CometBFT mempool and
  broadcasted throughout the network; otherwise, the transaction is discarded.
* The transaction will live in the mempool until it is included in a block
  proposal by a proposer.
* Once inside a proposed block, the transaction will be executed by `FinalizeBlock`
  during that block's lifecycle. At this point, the transaction will either
  execute successfully or fail. If the transaction fails, it will still be included
  in the block, but with a failure result, and will not have made any state changes.

## ABCI queries

TBD

## Fees

All `Action`s implement fee payment via the same linear formula:

```text
base + (variable_component * multiplier)
```

This formula makes it easy to estimate and replicate fees. For actions with no
associated fees, the `base` and `multiplier` are simply stored in the app state
as "0". If there are no fees stored in the state (as opposed to *explicitly* "0"),
the action is de facto "deactivated", and will fail execution. Actions can be "activated"
by calling `FeeChange` for the given action and assigning it fees. Currently, there
is no way to deactivate an action after it has been assigned fees in the app state.
