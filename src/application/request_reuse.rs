//! Shared MusicIndex request identity and active-request sharing.
//!
//! ADR 0075 section 6 and packet 018 Part A. A caller asks this owner for a
//! MusicIndex resource instead of calling `api::Client` on its own. The
//! owner sends at most one request for one identity at one time. A second
//! caller with the same identity joins the request in flight and gets the
//! same result.
//!
//! Part A holds active requests only. It keeps no completed response for
//! later reuse, and it retains no data across a restart. Packet 018 Part B
//! adds a reuse window for completed responses.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};

use crate::api::{Feed, Track};
use crate::provider_observation::{ObservationStorageError, ObservationWriteFailure};

/// One shared MusicIndex request identity: the endpoint, the scoped
/// subject, and the include list that names the packet 017 request
/// profile.
///
/// This identity must name the same request as the storage layer's own
/// identity, built by `db::provider_observations::request_identity` from
/// one `ProviderRequestSpec`. That function stays private to its module,
/// so it cannot be called from here. `request_reuse_tests` proves
/// agreement against the real storage identity a different way: it drives
/// both the in-memory key and a real `begin_provider_request` call from
/// the same logical request, and checks that both treat two requests as
/// the same identity, or as different identities, together.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct RequestKey {
    provider_identity: String,
    subject: RequestSubject,
    include: Option<&'static str>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum RequestSubject {
    Feed(String),
    ScopedTrack(String, String),
    UnscopedTrack(String),
}

impl RequestKey {
    /// The identity of a feed request.
    pub(crate) fn feed(
        provider_identity: impl Into<String>,
        feed_guid: &str,
        include: Option<&'static str>,
    ) -> Self {
        Self {
            provider_identity: provider_identity.into(),
            subject: RequestSubject::Feed(feed_guid.to_owned()),
            include,
        }
    }

    /// The identity of a track request scoped to its feed.
    pub(crate) fn scoped_track(
        provider_identity: impl Into<String>,
        feed_guid: &str,
        track_guid: &str,
        include: Option<&'static str>,
    ) -> Self {
        Self {
            provider_identity: provider_identity.into(),
            subject: RequestSubject::ScopedTrack(feed_guid.to_owned(), track_guid.to_owned()),
            include,
        }
    }

    /// The identity of a track request with no feed scope.
    pub(crate) fn unscoped_track(
        provider_identity: impl Into<String>,
        track_guid: &str,
        include: Option<&'static str>,
    ) -> Self {
        Self {
            provider_identity: provider_identity.into(),
            subject: RequestSubject::UnscopedTrack(track_guid.to_owned()),
            include,
        }
    }
}

/// Whether a request may join an active request, or must bypass reuse.
///
/// ADR 0075 section 6 and packet 018 P18-7.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RefreshIntent {
    /// A passive read. It may join an active request, started under either
    /// intent.
    Normal,
    /// An explicit refresh. It always sends a new request. It never joins
    /// an active request that started without this intent (packet 018
    /// R18A-09).
    Explicit,
}

/// A fetch failure shared between a winning caller and every caller that
/// joins its request.
///
/// The original `anyhow::Error` is not `Clone`, so every caller of a
/// shared request receives this wrapper instead. `into_anyhow` restores an
/// `ObservationWriteFailure` or an `ObservationStorageError` to its own
/// downcastable type, because `provider_observation::propagate_storage_failure`
/// and the command layer classify a request failure by that type.
#[derive(Clone)]
pub(crate) struct SharedFetchError(Arc<anyhow::Error>);

impl SharedFetchError {
    /// Converts back to `anyhow::Error`.
    ///
    /// A storage-classified failure keeps its own type, so a caller can
    /// still downcast it. Every other failure becomes a plain error that
    /// carries the same display text.
    pub(crate) fn into_anyhow(self) -> anyhow::Error {
        if let Some(failure) = self.0.downcast_ref::<ObservationWriteFailure>() {
            return anyhow::Error::new(failure.clone());
        }
        if let Some(failure) = self.0.downcast_ref::<ObservationStorageError>() {
            return anyhow::Error::new(*failure);
        }
        anyhow::anyhow!("{self}")
    }
}

