//! Tracks the current submission state of sequencer-relayer and syncs it to disk.

use std::{
    fmt::{
        self,
        Display,
        Formatter,
    },
    path::{
        Path,
        PathBuf,
    },
    time::{
        Duration,
        SystemTime,
    },
};

use astria_eyre::eyre::{
    self,
    ensure,
    WrapErr as _,
};
use serde::{
    Deserialize,
    Serialize,
};
use tendermint::block::Height as SequencerHeight;
use tracing::debug;

use super::BlobTxHash;

/// Represents a submission made to Celestia which has been confirmed as stored via a successful
/// `GetTx` call.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(super) struct CompletedSubmission {
    /// The height of the Celestia block in which the submission was stored.
    celestia_height: u64,
    /// The highest sequencer block height contained in the submission.
    #[serde(with = "as_number")]
    sequencer_height: SequencerHeight,
}

impl CompletedSubmission {
    fn new(celestia_height: u64, sequencer_height: SequencerHeight) -> Self {
        Self {
            celestia_height,
            sequencer_height,
        }
    }
}

/// Newtype wrapper for the file path of the submission state.
#[derive(Clone, Debug)]
struct StateFilePath(PathBuf);

/// Newtype wrapper for the file path of the temp file used when writing submission state to disk.
#[derive(Clone, Debug)]
struct TempFilePath(PathBuf);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "state")]
enum State {
    /// Indicates the first run of the sequencer, i.e. no previous session occurred.
    Fresh,
    /// Indicates that we have started to prepare a new submission. Preparation involves fetching
    /// information from the Celestia app (nonce, prices, etc.), and using that to create a signed
    /// blob transaction.
    Started {
        last_submission: CompletedSubmission,
    },
    /// Indicates that preparation of a signed blob transaction has happened, and we are now in the
    /// process of submitting the transaction (sending a `broadcast_tx` gRPC) and confirming its
    /// submission (polling via `get_tx` gRPCs).
    Prepared {
        #[serde(with = "as_number")]
        sequencer_height: SequencerHeight,
        last_submission: CompletedSubmission,
        blob_tx_hash: BlobTxHash,
        #[serde(with = "humantime_serde")]
        at: SystemTime,
    },
}

impl State {
    fn new_started(last_submission: CompletedSubmission) -> Self {
        Self::Started {
            last_submission,
        }
    }

    fn new_prepared(
        sequencer_height: SequencerHeight,
        last_submission: CompletedSubmission,
        blob_tx_hash: BlobTxHash,
        at: SystemTime,
    ) -> Self {
        Self::Prepared {
            sequencer_height,
            last_submission,
            blob_tx_hash,
            at,
        }
    }

    /// Constructs an instance of `State` by parsing from `source`: a JSON-encoded file.
    async fn read(source: &StateFilePath) -> eyre::Result<Self> {
        let contents = tokio::fs::read_to_string(&source.0)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed reading submission state file at `{}`",
                    source.0.display()
                )
            })?;
        let state: State = serde_json::from_str(&contents)
            .wrap_err_with(|| format!("failed parsing the contents of `{}`", source.0.display()))?;

        // Ensure the parsed values are sane.
        match &state {
            State::Fresh
            | State::Started {
                ..
            } => {}
            State::Prepared {
                sequencer_height,
                last_submission,
                ..
            } => ensure!(
                *sequencer_height > last_submission.sequencer_height,
                "submission state file `{}` invalid: current sequencer height \
                 ({sequencer_height}) should be greater than last successful submission sequencer \
                 height ({})",
                source.0.display(),
                last_submission.sequencer_height
            ),
        }

        Ok(state)
    }

    /// Writes JSON-encoded `self` to `temp_file`, then renames `temp_file` to `destination`.
    async fn write(
        &self,
        destination: &StateFilePath,
        temp_file: &TempFilePath,
    ) -> eyre::Result<()> {
        let contents =
            serde_json::to_string_pretty(self).wrap_err("failed json-encoding submission state")?;
        tokio::fs::write(&temp_file.0, &contents)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed writing submission state to `{}`",
                    temp_file.0.display()
                )
            })?;
        tokio::fs::rename(&temp_file.0, &destination.0)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed moving `{}` to `{}`",
                    temp_file.0.display(),
                    destination.0.display()
                )
            })
    }
}

