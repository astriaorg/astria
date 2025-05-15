# Testing Hermes With Multiple Relayers

## Setup

```bash
just deploy cluster && just deploy ingress-controller
just wait-for ingress-controller
just deploy celestia-local
just deploy sequencer single
just deploy sequencer full-node
just deploy hermes local
just deploy hermes local1
just wait-for hermes local1
```

## Testing

### Sending Transactions

```bash
just run ibc-test do-ibc-transfer astria-dev-cluster astria1mg49ywffq0tt7rkunfmd7paxcvrtvqn5yr53rq
```

### Observing Funds

```bash
# optional
cargo build --release -p astria-cli
# observe funds on full-node
target/release/astria-cli sequencer account balance --sequencer-url http://rpc.full-node.sequencer.127.0.0.1.nip.io astria1mg49ywffq0tt7rkunfmd7paxcvrtvqn5yr53rq
# observe funds on validator
target/release/astria-cli sequencer account balance --sequencer-url http://rpc.sequencer.127.0.0.1.nip.io astria1mg49ywffq0tt7rkunfmd7paxcvrtvqn5yr53rq
```

### Observe Mempool via Buf Studio

- Start Buf studio agent on `127.0.0.1:8080`:
```bash
  buf beta studio-agent --debug --timeout 0
```

- Use [Buf Studio](https://buf.build/studio/astria/mempool-apis/astria.mempool.v1.TransactionService/GetTransactionStatus?target=http%3A%2F%2Fgrpc.full-node.sequencer.127.0.0.1.nip.io&selectedProtocol=grpc&agentUrl=http%3A%2F%2F127.0.0.1%3A8080)
  to query transaction status from `full-node`
- Use [Buf Studio](https://buf.build/studio/astria/mempool-apis/astria.mempool.v1.TransactionService/GetTransactionStatus?target=http%3A%2F%2Fgrpc.sequencer.127.0.0.1.nip.io&selectedProtocol=grpc&agentUrl=http%3A%2F%2F127.0.0.1%3A8080)
 to query transaction status from validator