impl std::fmt::Display for SharedFetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::fmt::Debug for SharedFetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl std::error::Error for SharedFetchError {}

impl From<anyhow::Error> for SharedFetchError {
    fn from(error: anyhow::Error) -> Self {
        Self(Arc::new(error))
    }
}

/// One in-flight or just-finished request for one identity.
struct Slot<T> {
    /// The sequence value this slot's owner allocated when the request
    /// started. It is not a fetch time and not a source time (R18A-07).
    generation: i64,
    state: Mutex<SlotState<T>>,
    ready: Condvar,
}

enum SlotState<T> {
    Pending,
    Done(Result<T, SharedFetchError>),
}

impl<T: Clone> Slot<T> {
    /// Blocks until the request finishes, then returns its result.
    ///
    /// This never locks the shared registry, so it cannot deadlock against
    /// the caller that is performing the request.
    fn wait(&self) -> Result<T, SharedFetchError> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        loop {
            match &*state {
                SlotState::Done(result) => return result.clone(),
                SlotState::Pending => {
                    state = self
                        .ready
                        .wait(state)
                        .unwrap_or_else(PoisonError::into_inner);
                }
            }
        }
    }
}

type Registry<T> = Mutex<HashMap<RequestKey, Arc<Slot<T>>>>;

/// Completes an abandoned slot when the requesting thread unwinds.
///
/// A panic in the request closure would otherwise leave the slot
/// `Pending`. Each joined caller would then wait without end, and the
/// identity would stay in the registry. This guard fails the slot, wakes
/// every joined caller, and removes the identity, so that a later caller
/// starts a new request.
struct SlotCompletion<'a, T> {
    registry: &'a Registry<T>,
    key: &'a RequestKey,
    slot: &'a Arc<Slot<T>>,
    finished: bool,
}

impl<T> Drop for SlotCompletion<'_, T> {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        {
            let mut state = self
                .slot
                .state
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if matches!(*state, SlotState::Pending) {
                *state = SlotState::Done(Err(SharedFetchError::from(anyhow::anyhow!(
                    "the MusicIndex request stopped before it finished"
                ))));
            }
        }
        self.slot.ready.notify_all();
        let mut guard = self.registry.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(current) = guard.get(self.key) {
            if Arc::ptr_eq(current, self.slot) {
                guard.remove(self.key);
            }
        }
    }
}

/// Sends or joins one request for one identity.
///
/// The caller-supplied `fetch` closure runs only for the caller that wins
/// the identity (a cache miss, or an explicit refresh). It never runs
/// twice for one active identity, and it never runs while the registry
/// lock is held: the lock is released before `fetch` starts and is
/// re-acquired only after `fetch` returns.
fn single_flight<T: Clone>(
    registry: &Registry<T>,
    sequence: &AtomicI64,
    key: RequestKey,
    refresh: RefreshIntent,
    fetch: impl FnOnce() -> Result<T, SharedFetchError>,
) -> (Result<T, SharedFetchError>, i64) {
    let mut guard = registry.lock().unwrap_or_else(PoisonError::into_inner);
    if refresh == RefreshIntent::Normal {
        if let Some(existing) = guard.get(&key) {
            let slot = Arc::clone(existing);
            drop(guard);
            let generation = slot.generation;
            return (slot.wait(), generation);
        }
    }

    // A cache miss, or an explicit refresh: this caller starts the request.
    // The sequence value is allocated here, when the request starts, not
    // when it finishes (R18A-07).
    let generation = sequence.fetch_add(1, Ordering::SeqCst) + 1;
    let slot = Arc::new(Slot {
        generation,
        state: Mutex::new(SlotState::Pending),
        ready: Condvar::new(),
    });
    guard.insert(key.clone(), Arc::clone(&slot));
    drop(guard);

    let mut completion = SlotCompletion {
        registry,
        key: &key,
        slot: &slot,
        finished: false,
    };
    let result = fetch();
    {
        let mut state = slot.state.lock().unwrap_or_else(PoisonError::into_inner);
        *state = SlotState::Done(result.clone());
    }
    completion.finished = true;
    slot.ready.notify_all();

    // Guarded cleanup: remove this slot only while it is still the
    // registry's current entry for this identity. An explicit refresh can
    // supersede an older active slot for the same identity (R18A-09); that
    // older slot's later completion must not remove or replace the newer
    // one (R18A-06).
    let mut guard = registry.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(current) = guard.get(&key) {
        if Arc::ptr_eq(current, &slot) {
            guard.remove(&key);
        }
    }
    (result, generation)
}

