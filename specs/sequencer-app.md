# Sequencer Application

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

## Background and Transaction Types

The sequencer chain's primary purpose is to sequence (order) data. This data is
not executed on the sequencer chain, as it's destined for other chains (i.e.
rollups).

Additionally, the sequencer chain has a native token used to pay fees for
sequencing. The sequencer is account-based, so every account has an associated
balance.

### Accounts and Keys

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
previously submitted by the transaction signer; thus it starts at 0, and must strictly
increase by 1 for each following transaction.

The structure of a (signed) transaction is as follows:

```rust
pub struct Transaction {
    /// The transaction signature.
    signature: ed25519_consensus::Signature,
    /// The verification key of the signer.
    verification_key: ed25519_consensus::VerificationKey,
    /// The transaction body which is signed.
    body: TransactionBody,
    /// A bytes representation of the transaction body that was signed. Re-encoding
    /// the body with protobuf may not be deterministic.
    body_bytes: Bytes,
}
```

The address corresponding to the signer is derived from the
`ed25519_consensus::VerificationKey` (i.e. the public key).

## [Actions](https://buf.build/astria/protocol-apis/docs/main:astria.protocol.transaction.v1)

The following is an exhaustive list of all user-accessible actions available
to be submitted.

### Core Protocol Actions

* `Transfer`: represents a value transfer action between two accounts. It consists
of the following fields:

  | **Field** | **Type** | **Description** |
  | --------- | -------- | ----------- |
  | to        | `Address`| The recipient of the transfer. The "from" address is assumed|
  ||| to be the signer of the transaction. |
  | amount    | `uint128`| The amount to transfer. |
  | asset     | `string` | The asset to transfer. |
  | fee_asset | `string` | The asset used to pay for the action's fees. |

* `RollupDataSubmission`: a transaction ordered by the sequencer, whose destination
is another chain. It consists of the following:

  | **Field** | **Type** | **Description** |
  | --------- | -------- | ----------- |
  | rollup_id | `RollupId`| ID of the destination chain. |
  | data      | `bytes`  | The opaque transaction data. |
  | fee_asset | `string` | The asset used to pay for the action's fees. |

### Bridge Actions

These actions deal with transfering funds to/from a bridge account to be used on
a rollup.

* `InitBridgeAccount`: initializes a bridge account for the given rollup on the
sequencer chain. The `withdrawer_address` is the only actor authorized to transfer
out of the account. It is set by default to the signer of the transaction, but
can also be set to a different address within the action itself, or changed later
via a `BridgeSudoChange` action. `InitBridgeAccount` consists of the following
fields:

  | **Field** | **Type** | **Description** |
  | --------- | -------- | ----------- |
  | rollup_id | `RollupId` | The rollup ID with which to register the bridge account.|
  | asset     | `string`   | The asset ID that will be accepted by the account.|
  | fee_asset | `string`   | The asset with which to pay fees for this action. |
  | sudo_address | `Address` | The address which has authority over the bridge|
  ||| account. If empty, assigned to the signer. |
  | withdrawer_address | `Address` | The address which is allowed to withdraw funds|
  ||| from the bridge account. If empty, assigned to the signer.|

* `BridgeLock`: transfers funds from a sequencer account to a bridge account.
It is effectively similar to `Transfer`. Upon execution of a bridge lock action,
a `Deposit` event will be included in the block data for the rollup this bridge
account is registered to, containing the information of the transfer.

  | **Field** | **Type** | **Description** |
  | --------- | -------- | ----------- |
  | to        | `Address`| The bridge account to transfer to. The "from" address|
  ||| is assumed to be the signer of the transaction. |
  | amount    | `uint128`| The amount to transfer. |
  | asset     | `string` | The asset to transfer. |
  | fee_asset | `string` | The asset used to pay for the action's fees. |
  | destination_chain_address | `string` | The address on the destination chain|
  ||| which will receive the bridged funds. |

* `BridgeUnlock`: transfers funds from a bridge account to a sequencer account.
The signer of this transaciton *must* be the authorized withdrawer for this `bridge_address`.
Effectively similar to `Transfer`, it contains the following fields:

  | **Field** | **Type** | **Description** |
  | --------- | -------- | ----------- |
  | to        | `Address`| The account to transfer to. |
  | amount    | `uint128`| The amount to transfer. |
  | fee_asset | `string` | The asset used to pay for the action's fees. |
  | memo      | `string` | Can be used to provide unique, identifying information|
  ||| about the bridge unlock action. |
  | bridge_address | `Address` | The address of the bridge account to transfer|
  ||| from. |
  | rollup_block_number | `uint64` | The block number on the rollup which triggered|
  ||| the transaction underlying the bridge unlock. |
  | rollup_withdrawal_event_id | `string` | An identifier of the rollup withdrawal|
  ||| transaction which can be used to trace distinct rollup events from the bridge.|

