# Sequencer application

A sequencer blockchain node consists of two components: [cometbft](https://github.com/cometbft/cometbft) (formerly known as tendermint) and the [sequencer application](https://github.com/astriaorg/astria/tree/main/crates/astria-sequencer). This splits the node logic into two separate components that communicate over [ABCI](https://docs.cometbft.com/v0.37/spec/abci/). Cometbft contains the logic for consensus, including the required p2p networking, while the sequencer application contains the state-transition (application) logic of the blockchain. Cometbft drives the formation of new blocks and finality of blocks, calling into the application when necessary to execute the application logic.

This document aims to specify the application logic of the sequencer chain.

## Background

The sequencer chain's primary purpose is to sequence (order) data. This data is not executed on the sequencer chain, as it's destined for other chains (ie. rollups). 

Additionally, the sequencer chain has a native token used to pay fees for sequencing. The sequencer is account-based, so every account has an associated balance. 

### Accounts and keys

Currently, the sequencer supports ed25519 keys for accounts and signing. 

An address is specified by the first 20 bytes of the sha256 hash of the encoded public key. Similarly to Ethereum, every account implicitly exists on the chain, thus funds can be owned by, and transferred to, any 20-byte address. This is unlike Cosmos-based chains where accounts need to be initialized explicitly.

### Transactions

Transactions are submitted by users to modify the chain state. Transactions can consist of multiple `Action`s, where an `Action` acts on one specific component of the state (for example, account balance). This is analogous to the cosmos-sdk idea of "messages", where every transaction can contain multiple messages. The benefit of this is that it guarantees that multiple actions can be executed atomically.

The structure of an unsigned transaction is as follows:
```rust
pub struct Unsigned {
    pub(crate) nonce: Nonce,
    pub(crate) actions: Vec<Action>,
}
```

`Nonce` is an incrementing value that represents how many transactions have been previously submitted by this account; thus it starts at 0, and must strictly increase by 1 for each transaction.

The structure of a signed transaction is as follows:
```rust
pub struct Signed {
    /// transaction signature
    pub(crate) signature: ed25519_consensus::Signature,
    /// the public key of the signer
    pub(crate) public_key: ed25519_consensus::VerificationKey,
    /// the transaction that was signed
    pub(crate) transaction: Unsigned,
}
```

The address corresponding to the signer is derived from the `ed25519_consensus::VerificationKey` (ie. the public key).

### Actions

There are currently 2 types of actions implemented.

The first, part of the `accounts` component, is a value-transfer action:
```rust
pub struct Transfer {
    to: Address,
    amount: Balance,
}
```

This action transfers the specified amount funds from the sender to the recipient.

The second, part of the `sequence` component, accepts generic data with a specified chain ID:
```rust
pub struct Action {
    pub(crate) chain_id: Vec<u8>,
    pub(crate) data: Vec<u8>,
}
```

Note that this action does not have any effect on the state of the sequencer; it is simply ordered by the sequencer and placed into a block.

## ABCI block lifecycle

Cometbft makes progress through successive consensus rounds. During each round, a new block is proposed, voted on by validators, and if >2/3 of validator staking power votes on a block, it will be committed (finalized). During each round, cometbft calls into the sequencer app to execute the state transition logic via ABCI (application blockchain interface).

As of cometbft v0.37, The ABCI methods called during a one-round period are as follows:
1. [PrepareProposal](https://docs.cometbft.com/v0.37/spec/abci/abci++_methods#prepareproposal) (if the node is a proposer), [ProcessProposal](https://docs.cometbft.com/v0.37/spec/abci/abci++_methods#processproposal) (if the node is a validator but not a proposer)
2. [BeginBlock](https://docs.cometbft.com/v0.37/spec/abci/abci++_methods#beginblock)
3. [DeliverTx](https://docs.cometbft.com/v0.37/spec/abci/abci++_methods#delivertx) (called once for every transaction in the block)
4. [EndBlock](https://docs.cometbft.com/v0.37/spec/abci/abci++_methods#endblock)
5. [Commit](https://docs.cometbft.com/v0.37/spec/abci/abci++_methods#commit)
