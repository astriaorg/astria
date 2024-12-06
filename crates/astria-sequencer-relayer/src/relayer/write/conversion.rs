use std::{
    collections::{
        BTreeSet,
        HashMap,
        HashSet,
    },
    pin::Pin,
    task::Poll,
};

use astria_core::{
    brotli::compress_bytes,
    generated::astria::sequencerblock::v1::{
        SubmittedMetadata,
        SubmittedMetadataList,
        SubmittedRollupData,
        SubmittedRollupDataList,
    },
    primitive::v1::RollupId,
};
use celestia_types::{
    nmt::Namespace,
    Blob,
};
use futures::Future;
use pin_project_lite::pin_project;
use sequencer_client::SequencerBlock;
use tendermint::block::Height as SequencerHeight;
use tracing::{
    error,
    trace,
    warn,
};

use crate::{
    metrics::Metrics,
    IncludeRollup,
};

/// The maximum permitted payload size in bytes that relayer will send to Celestia.
///
/// Taken as half the maximum block size that Celestia currently allows (2 MB at the moment).
const MAX_PAYLOAD_SIZE_BYTES: usize = 1_000_000;

pub(super) struct Submission {
    input: Input,
    payload: Payload,
}

impl Submission {
    pub(super) fn into_blobs(self) -> Vec<Blob> {
        self.payload.blobs
    }

    pub(super) fn input_metadata(&self) -> &InputMeta {
        self.input.meta()
    }

    pub(super) fn num_blobs(&self) -> usize {
        self.payload.num_blobs()
    }

    pub(super) fn num_blocks(&self) -> usize {
        self.input.num_blocks()
    }

    pub(super) fn greatest_sequencer_height(&self) -> SequencerHeight {
        self.input.greatest_sequencer_height().expect(
            "`Submission` should not be constructed if no blocks are present in the input. This \
             is a bug",
        )
    }