/// Holds active MusicIndex requests and shares them.
///
/// One owner serves the whole process (packet 018 P18-6: memory only, a
/// restart clears it). Production call sites reach it through `shared`.
/// Tests construct their own instance so their active requests cannot
/// cross into another test.
pub(crate) struct MetadataRequestOwner {
    sequence: AtomicI64,
    feeds: Registry<Feed>,
    tracks: Registry<Track>,
    feeds_with_receipts: Registry<(Feed, Vec<crate::provider_observation::ObservationReceipt>)>,
}

impl MetadataRequestOwner {
    pub(crate) fn new() -> Self {
        Self {
            sequence: AtomicI64::new(0),
            feeds: Mutex::new(HashMap::new()),
            tracks: Mutex::new(HashMap::new()),
            feeds_with_receipts: Mutex::new(HashMap::new()),
        }
    }

    /// Sends or joins a feed request.
    ///
    /// Returns the result and the sequence value of the request this
    /// caller ended up with (its own, if it started one, or the one it
    /// joined).
    pub(crate) fn fetch_feed(
        &self,
        key: RequestKey,
        refresh: RefreshIntent,
        fetch: impl FnOnce() -> anyhow::Result<Feed>,
    ) -> (Result<Feed, SharedFetchError>, i64) {
        single_flight(&self.feeds, &self.sequence, key, refresh, || {
            fetch().map_err(SharedFetchError::from)
        })
    }

    /// Sends or joins a track request.
    pub(crate) fn fetch_track(
        &self,
        key: RequestKey,
        refresh: RefreshIntent,
        fetch: impl FnOnce() -> anyhow::Result<Track>,
    ) -> (Result<Track, SharedFetchError>, i64) {
        single_flight(&self.tracks, &self.sequence, key, refresh, || {
            fetch().map_err(SharedFetchError::from)
        })
    }

    /// Sends or joins a feed request whose caller also wants the
    /// observation receipts that request produced (packet 018 R18A-05).
    ///
    /// The `fetch` closure runs only for the winning caller. It reads the
    /// feed and returns its receipts together, so a joining caller
    /// receives the exact receipts of that one request and records no
    /// second observation.
    pub(crate) fn fetch_feed_with_receipts(
        &self,
        key: RequestKey,
        refresh: RefreshIntent,
        fetch: impl FnOnce() -> anyhow::Result<(
            Feed,
            Vec<crate::provider_observation::ObservationReceipt>,
        )>,
    ) -> (
        Result<(Feed, Vec<crate::provider_observation::ObservationReceipt>), SharedFetchError>,
        i64,
    ) {
        single_flight(
            &self.feeds_with_receipts,
            &self.sequence,
            key,
            refresh,
            || fetch().map_err(SharedFetchError::from),
        )
    }

    #[cfg(test)]
    fn active_feed_count(&self) -> usize {
        self.feeds
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }
}

static SHARED: OnceLock<MetadataRequestOwner> = OnceLock::new();

