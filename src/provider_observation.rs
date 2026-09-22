//! Retained provider evidence and request receipts for ADR 0075.

use std::fmt;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use serde_json::{json, Value};

pub(crate) mod contracts;
pub(crate) mod http;
pub(crate) mod musicindex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderKind {
    MusicIndex,
    Rss,
}
impl ProviderKind {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::MusicIndex => "musicindex",
            Self::Rss => "rss",
        }
    }
}

/// Exact scoped identity. An item GUID alone cannot construct a track subject.
#[derive(Clone, PartialEq, Eq, serde::Serialize)]
pub struct SubjectKey {
    pub kind: String,
    pub scope_kind: String,
    pub scope: String,
    pub item_guid: Option<String>,
}
impl SubjectKey {
    pub fn guid(feed: &str, item: Option<&str>) -> Self {
        Self {
            kind: if item.is_some() { "track" } else { "feed" }.into(),
            scope_kind: "guid".into(),
            scope: feed.into(),
            item_guid: item.map(str::to_owned),
        }
    }
    pub(crate) fn json(&self) -> Value {
        match &self.item_guid {
            Some(item) => json!([1, self.kind, self.scope_kind, self.scope, item]),
            None => json!([1, self.kind, self.scope_kind, self.scope]),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ProviderRequestSpec {
    pub provider: ProviderKind,
    pub provider_identity: String,
    pub request_uri: String,
    pub requested_subject: Option<SubjectKey>,
    pub requested_parameters: Value,
    pub profile: Value,
    pub started_at_us: i64,
}

#[derive(Clone)]
pub struct RequestToken {
    pub(crate) key: String,
    pub(crate) generation: i64,
    pub(crate) provider_id: i64,
    pub(crate) resource_id: i64,
    pub(crate) subject_id: Option<i64>,
    pub(crate) spec: Arc<ProviderRequestSpec>,
    pub(crate) write_state: Arc<Mutex<RequestWriteState>>,
}
pub(crate) enum RequestWriteState {
    Ready,
    Retry(Arc<ProviderObservation>),
    Committed(Arc<ProviderObservation>, Box<ObservationReceipt>),
    Uncertain,
}
impl PartialEq for RequestToken {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.generation == other.generation
            && self.spec == other.spec
            && Arc::ptr_eq(&self.write_state, &other.write_state)
    }
}
impl Eq for RequestToken {}
impl RequestToken {
    pub fn generation(&self) -> i64 {
        self.generation
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationOutcome {
    Success,
    Partial,
    Failed,
}
impl ObservationOutcome {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Partial => "partial",
            Self::Failed => "failed",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropertyPresence {
    Missing,
    Null,
    Empty,
    Populated,
    Invalid,
}
impl PropertyPresence {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Null => "null",
            Self::Empty => "empty",
            Self::Populated => "populated",
            Self::Invalid => "invalid",
        }
    }
    pub(crate) fn from_json(value: Option<&Value>, collection: bool) -> Self {
        match value {
            None => Self::Missing,
            Some(Value::Null) => Self::Null,
            Some(Value::Array(values)) if collection => {
                if values.is_empty() {
                    Self::Empty
                } else {
                    Self::Populated
                }
            }
            Some(_) if collection => Self::Invalid,
            Some(Value::String(s)) if s.is_empty() => Self::Empty,
            Some(_) => Self::Populated,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationRetention {
    Unknown,
    Partial,
    Failed,
    Repeated,
    SupersededAttempt,
}
impl ObservationRetention {
    pub(crate) fn completeness(self) -> &'static str {
        match self {
            Self::Partial => "partial",
            Self::Failed => "failed",
            _ => "unknown",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct CoverageEvidence {
    pub collection: String,
    pub target: Option<SubjectKey>,
    pub target_owner: Value,
    pub request_intent: String,
    pub presence: PropertyPresence,
    pub retention: ObservationRetention,
    pub basis: Value,
    pub facts: Vec<FactEvidence>,
    pub(crate) proof: Option<contracts::VerifiedCoverage>,
}
#[derive(Clone, PartialEq, Eq, serde::Serialize)]
pub struct FactEvidence {
    pub subject: Option<SubjectKey>,
    pub declared_owner: Value,
    pub owner_basis: Value,
    pub kind: String,
    pub assertion_source: Option<String>,
    pub source_position: Option<i64>,
    pub extraction_path: Option<String>,
    pub source_observed: Option<Value>,
    pub representation: String,
    pub validation: String,
    pub value: Value,
    pub raw_member: Option<Value>,
    pub body_locator: Value,
}
#[derive(Clone, PartialEq, Eq)]
pub struct ProviderObservation {
    pub body: Option<Arc<[u8]>>,
    pub http_status: Option<u16>,
    pub response_uri: Option<String>,
    pub interpretation: Value,
    pub source_revision: Option<Value>,
    pub source_times: Value,
    pub decoder_version: String,
    pub outcome: ObservationOutcome,
    pub failure: Option<Value>,
    pub finished_at_us: i64,
    pub fetched_at_us: Option<i64>,
    pub occurrence: Value,
    pub coverage: Vec<CoverageEvidence>,
}
impl ProviderObservation {
    pub(crate) fn fail(&mut self, reason: &'static str) {
        self.outcome = ObservationOutcome::Failed;
        self.failure = Some(json!({"reason": reason}));
    }
}

#[must_use]
#[derive(Clone, PartialEq, Eq)]
pub struct ObservationReceipt {
    pub observation_id: i64,
    pub generation: i64,
    pub provider_id: i64,
    pub resource_id: i64,
    pub request_uri: String,
    pub response_uri: Option<String>,
    pub body_key: Option<String>,
    pub started_at_us: i64,
    pub finished_at_us: i64,
    pub fetched_at_us: Option<i64>,
    pub occurrence: Value,
    pub outcome: ObservationOutcome,
    pub retention: ObservationRetention,
    pub collections: Vec<CollectionOutcome>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionRetention {
    KeptUnknown,
    KeptPartial,
    KeptFailed,
    RejectedSuperseded,
    ReplacedEmpty,
    ReplacedPopulated,
    Repeated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectionOutcome {
    pub provider_id: i64,
    pub subject: Option<SubjectKey>,
    pub collection: String,
    pub scope_ordinal: usize,
    pub retention: CollectionRetention,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProviderTrackState {
    pub collections: Vec<ProviderCollection>,
    pub request_refresh: Option<RequestRefresh>,
    pub binding: ProviderBinding,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProviderBinding {
    #[default]
    Unavailable,
    RequestOnly,
    Proven,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderCollection {
    pub provider_id: i64,
    pub subject: SubjectKey,
    pub collection: String,
    pub state: CollectionState,
    pub refresh: Option<CollectionRefresh>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CollectionState {
    NoSnapshot,
    CompleteEmpty(AcceptedSnapshot),
    CompletePopulated(AcceptedSnapshot),
}

#[derive(Clone, PartialEq, Eq)]
pub struct AcceptedSnapshot {
    pub snapshot_id: i64,
    pub generation: i64,
    pub observation_id: i64,
    pub scope_ordinal: i64,
    pub fetched_at_us: Option<i64>,
    pub occurrence: Value,
    pub members: Vec<StoredFact>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredFact {
    pub fact_id: i64,
    pub observation_id: i64,
    pub scope_ordinal: i64,
    pub transport_ordinal: i64,
    pub evidence: FactEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectionRefresh {
    pub generation: i64,
    pub state: RefreshState,
    pub observation_id: Option<i64>,
    pub failure_id: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshState {
    None,
    Pending,
    Success,
    Partial,
    Failed,
    Superseded,
    Abandoned,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestRefresh {
    pub request: ProviderRequestSpec,
    pub generation: i64,
    pub state: RefreshState,
    pub observation_id: Option<i64>,
    pub failure_id: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationStorageError {
    RequestAllocation,
    ResponseWrite,
}
impl fmt::Display for ObservationStorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self { Self::RequestAllocation => "Metadata storage could not allocate the request. The request did not run.", Self::ResponseWrite => "Metadata storage could not confirm response retention. The response remains in this Library session." })
    }
}
impl std::error::Error for ObservationStorageError {}

#[derive(Clone, PartialEq, Eq)]
pub struct ObservationWriteFailure {
    pub token: RequestToken,
    pub observation: Arc<ProviderObservation>,
    pub operation: ObservationStorageError,
    pub retry: StorageRetry,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageRetry {
    VerifiedRollback,
    Blocked,
}

/// Every failed command retains receipts for requests that committed earlier.
#[derive(Clone, PartialEq, Eq)]
pub struct ObservationCommandFailure {
    pub write_failure: Option<Arc<ObservationWriteFailure>>,
    pub storage_error: Option<ObservationStorageError>,
    pub receipts: Arc<[ObservationReceipt]>,
    pub read_error: Option<ProviderReadError>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderReadError {
    Storage,
}
impl fmt::Display for ObservationCommandFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(failure) = &self.write_failure {
            return failure.fmt(f);
        }
        if let Some(error) = self.storage_error {
            return error.fmt(f);
        }
        if self.read_error.is_some() {
            return f.write_str("Metadata storage could not read provider state. Committed observations remain stored.");
        }
        if self.receipts.is_empty() {
            f.write_str("The metadata query failed.")
        } else {
            f.write_str("The metadata query failed. Earlier observations remain stored.")
        }
    }
}
impl std::error::Error for ObservationCommandFailure {}
impl fmt::Display for ObservationWriteFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.operation.fmt(f)
    }
}
impl std::error::Error for ObservationWriteFailure {}

/// The selected command owns this recorder and consumes all committed receipts.
pub struct ProviderObservationRecorder {
    conn: Arc<Mutex<Connection>>,
    receipts: Mutex<Vec<ObservationReceipt>>,
}
impl ProviderObservationRecorder {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            conn,
            receipts: Mutex::new(Vec::new()),
        }
    }
    pub fn begin(
        &self,
        spec: ProviderRequestSpec,
    ) -> Result<RequestToken, ObservationStorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ObservationStorageError::RequestAllocation)?;
        crate::db::provider_observations::begin_provider_request(&conn, spec)
    }
    /// Writes one observation and returns its receipt.
    ///
    /// The receipt also stays in this recorder, for `take_receipts`. A
    /// caller that retains a response for reuse keeps the returned receipt
    /// and replays it later through `replay` (ADR 0075 packet 018,
    /// R18B-07).
    pub fn record(
        &self,
        token: RequestToken,
        observation: ProviderObservation,
    ) -> Result<ObservationReceipt, ObservationWriteFailure> {
        let observation = Arc::new(observation);
        let mut receipts = self.receipts.lock().map_err(|_| ObservationWriteFailure {
            token: token.clone(),
            observation: Arc::clone(&observation),
            operation: ObservationStorageError::ResponseWrite,
            retry: StorageRetry::Blocked,
        })?;
        let conn = self.conn.lock().map_err(|_| ObservationWriteFailure {
            token: token.clone(),
            observation: Arc::clone(&observation),
            operation: ObservationStorageError::ResponseWrite,
            retry: StorageRetry::Blocked,
        })?;
        let receipt = crate::db::provider_observations::record_provider_observation(
            &conn,
            token,
            observation,
        )?;
        receipts.push(receipt.clone());
        Ok(receipt)
    }

    /// Adds the receipt of an earlier observation to this recorder,
    /// without a new write.
    ///
    /// A reused response names the observation that produced it, and it
    /// creates no second observation of the same fetch. ADR 0075 packet
    /// 018, R18B-07.
    pub fn replay(&self, receipt: ObservationReceipt) {
        self.receipts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(receipt);
    }
    pub fn take_receipts(&self) -> Vec<ObservationReceipt> {
        std::mem::take(
            &mut *self
                .receipts
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }
}

pub(crate) fn propagate_storage_failure<T>(result: anyhow::Result<T>) -> anyhow::Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error)
            if error.is::<ObservationWriteFailure>() || error.is::<ObservationStorageError>() =>
        {
            Err(error)
        }
        Err(_) => Ok(None),
    }
}

macro_rules! redacted_debug {
    ($($name:ty),+ $(,)?) => { $(impl fmt::Debug for $name {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_struct(stringify!($name)).finish_non_exhaustive() }
    })+ };
}
redacted_debug!(
    SubjectKey,
    ProviderRequestSpec,
    RequestToken,
    CoverageEvidence,
    FactEvidence,
    ProviderObservation,
    ObservationReceipt,
    ProviderObservationRecorder,
    ObservationCommandFailure,
    AcceptedSnapshot
);
impl fmt::Debug for ObservationWriteFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObservationWriteFailure")
            .field("generation", &self.token.generation)
            .field("operation", &self.operation)
            .finish_non_exhaustive()
    }
}