impl Display for State {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if formatter.alternate() {
            write!(formatter, "{}", serde_json::to_string_pretty(self).unwrap())
        } else {
            write!(formatter, "{}", serde_json::to_string(self).unwrap())
        }
    }
}

/// State indicating the relayer has not performed any submissions previously.
#[derive(Debug)]
pub(super) struct FreshSubmission {
    state_file_path: StateFilePath,
    temp_file_path: TempFilePath,
}

impl FreshSubmission {
    /// Converts `self` into a `StartedSubmission` with last submission being given celestia and
    /// sequencer heights of 0.
    ///
    /// The new state is not written to disk, as it is not required - a restart will perform the
    /// correct operation if the state on disk is left as `fresh`.
    pub(super) fn into_started(self) -> StartedSubmission {
        let last_submission = CompletedSubmission::new(0, SequencerHeight::from(0_u8));
        StartedSubmission {
            last_submission,
            state_file_path: self.state_file_path,
            temp_file_path: self.temp_file_path,
        }
    }
}

/// State indicating the relayer has started to prepare a new submission.
#[derive(Clone, Debug)]
pub(super) struct StartedSubmission {
    last_submission: CompletedSubmission,
    state_file_path: StateFilePath,
    temp_file_path: TempFilePath,
}

impl StartedSubmission {
    /// Constructs a new `StartedSubmission` and writes the state to disk.
    async fn construct_and_write(
        last_submission: CompletedSubmission,
        state_file_path: StateFilePath,
        temp_file_path: TempFilePath,
    ) -> eyre::Result<Self> {
        let state = State::new_started(last_submission);
        debug!(%state, "writing submission started state to file");
        state
            .write(&state_file_path, &temp_file_path)
            .await
            .wrap_err("failed commiting submission started state to disk")?;
        Ok(Self {
            last_submission,
            state_file_path,
            temp_file_path,
        })
    }

    /// Returns the celestia block height from the last completed submission.
    pub(super) fn last_submission_celestia_height(&self) -> u64 {
        self.last_submission.celestia_height
    }

    /// Returns the sequencer block height from the last completed submission.
    pub(super) fn last_submission_sequencer_height(&self) -> SequencerHeight {
        self.last_submission.sequencer_height
    }

    /// Converts `self` into a `PreparedSubmission` and writes the new state to disk.
    pub(super) async fn into_prepared(
        self,
        new_sequencer_height: SequencerHeight,
        blob_tx_hash: BlobTxHash,
    ) -> eyre::Result<PreparedSubmission> {
        PreparedSubmission::construct_and_write(
            new_sequencer_height,
            self.last_submission,
            blob_tx_hash,
            self.state_file_path,
            self.temp_file_path,
        )
        .await
    }
}

impl Display for StartedSubmission {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "file: {}, {}",
            self.state_file_path.0.display(),
            State::new_started(self.last_submission)
        )
    }
}

/// State indicating the relayer has prepared a new submission and is about to broadcast it to the
/// Celestia app.
#[derive(Clone, Debug)]
pub(super) struct PreparedSubmission {
    sequencer_height: SequencerHeight,
    last_submission: CompletedSubmission,
    blob_tx_hash: BlobTxHash,
    created_at: SystemTime,
    state_file_path: StateFilePath,
    temp_file_path: TempFilePath,
}

impl PreparedSubmission {
    /// Constructs a new `PreparedSubmission` and writes the state to disk.
    async fn construct_and_write(
        sequencer_height: SequencerHeight,
        last_submission: CompletedSubmission,
        blob_tx_hash: BlobTxHash,
        state_file_path: StateFilePath,
        temp_file_path: TempFilePath,
    ) -> eyre::Result<Self> {
        ensure!(
            sequencer_height > last_submission.sequencer_height,
            "cannot submit a sequencer block at height below or equal to what was already \
             successfully submitted"
        );
        let created_at = SystemTime::now();
        let state =
            State::new_prepared(sequencer_height, last_submission, blob_tx_hash, created_at);
        state
            .write(&state_file_path, &temp_file_path)
            .await
            .wrap_err("failed commiting submission prepared state to disk")?;
        Ok(Self {
            sequencer_height,
            last_submission,
            blob_tx_hash,
            created_at,
            state_file_path,
            temp_file_path,
        })
    }