    /// The ratio of uncompressed blob size to compressed size.
    #[expect(
        clippy::cast_precision_loss,
        reason = "used for metric gauges, which require f64. Precision loss is ok and of no \
                  significance"
    )]
    pub(super) fn compression_ratio(&self) -> f64 {
        self.uncompressed_size() as f64 / self.compressed_size() as f64
    }

    pub(super) fn compressed_size(&self) -> usize {
        self.payload.compressed_size()
    }

    pub(super) fn uncompressed_size(&self) -> usize {
        self.payload.uncompressed_size()
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum PayloadError {
    #[error("failed to compress protobuf encoded bytes")]
    Compress(#[from] std::io::Error),
    #[error("failed to create Celestia blob from compressed bytes")]
    Blob(#[from] celestia_types::Error),
}

#[derive(Debug, Default)]
struct Payload {
    compressed_size: usize,
    uncompressed_size: usize,
    blobs: Vec<Blob>,
}

impl Payload {
    fn new() -> Self {
        Self::default()
    }

    fn with_capacity(cap: usize) -> Self {
        Self {
            uncompressed_size: 0,
            compressed_size: 0,
            blobs: Vec::with_capacity(cap),
        }
    }

    fn is_empty(&self) -> bool {
        self.blobs.is_empty()
    }

    fn num_blobs(&self) -> usize {
        self.blobs.len()
    }

    /// Adds `value` to the payload.
    ///
    /// Encodes `value` as protobuf, compresses it, and creates a Celestia [`Blob`] under
    /// `namespace`.
    fn try_add<T: prost::Message>(
        &mut self,
        namespace: Namespace,
        value: &T,
    ) -> Result<(), PayloadError> {
        let encoded = value.encode_to_vec();
        let compressed = compress_bytes(&encoded)?;
        let blob = Blob::new(namespace, compressed)?;
        self.uncompressed_size = self
            .uncompressed_size
            .checked_add(encoded.len())
            .unwrap_or_else(|| {
                error!(
                    uncompressed_size = self.uncompressed_size,
                    encoded_len = encoded.len(),
                    "overflowed uncompressed size while adding new value; setting to `usize::MAX`"
                );
                usize::MAX
            });
        self.compressed_size = self
            .compressed_size
            .checked_add(blob.data.len())
            .unwrap_or_else(|| {
                error!(
                    compressed_size = self.compressed_size,
                    blob_data_len = blob.data.len(),
                    "overflowed compressed size while adding new value; setting to `usize::MAX`"
                );
                usize::MAX
            });
        self.blobs.push(blob);
        Ok(())
    }

    fn compressed_size(&self) -> usize {
        self.compressed_size
    }

    fn uncompressed_size(&self) -> usize {
        self.uncompressed_size
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum TryIntoPayloadError {
    #[error("failed adding protobuf `{type_url}` to Celestia blob payload")]
    AddToPayload {
        source: PayloadError,
        type_url: String,
    },
    #[error(
        "there was no sequencer namespace present in the input so a payload of Celestia blobs \
         could not be constructed"
    )]
    NoSequencerNamespacePresent,
    #[error("the payload size exceeded `u64::MAX` bytes")]
    PayloadSize,
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub(super) struct InputMeta {
    #[serde(serialize_with = "serialize_sequencer_heights")]
    sequencer_heights: BTreeSet<SequencerHeight>,
    #[serde(serialize_with = "serialize_opt_namespace")]
    sequencer_namespace: Option<Namespace>,
    #[serde(serialize_with = "serialize_included_rollups")]
    rollups_included: HashMap<RollupId, Namespace>,
    rollups_excluded: HashSet<RollupId>,
}

#[derive(Clone, Debug, Default)]
struct Input {
    metadata: Vec<SubmittedMetadata>,
    rollup_data_for_namespace: HashMap<Namespace, Vec<SubmittedRollupData>>,
    meta: InputMeta,
}

impl Input {
    fn new() -> Self {
        Self::default()
    }

    fn meta(&self) -> &InputMeta {
        &self.meta
    }

    fn num_blocks(&self) -> usize {
        self.metadata.len()
    }

    fn extend_from_sequencer_block(
        &mut self,
        block: SequencerBlock,
        rollup_filter: &IncludeRollup,
    ) {
        if !self.meta.sequencer_heights.insert(block.height()) {
            warn!(
                sequencer_height = block.height().value(),
                "a Sequencer Block was added to the next submission input but its height was \
                 already present; carrying on, but this shouldn't happen",
            );
        }
        let (metadata, rollup_data) = block.split_for_celestia();
        let metadata = metadata.into_raw();

        // XXX: This should really be set at the beginning of the sequencer-relayer and reused
        // everywhere.
        self.meta
            .sequencer_namespace
            .get_or_insert_with(|| sequencer_namespace(&metadata));
        self.metadata.push(metadata);
        for elem in rollup_data {
            if rollup_filter.should_include(&elem.rollup_id()) {
                let namespace =
                    astria_core::celestia::namespace_v0_from_rollup_id(elem.rollup_id());
                self.meta
                    .rollups_included
                    .insert(elem.rollup_id(), namespace);
                let list = self.rollup_data_for_namespace.entry(namespace).or_default();
                list.push(elem.into_raw());
            } else {
                self.meta.rollups_excluded.insert(elem.rollup_id());
            }
        }
    }

    fn greatest_sequencer_height(&self) -> Option<SequencerHeight> {
        self.meta.sequencer_heights.last().copied()
    }

    /// Attempts to convert the input into a payload of Celestia blobs.
    fn try_into_payload(self) -> Result<Payload, TryIntoPayloadError> {
        use prost::Name as _;

        let payload_len = self
            .metadata
            .len()
            .checked_add(self.rollup_data_for_namespace.len())
            .ok_or(TryIntoPayloadError::PayloadSize)?;
        let mut payload = Payload::with_capacity(payload_len);

        let sequencer_namespace = self
            .meta
            .sequencer_namespace
            .ok_or(TryIntoPayloadError::NoSequencerNamespacePresent)?;
        payload
            .try_add(
                sequencer_namespace,
                &SubmittedMetadataList {
                    entries: self.metadata,
                },
            )
            .map_err(|source| TryIntoPayloadError::AddToPayload {
                source,
                type_url: SubmittedMetadataList::type_url(),
            })?;

        for (namespace, entries) in self.rollup_data_for_namespace {
            payload
                .try_add(
                    namespace,
                    &SubmittedRollupDataList {
                        entries,
                    },
                )
                .map_err(|source| TryIntoPayloadError::AddToPayload {
                    source,
                    type_url: SubmittedRollupDataList::full_name(),
                })?;
        }
        Ok(payload)
    }
}

pub(super) struct NextSubmission {
    rollup_filter: IncludeRollup,
    input: Input,
    payload: Payload,
    metrics: &'static Metrics,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum TryAddError {
    #[error("next submission is full")]
    Full(Box<SequencerBlock>),
    #[error("failed converting input into payload of Celestia blobs")]
    IntoPayload(#[from] TryIntoPayloadError),
    #[error(
        "sequencer block at height `{sequencer_height}` is too large; its compressed single-block
         payload has size `{compressed_size}` bytes, which is larger than the maximum exception
         threshold of `{MAX_PAYLOAD_SIZE_BYTES}` bytes"
    )]
    OversizedBlock {
        sequencer_height: SequencerHeight,
        compressed_size: usize,
    },
}

impl NextSubmission {
    pub(super) fn new(rollup_filter: IncludeRollup, metrics: &'static Metrics) -> Self {
        Self {
            rollup_filter,
            input: Input::new(),
            payload: Payload::new(),
            metrics,
        }
    }

    /// Adds a [`SequencerBlock`] to the next submission.
    ///
    /// This function works by cloning the current payload input, adding `block` to it,
    /// and generating a new payload. If the new payload is sufficiently small, `block`
    /// will be included in the next submission. If it would exceed the maximum payload
    /// size it is returned as an error.
    pub(super) fn try_add(&mut self, block: SequencerBlock) -> Result<(), TryAddError> {
        let mut input_candidate = self.input.clone();
        input_candidate.extend_from_sequencer_block(block.clone(), &self.rollup_filter);

        let payload_creation_start = std::time::Instant::now();
        let payload_candidate = input_candidate.clone().try_into_payload()?;
        self.metrics
            .record_celestia_payload_creation_latency(payload_creation_start.elapsed());

        if payload_candidate.compressed_size <= MAX_PAYLOAD_SIZE_BYTES {
            self.input = input_candidate;
            self.payload = payload_candidate;
            Ok(())
        } else if input_candidate.num_blocks() == 1
            && payload_candidate.compressed_size > MAX_PAYLOAD_SIZE_BYTES
        {
            Err(TryAddError::OversizedBlock {
                sequencer_height: block.height(),
                compressed_size: payload_candidate.compressed_size,
            })
        } else {
            Err(TryAddError::Full(block.into()))
        }
    }

    /// Lazily move the currently items out of the next submission.
    ///
    /// The main reason for this method to exist is to work around async-cancellation.
    /// Only when the returned [`TakeNextSubmission`] future is polled is the data moved
    /// out, leaving behind an empty [`NextSubmission`] that can be used to accumulate more blocks.
    pub(super) fn take(&mut self) -> TakeSubmission<'_> {
        TakeSubmission {
            inner: Some(self),
        }
    }
}

pin_project! {
    pub(super) struct TakeSubmission<'a> {
        inner: Option<&'a mut NextSubmission>,
    }
}

impl<'a> Future for TakeSubmission<'a> {
    type Output = Option<Submission>;

    fn poll(self: Pin<&mut Self>, _: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let next = self
            .project()
            .inner
            .take()
            .expect("future must not be polled twice");
        let input = std::mem::take(&mut next.input);
        let payload = std::mem::take(&mut next.payload);
        if payload.is_empty() {
            trace!("payload is empty");
            Poll::Ready(None)
        } else {
            trace!(
                number_of_blobs = payload.num_blobs(),
                number_of_blocks = input.num_blocks(),
                "returning payload"
            );
            Poll::Ready(Some(Submission {
                input,
                payload,
            }))
        }
    }
}

/// Constructs a Celestia [`Namespace`] from a [`SubmittedMetadata`].
///
/// # Note
/// This should be constructed once at the beginning of sequencer-relayer and then
/// injected everywhere.
///
/// # Panics
/// Panics if the `header.header` field is unset. This is OK because the argument to this
/// function should only come from a [`SubmittedMetadata`] that was created from its verified
/// counterpart [`astria_core::sequencerblock::v1::SubmittedMetadata::into_raw`].
fn sequencer_namespace(metadata: &SubmittedMetadata) -> Namespace {
    use const_format::concatcp;
    use prost::Name;
    const HEADER_EXPECT_MSG: &str =
        concatcp!(SubmittedMetadata::PACKAGE, ".", SubmittedMetadata::NAME,);

    astria_core::celestia::namespace_v0_from_sha256_of_bytes(
        metadata
            .header
            .as_ref()
            .expect(HEADER_EXPECT_MSG)
            .chain_id
            .as_bytes(),
    )
}

fn serialize_opt_namespace<S>(
    namespace: &Option<Namespace>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
{
    use serde::ser::Serialize as _;
    namespace
        .as_ref()
        .map(|ns| telemetry::display::base64(ns.as_bytes()))
        .serialize(serializer)
}

fn serialize_sequencer_heights<'a, I, S>(heights: I, serializer: S) -> Result<S::Ok, S::Error>
where
    I: IntoIterator<Item = &'a SequencerHeight>,
    S: serde::ser::Serializer,
{
    serializer.collect_seq(heights.into_iter().map(tendermint::block::Height::value))
}

fn serialize_included_rollups<'a, I, S>(rollups: I, serializer: S) -> Result<S::Ok, S::Error>
where
    I: IntoIterator<Item = (&'a RollupId, &'a Namespace)>,
    S: serde::ser::Serializer,
{
    serializer.collect_map(
        rollups
            .into_iter()
            .map(|(id, ns)| (id, telemetry::display::base64(ns.as_bytes()))),
    )
}

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::RollupId,
        protocol::test_utils::ConfigureSequencerBlock,
    };
    use rand_chacha::{
        rand_core::{
            RngCore as _,
            SeedableRng as _,
        },
        ChaChaRng,
    };
    use sequencer_client::SequencerBlock;
    use telemetry::Metrics as _;

    use super::{
        Input,
        NextSubmission,
    };
    use crate::{
        metrics::Metrics,
        relayer::write::conversion::{
            TryAddError,
            MAX_PAYLOAD_SIZE_BYTES,
        },
        IncludeRollup,
    };

    fn include_all_rollups() -> IncludeRollup {
        IncludeRollup::parse("").unwrap()
    }

    fn metrics() -> &'static Metrics {
        Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()))
    }

    fn block(height: u32) -> SequencerBlock {
        ConfigureSequencerBlock {
            chain_id: Some("sequencer-0".to_string()),
            height,
            sequence_data: vec![(
                RollupId::from_unhashed_bytes(b"rollup-0"),
                b"hello world!".to_vec(),
            )],
            ..ConfigureSequencerBlock::default()
        }
        .make()
    }

    fn block_with_random_data(
        height: u32,
        num_bytes: usize,
        rng: &mut ChaChaRng,
    ) -> SequencerBlock {
        let mut random_bytes = vec![0; num_bytes];
        rng.fill_bytes(&mut random_bytes);
        ConfigureSequencerBlock {
            chain_id: Some("sequencer-0".to_string()),
            height,
            sequence_data: vec![(RollupId::from_unhashed_bytes(b"rollup-0"), random_bytes)],
            ..ConfigureSequencerBlock::default()
        }
        .make()
    }

    #[tokio::test]
    async fn add_sequencer_block_to_empty_next_submission() {
        let mut next_submission = NextSubmission::new(include_all_rollups(), metrics());
        next_submission.try_add(block(1)).unwrap();
        let submission = next_submission.take().await.unwrap();
        assert_eq!(1, submission.num_blocks());
        assert_eq!(2, submission.num_blobs());
    }

    #[test]
    fn adding_three_sequencer_blocks_with_same_ids_doesnt_change_number_of_blobs() {
        let mut next_submission = NextSubmission::new(include_all_rollups(), metrics());
        next_submission.try_add(block(1)).unwrap();
        next_submission.try_add(block(2)).unwrap();
        next_submission.try_add(block(3)).unwrap();
        let submission = tokio_test::block_on(next_submission.take()).unwrap();
        assert_eq!(3, submission.num_blocks());
        assert_eq!(2, submission.num_blobs());
    }

    #[test]
    fn adding_block_to_full_submission_gets_rejected() {
        // this test makes use of the fact that random data is essentially incompressible so
        // that size(uncompressed_payload) ~= size(compressed_payload).
        let mut rng = ChaChaRng::seed_from_u64(0);
        let mut next_submission = NextSubmission::new(include_all_rollups(), metrics());
        // adding 9 blocks with 100KB random data each, which gives a (compressed) payload slightly
        // above 900KB.
        let num_bytes = 100_000usize;
        for height in 1..=9 {
            next_submission
                .try_add(block_with_random_data(height, num_bytes, &mut rng))
                .unwrap();
        }
        let overflowing_block = block_with_random_data(10, num_bytes, &mut rng);
        let rejected_block = match next_submission.try_add(overflowing_block.clone()) {
            Err(TryAddError::Full(block)) => *block,
            other => panic!("expected a `Err(TryAddError::Full)`, but got `{other:?}`"),
        };
        assert_eq!(overflowing_block, rejected_block);
    }

    #[test]
    fn oversized_block_is_rejected() {
        // this test makes use of the fact that random data is essentially incompressible so
        // that size(uncompressed_payload) ~= size(compressed_payload).
        let mut rng = ChaChaRng::seed_from_u64(0);
        let mut next_submission = NextSubmission::new(include_all_rollups(), metrics());

        // using the upper limit defined in the constant and add 1KB of extra bytes to ensure
        // the block is too large
        let oversized_block = block_with_random_data(10, MAX_PAYLOAD_SIZE_BYTES + 1_000, &mut rng);
        match next_submission.try_add(oversized_block) {
            Err(TryAddError::OversizedBlock {
                sequencer_height, ..
            }) => assert_eq!(sequencer_height.value(), 10),
            other => panic!("expected a `Err(TryAddError::OversizedBlock)`, but got `{other:?}`"),
        }
    }

    #[test]
    fn extend_empty_input_from_sequencer_block() {
        let mut input = Input::new();
        input.extend_from_sequencer_block(block(1), &include_all_rollups());
        assert_eq!(1, input.num_blocks());
    }

    #[test]
    fn convert_input_to_payload() {
        let mut input = Input::new();
        input.extend_from_sequencer_block(block(1), &include_all_rollups());
        let payload = input.try_into_payload().unwrap();
        assert_eq!(2, payload.num_blobs());
    }
}
