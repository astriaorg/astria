# Transaction Service Specification

## Overview

The Mempool API houses the [Transaction Service](https://buf.build/astria/mempool-apis/docs/main:astria.mempool.v1#astria.mempool.v1.TransactionService),
a gRPC service meant to provide direct access to transactions within the app-side
mempool. It currently provides methods to submit transactions directly to the mempool
as well as query their status and associated fees.

## RPCs

### GetTransactionStatus

`GetTransactionStatus` takes a request with the desired transaction's hash bytes
and returns a `TransactionStatus` (see [Resources](#resources)) for details.

#### GetTransactionStatusRequest

```proto
message GetTransactionStatusRequest {
  bytes transaction_hash = 1;
}
```

### SubmitTransaction

`SubmitTransaction` submits a transaction directly to the app-side mempool. Note
that this results in two main differences to submitting via CometBFT. Firstly,
transactions submitted directly to the mempool are currently not gossiped to other
nodes. As a result, the transaction can only be proposed by the node which receives
the transaction, or cannot be proposed at all if the transaction is submitted to
a non-validator. Secondly, as a result of this, transactions submitted via this
RPC are not persisted, and if the Sequencer app restarts for any reason, the transaction
will be lost.

#### SubmitTransactionRequest

See here for more details on `astria.protocol.transaction.v1.Transaction`:
[BUF docs](https://buf.build/astria/protocol-apis/docs/main:astria.protocol.transaction.v1#astria.protocol.transaction.v1.Transaction).

```proto
message SubmitTransactionRequest {
    astria.protocol.transaction.v1.Transaction transaction = 1;
}
```

#### SubmitTransactionResponse

```proto
message SubmitTransactionResponse {
    astria.mempool.v1.TransactionStatus status = 1;
    bool duplicate = 2;
}
```

### GetTransactionFees

`GetTransactionFees` obtains the estimated fees to be incurred by the given transaction
body at the time of calling. Note that a successful response to this method neither
gaurantees that the estimated fees are correct nor that the transaction will succeed
upon construction or execution.

#### GetTransactionFeesRequest

See here for more details on `astria.protocol.transaction.v1.TransactionBody`
[BUF docs](https://buf.build/astria/protocol-apis/docs/main:astria.protocol.transaction.v1#astria.protocol.transaction.v1.TransactionBody).

```proto
message GetTransactionFeesRequest {
    astria.protocol.transaction.v1.TransactionBody transaction_body = 1;
}
```

#### GetTransactionFeesResponse

See here for more details on `astria.protocol.fees.v1.TransactionFee`:
[BUF docs](https://buf.build/astria/protocol-apis/docs/main:astria.protocol.fees.v1#astria.protocol.fees.v1.TransactionFee).

```proto
message GetTransactionFeesResponse {
  uint64 block_height = 1;
  repeated astria.protocol.fees.v1.TransactionFee fees = 2;
}
```

## Resources

The mempool API is a resource-based API as outlined by aip.dev, though it currently
contains one resource:

### TransactionStatus

Represents the status of a transaction in the app-side mempool.

```proto
message TransactionStatus {
  bytes transaction_hash = 1;

  oneof status {
    Pending pending = 2;
    Parked parked = 3;
    Removed removed = 4;
    Executed executed = 5;
  }

  message Pending {}

  message Parked {}

  message Removed {
    string reason = 1;
  }

  message Executed {
    uint64 height = 1;
    tendermint.abci.ExecTxResult result = 2;
  }
}
```