    /// Returns the transaction hash of the prepared `BlobTx`.
    pub(super) fn blob_tx_hash(&self) -> &BlobTxHash {
        &self.blob_tx_hash
    }

    /// Returns the maximum duration for which the Celestia app should be polled with `GetTx`
    /// requests to confirm successful storage of the associated `BlobTx`.
    ///
    /// This is at least 15 seconds, but up to a maximum of a minute from when the submission was
    /// first attempted.
    pub(super) fn confirmation_timeout(&self) -> Duration {
        std::cmp::max(
            Duration::from_secs(15),
            Duration::from_secs(60).saturating_sub(self.created_at.elapsed().unwrap_or_default()),
        )
    }

    /// Converts `self` into a `StartedSubmission` with last submission being recorded using the
    /// provided celestia height and the sequencer height from `self`. Writes the new state to disk.
    pub(super) async fn into_started(
        self,
        celestia_height: u64,
    ) -> eyre::Result<StartedSubmission> {
        let last_submission = CompletedSubmission::new(celestia_height, self.sequencer_height);
        StartedSubmission::construct_and_write(
            last_submission,
            self.state_file_path,
            self.temp_file_path,
        )
        .await
    }

    /// Reverts `self` into a `StartedSubmission` retaining the last submission from `self` as the
    /// last submission. Writes the new state to disk.
    pub(super) async fn revert(self) -> eyre::Result<StartedSubmission> {
        StartedSubmission::construct_and_write(
            self.last_submission,
            self.state_file_path,
            self.temp_file_path,
        )
        .await
    }
}

impl Display for PreparedSubmission {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "file: {}, {}",
            self.state_file_path.0.display(),
            State::new_prepared(
                self.sequencer_height,
                self.last_submission,
                self.blob_tx_hash,
                self.created_at
            )
        )
    }
}

#[derive(Debug)]
pub(super) enum SubmissionStateAtStartup {
    Fresh(FreshSubmission),
    Started(StartedSubmission),
    Prepared(PreparedSubmission),
}

impl SubmissionStateAtStartup {
    /// Constructs a new `SubmissionStateAtStartup` by reading from the given `source`.
    ///
    /// `source` should be a JSON-encoded `State`, and should be writable.
    pub(super) async fn new_from_path<P: AsRef<Path>>(source: P) -> eyre::Result<Self> {
        let file_path = source.as_ref();
        let state_file_path = StateFilePath(file_path.to_path_buf());
        let state = State::read(&state_file_path).await?;
        let temp_file_path = match file_path.extension().and_then(|extn| extn.to_str()) {
            Some(extn) => TempFilePath(file_path.with_extension(format!("{extn}.tmp"))),
            None => TempFilePath(file_path.with_extension("tmp")),
        };

        // Ensure the state can be written.
        state
            .write(&state_file_path, &temp_file_path)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed writing just-read submission state to disk at `{}`",
                    state_file_path.0.display()
                )
            })?;

        match state {
            State::Fresh => Ok(Self::Fresh(FreshSubmission {
                state_file_path,
                temp_file_path,
            })),
            State::Started {
                last_submission,
            } => Ok(Self::Started(StartedSubmission {
                last_submission,
                state_file_path,
                temp_file_path,
            })),
            State::Prepared {
                sequencer_height,
                last_submission,
                blob_tx_hash,
                at,
            } => Ok(Self::Prepared(PreparedSubmission {
                sequencer_height,
                last_submission,
                blob_tx_hash,
                created_at: at,
                state_file_path,
                temp_file_path,
            })),
        }
    }

    /// Returns the sequencer height of the last completed submission, or `None` if the state is
    /// `Fresh`.
    pub(super) fn last_completed_sequencer_height(&self) -> Option<SequencerHeight> {
        match &self {
            SubmissionStateAtStartup::Fresh {
                ..
            } => None,
            SubmissionStateAtStartup::Started(StartedSubmission {
                last_submission, ..
            })
            | SubmissionStateAtStartup::Prepared(PreparedSubmission {
                last_submission, ..
            }) => Some(last_submission.sequencer_height),
        }
    }
}

