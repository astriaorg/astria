use std::{
    collections::{
        BTreeSet,
        HashMap,
    },
    pin::Pin,
    task::Poll,
};

use astria_core::{
    brotli::compress_bytes,
    generated::sequencerblock::v1alpha1::{
        CelestiaHeader,
        CelestiaHeaderList,
        CelestiaRollupData,
        CelestiaRollupDataList,
    },
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
    trace,
    warn,
};

use crate::IncludeRollup;

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

    // allow: used for metric gauges, which require f64. Precision loss is ok and of no
    // significance.
    #[allow(clippy::cast_precision_loss)]
    pub(super) fn compression_ratio(&self) -> f64 {
        self.compressed_size() as f64 / self.uncompressed_size() as f64
    }

    pub(super) fn compressed_size(&self) -> usize {
        self.payload.compressed_size()
    }

    pub(super) fn uncompressed_size(&self) -> usize {
        self.payload.uncompressed_size()
    }
}

#[derive(Debug, thiserror::Error)]
enum PayloadError {
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
        self.uncompressed_size += encoded.len();
        self.compressed_size += blob.data.len();
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
#[error("failed adding protobuf `{type_url}` to Celestia blob payload")]
pub(super) struct TryIntoPayloadError {
    source: PayloadError,
    type_url: String,
}

#[derive(Clone, Debug, Default)]
struct Input {
    sequencer_heights: BTreeSet<SequencerHeight>,
    headers: Vec<CelestiaHeader>,
    rollup_data_for_namespace: HashMap<Namespace, Vec<CelestiaRollupData>>,
}

impl Input {
    fn new() -> Self {
        Self::default()
    }

    fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }

    fn num_blocks(&self) -> usize {
        self.headers.len()
    }

    fn extend_from_sequencer_block(
        &mut self,
        block: SequencerBlock,
        rollup_filter: &IncludeRollup,
    ) {
        if !self.sequencer_heights.insert(block.height()) {
            warn!(
                sequencer_height = block.height().value(),
                "a Sequencer Block was added to the next submission input but its height was \
                 already present; carrying on, but this shouldn't happen",
            );
        }
        let (header, rollup_elements) = block.split_for_celestia();
        self.headers.push(header.into_raw());
        for elem in rollup_elements {
            if rollup_filter.should_include(&elem.rollup_id()) {
                let namespace =
                    astria_core::celestia::namespace_v0_from_rollup_id(elem.rollup_id());
                let list = self.rollup_data_for_namespace.entry(namespace).or_default();
                list.push(elem.into_raw());
            }
        }
    }

    fn greatest_sequencer_height(&self) -> Option<SequencerHeight> {
        self.sequencer_heights.last().copied()
    }

    fn try_into_payload(self) -> Result<Payload, TryIntoPayloadError> {
        use prost::Name as _;

        let mut payload =
            Payload::with_capacity(self.headers.len() + self.rollup_data_for_namespace.len());

        let sequencer_namespace = sequencer_namespace(self.headers.last().expect(
            "Input::try_to_payload must only be called if there is an actual input to convert",
        ));
        payload
            .try_add(
                sequencer_namespace,
                &CelestiaHeaderList {
                    headers: self.headers,
                },
            )
            .map_err(|source| TryIntoPayloadError {
                source,
                type_url: CelestiaHeaderList::type_url(),
            })?;

        for (namespace, entries) in self.rollup_data_for_namespace {
            payload
                .try_add(
                    namespace,
                    &CelestiaRollupDataList {
                        entries,
                    },
                )
                .map_err(|source| TryIntoPayloadError {
                    source,
                    type_url: CelestiaRollupDataList::full_name(),
                })?;
        }
        Ok(payload)
    }
}

