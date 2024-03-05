//! Tracks the current submission state of sequencer-relayer and syncs it to disk.

use std::path::{
    Path,
    PathBuf,
};

use astria_eyre::eyre::{
    self,
    bail,
    WrapErr as _,
};
use sequencer_client::tendermint::block::Height as SequencerHeight;
use serde::{
    Deserialize,
    Serialize,
};
use tracing::debug;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "state")]
enum PostSubmission {
    Fresh,
    Submitted {
        celestia_height: u64,
        sequencer_height: u64,
    },
}
impl PostSubmission {
    fn from_path<P: AsRef<Path>>(path: P) -> eyre::Result<Self> {
        let file = std::fs::File::open(&path)
            .wrap_err("failed opening provided file path for reading post-submission state")?;
        let state = serde_json::from_reader(file)
            .wrap_err("failed reading contents of post-submission file")?;
        Ok(state)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "state")]
enum PreSubmission {
    Ignore,
    Started {
        sequencer_height: u64,
        last_submission: PostSubmission,
    },
}
impl PreSubmission {
    fn from_path<P: AsRef<Path>>(path: P) -> eyre::Result<Self> {
        let file = std::fs::File::open(&path)
            .wrap_err("failed opening provided file path for reading pre-submission state")?;
        let state = serde_json::from_reader(file)
            .wrap_err("failed reading contents of pre-submission file")?;
        Ok(state)
    }
}

#[derive(Clone, Debug)]
pub(super) struct SubmissionState {
    pre: PreSubmission,
    post: PostSubmission,
    pre_path: PathBuf,
    post_path: PathBuf,
}

pub(super) struct Started(SubmissionState);

impl Started {
    pub(super) fn finalize(self, celestia_height: u64) -> eyre::Result<SubmissionState> {
        let Self(SubmissionState {
            pre,
            pre_path,
            post_path,
            ..
        }) = self;

        let PreSubmission::Started {
            sequencer_height, ..
        } = pre
        else {
            panic!(
                "once intialized, a submission's `pre` field must always be `Started`. Here it is \
                 not. This is a bug"
            );
        };

        let new = SubmissionState {
            pre,
            post: PostSubmission::Submitted {
                celestia_height,
                sequencer_height,
            },
            pre_path,
            post_path,
        };

        debug!(
            state = serde_json::to_string(&new.post).expect("type contains no non-ascii keys"),
            "finalizing submission by writing post-submit state to file",
        );
        let f = std::fs::File::options()
            .write(true)
            .truncate(true)
            .open(&new.post_path)
            .wrap_err("failed opening post-submit file for writing state")?;
        serde_json::to_writer(&f, &new.post)
            .wrap_err("failed writing post-submit state to file")?;
        f.sync_all()
            .wrap_err("failed syncing post-submit file to disk")?;
        Ok(new)
    }
}

impl SubmissionState {
    pub(super) fn last_fetched_height(&self) -> Option<u64> {
        match self.post {
            PostSubmission::Fresh => None,
            PostSubmission::Submitted {
                sequencer_height, ..
            } => Some(sequencer_height),
        }
    }

    pub(super) fn initialize(self, sequencer_height: SequencerHeight) -> eyre::Result<Started> {
        let new = Self {
            pre: PreSubmission::Started {
                sequencer_height: sequencer_height.value(),
                last_submission: self.post,
            },
            ..self
        };
        debug!(
            state = serde_json::to_string(&new.pre).expect("type contains no non-ascii keys"),
            "initializing submission by writing pre-submit state to file",
        );
        let f = std::fs::File::options()
            .write(true)
            .truncate(true)
            .open(&new.pre_path)
            .wrap_err("failed opening presubmit file for writing state")?;
        serde_json::to_writer(&f, &new.pre)
            .wrap_err("failed writing presubmission state to file")?;
        f.sync_all()
            .wrap_err("failed syncing presubmission file to disk")?;
        Ok(Started(new))
    }

    pub(super) fn from_paths<P1: AsRef<Path>, P2: AsRef<Path>>(
        pre_path: P1,
        post_path: P2,
    ) -> eyre::Result<Self> {
        let pre_path = pre_path.as_ref().to_path_buf();
        let post_path = post_path.as_ref().to_path_buf();

        let pre = PreSubmission::from_path(&pre_path).wrap_err(
            "failed constructing post submit state from provided path; if the post-submit state is
                otherwise present and correctly formatted - and contains the correct information \
             about the
                last submitted sequencer height and its inclusion height on Celestia - then \
             override the
                contents of the pre-submission file with `{{\"state\": \"ignore\"}}`",
        )?;
        let post = PostSubmission::from_path(&post_path).wrap_err(
            "failed constructing post submit state from provided path; if the pre-submit state is \
             otherwise present and correctly formatted then this indicates a file corruption. \
             Because the post-submit state is critical for determining the starting height for \
             relaying sequencer blocks, make sure it's set correctly.",
        )?;

        let state = match (pre, post) {
            (PreSubmission::Ignore, PostSubmission::Fresh) => Self {
                pre,
                post,
                pre_path,
                post_path,
            },

            (
                pre @ PreSubmission::Ignore,
                post @ PostSubmission::Submitted {
                    ..
                },
            ) => Self {
                pre,
                post,
                pre_path,
                post_path,
            },

            (
                PreSubmission::Started {
                    sequencer_height,
                    last_submission: post_state_in_pre,
                },
                post_state_in_post,
            ) if post_state_in_pre == post_state_in_post => bail!(
                "the last post-submission state recorded in the pre-submit file matches the state \
                 in the post-submission file. This indicates that either submission to Celestia \
                 failed, or that relayer failed to write its post-submission state to disk. \
                 Verify that the block at sequencer height `{sequencer_height}` was written to \
                 Celestia and update the post-submission file with the sequencer and Celestia \
                 heights or, if you want to have relayer start submitting from sequencer height \
                 1, update the pre-submission file to read `{{\"state\": \"fresh\"}}`"
            ),

            (pre, post) => Self {
                pre,
                post,
                pre_path,
                post_path,
            },
        };
        Ok(state)
    }
}
