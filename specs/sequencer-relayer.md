# Sequencer-Relayer

## Overview

The sequencer-relayer, a.k.a relayer, is an application whose primary
responsibility is to publish sequenced rollup data on a remote data availability
layer (DA layer).

The immediate source of the sequenced data is the sequencer blockchain network,
where rollups' data is sequenced and included in sequencer blocks, while the DA
layer is the Celestia blockchain.

## Application Logic

The application logic of the relayer is comprised of three main tasks: getting
the data from a sequencer node, transforming the data in preparation for
publishing, and putting the transformed data onto the Celestia network.

### Getting the Data from a Sequencer Node

The relayer runs an endlessly repeating task, the "reader task", which does the
following:

1. polls the sequencer node for the latest (highest) sequencer block height at
  a fixed frequency (once per second currently)
1. upon learning of a new height, fetches the block at that height from the
  sequencer node
1. upon receiving the sequencer block, forwards it to the transformation task

Note that currently the relayer only communicates with a single sequencer node,
and that this node is comprised of two separate processes, each providing their
own http servers. Consequently, the relayer queries two different endpoints
using two different protocols; CometBFT's ABCI `Info` for the block height via
JSONRPC, and the sequencer app's `GetSequencerBlock` via gRPC. This is likely to
improve in the near future, as work is under way to extend the sequencer app's
API to the extent that no direct communication with the CometBFT node will be
required.

Should the sequencer fail to fetch the latest sequencer block height, no action
is taken other than logging and recording the fact in metrics - polling
continues at the same frequency under the assumption that the error is a
transient one.

Should the sequencer fail to fetch a block at a given height, it will keep
retrying endlessly with an exponential backoff between attempts ranging from 100
milliseconds to a maximum of one second.  As above, each failed attempt is
logged and recorded in metrics. During retries, no other sequencer blocks are
fetched, even if the relayer learns of newer ones.

If a received block is invalid (e.g. fails validation checks or is not the
requested height) it is a terminal error, and the reader task exits.  This
should not occur in practice.

### Transforming the Data

Transforming the data is done as part of an endlessly repeating task, the
"submitter task", where the sequencer blocks are received from the reader task
(generally once per second), accumulated into a batch and sent to the Celestia
node as soon confirmation of the previous submission has been received.  In
practice, the submission rate is generally once per 12 seconds, i.e. the
Celestia block time.

The data from the sequencer block undergoes multiple transformations prior to
sending to the DA layer, and generally data from multiple consecutive sequencer
blocks is batched together as part of the transformation process. Ultimately,
the final form is a collection of Celestia `Blob`s, one per individual rollup,
and a single one containing metadata about the batched data. Each rollup has its
own Celestia namespace, as does the sequencer metadata.

On receiving each new sequencer block, the relayer transforms it (see below for
details) and tries to add it to the next batch for submission to the Celestia
node. If adding it would cause the batch size to exceed an upper limit
(currently 1MB), then it is temporarily cached instead and receiving further
sequencer blocks from the reader task is paused.

Note that the reader task itself isn't immediately stopped in this case. A
bounded channel is used to send sequencer blocks from the reader task to the
submitter task, currently with a limit of 128 blocks. Normally this channel will
have 0 or 1 blocks, but if the submitter task pauses consumption of the blocks,
the reader task will continue to send blocks into the channel until it's full,
at which point the reader task also pauses until the channel has capacity to
send again.

The transformation steps on receiving a new sequencer block are as follows:

1. The rollup transactions are extracted into a separate collection of
  `SubmittedRollupData`s (one per rollup) where each includes the sequencer
  block hash. The rest of the block's contents and the list of included rollup
  IDs is moved to a single `SubmittedMetadata`.
1. These are then appended to lists for batching, one `SubmittedRollupDataList`
  per rollup (a collection of `SubmittedRollupData`s), and a
  `SubmittedMetadataList` (a collection of `SubmittedMetadata`s). At this stage,
  filtering of rollups is applied if enabled: only rollups specified in the
  `ASTRIA_SEQUENCER_RELAYER_ONLY_INCLUDE_ROLLUPS` env var are included, or else
  no filtering is done if the env var is empty.
1. All the lists are converted to a single payload of Celestia blobs, one list
  per blob. Each list is encoded to bytes using Protobuf serialization then
  compressed using Brotli.

Any error encountered during data transformation is cause for the entire process
to exit, with the exception of exceeding the batch size limit, where the last
block is instead cached until the current batch has been successfully put to
Celestia.