/// The one owner shared by this process.
pub(crate) fn shared() -> &'static MetadataRequestOwner {
    SHARED.get_or_init(MetadataRequestOwner::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::thread;
    use std::time::Duration;

    fn endpoint_identity(port_marker: &str) -> String {
        format!("https://musicindex.invalid.test/{port_marker}")
    }

    fn sample_feed(feed_guid: &str) -> Feed {
        Feed {
            feed_guid: Some(feed_guid.to_owned()),
            ..Feed::default()
        }
    }

    /// R18A-01: a key holds the endpoint, the scoped subject, and the
    /// profile. Two requests with different endpoints have different keys.
    #[test]
    fn adr_0075_request_reuse_key_differs_by_endpoint_subject_and_profile() {
        let a = RequestKey::feed("https://a.invalid", "f1", Some("include-a"));
        let b = RequestKey::feed("https://b.invalid", "f1", Some("include-a"));
        assert_ne!(a, b, "different endpoints must produce different keys");

        let c = RequestKey::feed("https://a.invalid", "f2", Some("include-a"));
        assert_ne!(a, c, "different subjects must produce different keys");

        let d = RequestKey::feed("https://a.invalid", "f1", Some("include-b"));
        assert_ne!(a, d, "different profiles must produce different keys");

        let e = RequestKey::feed("https://a.invalid", "f1", Some("include-a"));
        assert_eq!(a, e, "identical endpoint, subject and profile must agree");

        let scoped = RequestKey::scoped_track("https://a.invalid", "f1", "t1", Some("include-a"));
        let unscoped = RequestKey::unscoped_track("https://a.invalid", "t1", Some("include-a"));
        assert_ne!(
            scoped, unscoped,
            "a scoped and an unscoped track request are different identities"
        );
    }

    /// R18A-02: the in-memory key and the storage identity agree. This
    /// test builds a `ProviderRequestSpec` the same way
    /// `api::Client::get_observed_json` does, and drives the real storage
    /// identity through the public `begin_provider_request`, because
    /// `db::provider_observations::request_identity` is private to its
    /// module. Two specs that share their endpoint, subject and profile
    /// must produce one identity in both places; a spec that differs in
    /// one of those fields must produce a different identity in both
    /// places. `started_at_us` is not part of the identity in either
    /// place.
    #[test]
    fn adr_0075_request_reuse_key_agrees_with_storage_identity() {
        use crate::db;
        use crate::provider_observation::{ProviderKind, ProviderRequestSpec, SubjectKey};
        use rusqlite::Connection;
        use serde_json::json;

        fn spec(started_at_us: i64, feed_guid: &str, include: &str) -> ProviderRequestSpec {
            let path = ["v1", "feeds", feed_guid];
            let query = [("include", include.to_string())];
            ProviderRequestSpec {
                provider: ProviderKind::MusicIndex,
                provider_identity: "https://musicindex.invalid.test".into(),
                request_uri: format!(
                    "https://musicindex.invalid.test/v1/feeds/{feed_guid}?include={include}"
                ),
                requested_subject: Some(SubjectKey::guid(feed_guid, None)),
                requested_parameters: json!({"path": path, "query": query}),
                profile: json!({"version": 1, "path": path, "query": query}),
                started_at_us,
            }
        }

        let conn = Connection::open_in_memory().unwrap();
        db::upgrades::create_fixture(&conn, 12).unwrap();

        let storage_key = |spec: ProviderRequestSpec| {
            db::provider_observations::begin_provider_request(&conn, spec)
                .unwrap()
                .key
                .clone()
        };

        let spec_a = spec(100, "f1", "source_links");
        let spec_a_again = spec(999, "f1", "source_links"); // only the fetch time differs
        let spec_b_guid = spec(100, "f2", "source_links");
        let spec_b_profile = spec(100, "f1", "source_ids");

        let key_a = RequestKey::feed(spec_a.provider_identity.clone(), "f1", Some("source_links"));
        let key_a_again = RequestKey::feed(
            spec_a_again.provider_identity.clone(),
            "f1",
            Some("source_links"),
        );
        let key_b_guid = RequestKey::feed(
            spec_b_guid.provider_identity.clone(),
            "f2",
            Some("source_links"),
        );
        let key_b_profile = RequestKey::feed(
            spec_b_profile.provider_identity.clone(),
            "f1",
            Some("source_ids"),
        );

        let storage_a = storage_key(spec_a);
        let storage_a_again = storage_key(spec_a_again);
        let storage_b_guid = storage_key(spec_b_guid);
        let storage_b_profile = storage_key(spec_b_profile);

        assert_eq!(
            key_a, key_a_again,
            "the in-memory key ignores the fetch time"
        );
        assert_eq!(
            storage_a, storage_a_again,
            "the storage identity ignores the fetch time"
        );

        assert_ne!(key_a, key_b_guid);
        assert_ne!(storage_a, storage_b_guid);
        assert_ne!(key_a, key_b_profile);
        assert_ne!(storage_a, storage_b_profile);

        // The two computations agree on every case above: same in-memory
        // key exactly when same storage identity, for all four specs.
        let cases = [
            (&key_a, &storage_a),
            (&key_a_again, &storage_a_again),
            (&key_b_guid, &storage_b_guid),
            (&key_b_profile, &storage_b_profile),
        ];
        for (i, (key_i, storage_i)) in cases.iter().enumerate() {
            for (j, (key_j, storage_j)) in cases.iter().enumerate() {
                assert_eq!(
                    key_i == key_j,
                    storage_i == storage_j,
                    "case {i} vs {j}: the in-memory key and the storage identity disagree"
                );
            }
        }
    }

    /// A panic in the request closure must not leave a joined caller
    /// waiting. The abandoned slot fails, it wakes its joined caller, and
    /// it leaves the registry so that a later caller starts a new request.
    #[test]
    fn adr_0075_request_reuse_abandoned_request_releases_a_joined_caller() {
        let owner = MetadataRequestOwner::new();
        let release = Arc::new((Mutex::new(false), Condvar::new()));
        let key = RequestKey::feed(endpoint_identity("abandoned"), "f1", None);

        thread::scope(|scope| {
            let owner = &owner;
            let release_a = Arc::clone(&release);
            let key_a = key.clone();
            let first = scope.spawn(move || {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    owner.fetch_feed(key_a, RefreshIntent::Normal, || {
                        let (lock, condvar) = &*release_a;
                        let mut ready = lock.lock().unwrap();
                        while !*ready {
                            ready = condvar.wait(ready).unwrap();
                        }
                        panic!("the request thread stopped");
                    })
                }))
            });

            // Give the joined caller time to start its wait.
            thread::sleep(Duration::from_millis(50));
            let key_b = key.clone();
            let second = scope.spawn(move || {
                owner.fetch_feed(key_b, RefreshIntent::Normal, || Ok(sample_feed("f1")))
            });

            thread::sleep(Duration::from_millis(50));
            {
                let (lock, condvar) = &*release;
                let mut ready = lock.lock().unwrap();
                *ready = true;
                condvar.notify_all();
            }

            assert!(
                first.join().unwrap().is_err(),
                "the requesting thread must unwind"
            );
            let (joined, _) = second.join().unwrap();
            assert!(
                joined.is_err(),
                "an abandoned request must release its joined caller"
            );
        });

        let (later, _) = owner.fetch_feed(key, RefreshIntent::Normal, || Ok(sample_feed("f1")));
        assert!(
            later.is_ok(),
            "an abandoned identity must leave the registry"
        );
    }

    /// R18A-03: two concurrent callers with one identity produce one HTTP
    /// request (represented here by one closure invocation). Both receive
    /// the same result.
    #[test]
    fn adr_0075_request_reuse_concurrent_callers_share_one_identity() {
        let owner = MetadataRequestOwner::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let release = Arc::new((Mutex::new(false), Condvar::new()));

        let key = RequestKey::feed(endpoint_identity("shared"), "f1", None);
        thread::scope(|scope| {
            let owner = &owner;
            let calls_a = Arc::clone(&calls);
            let release_a = Arc::clone(&release);
            let key_a = key.clone();
            let first = scope.spawn(move || {
                owner.fetch_feed(key_a, RefreshIntent::Normal, || {
                    calls_a.fetch_add(1, Ordering::SeqCst);
                    let (lock, condvar) = &*release_a;
                    let mut ready = lock.lock().unwrap();
                    while !*ready {
                        ready = condvar.wait(ready).unwrap();
                    }
                    Ok(sample_feed("f1"))
                })
            });

            // Give the first caller time to register its active request
            // before the second caller asks.
            thread::sleep(Duration::from_millis(50));
            let calls_b = Arc::clone(&calls);
            let key_b = key.clone();
            let second = scope.spawn(move || {
                owner.fetch_feed(key_b, RefreshIntent::Normal, || {
                    calls_b.fetch_add(1, Ordering::SeqCst);
                    Ok(sample_feed("f1"))
                })
            });

            thread::sleep(Duration::from_millis(50));
            {
                let (lock, condvar) = &*release;
                let mut ready = lock.lock().unwrap();
                *ready = true;
                condvar.notify_all();
            }

            let (first_result, first_generation) = first.join().unwrap();
            let (second_result, second_generation) = second.join().unwrap();
            assert_eq!(calls.load(Ordering::SeqCst), 1, "one identity, one request");
            assert_eq!(
                first_generation, second_generation,
                "the joiner gets the same generation"
            );
            assert_eq!(
                first_result.unwrap().feed_guid,
                second_result.unwrap().feed_guid,
                "both callers receive the same result"
            );
        });
        assert_eq!(
            owner.active_feed_count(),
            0,
            "the slot is cleaned up once finished"
        );
    }

    /// R18A-04: two concurrent callers with different identities produce
    /// two HTTP requests.
    #[test]
    fn adr_0075_request_reuse_concurrent_callers_with_different_identities_both_fetch() {
        let owner = MetadataRequestOwner::new();
        let calls = Arc::new(AtomicUsize::new(0));

        thread::scope(|scope| {
            let owner = &owner;
            for feed_guid in ["f1", "f2"] {
                let calls = Arc::clone(&calls);
                let key = RequestKey::feed(endpoint_identity("distinct"), feed_guid, None);
                scope.spawn(move || {
                    owner.fetch_feed(key, RefreshIntent::Normal, || {
                        calls.fetch_add(1, Ordering::SeqCst);
                        thread::sleep(Duration::from_millis(20));
                        Ok(sample_feed(feed_guid))
                    })
                });
            }
        });

        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "different identities each fetch"
        );
    }

    /// R18A-05: a joining caller receives the receipts of the one request
    /// that its winner made, and creates no second observation. This test
    /// uses `fetch_feed_with_receipts`, which is the API the fully-owned
    /// call path (`hydrate_album_identity_facts`) uses.
    #[test]
    fn adr_0075_request_reuse_joining_caller_receives_the_winners_receipts() {
        use crate::provider_observation::{ObservationOutcome, ObservationReceipt};
        use serde_json::json;

        fn sample_receipt() -> ObservationReceipt {
            ObservationReceipt {
                observation_id: 1,
                generation: 1,
                provider_id: 1,
                resource_id: 1,
                request_uri: "https://musicindex.invalid.test/v1/feeds/f1".into(),
                response_uri: None,
                body_key: None,
                started_at_us: 0,
                finished_at_us: 0,
                fetched_at_us: None,
                occurrence: json!({}),
                outcome: ObservationOutcome::Success,
                retention: crate::provider_observation::ObservationRetention::Unknown,
                collections: Vec::new(),
            }
        }
        let owner = MetadataRequestOwner::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let release = Arc::new((Mutex::new(false), Condvar::new()));
        let key = RequestKey::feed(endpoint_identity("receipts"), "f1", None);

        thread::scope(|scope| {
            let owner = &owner;
            let calls_a = Arc::clone(&calls);
            let release_a = Arc::clone(&release);
            let key_a = key.clone();
            let first = scope.spawn(move || {
                owner.fetch_feed_with_receipts(key_a, RefreshIntent::Normal, || {
                    calls_a.fetch_add(1, Ordering::SeqCst);
                    let (lock, condvar) = &*release_a;
                    let mut ready = lock.lock().unwrap();
                    while !*ready {
                        ready = condvar.wait(ready).unwrap();
                    }
                    Ok((sample_feed("f1"), vec![sample_receipt()]))
                })
            });

            thread::sleep(Duration::from_millis(50));
            let key_b = key.clone();
            let second = scope.spawn(move || {
                owner.fetch_feed_with_receipts(key_b, RefreshIntent::Normal, || {
                    unreachable!("a joiner must not fetch")
                })
            });

            thread::sleep(Duration::from_millis(50));
            {
                let (lock, condvar) = &*release;
                let mut ready = lock.lock().unwrap();
                *ready = true;
                condvar.notify_all();
            }

            let (first_result, _) = first.join().unwrap();
            let (second_result, _) = second.join().unwrap();
            let (_, first_receipts) = first_result.unwrap();
            let (_, second_receipts) = second_result.unwrap();
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert_eq!(first_receipts.len(), 1);
            assert_eq!(
                second_receipts.len(),
                1,
                "the joining caller must receive the winner's receipt count"
            );
            assert_eq!(
                first_receipts[0].observation_id, second_receipts[0].observation_id,
                "the joining caller must receive the same observation, not a new one"
            );
        });
    }

    /// R18A-06: a response of an older sequence value does not replace or
    /// disturb the registry entry of a newer one. An explicit refresh
    /// starts a second, newer slot while the first, older one is still in
    /// flight; whichever finishes last must not corrupt the registry.
    #[test]
    fn adr_0075_request_reuse_older_completion_does_not_replace_a_newer_entry() {
        let owner = MetadataRequestOwner::new();
        let older_release = Arc::new((Mutex::new(false), Condvar::new()));
        let key = RequestKey::feed(endpoint_identity("ordering"), "f1", None);

        thread::scope(|scope| {
            let owner = &owner;
            let older_release_thread = Arc::clone(&older_release);
            let key_older = key.clone();
            let older = scope.spawn(move || {
                owner.fetch_feed(key_older, RefreshIntent::Normal, || {
                    let (lock, condvar) = &*older_release_thread;
                    let mut ready = lock.lock().unwrap();
                    while !*ready {
                        ready = condvar.wait(ready).unwrap();
                    }
                    Ok(sample_feed("f1"))
                })
            });

            // Give the older request time to register before the explicit
            // refresh supersedes it.
            thread::sleep(Duration::from_millis(50));
            let key_newer = key.clone();
            let (newer_result, newer_generation) =
                owner.fetch_feed(key_newer, RefreshIntent::Explicit, || Ok(sample_feed("f1")));
            assert!(newer_result.is_ok());

            // The newer, explicit request must already have finished and
            // vacated the registry.
            assert_eq!(owner.active_feed_count(), 0);

            // Now release the older request. Its late completion must not
            // resurrect a stale registry entry.
            {
                let (lock, condvar) = &*older_release;
                let mut ready = lock.lock().unwrap();
                *ready = true;
                condvar.notify_all();
            }
            let (older_result, older_generation) = older.join().unwrap();
            assert!(older_result.is_ok());
            assert!(
                older_generation < newer_generation,
                "the explicit refresh must allocate a newer sequence value"
            );
            assert_eq!(
                owner.active_feed_count(),
                0,
                "the older request's late completion must not leave a stale entry"
            );

            // A following caller for the same identity must fetch again:
            // no stale slot was left behind by either completion.
            let calls = Arc::new(AtomicUsize::new(0));
            let calls_after = Arc::clone(&calls);
            let key_after = key.clone();
            let _ = owner.fetch_feed(key_after, RefreshIntent::Normal, || {
                calls_after.fetch_add(1, Ordering::SeqCst);
                Ok(sample_feed("f1"))
            });
            assert_eq!(calls.load(Ordering::SeqCst), 1);
        });
    }

    /// R18A-07: the owner allocates the sequence value when the request
    /// starts. It is a small monotonic counter, not a fetch time and not a
    /// source time: elapsed wall-clock time between two requests does not
    /// change how much the sequence advances.
    #[test]
    fn adr_0075_request_reuse_sequence_is_a_monotonic_counter_not_a_time() {
        let owner = MetadataRequestOwner::new();
        let key_a = RequestKey::feed(endpoint_identity("sequence"), "f1", None);
        let (_, generation_a) =
            owner.fetch_feed(key_a, RefreshIntent::Normal, || Ok(sample_feed("f1")));

        thread::sleep(Duration::from_millis(120));

        let key_b = RequestKey::feed(endpoint_identity("sequence"), "f2", None);
        let (_, generation_b) =
            owner.fetch_feed(key_b, RefreshIntent::Normal, || Ok(sample_feed("f2")));

        assert_eq!(
            generation_b,
            generation_a + 1,
            "the sequence advances by exactly one request, regardless of elapsed time"
        );
        assert!(
            generation_b < 1_000_000,
            "a source or fetch time in microseconds would be far larger than a request count"
        );
    }

    /// R18A-08: a failed request records its failure and returns the typed
    /// failed state to every caller. It stores no value in the owner.
    #[test]
    fn adr_0075_request_reuse_failed_request_returns_typed_failure_and_stores_nothing() {
        let owner = MetadataRequestOwner::new();
        let key = RequestKey::feed(endpoint_identity("failure"), "f1", None);
        let (result, _) = owner.fetch_feed(key.clone(), RefreshIntent::Normal, || {
            Err(anyhow::anyhow!("musicindex unavailable"))
        });
        assert!(result.is_err());
        assert_eq!(
            owner.active_feed_count(),
            0,
            "a failed request stores no value"
        );

        // The next ask for the same identity fetches again; the failure was
        // not cached for reuse.
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_after = Arc::clone(&calls);
        let _ = owner.fetch_feed(key, RefreshIntent::Normal, || {
            calls_after.fetch_add(1, Ordering::SeqCst);
            Ok(sample_feed("f1"))
        });
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    /// R18A-09: an explicit refresh sends a new request. It does not join
    /// an active request that started without the intent.
    #[test]
    fn adr_0075_request_reuse_explicit_refresh_bypasses_an_active_normal_request() {
        let owner = MetadataRequestOwner::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let release = Arc::new((Mutex::new(false), Condvar::new()));
        let key = RequestKey::feed(endpoint_identity("explicit"), "f1", None);

        thread::scope(|scope| {
            let owner = &owner;
            let calls_a = Arc::clone(&calls);
            let release_a = Arc::clone(&release);
            let key_a = key.clone();
            let normal = scope.spawn(move || {
                owner.fetch_feed(key_a, RefreshIntent::Normal, || {
                    calls_a.fetch_add(1, Ordering::SeqCst);
                    let (lock, condvar) = &*release_a;
                    let mut ready = lock.lock().unwrap();
                    while !*ready {
                        ready = condvar.wait(ready).unwrap();
                    }
                    Ok(sample_feed("f1"))
                })
            });

            thread::sleep(Duration::from_millis(50));
            let calls_b = Arc::clone(&calls);
            let key_b = key.clone();
            let (explicit_result, _) = owner.fetch_feed(key_b, RefreshIntent::Explicit, || {
                calls_b.fetch_add(1, Ordering::SeqCst);
                Ok(sample_feed("f1"))
            });
            assert!(explicit_result.is_ok());
            assert_eq!(
                calls.load(Ordering::SeqCst),
                2,
                "the explicit refresh must send its own request, not join the active one"
            );

            {
                let (lock, condvar) = &*release;
                let mut ready = lock.lock().unwrap();
                *ready = true;
                condvar.notify_all();
            }
            let _ = normal.join().unwrap();
        });
    }

    /// R18A-11: sequential callers, with no overlapping active request,
    /// always fetch. Part A changes nothing about that baseline count.
    #[test]
    fn adr_0075_request_reuse_sequential_callers_each_fetch() {
        let owner = MetadataRequestOwner::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let key = RequestKey::feed(endpoint_identity("sequential"), "f1", None);

        for _ in 0..3 {
            let calls = Arc::clone(&calls);
            let key = key.clone();
            let (result, _) = owner.fetch_feed(key, RefreshIntent::Normal, || {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(sample_feed("f1"))
            });
            assert!(result.is_ok());
        }
        assert_eq!(
            calls.load(Ordering::SeqCst),
            3,
            "Part A does not change the sequential request count; only a truly concurrent \
             duplicate is shared"
        );
    }
}