mod as_number {
    //! Logic to serialize sequencer heights as number, deserialize numbers as sequencer heights.
    //!
    //! This is unfortunately necessary because the [`serde::Serialize`], [`serde::Deserialize`]
    //! implementations for [`tendermint::block::Height`] write the integer as a string, probably
    //! due to tendermint's/cometbft's go-legacy.
    use serde::{
        Deserialize as _,
        Deserializer,
        Serializer,
    };

    use super::SequencerHeight;

    // Allow: the function signature is dictated by the serde(with) attribute.
    #[allow(clippy::trivially_copy_pass_by_ref)]
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
        let height = u64::deserialize(deserializer)?;
        SequencerHeight::try_from(height).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;
    use tempfile::NamedTempFile;

    use super::*;

    const CELESTIA_HEIGHT: u64 = 1234;
    const SEQUENCER_HEIGHT_LOW: u32 = 111;
    const SEQUENCER_HEIGHT_HIGH: u32 = 222;
    const BLOB_TX_HASH_STR: &str =
        "0909090909090909090909090909090909090909090909090909090909090909";
    const BLOB_TX_HASH: BlobTxHash = BlobTxHash::from_raw([9; 32]);
    const AT_STR: &str = "2024-06-24T22:22:22.222222222Z";
    const AT_DURATION_SINCE_EPOCH: Duration = Duration::from_nanos(1_719_267_742_222_222_222);

