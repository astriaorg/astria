//! Tracks the current submission state of sequencer-relayer and syncs it to disk.

use std::path::{
    Path,
    PathBuf,
};

use astria_eyre::eyre::{
    self,
    bail,
    ensure,
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
        #[serde(with = "as_number")]
        sequencer_height: SequencerHeight,
    },
}

impl PostSubmission {
    fn is_fresh(&self) -> bool {
        matches!(self, PostSubmission::Fresh)
    }

    fn is_submitted(&self) -> bool {
        matches!(self, PostSubmission::Submitted { .. })
    }

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
        #[serde(with = "as_number")]
        sequencer_height: SequencerHeight,
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

#[derive(Debug)]
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
    pub(super) fn last_submitted_height(&self) -> Option<SequencerHeight> {
        match self.post {
            PostSubmission::Fresh => None,
            PostSubmission::Submitted {
                sequencer_height, ..
            } => Some(sequencer_height),
        }
    }

    pub(super) fn initialize(self, sequencer_height: SequencerHeight) -> eyre::Result<Started> {
        if let PostSubmission::Submitted {
            sequencer_height: latest_submitted,
            ..
        } = self.post
        {
            ensure!(
                sequencer_height > latest_submitted,
                "refusing to submit a sequencer block at heights below or at what was already \
                 submitted"
            );
        }
        let new = Self {
            pre: PreSubmission::Started {
                sequencer_height,
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
            (PreSubmission::Ignore, post) => Self {
                pre,
                post,
                pre_path,
                post_path,
            },

            (
                PreSubmission::Started {
                    sequencer_height,
                    last_submission,
                },
                post,
            ) => {
                ensure_consistent(sequencer_height, last_submission, post)
                    .wrap_err("on-disk states are inconsistent")?;
                Self {
                    pre,
                    post,
                    pre_path,
                    post_path,
                }
            }
        };
        Ok(state)
    }
}

fn ensure_consistent(
    sequencer_height_started: SequencerHeight,
    last_submission: PostSubmission,
    current_submission: PostSubmission,
) -> eyre::Result<()> {
    ensure_height_pre_submission_is_height_post_submission(
        sequencer_height_started,
        current_submission,
    )?;
    ensure_last_and_current_are_different(last_submission, current_submission)?;
    ensure_last_is_not_submitted_while_current_is_fresh(last_submission, current_submission)?;
    ensure_height_in_last_is_less_than_height_in_current(last_submission, current_submission)?;
    Ok(())
}

fn ensure_height_pre_submission_is_height_post_submission(
    sequencer_height_started: SequencerHeight,
    current_submission: PostSubmission,
) -> eyre::Result<()> {
    let PostSubmission::Submitted {
        sequencer_height, ..
    } = current_submission
    else {
        bail!(
            "the pre-submit file indicated that a new submission was started, but the post-submit \
             file still contained a \"fresh\" state. This indicates that the submission was not \
             finalized."
        );
    };
    ensure!(
        sequencer_height_started == sequencer_height,
        "the initialized `sequencer_height` in the pre-submit file does not match the sequencer \
         height in the post-submit file. This indicates that a new submission to Celestia was \
         started but not finalized. This is becasue a succesful submission records the very same \
         `sequencer_height` in the post-submit file."
    );
    Ok(())
}

fn ensure_last_and_current_are_different(
    last_submission: PostSubmission,
    current_submission: PostSubmission,
) -> eyre::Result<()> {
    ensure!(
        last_submission != current_submission,
        "the `last_submission` field of the pre-submit file matches the object found in the \
         post-submit file. This indicates that a new submission to Celestia was started but not \
         finalized. This is because when starting a new submission the object in the post-submit \
         file is written to `last_submission`."
    );
    Ok(())
}

fn ensure_last_is_not_submitted_while_current_is_fresh(
    last_submission: PostSubmission,
    current_submission: PostSubmission,
) -> eyre::Result<()> {
    ensure!(
        !(last_submission.is_submitted() && current_submission.is_fresh()),
        "the submission recorded in the post-submit file cannot be `fresh` while \
         `last_submission` in the pre-submit file is `submitted`",
    );
    Ok(())
}

fn ensure_height_in_last_is_less_than_height_in_current(
    last_submission: PostSubmission,
    current_submission: PostSubmission,
) -> eyre::Result<()> {
    let PostSubmission::Submitted {
        sequencer_height: height_in_last,
        ..
    } = last_submission
    else {
        return Ok(());
    };
    let PostSubmission::Submitted {
        sequencer_height: height_in_current,
        ..
    } = current_submission
    else {
        return Ok(());
    };
    ensure!(
        height_in_last < height_in_current,
        "the `sequencer_height` in the post-submit file is not greater than the \
         `sequencer_height` stored in the `last_submission` field of the pre-submit file.
        This indicates that a new submission was started not but finalized."
    );
    Ok(())
}

mod as_number {
    //! Logic to serialize sequencer heights as number, deserialize numbers as sequencer heights.
    //!
    //! This is unfortunately necessary because the [`serde::Serialize`], [`serde::Deserialize`]
    //! implementations for [`sequencer_client::tendermint::block::Height`] write the integer as
    //! string, probably due to tendermint's/cometbft's go-legacy.
    use serde::{
        Deserialize as _,
        Deserializer,
        Serializer,
    };