* `BridgeSudoChange`: changes the sudo and/or withdrawer address for the given
bridge account. The signer must be the current bridge sudo account.

  | **Field** | **Type** | **Description** |
  | --------- | -------- | ----------- |
  | bridge_address | `Address` | The address of the bridge account to which these|
  ||| changes should be made. |
  | new_sudo_address | `Address` | The new sudo address for the bridge account.|
  ||| If unset, will stay the same. |
  | new_withdrawer_address | `Address` | The new withdrawer address for the bridge|
  ||| account. If unset, will stay the same. |
  | fee_asset | `string` | The asset with which to pay fees for this action.|

### IBC User Actions

Actions which deal with the IBC protocol.

* `IbcRelay`: transmits data packets between the sequencer chain and another
chain using the [IBC](https://www.ibcprotocol.dev/) protocol. This is a permissioned
action, and only authorized accounts can relay. As a result, this action is also
free to submit. It has one field:

  | **Field** | **Type** | **Description** |
  | --------- | -------- | ----------- |
  | raw_action | `google.protobuf.Any` | The raw IBC action. Can be any of |
  ||| [these types](https://github.com/penumbra-zone/penumbra/blob/c23270bd3610f0b6b139d4c2e13c8a4a5bb16f07/crates/core/component/ibc/src/ibc_action.rs#L41).|

* `Ics20Withdrawal`: transfers tokens from a sequencer account to a different
chain via [ICS-20 protocol](https://github.com/cosmos/ibc/blob/main/spec/app/ics-020-fungible-token-transfer/README.md).
It consists of the following:

  | **Field** | **Type** | **Description** |
  | --------- | -------- | ----------- |
  | amount    | `uint218`| The amount to transfer. |
  | denom     | `string` | The denomination to transfer. |
  | destination_chain_address | `string` | The address on the destination chain|
  ||| to send the transfer to. Not validated by Astria. |
  | return_address | `Address` | The sequencer chain address to return funds to|
  ||| in case the withdrawal fails. |
  | timeout_height | `IbcHeight` | The counterparty height at which this action expires.|
  | timeout_time | `uint64` | The unix timestamp (ns) at which this transfer expires.|
  | source_channel | `string` | The source channel used for the withdrawal. |
  | fee_asset | `string` | The asset used to pay fees with. |
  | memo | `string` | A memo to include with the transfer. |

## ABCI Block Lifecycle

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
   (if the node is not a proposer)
2. [FinalizeBlock](https://docs.cometbft.com/v0.38/spec/abci/abci++_methods#finalizeblock)
3. [Commit](https://docs.cometbft.com/v0.38/spec/abci/abci++_methods#commit)

### PrepareProposal

If the given node is a validator as well as the proposer for this round,
`PrepareProposal` is called. `PrepareProposal` allows the list of transactions
suggested by CometBFT to be modified. Currently, the only modification we make is
adding commitments to the rollup data for each block. See the [related spec](./sequencer-inclusion-proofs.md)
for more details.

### ProcessProposal

For all nodes,`ProcessProposal` is called. However, only validator nodes need to
validate and vote on the proposal. This checks if the commitment to the rollup
data is correct. If it is not correct, the validator rejects the block.

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

## Transaction Lifecycle

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
* During the proposal phase, the proposer executes the transactions it wishes to
include, and only includes ones which succeed. Any transactions which fail execution
are removed from the mempool. Other nodes only accept blocks where all transactions
succeed.

## ABCI Queries

The sequencer supports queries directly into its state via ABCI. The current queries
support by the sequencer are the following:

* **Account Balance:** returns a list of assets and their corresponding balances
for the given account, at the current block height. Usage:

```sh
abci-cli query --path=accounts/balance/<ADDRESS>
```

* **Account Nonce:** returns the account's current nonce. Usage:

```sh
abci-cli query --path=accounts/nonce/<ADDRESS>
```

* **Denom Request:** returns the full asset denomination given the asset ID. Usage:

```sh
abci-cli query --path=asset/denom/<DENOM_ID>
```

* **Allowed Fee Assets:** returns a list of all currently allowed fee assets. Usage:

```sh
abci-cli query --path=asset/allowed_fee_assets
```

* **Last Bridge TX Hash:** returns the hash of the last transaction that interacted
with the given bridge account. Usage:

```sh
abci-cli query --path=bridge/account_last_tx_hash/<BRIDGE_ADDRESS>
```

* **Transaction Fee:** returns the estimated fees a given transaction will incur.
Usage:

```sh
abci-cli query --path=transaction/fee --data=<TRANSACTION_BODY_BYTES>
```

* **Bridge Account Info:** returns the `rollup_id`, `asset`, `sudo_address`, and
`withdrawer_address` for the given bridge account. Usage:

```sh
abci-cli query --path=bridge/account_info/<BRIDGE_ADDRESS>
```

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