    #[track_caller]
    fn write(val: &serde_json::Value) -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        serde_json::to_writer(&file, val).unwrap();
        file
    }

    fn write_fresh_state() -> NamedTempFile {
        write(&json!({ "state": "fresh" }))
    }

    fn write_started_state() -> NamedTempFile {
        write(&json!({
            "state": "started",
            "last_submission": {
                "celestia_height": CELESTIA_HEIGHT,
                "sequencer_height": SEQUENCER_HEIGHT_LOW
            }
        }))
    }

    fn write_prepared_state() -> NamedTempFile {
        write(&json!({
            "state": "prepared",
            "sequencer_height": SEQUENCER_HEIGHT_HIGH,
            "last_submission": {
                "celestia_height": CELESTIA_HEIGHT,
                "sequencer_height": SEQUENCER_HEIGHT_LOW
            },
            "blob_tx_hash": BLOB_TX_HASH_STR,
            "at": AT_STR
        }))
    }

    #[tokio::test]
    async fn should_read_fresh_state() {
        let file = write_fresh_state();
        let parsed = State::read(&StateFilePath(file.path().to_path_buf()))
            .await
            .unwrap();
        match parsed {
            State::Fresh => (),
            _ => panic!("expected fresh state, got:\n{parsed:#}"),
        }
    }

    #[tokio::test]
    async fn should_read_started_state() {
        let file = write_started_state();
        let parsed = State::read(&StateFilePath(file.path().to_path_buf()))
            .await
            .unwrap();
        match parsed {
            State::Started {
                last_submission,
            } => {
                let expected_submission = CompletedSubmission::new(
                    CELESTIA_HEIGHT,
                    SequencerHeight::from(SEQUENCER_HEIGHT_LOW),
                );
                assert_eq!(last_submission, expected_submission);
            }
            _ => panic!("expected started state, got:\n{parsed:#}"),
        }
    }

    #[tokio::test]
    async fn should_read_prepared_state() {
        let file = write_prepared_state();
        let parsed = State::read(&StateFilePath(file.path().to_path_buf()))
            .await
            .unwrap();
        match parsed {
            State::Prepared {
                sequencer_height,
                last_submission,
                blob_tx_hash,
                at,
            } => {
                assert_eq!(
                    sequencer_height,
                    SequencerHeight::from(SEQUENCER_HEIGHT_HIGH)
                );
                let expected_submission = CompletedSubmission::new(
                    CELESTIA_HEIGHT,
                    SequencerHeight::from(SEQUENCER_HEIGHT_LOW),
                );
                assert_eq!(last_submission, expected_submission);
                assert_eq!(blob_tx_hash, BLOB_TX_HASH);
                assert_eq!(at, SystemTime::UNIX_EPOCH + AT_DURATION_SINCE_EPOCH);
            }
            _ => panic!("expected prepared state, got:\n{parsed:?}"),
        }
    }

    #[tokio::test]
    async fn should_fail_to_read_missing_state_file() {
        let bad_path = "bad path";
        let error = State::read(&StateFilePath(Path::new(bad_path).to_path_buf()))
            .await
            .unwrap_err();
        let full_error = format!("{error:#}");
        assert!(full_error.contains(bad_path));
        assert!(full_error.contains("failed reading submission state file"));
    }

    #[tokio::test]
    async fn should_fail_to_read_invalid_state_file() {
        let file = write(&json!({ "state": "invalid" }));
        let error = State::read(&StateFilePath(file.path().to_path_buf()))
            .await
            .unwrap_err();
        let full_error = format!("{error:#}");
        assert!(full_error.contains(&file.path().display().to_string()));
        assert!(full_error.contains("failed parsing the contents"));
    }

    #[tokio::test]
    async fn should_fail_to_read_state_file_with_broken_invariant() {
        // The current sequencer height must be greater than the last submission's.
        let file = write(&json!({
            "state": "prepared",
            "sequencer_height": SEQUENCER_HEIGHT_HIGH,
            "last_submission": {
                "celestia_height": CELESTIA_HEIGHT,
                "sequencer_height": SEQUENCER_HEIGHT_HIGH
            },
            "blob_tx_hash": BLOB_TX_HASH_STR,
            "at": AT_STR
        }));
        let error = State::read(&StateFilePath(file.path().to_path_buf()))
            .await
            .unwrap_err();
        let full_error = format!("{error:#}");
        assert!(full_error.contains(&file.path().display().to_string()));
        assert!(full_error.contains("should be greater than last successful submission sequencer"));
    }

    async fn should_write_state(state: State) {
        let tempdir = tempfile::tempdir().unwrap();
        let destination = StateFilePath(tempdir.path().join("state.json"));
        let temp_file = TempFilePath(tempdir.path().join("state.json.tmp"));
        state.write(&destination, &temp_file).await.unwrap();

        let parsed_state = State::read(&destination).await.unwrap();
        assert_eq!(state, parsed_state);
        assert!(!temp_file.0.exists());
    }

    #[tokio::test]
    async fn should_write_fresh_state() {
        should_write_state(State::Fresh).await;
    }

    #[tokio::test]
    async fn should_write_started_state() {
        let last_submission =
            CompletedSubmission::new(CELESTIA_HEIGHT, SequencerHeight::from(SEQUENCER_HEIGHT_LOW));
        should_write_state(State::new_started(last_submission)).await;
    }

    #[tokio::test]
    async fn should_write_prepared_state() {
        let sequencer_height = SequencerHeight::from(SEQUENCER_HEIGHT_HIGH);
        let last_submission =
            CompletedSubmission::new(CELESTIA_HEIGHT, SequencerHeight::from(SEQUENCER_HEIGHT_LOW));
        let at = SystemTime::UNIX_EPOCH + AT_DURATION_SINCE_EPOCH;
        let state = State::new_prepared(sequencer_height, last_submission, BLOB_TX_HASH, at);
        should_write_state(state).await;
    }

    #[tokio::test]
    async fn started_submission_should_transition_to_prepared() {
        let last_submission =
            CompletedSubmission::new(CELESTIA_HEIGHT, SequencerHeight::from(SEQUENCER_HEIGHT_LOW));
        let tempdir = tempfile::tempdir().unwrap();
        let destination = StateFilePath(tempdir.path().join("state.json"));
        let temp_file = TempFilePath(tempdir.path().join("state.json.tmp"));
        let started_submission = StartedSubmission {
            last_submission,
            state_file_path: destination.clone(),
            temp_file_path: temp_file.clone(),
        };

        // Transition to prepared.
        let new_sequencer_height = SequencerHeight::from(SEQUENCER_HEIGHT_HIGH);
        let prepared_submission = started_submission
            .into_prepared(new_sequencer_height, BLOB_TX_HASH)
            .await
            .unwrap();
        assert_eq!(prepared_submission.sequencer_height, new_sequencer_height);
        assert_eq!(prepared_submission.last_submission, last_submission);
        assert_eq!(prepared_submission.blob_tx_hash, BLOB_TX_HASH);
        assert_eq!(prepared_submission.state_file_path.0, destination.0);
        assert_eq!(prepared_submission.temp_file_path.0, temp_file.0);

        // Ensure the new state was written to disk.
        let parsed_state = State::read(&destination).await.unwrap();
        match parsed_state {
            State::Prepared {
                ..
            } => (),
            _ => panic!("expected prepared state, got:\n{parsed_state:?}"),
        }
    }

    #[tokio::test]
    async fn started_submission_should_not_transition_with_broken_invariant() {
        let last_submission =
            CompletedSubmission::new(CELESTIA_HEIGHT, SequencerHeight::from(SEQUENCER_HEIGHT_LOW));
        let tempdir = tempfile::tempdir().unwrap();
        let destination = StateFilePath(tempdir.path().join("state.json"));
        let temp_file = TempFilePath(tempdir.path().join("state.json.tmp"));
        let started_submission = StartedSubmission {
            last_submission,
            state_file_path: destination.clone(),
            temp_file_path: temp_file.clone(),
        };

        // Try to transition to prepared - should fail as new sequencer height == last sequencer
        // height.
        let new_sequencer_height = SequencerHeight::from(SEQUENCER_HEIGHT_LOW);
        let error = started_submission
            .into_prepared(new_sequencer_height, BLOB_TX_HASH)
            .await
            .unwrap_err();
        let full_error = format!("{error:#}");
        assert!(full_error.contains("cannot submit a sequencer block at height below or"));

        // Ensure the new state was not written to disk.
        let error = State::read(&destination).await.unwrap_err();
        let full_error = format!("{error:#}");
        assert!(full_error.contains("failed reading submission state file"));
    }

    #[tokio::test]
    async fn prepared_submission_should_transition_to_started() {
        let sequencer_height = SequencerHeight::from(SEQUENCER_HEIGHT_HIGH);
        let last_submission =
            CompletedSubmission::new(CELESTIA_HEIGHT, SequencerHeight::from(SEQUENCER_HEIGHT_LOW));
        let created_at = SystemTime::UNIX_EPOCH + AT_DURATION_SINCE_EPOCH;
        let tempdir = tempfile::tempdir().unwrap();
        let destination = StateFilePath(tempdir.path().join("state.json"));
        let temp_file = TempFilePath(tempdir.path().join("state.json.tmp"));
        let prepared_submission = PreparedSubmission {
            sequencer_height,
            last_submission,
            blob_tx_hash: BLOB_TX_HASH,
            created_at,
            state_file_path: destination.clone(),
            temp_file_path: temp_file.clone(),
        };

        // Transition to started.
        let new_celestia_height = CELESTIA_HEIGHT + 1;
        let started_submission = prepared_submission
            .into_started(new_celestia_height)
            .await
            .unwrap();
        let expected_last_submission =
            CompletedSubmission::new(new_celestia_height, sequencer_height);
        assert_eq!(started_submission.last_submission, expected_last_submission);
        assert_eq!(started_submission.state_file_path.0, destination.0);
        assert_eq!(started_submission.temp_file_path.0, temp_file.0);

        // Ensure the new state was written to disk.
        let parsed_state = State::read(&destination).await.unwrap();
        match parsed_state {
            State::Started {
                ..
            } => (),
            _ => panic!("expected started state, got:\n{parsed_state:?}"),
        }
    }

    #[tokio::test]
    async fn prepared_submission_should_revert_to_started() {
        let sequencer_height = SequencerHeight::from(SEQUENCER_HEIGHT_HIGH);
        let last_submission =
            CompletedSubmission::new(CELESTIA_HEIGHT, SequencerHeight::from(SEQUENCER_HEIGHT_LOW));
        let created_at = SystemTime::UNIX_EPOCH + AT_DURATION_SINCE_EPOCH;
        let tempdir = tempfile::tempdir().unwrap();
        let destination = StateFilePath(tempdir.path().join("state.json"));
        let temp_file = TempFilePath(tempdir.path().join("state.json.tmp"));
        let prepared_submission = PreparedSubmission {
            sequencer_height,
            last_submission,
            blob_tx_hash: BLOB_TX_HASH,
            created_at,
            state_file_path: destination.clone(),
            temp_file_path: temp_file.clone(),
        };

        // Revert to started - should hold last submission.
        let reverted_submission = prepared_submission.revert().await.unwrap();
        assert_eq!(reverted_submission.last_submission, last_submission);
        assert_eq!(reverted_submission.state_file_path.0, destination.0);
        assert_eq!(reverted_submission.temp_file_path.0, temp_file.0);

        // Ensure the new state was written to disk.
        let parsed_state = State::read(&destination).await.unwrap();
        match parsed_state {
            State::Started {
                ..
            } => (),
            _ => panic!("expected started state, got:\n{parsed_state:?}"),
        }
    }

    #[test]
    fn confirmation_timeout_should_respect_limits() {
        let last_submission =
            CompletedSubmission::new(CELESTIA_HEIGHT, SequencerHeight::from(SEQUENCER_HEIGHT_LOW));
        let mut prepared_submission = PreparedSubmission {
            sequencer_height: SequencerHeight::from(SEQUENCER_HEIGHT_HIGH),
            last_submission,
            blob_tx_hash: BLOB_TX_HASH,
            created_at: SystemTime::UNIX_EPOCH,
            state_file_path: StateFilePath(PathBuf::new()),
            temp_file_path: TempFilePath(PathBuf::new()),
        };

        // With a creation time far in the past, timeout should be 15 seconds.
        assert_eq!(
            prepared_submission.confirmation_timeout(),
            Duration::from_secs(15)
        );

        // With a creation time in the future, timeout should be 60 seconds.
        prepared_submission.created_at = SystemTime::now() + Duration::from_secs(1000);
        assert_eq!(
            prepared_submission.confirmation_timeout(),
            Duration::from_secs(60)
        );
    }

    #[tokio::test]
    async fn should_construct_fresh_submission_state_at_startup() {
        let file = write_fresh_state();
        let parsed = SubmissionStateAtStartup::new_from_path(file.path())
            .await
            .unwrap();
        match parsed {
            SubmissionStateAtStartup::Fresh(FreshSubmission {
                state_file_path,
                temp_file_path,
            }) => {
                assert_eq!(state_file_path.0, file.path());
                assert_eq!(
                    temp_file_path.0.display().to_string(),
                    format!("{}.tmp", file.path().display())
                );
            }
            _ => panic!("expected fresh state, got: {parsed:?}"),
        }
    }

    #[tokio::test]
    async fn should_construct_started_submission_state_at_startup() {
        let file = write_started_state();
        let parsed = SubmissionStateAtStartup::new_from_path(file.path())
            .await
            .unwrap();
        match parsed {
            SubmissionStateAtStartup::Started(StartedSubmission {
                state_file_path,
                temp_file_path,
                ..
            }) => {
                assert_eq!(state_file_path.0, file.path());
                assert_eq!(
                    temp_file_path.0.display().to_string(),
                    format!("{}.tmp", file.path().display())
                );
            }
            _ => panic!("expected started state, got: {parsed:?}"),
        }
    }

    #[tokio::test]
    async fn should_construct_prepared_submission_state_at_startup() {
        let file = write_prepared_state();
        let parsed = SubmissionStateAtStartup::new_from_path(file.path())
            .await
            .unwrap();
        match parsed {
            SubmissionStateAtStartup::Prepared(PreparedSubmission {
                state_file_path,
                temp_file_path,
                ..
            }) => {
                assert_eq!(state_file_path.0, file.path());
                assert_eq!(
                    temp_file_path.0.display().to_string(),
                    format!("{}.tmp", file.path().display())
                );
            }
            _ => panic!("expected prepared state, got: {parsed:?}"),
        }
    }

    #[tokio::test]
    async fn should_fail_to_construct_if_not_writable() {
        let file = write_prepared_state();
        // Create a folder at the path where the temp file would be written.
        std::fs::create_dir(format!("{}.tmp", file.path().display())).unwrap();
        let error = SubmissionStateAtStartup::new_from_path(file.path())
            .await
            .unwrap_err();
        let full_error = format!("{error:#}");
        assert!(full_error.contains(&file.path().display().to_string()));
        assert!(full_error.contains("failed writing just-read submission state to disk at"));
    }
}
