//! Shared command error channel.

use std::fmt;
use std::sync::Arc;

use crate::provider_observation::{
    ObservationCommandFailure, ObservationReceipt, ObservationStorageError, ObservationWriteFailure,
};

/// ADR 0075 preserves committed receipts when ordinary query work fails.
#[derive(Clone, Eq, PartialEq)]
pub struct ObservedQueryFailure {
    message: String,
    receipts: Arc<[ObservationReceipt]>,
}

impl ObservedQueryFailure {
    pub(crate) fn new(message: String, receipts: Vec<ObservationReceipt>) -> Self {
        Self {
            message,
            receipts: receipts.into(),
        }
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }

    pub(crate) fn receipts(&self) -> &[ObservationReceipt] {
        &self.receipts
    }
}

impl fmt::Debug for ObservedQueryFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObservedQueryFailure")
            .finish_non_exhaustive()
    }
}

/// ADR 0075 preserves committed receipts when an ordinary command fails.
///
/// The wrapper keeps the original typed cause, so display, error detail and
/// cancellation inspection stay equal to the unobserved failure.
#[derive(Clone, Eq, PartialEq)]
pub struct ObservedCommandFailure {
    cause: Arc<CommandError>,
    receipts: Arc<[ObservationReceipt]>,
}

impl ObservedCommandFailure {
    /// Returns the original command failure.
    pub(crate) fn cause(&self) -> &CommandError {
        &self.cause
    }

    /// Returns the receipts that committed before the failure.
    pub(crate) fn receipts(&self) -> &[ObservationReceipt] {
        &self.receipts
    }
}

impl fmt::Debug for ObservedCommandFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObservedCommandFailure")
            .finish_non_exhaustive()
    }
}

/// ADR 0075 classifies a provider storage failure before any ordinary error.
///
/// The caller returns the typed storage variant for a result of `Some`, and
/// keeps its own ordinary error family otherwise.
pub(crate) fn observation_storage_failure(
    error: &anyhow::Error,
) -> Option<ObservationCommandFailure> {
    if let Some(failure) = error.downcast_ref::<ObservationCommandFailure>() {
        return Some(failure.clone());
    }
    let write_failure = error
        .downcast_ref::<ObservationWriteFailure>()
        .cloned()
        .map(Arc::new);
    let storage_error = error.downcast_ref::<ObservationStorageError>().copied();
    (write_failure.is_some() || storage_error.is_some()).then_some(ObservationCommandFailure {
        write_failure,
        storage_error,
        receipts: Arc::from([]),
        read_error: None,
    })
}

/// ADR 0075 attaches committed receipts to a failed command or a cancellation.
///
/// An empty receipt list returns the original error. An existing observation
/// wrapper keeps its error family and its earlier receipts.
pub(crate) fn attach_observation_receipts(
    error: CommandError,
    receipts: Vec<ObservationReceipt>,
) -> CommandError {
    if receipts.is_empty() {
        return error;
    }
    match error {
        CommandError::ObservationWriteFailure(failure) => {
            CommandError::ObservationWriteFailure(Arc::new(ObservationCommandFailure {
                write_failure: failure.write_failure.clone(),
                storage_error: failure.storage_error,
                read_error: failure.read_error,
                receipts: merged_receipts(&failure.receipts, receipts),
            }))
        }
        CommandError::ObservedQueryFailure(failure) => {
            CommandError::ObservedQueryFailure(Arc::new(ObservedQueryFailure {
                message: failure.message.clone(),
                receipts: merged_receipts(&failure.receipts, receipts),
            }))
        }
        CommandError::ObservedCommandFailure(failure) => {
            CommandError::ObservedCommandFailure(Arc::new(ObservedCommandFailure {
                cause: Arc::clone(&failure.cause),
                receipts: merged_receipts(&failure.receipts, receipts),
            }))
        }
        cause => CommandError::ObservedCommandFailure(Arc::new(ObservedCommandFailure {
            cause: Arc::new(cause),
            receipts: receipts.into(),
        })),
    }
}

fn merged_receipts(
    committed: &[ObservationReceipt],
    later: Vec<ObservationReceipt>,
) -> Arc<[ObservationReceipt]> {
    let mut merged = committed.to_vec();
    merged.extend(later);
    merged.into()
}

/// Error returned by command execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandError {
    /// The old app session no longer admits work.
    SessionDraining(u64),
    /// An execution dependency is unavailable; the remedy remains callable.
    Unavailable(crate::application::capability::ExecutionUnavailable),
    /// Playlist command failed.
    Playlist(String),
    /// Feed command failed.
    Feed(String),
    /// Download command failed.
    Download(String),
    /// Metadata command failed.
    Metadata(String),
    /// Playback command failed.
    Playback(String),
    /// Query refresh required by a command failed.
    Query(String),
    /// ADR 0075 preserves an ordinary query failure and its committed receipts.
    ObservedQueryFailure(Arc<ObservedQueryFailure>),
    /// ADR 0075 preserves an ordinary command failure and its committed receipts.
    ObservedCommandFailure(Arc<ObservedCommandFailure>),
    /// ADR 0075 retains response input for a later explicit storage retry.
    ObservationWriteFailure(std::sync::Arc<crate::provider_observation::ObservationCommandFailure>),
    /// Command was cancelled before completion.
    Cancelled,
    /// Command failed outside a narrower family.
    Other(String),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(reason) => reason.fmt(f),
            Self::SessionDraining(generation) => write!(f, "App session {generation} is ending; the command did not run. Open a fresh session after maintenance."),
            Self::Playlist(message) => write!(f, "playlist command failed: {message}"),
            Self::Feed(message) => write!(f, "feed command failed: {message}"),
            Self::Download(message) => write!(f, "download command failed: {message}"),
            Self::Metadata(message) => write!(f, "metadata command failed: {message}"),
            Self::Playback(message) => write!(f, "playback command failed: {message}"),
            Self::Query(message) => write!(f, "query refresh failed: {message}"),
            Self::ObservedQueryFailure(failure) => write!(f, "query refresh failed: {}", failure.message()),
            Self::ObservedCommandFailure(failure) => failure.cause().fmt(f),
            Self::ObservationWriteFailure(failure) => failure.fmt(f),
            Self::Cancelled => f.write_str("command cancelled"),
            Self::Other(message) => write!(f, "command failed: {message}"),
        }
    }
}

impl std::error::Error for CommandError {}