    use super::SequencerHeight;
    pub(super) fn serialize<S>(height: &SequencerHeight, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(height.value())
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<SequencerHeight, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        let height = u64::deserialize(deserializer)?;
        SequencerHeight::try_from(height).map_err(Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::NamedTempFile;

    use super::SubmissionState;
    use crate::relayer::submission::PostSubmission;

    #[track_caller]
    fn create_files() -> (NamedTempFile, NamedTempFile) {
        let pre = NamedTempFile::new()
            .expect("must be able to create an empty pre submit state file to run tests");
        let post = NamedTempFile::new()
            .expect("must be able to create an empty post submit state file to run tests");
        (pre, post)
    }

    fn write(f: &NamedTempFile, val: &serde_json::Value) {
        serde_json::to_writer(f, val).expect("must be able to write state to run tests");
    }

    #[test]
    fn fresh_with_ignored_is_ok() {
        let (pre, post) = create_files();
        write(&pre, &json!({ "state": "ignore" }));
        write(&post, &json!({ "state": "fresh" }));
        SubmissionState::from_paths(pre.path(), post.path())
            .expect("states `ignore` and `fresh` give a working submission state");
    }

    #[test]
    fn submitted_with_ignored_is_ok() {
        let (pre, post) = create_files();
        write(&pre, &json!({ "state": "ignore" }));
        write(
            &post,
            &json!({ "state": "submitted", "celestia_height": 5, "sequencer_height": 2 }),
        );
        SubmissionState::from_paths(pre.path(), post.path())
            .expect("states `ignore` and `submitted` give a working submission state");
    }

    #[test]
    fn started_with_same_fresh_in_last_and_current_is_err() {
        let (pre, post) = create_files();
        write(
            &pre,
            &json!({ "state": "started", "sequencer_height": 5, "last_submission": { "state": "fresh"} }),
        );
        write(&post, &json!({ "state": "fresh" }));
        let _ = SubmissionState::from_paths(pre.path(), post.path())
            .expect_err("started state with `fresh` in last and current gives error");
    }

    #[test]
    fn started_with_height_before_current_is_err() {
        let (pre, post) = create_files();
        write(
            &pre,
            &json!({ "state": "started", "sequencer_height": 5, "last_submission": { "state": "fresh"} }),
        );
        write(
            &post,
            &json!({ "state": "submitted", "sequencer_height": 6, "celestia_height": 2 }),
        );
        let _ = SubmissionState::from_paths(pre.path(), post.path()).expect_err(
            "started state with sequencer height less then sequencer height recorded submitted \
             gives error",
        );
    }

    #[test]
    fn started_with_same_submitted_in_last_and_current_is_err() {
        let (pre, post) = create_files();
        write(
            &pre,
            &json!({ "state": "started", "sequencer_height": 2, "last_submission": { "state": "submitted", "celestia_height": 5, "sequencer_height": 2} }),
        );
        write(
            &post,
            &json!({ "state": "submitted", "celestia_height": 5, "sequencer_height": 2 }),
        );
        let _ = SubmissionState::from_paths(pre.path(), post.path()).expect_err(
            "started state with the same `submitted` in last and current give an error",
        );
    }

    #[test]
    fn started_with_different_last_fresh_and_current_submitted_is_ok() {
        let (pre, post) = create_files();
        write(
            &pre,
            &json!({ "state": "started", "sequencer_height": 2, "last_submission": { "state": "fresh" }}),
        );
        write(
            &post,
            &json!({ "state": "submitted", "celestia_height": 5, "sequencer_height": 2 }),
        );
        let _ = SubmissionState::from_paths(pre.path(), post.path()).expect(
            "started state with the `fresh` in last and `submitted` in current gives working \
             submission state",
        );
    }

    #[test]
    fn submit_initialize_finalize_flow_works() {
        let (pre, post) = create_files();
        write(
            &pre,
            &json!({ "state": "started", "sequencer_height": 2, "last_submission": { "state": "fresh" }}),
        );
        write(
            &post,
            &json!({ "state": "submitted", "celestia_height": 5, "sequencer_height": 2 }),
        );
        let state = SubmissionState::from_paths(pre.path(), post.path()).expect(
            "started state with the `fresh` in last and `submitted` in current gives working \
             submission state",
        );
        let started = state.initialize(3u32.into()).unwrap();
        let finalized = started.finalize(6).unwrap();
        let PostSubmission::Submitted {
            celestia_height,
            sequencer_height,
        } = finalized.post
        else {
            panic!("the post submission state should be `submitted`");
        };
        assert_eq!(celestia_height, 6);
        assert_eq!(sequencer_height.value(), 3);
    }

    #[test]
    fn submit_old_blocks_gives_error() {
        let (pre, post) = create_files();
        write(
            &pre,
            &json!({ "state": "started", "sequencer_height": 2, "last_submission": { "state": "fresh" }}),
        );
        write(
            &post,
            &json!({ "state": "submitted", "celestia_height": 5, "sequencer_height": 2 }),
        );
        let state = SubmissionState::from_paths(pre.path(), post.path()).expect(
            "started state with `fresh` in last and `submitted` in current gives working \
             submission state",
        );
        let _ = state
            .initialize(2u32.into())
            .expect_err("trying to submit the same sequencer height is an error");
    }
}