#[derive(Debug)]
pub(super) struct NextSubmission {
    rollup_filter: IncludeRollup,
    input: Input,
    payload: Payload,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum TryAddError {
    #[error("next submission is full")]
    Full(Box<SequencerBlock>),
    #[error("failed converting input into payload of Celestia blobs")]
    IntoPayload(#[from] TryIntoPayloadError),
}

impl NextSubmission {
    pub(super) fn new(rollup_filter: IncludeRollup) -> Self {
        Self {
            rollup_filter,
            input: Input::new(),
            payload: Payload::new(),
        }
    }

    /// Adds a [`SequencerBlock`] to the next submission.
    ///
    /// Returns the block if it was rejected by the next submission (this happens if
    /// if adding it would exceed the hard coded limit).
    pub(super) fn try_add(&mut self, block: SequencerBlock) -> Result<(), TryAddError> {
        let mut input_candidate = self.input.clone();
        input_candidate.extend_from_sequencer_block(block.clone(), &self.rollup_filter);
        let payload_candidate = input_candidate.clone().try_into_payload()?;

        // Always include a block into the next submission if empty. This ensures that no blocks
        // are dropped.
        if self.input.is_empty() || payload_candidate.compressed_size <= MAX_PAYLOAD_SIZE_BYTES {
            self.input = input_candidate;
            self.payload = payload_candidate;
            Ok(())
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

/// Constructs a Celestia [`Namespace`] from a [`CelestiaHeader`].
///
/// # Note
/// This should be constructed once at the beginning of sequencer-relayer and then
/// injected everywhere.
///
/// # Panics
/// Panics if the `header.header` field is unset. This is OK because the argument to this
/// function should only come from a [`CelestiaHeader`] that was created from its verified
/// counterpart [`astria_core::sequencerblock::v1alpha1::CelestiaHeader::into_raw`].
fn sequencer_namespace(header: &CelestiaHeader) -> Namespace {
    use const_format::concatcp;
    use prost::Name;
    const HEADER_EXPECT_MSG: &str = concatcp!(CelestiaHeader::PACKAGE, ".", CelestiaHeader::NAME,);

    astria_core::celestia::namespace_v0_from_sha256_of_bytes(
        header
            .header
            .as_ref()
            .expect(HEADER_EXPECT_MSG)
            .chain_id
            .as_bytes(),
    )
}

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::RollupId,
        protocol::test_utils::ConfigureSequencerBlock,
    };
    use sequencer_client::SequencerBlock;

    use super::{
        Input,
        NextSubmission,
    };
    use crate::IncludeRollup;

    fn include_all_rollups() -> IncludeRollup {
        IncludeRollup::parse("").unwrap()
    }

    fn sequencer_block(height: u32) -> SequencerBlock {
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

    #[tokio::test]
    async fn add_sequencer_block_to_empty_next_submission() {
        let mut next_submission = NextSubmission::new(include_all_rollups());
        next_submission.try_add(sequencer_block(1)).unwrap();
        let submission = next_submission.take().await.unwrap();
        assert_eq!(1, submission.num_blocks());
        assert_eq!(2, submission.num_blobs());
    }

    #[tokio::test]
    async fn adding_three_sequencer_blocks_with_same_ids_doesnt_change_number_of_blobs() {
        let mut next_submission = NextSubmission::new(include_all_rollups());
        next_submission.try_add(sequencer_block(1)).unwrap();
        next_submission.try_add(sequencer_block(2)).unwrap();
        next_submission.try_add(sequencer_block(3)).unwrap();
        let submission = next_submission.take().await.unwrap();
        assert_eq!(3, submission.num_blocks());
        assert_eq!(2, submission.num_blobs());
    }

    #[test]
    fn extend_empty_input_from_sequencer_block() {
        let mut input = Input::new();
        input.extend_from_sequencer_block(sequencer_block(1), &include_all_rollups());
        assert_eq!(1, input.num_blocks());
    }

    #[test]
    fn convert_input_to_payload() {
        let mut input = Input::new();
        input.extend_from_sequencer_block(sequencer_block(1), &include_all_rollups());
        let payload = input.try_into_payload().unwrap();
        assert_eq!(2, payload.num_blobs());
    }
}