### Putting the Data onto Celestia

Submitting the data in the form of Celestia blobs is part of the submitter task,
and involves several steps where it communicates with a single Celestia app via
gRPC as follows:

1. Four RPCs are sent in parallel to retrieve the relayer's Celestia account
  nonce (a.k.a "sequence") and values needed for a gas limit estimation (a.k.a.
  "cost params").
1. A `MsgPayForBlobs` containing information about all the Celestia blobs from
  the transformation step is constructed.
1. The gas limit is estimated using the retrieved cost params and blob sizes.
1. The fee is calculated based on the cost params and gas limit. At this stage,
  if a previous attempt to store this data failed, the error returned by the
  Celestia app in the previous attempt is examined for a log message indicating
  what the required fee should be. If found, that value is used rather than the
  one calculated from the cost params.
1. A `BlobTx` is constructed comprised of a signed `Tx` and the Celestia blobs.
1. This is sent to the Celestia app via a `BroadcastTx` gRPC using `Sync`
  broadcast mode. On success, the response provides the transaction hash.
1. Confirmation is done by polling: repeatedly sending a `GetTx` gRPC to the
  Celestia app with an exponential backoff between attempts ranging from one
  second to a maximum of 12 seconds. Polling completes once the server responds
  with a success and indicates the Celestia block height in which the
  transaction was executed.

Should any of steps 1 to 6 fail, the relayer will keep retrying endlessly with
an exponential backoff between attempts ranging from 100 milliseconds to a
maximum of 12 seconds. Each failed attempt is logged and recorded in metrics.
During retries, no other sequencer blocks are fetched, even if the relayer
learns of newer ones.

During retries or while polling for confirmation of success, the next batch for
submission will continue to have new sequencer blocks added until it is full, at
which point backpressure will cause the reader task to pause as detailed above.

## Further Details

### Submission State File

During attempts to put data onto Celestia, the relayer writes some pertinent
information to disk in the form of a JSON file; the submission-state file. This
allows the relayer to restart and continue submitting from where it left off.

This file needs to exist and be writable whenever the relayer starts, even on
first run.

The submission state is one of three variants; `fresh`, `started` or `prepared`.

#### `fresh` State

The contents of the submission-state file when in `fresh` state are:

```json
{"state": "fresh"}
```

This state is never written by the relayer: it needs to be provided externally
before the relayer is started. It indicates that the relayer should start
relaying from sequencer block 1.

#### `started` State

The contents of the submission-state file when in `started` state are:

```json
{
  "state": "started",
  "last_submission": {
    "celestia_height": <number>,
    "sequencer_height": <number>
  }
}
```

This state is written by the relayer at the start of each new submission.
`last_submission` records the last successful submission: the Celestia block
height at which the submission was stored, and the highest sequencer block
included in the submission.

With the file in this state, on startup the relayer will begin submitting
sequencer blocks starting from `[last_submission.sequencer_height] + 1`.

#### `prepared` State

The contents of the submission-state file when in `prepared` state are:

```json
{
  "state": "prepared",
  "sequencer_height": <number>,
  "last_submission": {
    "celestia_height": <number>,
    "sequencer_height": <number>
  },
  "blob_tx_hash": "<64-character hex string>",
  "at": "<timestamp in RFC-3339 format>"
}
```

This state is written by the relayer when a submission has been prepared and is
about to be sent to the Celestia app for execution. `sequencer_height` indicates
the highest sequencer block included in the attempt, while `last_submission` is
as per that in `started` state. `blob_tx_hash` is the hex-encoded SHA-256 digest
of the `BlobTx` containing the data to be submitted, and `at` is the time at
which the `BlobTx` was created.

With the file in this state, on startup the relayer has to first establish
whether that submission succeeded or not. It queries the Celestia app for the
given blob hash, and if not confirmed as stored repeats the query at a rate of
once per second until it is confirmed or until a timeout is hit.  The timeout is
one minute after the `at` timestamp or 15 seconds, whichever is the greater.

If the submission is confirmed, the relayer will then begin submitting sequencer
blocks starting from `[sequencer_height] + 1`, otherwise it begins from
`[last_submission.sequencer_height] + 1`.

### HTTP Servers

The relayer runs a small http server providing three endpoints; `/healthz`,
`/readyz` (Kubernetes API health endpoints) and `/status` which reports a few
facets of the current state of the relayer.

There is also an optional metrics http server which supports scraping Prometheus
metrics.
