//! Shared MusicIndex request identity, active-request sharing, and
//! completed-response reuse.
//!
//! ADR 0075 section 6 and packet 018. A caller asks this owner for a
//! MusicIndex resource instead of calling `api::Client` on its own. The
//! owner sends at most one request for one identity at one time. A second
//! caller with the same identity joins the request in flight and gets the
//! same result.
//!
//! Part A holds active requests only. Packet 018 Part B adds a reuse window
//! for completed responses (P18-1, P18-2), a failure rule that retains
//! nothing (P18-4), a capacity limit with least-recently-used eviction
//! (P18-5), and an explicit-refresh invalidation that clears one feed and
//! its scoped tracks (P18-7). Every retained response stays in memory only
//! (P18-6): a restart empties the owner.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use crate::api::{Feed, Track};
use crate::provider_observation::{
    ObservationReceipt, ObservationStorageError, ObservationWriteFailure,
};

/// P18-1: a successful Library track detail response — the scoped or
/// unscoped track fetch — stays reusable for this long. This is the one
/// place this value appears in the code (R18B-03).
const TRACK_RESPONSE_REUSE_WINDOW: Duration = Duration::from_secs(30 * 60);

/// P18-2: a successful feed response stays reusable for this long, for
/// each distinct include list. This is the one place this value appears in
/// the code (R18B-03).
const FEED_RESPONSE_REUSE_WINDOW: Duration = Duration::from_secs(15 * 60);

/// P18-5: the owner holds at most this many feed responses.
const FEED_RESPONSE_CAPACITY: usize = 64;

/// P18-5: the owner holds at most this many track responses.
const TRACK_RESPONSE_CAPACITY: usize = 256;

/// The generation value `fetch_feed_with_receipts` and
/// `fetch_track_with_receipts` return for a retained-cache hit (R18B-01).
/// A real request's generation always starts at 1 and only rises
/// (R18A-07), so `0` cannot collide with one. A caller that must not
/// repeat a write cascade a genuinely new response alone justifies — for
/// example `library::hydrate_album_identity_facts`, whose own local
/// persistence follows a successful fetch — reads this value to tell a
/// reused response from a fetched one.
pub(crate) const REUSED_GENERATION: i64 = 0;

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

    /// Names the feed this key belongs to, for packet 018 P18-7.
    ///
    /// A feed key and a scoped-track key both carry a feed GUID, so an
    /// explicit refresh of one feed can find both kinds together. An
    /// unscoped track key carries no feed GUID at all: the app never
    /// learns which feed it belongs to from the key alone, so P18-7
    /// cannot address it by feed. Its own reuse window still bounds how
    /// long it stays retained.
    fn feed_identity(&self) -> Option<(String, String)> {
        match &self.subject {
            RequestSubject::Feed(feed_guid) | RequestSubject::ScopedTrack(feed_guid, _) => {
                Some((self.provider_identity.clone(), feed_guid.clone()))
            }
            RequestSubject::UnscopedTrack(_) => None,
        }
    }
}

/// Whether a request may join an active request, or must bypass reuse.
///
/// ADR 0075 section 6 and packet 018 P18-7.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RefreshIntent {
    /// A passive read. It may join an active request, started under either
    /// intent, and it may reuse a retained response inside its window.
    Normal,
    /// An explicit refresh. It always sends a new request. It never joins
    /// an active request that started without this intent (packet 018
    /// R18A-09), and it never reuses a retained response (R18B-01
    /// applies to `Normal` only). Its own successful result still
    /// replaces whatever was retained, for a later `Normal` caller.
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
///
/// Generic over the identity's key type `K`, not only `RequestKey`: packet
/// 018 Part B's `rss::enrich` module reuses this same machinery, keyed by
/// feed URL instead, to share an in-flight RSS fetch (P18-3's own module,
/// closing the gap the packet's concurrent measurement found). Sharing one
/// generic implementation, rather than a second copy of it, keeps the
/// abandonment guarantee (`SlotCompletion` below) in one place.
pub(crate) struct Slot<T> {
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

/// A registry of in-flight or just-finished single-flight requests, keyed
/// by `K`. `MetadataRequestOwner` below uses `RequestKey`; `rss::enrich`
/// uses a plain feed URL `String` for its own P18-3 single-flight sharing.
pub(crate) type Registry<K, T> = Mutex<HashMap<K, Arc<Slot<T>>>>;

/// Completes an abandoned slot when the requesting thread unwinds.
///
/// A panic in the request closure would otherwise leave the slot
/// `Pending`. Each joined caller would then wait without end, and the
/// identity would stay in the registry. This guard fails the slot, wakes
/// every joined caller, and removes the identity, so that a later caller
/// starts a new request.
struct SlotCompletion<'a, K: Eq + std::hash::Hash, T> {
    registry: &'a Registry<K, T>,
    key: &'a K,
    slot: &'a Arc<Slot<T>>,
    finished: bool,
}

impl<K: Eq + std::hash::Hash, T> Drop for SlotCompletion<'_, K, T> {
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
///
/// Generic over the key type `K`: `MetadataRequestOwner` calls this with
/// `RequestKey`, and `rss::enrich` calls it with a feed URL `String`
/// (packet 018 Part B, closing the RSS active-request gap the packet's own
/// concurrent measurement found). `pub(crate)` so `rss::enrich` can reach
/// it from its own module.
pub(crate) fn single_flight<K: Eq + std::hash::Hash + Clone, T: Clone>(
    registry: &Registry<K, T>,
    sequence: &AtomicI64,
    key: K,
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

/// One retained, still-fresh completed response.
struct RetainedEntry<V> {
    value: V,
    completed_at: Instant,
    last_used: u64,
}

/// A bounded, time-windowed, least-recently-used cache of completed
/// responses.
///
/// Packet 018 P18-1/P18-2/P18-3 give each retained kind its own window;
/// P18-5 gives each its own capacity. `MetadataRequestOwner` holds one of
/// these for feed responses and one for track responses; `rss::enrich`
/// holds a separate one, of the same shape, for parsed RSS documents,
/// keyed by feed URL instead of `RequestKey`. The age of an entry is
/// measured with `std::time::Instant`, recorded when the response
/// completed, per packet 018's Part B design brief: a monotonic clock, not
/// a fetch time read from storage and not a wall-clock time, so a system
/// clock change cannot make a retained response look fresh.
pub(crate) struct RetainedCache<K, V> {
    window: Duration,
    capacity: usize,
    entries: HashMap<K, RetainedEntry<V>>,
    clock: u64,
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> RetainedCache<K, V> {
    pub(crate) fn new(window: Duration, capacity: usize) -> Self {
        Self {
            window,
            capacity,
            entries: HashMap::new(),
            clock: 0,
        }
    }

    /// Returns a clone of the retained value when the entry exists and its
    /// age is below the window (R18B-01). An entry whose age has reached
    /// the window is treated as a miss and removed (R18B-02): P18-4 keeps
    /// a failure from ever reaching this cache, so a removed entry can
    /// only be a response that aged out, never a retained failure.
    pub(crate) fn get(&mut self, key: &K) -> Option<V> {
        let expired = self
            .entries
            .get(key)
            .is_some_and(|entry| entry.completed_at.elapsed() >= self.window);
        if expired {
            self.entries.remove(key);
            return None;
        }
        self.clock += 1;
        let clock = self.clock;
        let entry = self.entries.get_mut(key)?;
        entry.last_used = clock;
        Some(entry.value.clone())
    }

    /// Retains a successful response. A caller must never call this for a
    /// failed request (P18-4): the owner methods below only call it after
    /// a successful fetch.
    pub(crate) fn store(&mut self, key: K, value: V) {
        self.clock += 1;
        let clock = self.clock;
        self.entries.insert(
            key,
            RetainedEntry {
                value,
                completed_at: Instant::now(),
                last_used: clock,
            },
        );
        self.evict_over_capacity();
    }

    pub(crate) fn remove(&mut self, key: &K) {
        self.entries.remove(key);
    }

    /// P18-5: removes the least recently used entry until the cache is at
    /// or under capacity. Capacities (64 feeds, 256 tracks, 32 RSS
    /// documents) are small, so a linear scan for the minimum is simple
    /// and cheap; no ordered-list bookkeeping is needed.
    fn evict_over_capacity(&mut self) {
        while self.entries.len() > self.capacity {
            let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            self.entries.remove(&oldest);
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Holds active MusicIndex requests and shares them, and holds completed
/// responses for packet 018 P18-1/P18-2 reuse.
///
/// One owner serves the whole process (packet 018 P18-6: memory only, a
/// restart clears it). Production call sites reach it through `shared`.
/// Tests construct their own instance so their active requests cannot
/// cross into another test.
pub(crate) struct MetadataRequestOwner {
    sequence: AtomicI64,
    feeds: Registry<RequestKey, (Feed, Vec<ObservationReceipt>)>,
    tracks: Registry<RequestKey, (Track, Vec<ObservationReceipt>)>,
    retained_feeds: Mutex<RetainedCache<RequestKey, (Feed, Vec<ObservationReceipt>)>>,
    retained_tracks: Mutex<RetainedCache<RequestKey, (Track, Vec<ObservationReceipt>)>>,
    /// P18-7's index from one feed identity to every retained key that
    /// belongs to it (the feed's own entries, under every include list,
    /// and every scoped-track entry of that feed). `RequestKey` alone
    /// cannot answer "every entry of this feed", because the registry key
    /// is provider identity plus subject plus include list: one feed can
    /// have several retained entries (one per include list), and a track
    /// entry names its own subject, not its feed, as the map key. This
    /// index is the chosen answer: every `store` that carries a feed
    /// identity (a feed entry, or a scoped-track entry) also records its
    /// key here, so `invalidate_feed` can find and remove them together.
    feed_index: Mutex<HashMap<(String, String), HashSet<RequestKey>>>,
}

impl MetadataRequestOwner {
    pub(crate) fn new() -> Self {
        Self::with_windows(FEED_RESPONSE_REUSE_WINDOW, TRACK_RESPONSE_REUSE_WINDOW)
    }

    /// Builds an owner with the given reuse windows. Production code
    /// always uses `new`, which supplies the one accepted window value for
    /// each kind (R18B-03). Tests use this constructor to prove reuse and
    /// expiry mechanically, without waiting out the real 15- and
    /// 30-minute windows.
    fn with_windows(feed_window: Duration, track_window: Duration) -> Self {
        Self {
            sequence: AtomicI64::new(0),
            feeds: Mutex::new(HashMap::new()),
            tracks: Mutex::new(HashMap::new()),
            retained_feeds: Mutex::new(RetainedCache::new(feed_window, FEED_RESPONSE_CAPACITY)),
            retained_tracks: Mutex::new(RetainedCache::new(track_window, TRACK_RESPONSE_CAPACITY)),
            feed_index: Mutex::new(HashMap::new()),
        }
    }

    /// Sends or joins a feed request whose caller also wants the
    /// observation receipts that request produced (packet 018 R18A-05,
    /// R18B-07, R18B-11).
    ///
    /// A `Normal` caller first asks the retained cache (R18B-01, R18B-02).
    /// A reusable response returns here, without a request and without
    /// touching the active-request registry, so it can never hold the
    /// registry lock across a request (Part A's locking discipline). A
    /// cache miss, and every `Explicit` call, falls through to
    /// `single_flight`, exactly as Part A already does. On success, the
    /// response is retained for later reuse and indexed for P18-7. The
    /// `fetch` closure runs only for the winning caller; a joining caller
    /// receives the exact receipts of that one request (R18A-05), and so
    /// does a later caller that reuses the retained response (R18B-07,
    /// R18B-11): both receive a clone of the same `ObservationReceipt`
    /// list, so both name the observation that produced the response, and
    /// neither creates a new one.
    pub(crate) fn fetch_feed_with_receipts(
        &self,
        key: RequestKey,
        refresh: RefreshIntent,
        fetch: impl FnOnce() -> anyhow::Result<(Feed, Vec<ObservationReceipt>)>,
    ) -> (
        Result<(Feed, Vec<ObservationReceipt>), SharedFetchError>,
        i64,
    ) {
        if refresh == RefreshIntent::Normal {
            if let Some(hit) = self
                .retained_feeds
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .get(&key)
            {
                return (Ok(hit), REUSED_GENERATION);
            }
        }
        let (result, generation) =
            single_flight(&self.feeds, &self.sequence, key.clone(), refresh, || {
                fetch().map_err(SharedFetchError::from)
            });
        if let Ok(value) = &result {
            self.retain_feed(key, value.clone());
        }
        (result, generation)
    }

    /// Sends or joins a track request whose caller also wants the
    /// observation receipts that request produced. Mirrors
    /// `fetch_feed_with_receipts`; see its documentation.
    pub(crate) fn fetch_track_with_receipts(
        &self,
        key: RequestKey,
        refresh: RefreshIntent,
        fetch: impl FnOnce() -> anyhow::Result<(Track, Vec<ObservationReceipt>)>,
    ) -> (
        Result<(Track, Vec<ObservationReceipt>), SharedFetchError>,
        i64,
    ) {
        if refresh == RefreshIntent::Normal {
            if let Some(hit) = self
                .retained_tracks
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .get(&key)
            {
                return (Ok(hit), REUSED_GENERATION);
            }
        }
        let (result, generation) =
            single_flight(&self.tracks, &self.sequence, key.clone(), refresh, || {
                fetch().map_err(SharedFetchError::from)
            });
        if let Ok(value) = &result {
            self.retain_track(key, value.clone());
        }
        (result, generation)
    }

    /// Shares an active track request without retaining its completed
    /// response for a later reuse.
    ///
    /// Packet 018 Job 1 (Part B follow-up, R18B-12): the Index route asks
    /// the owner for its track detail too, but it carries no accepted
    /// reuse window of its own — P18-1 names a Library track detail
    /// response only. This method gives an Index caller the same
    /// active-request sharing `fetch_track_with_receipts` gives a Library
    /// caller (ADR 0075 section 6), through the same registry, so a
    /// concurrent duplicate still joins instead of sending its own
    /// request. It never consults or updates the retained-response cache,
    /// so an Index request is never served from, and never left in, that
    /// cache for a later caller. The operator decides later whether an
    /// Index track detail response earns its own window; this method
    /// invents none.
    pub(crate) fn fetch_track_shared(
        &self,
        key: RequestKey,
        fetch: impl FnOnce() -> anyhow::Result<Track>,
    ) -> anyhow::Result<Track> {
        single_flight(
            &self.tracks,
            &self.sequence,
            key,
            RefreshIntent::Normal,
            || {
                fetch()
                    .map(|track| (track, Vec::new()))
                    .map_err(SharedFetchError::from)
            },
        )
        .0
        .map(|(track, _receipts)| track)
        .map_err(SharedFetchError::into_anyhow)
    }

    fn retain_feed(&self, key: RequestKey, value: (Feed, Vec<ObservationReceipt>)) {
        self.index_feed_key(&key);
        self.retained_feeds
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .store(key, value);
    }

    fn retain_track(&self, key: RequestKey, value: (Track, Vec<ObservationReceipt>)) {
        self.index_feed_key(&key);
        self.retained_tracks
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .store(key, value);
    }

    fn index_feed_key(&self, key: &RequestKey) {
        let Some(identity) = key.feed_identity() else {
            return;
        };
        self.feed_index
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(identity)
            .or_default()
            .insert(key.clone());
    }

    /// P18-7: an explicit refresh of one feed removes every retained entry
    /// of that feed and its scoped tracks, before the caller sends new
    /// requests. It does not touch the RSS document cache: `rss::enrich`
    /// owns that cache, keyed by feed URL rather than this owner's
    /// `RequestKey`, so its own `invalidate_feed_document` covers it
    /// separately.
    pub(crate) fn invalidate_feed(&self, provider_identity: &str, feed_guid: &str) {
        let identity = (provider_identity.to_owned(), feed_guid.to_owned());
        let keys = self
            .feed_index
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&identity)
            .unwrap_or_default();
        if keys.is_empty() {
            return;
        }
        let mut feeds = self
            .retained_feeds
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let mut tracks = self
            .retained_tracks
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        for key in &keys {
            feeds.remove(key);
            tracks.remove(key);
        }
    }

    #[cfg(test)]
    fn active_feed_count(&self) -> usize {
        self.feeds
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    #[cfg(test)]
    fn retained_feed_count(&self) -> usize {
        self.retained_feeds
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    #[cfg(test)]
    fn retained_track_count(&self) -> usize {
        self.retained_tracks
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
    use crate::provider_observation::{
        ObservationOutcome, ObservationReceipt, ObservationRetention,
    };
    use serde_json::json;
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

    fn sample_track(track_guid: &str) -> Track {
        Track {
            track_guid: Some(track_guid.to_owned()),
            ..Track::default()
        }
    }

    fn sample_receipt(observation_id: i64) -> ObservationReceipt {
        ObservationReceipt {
            observation_id,
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
            retention: ObservationRetention::Unknown,
            collections: Vec::new(),
        }
    }

    /// R17A-01 (retained as R18A-01): a key holds the endpoint, the scoped
    /// subject, and the profile. Two requests with different endpoints
    /// have different keys.
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
                    owner.fetch_feed_with_receipts(key_a, RefreshIntent::Normal, || {
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
                owner.fetch_feed_with_receipts(key_b, RefreshIntent::Normal, || {
                    Ok((sample_feed("f1"), Vec::new()))
                })
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

        let (later, _) = owner.fetch_feed_with_receipts(key, RefreshIntent::Normal, || {
            Ok((sample_feed("f1"), Vec::new()))
        });
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
                owner.fetch_feed_with_receipts(key_a, RefreshIntent::Normal, || {
                    calls_a.fetch_add(1, Ordering::SeqCst);
                    let (lock, condvar) = &*release_a;
                    let mut ready = lock.lock().unwrap();
                    while !*ready {
                        ready = condvar.wait(ready).unwrap();
                    }
                    Ok((sample_feed("f1"), Vec::new()))
                })
            });

            // Give the first caller time to register its active request
            // before the second caller asks.
            thread::sleep(Duration::from_millis(50));
            let calls_b = Arc::clone(&calls);
            let key_b = key.clone();
            let second = scope.spawn(move || {
                owner.fetch_feed_with_receipts(key_b, RefreshIntent::Normal, || {
                    calls_b.fetch_add(1, Ordering::SeqCst);
                    Ok((sample_feed("f1"), Vec::new()))
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
                first_result.unwrap().0.feed_guid,
                second_result.unwrap().0.feed_guid,
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
                    owner.fetch_feed_with_receipts(key, RefreshIntent::Normal, || {
                        calls.fetch_add(1, Ordering::SeqCst);
                        thread::sleep(Duration::from_millis(20));
                        Ok((sample_feed(feed_guid), Vec::new()))
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
    /// that its winner made, and creates no second observation.
    #[test]
    fn adr_0075_request_reuse_joining_caller_receives_the_winners_receipts() {
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
                    Ok((sample_feed("f1"), vec![sample_receipt(1)]))
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
                owner.fetch_feed_with_receipts(key_older, RefreshIntent::Normal, || {
                    let (lock, condvar) = &*older_release_thread;
                    let mut ready = lock.lock().unwrap();
                    while !*ready {
                        ready = condvar.wait(ready).unwrap();
                    }
                    Ok((sample_feed("f1"), Vec::new()))
                })
            });

            // Give the older request time to register before the explicit
            // refresh supersedes it.
            thread::sleep(Duration::from_millis(50));
            let key_newer = key.clone();
            let (newer_result, newer_generation) =
                owner.fetch_feed_with_receipts(key_newer, RefreshIntent::Explicit, || {
                    Ok((sample_feed("f1"), Vec::new()))
                });
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
            // no stale slot was left behind by either completion, and the
            // explicit refresh's own retained response does not apply
            // here because this next caller is itself explicit.
            let calls = Arc::new(AtomicUsize::new(0));
            let calls_after = Arc::clone(&calls);
            let key_after = key.clone();
            let _ = owner.fetch_feed_with_receipts(key_after, RefreshIntent::Explicit, move || {
                calls_after.fetch_add(1, Ordering::SeqCst);
                Ok((sample_feed("f1"), Vec::new()))
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
            owner.fetch_feed_with_receipts(key_a, RefreshIntent::Normal, || {
                Ok((sample_feed("f1"), Vec::new()))
            });

        thread::sleep(Duration::from_millis(120));

        let key_b = RequestKey::feed(endpoint_identity("sequence"), "f2", None);
        let (_, generation_b) =
            owner.fetch_feed_with_receipts(key_b, RefreshIntent::Normal, || {
                Ok((sample_feed("f2"), Vec::new()))
            });

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
    /// failed state to every caller. It stores no value in the owner, and
    /// it retains no value for later reuse (R18B-04).
    #[test]
    fn adr_0075_request_reuse_failed_request_returns_typed_failure_and_stores_nothing() {
        let owner = MetadataRequestOwner::new();
        let key = RequestKey::feed(endpoint_identity("failure"), "f1", None);
        let (result, _) =
            owner.fetch_feed_with_receipts(key.clone(), RefreshIntent::Normal, || {
                Err(anyhow::anyhow!("musicindex unavailable"))
            });
        assert!(result.is_err());
        assert_eq!(
            owner.active_feed_count(),
            0,
            "a failed request stores no value"
        );
        assert_eq!(
            owner.retained_feed_count(),
            0,
            "a failed request retains no value"
        );

        // The next ask for the same identity fetches again; the failure was
        // not cached for reuse.
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_after = Arc::clone(&calls);
        let _ = owner.fetch_feed_with_receipts(key, RefreshIntent::Normal, move || {
            calls_after.fetch_add(1, Ordering::SeqCst);
            Ok((sample_feed("f1"), Vec::new()))
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
                owner.fetch_feed_with_receipts(key_a, RefreshIntent::Normal, || {
                    calls_a.fetch_add(1, Ordering::SeqCst);
                    let (lock, condvar) = &*release_a;
                    let mut ready = lock.lock().unwrap();
                    while !*ready {
                        ready = condvar.wait(ready).unwrap();
                    }
                    Ok((sample_feed("f1"), Vec::new()))
                })
            });

            thread::sleep(Duration::from_millis(50));
            let calls_b = Arc::clone(&calls);
            let key_b = key.clone();
            let (explicit_result, _) =
                owner.fetch_feed_with_receipts(key_b, RefreshIntent::Explicit, move || {
                    calls_b.fetch_add(1, Ordering::SeqCst);
                    Ok((sample_feed("f1"), Vec::new()))
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

    /// R18A-11: sequential callers, with no overlapping active request and
    /// no retained response (a fresh owner per identity below), always
    /// fetch. Part A's baseline count is unchanged by Part B when nothing
    /// is retained yet.
    #[test]
    fn adr_0075_request_reuse_sequential_callers_each_fetch() {
        let owner = MetadataRequestOwner::new();
        let calls = Arc::new(AtomicUsize::new(0));

        for index in 0..3 {
            let calls = Arc::clone(&calls);
            // A distinct feed GUID for each call keeps this test about
            // Part A's sequential-request baseline, not about Part B's new
            // retained-response reuse, which the tests below prove
            // separately.
            let key = RequestKey::feed(endpoint_identity("sequential"), &format!("f{index}"), None);
            let (result, _) =
                owner.fetch_feed_with_receipts(key, RefreshIntent::Normal, move || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok((sample_feed("f1"), Vec::new()))
                });
            assert!(result.is_ok());
        }
        assert_eq!(
            calls.load(Ordering::SeqCst),
            3,
            "each distinct identity fetches"
        );
    }

    /// R18B-01: a retained feed response inside its window returns without
    /// a request.
    #[test]
    fn adr_0075_request_reuse_retained_feed_response_reused_within_window() {
        let owner = MetadataRequestOwner::with_windows(
            Duration::from_millis(300),
            Duration::from_secs(3600),
        );
        let calls = Arc::new(AtomicUsize::new(0));
        let key = RequestKey::feed(endpoint_identity("retained-feed-hit"), "f1", None);

        let calls_a = Arc::clone(&calls);
        let (first, _) =
            owner.fetch_feed_with_receipts(key.clone(), RefreshIntent::Normal, move || {
                calls_a.fetch_add(1, Ordering::SeqCst);
                Ok((sample_feed("f1"), Vec::new()))
            });
        assert!(first.is_ok());

        let (second, _) = owner.fetch_feed_with_receipts(key, RefreshIntent::Normal, || {
            unreachable!("a retained response inside its window must not send a request")
        });
        assert!(second.is_ok());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "R18B-01: the retained response is reused without a request"
        );
    }

    /// R18B-02: a retained feed response outside its window sends a new
    /// request.
    #[test]
    fn adr_0075_request_reuse_retained_feed_response_expires_outside_window() {
        let owner = MetadataRequestOwner::with_windows(
            Duration::from_millis(20),
            Duration::from_secs(3600),
        );
        let calls = Arc::new(AtomicUsize::new(0));
        let key = RequestKey::feed(endpoint_identity("retained-feed-expiry"), "f1", None);

        let calls_a = Arc::clone(&calls);
        let (first, _) =
            owner.fetch_feed_with_receipts(key.clone(), RefreshIntent::Normal, move || {
                calls_a.fetch_add(1, Ordering::SeqCst);
                Ok((sample_feed("f1"), Vec::new()))
            });
        assert!(first.is_ok());

        thread::sleep(Duration::from_millis(80));

        let calls_b = Arc::clone(&calls);
        let (second, _) = owner.fetch_feed_with_receipts(key, RefreshIntent::Normal, move || {
            calls_b.fetch_add(1, Ordering::SeqCst);
            Ok((sample_feed("f1"), Vec::new()))
        });
        assert!(second.is_ok());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "R18B-02: an expired retained response sends a new request"
        );
    }

    /// R18B-01/R18B-02 for a track response, mirroring the feed proof
    /// above at the other capacity/window pair.
    #[test]
    fn adr_0075_request_reuse_retained_track_response_honors_its_own_window() {
        let owner = MetadataRequestOwner::with_windows(
            Duration::from_secs(3600),
            Duration::from_millis(20),
        );
        let calls = Arc::new(AtomicUsize::new(0));
        let key = RequestKey::scoped_track(endpoint_identity("retained-track"), "f1", "t1", None);

        let calls_a = Arc::clone(&calls);
        let (first, _) =
            owner.fetch_track_with_receipts(key.clone(), RefreshIntent::Normal, move || {
                calls_a.fetch_add(1, Ordering::SeqCst);
                Ok((sample_track("t1"), Vec::new()))
            });
        assert!(first.is_ok());

        let (second, _) =
            owner.fetch_track_with_receipts(key.clone(), RefreshIntent::Normal, || {
                unreachable!("a retained track response inside its window must not send a request")
            });
        assert!(second.is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        thread::sleep(Duration::from_millis(80));
        let calls_b = Arc::clone(&calls);
        let (third, _) = owner.fetch_track_with_receipts(key, RefreshIntent::Normal, move || {
            calls_b.fetch_add(1, Ordering::SeqCst);
            Ok((sample_track("t1"), Vec::new()))
        });
        assert!(third.is_ok());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "the track response expires outside its own window and is fetched again"
        );
    }

    /// R18B-03: the accepted 30-minute track window and 15-minute feed
    /// window each appear once in the code, as the named constants this
    /// test reads. (The 15-minute RSS document window is a third, separate
    /// value, owned and proven by `rss::enrich`, since P18-3 assigns RSS
    /// document reuse to that module.)
    #[test]
    fn adr_0075_request_reuse_track_and_feed_windows_match_the_accepted_policy() {
        assert_eq!(TRACK_RESPONSE_REUSE_WINDOW, Duration::from_secs(30 * 60));
        assert_eq!(FEED_RESPONSE_REUSE_WINDOW, Duration::from_secs(15 * 60));
    }

    /// R18B-04: a failed request is never retained, proven at the owner
    /// method used in production (`fetch_feed_with_receipts`), beyond the
    /// R18A-08 proof above that already covers the failure-returns-nothing
    /// half of this rule.
    #[test]
    fn adr_0075_request_reuse_failed_response_is_never_retained_for_reuse() {
        let owner = MetadataRequestOwner::with_windows(
            Duration::from_secs(3600),
            Duration::from_secs(3600),
        );
        let key = RequestKey::feed(endpoint_identity("never-retained-failure"), "f1", None);
        let (first, _) = owner.fetch_feed_with_receipts(key.clone(), RefreshIntent::Normal, || {
            Err(anyhow::anyhow!("musicindex unavailable"))
        });
        assert!(first.is_err());

        let calls = Arc::new(AtomicUsize::new(0));
        let calls_a = Arc::clone(&calls);
        let (second, _) = owner.fetch_feed_with_receipts(key, RefreshIntent::Normal, move || {
            calls_a.fetch_add(1, Ordering::SeqCst);
            Ok((sample_feed("f1"), Vec::new()))
        });
        assert!(second.is_ok());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "R18B-04: the next ask always sends a new request; a failure was never a reusable entry"
        );
    }

    /// R18B-06: the retained cache holds at most its capacity, removing
    /// the least recently used entry first.
    #[test]
    fn adr_0075_request_reuse_retained_cache_capacity_evicts_least_recently_used() {
        let mut cache: RetainedCache<i32, &'static str> =
            RetainedCache::new(Duration::from_secs(3600), 3);
        cache.store(1, "a");
        cache.store(2, "b");
        cache.store(3, "c");

        // Touch key 1 so it becomes the most recently used, protecting it
        // from the next eviction.
        assert_eq!(cache.get(&1), Some("a"));

        cache.store(4, "d");
        assert_eq!(cache.len(), 3, "the cache never grows past its capacity");
        assert_eq!(
            cache.get(&2),
            None,
            "the least recently used entry (2) is evicted first"
        );
        assert_eq!(cache.get(&1), Some("a"));
        assert_eq!(cache.get(&3), Some("c"));
        assert_eq!(cache.get(&4), Some("d"));
    }

    /// R18B-05/P18-7: an explicit refresh removes every retained entry of
    /// the named feed, including its scoped tracks, and then a later ask
    /// sends a new request for each. An unscoped track carries no feed
    /// identity in its key, so P18-7 cannot address it by feed; its own
    /// window still governs its reuse, which this test also proves stays
    /// untouched by the invalidation.
    #[test]
    fn adr_0075_request_reuse_explicit_refresh_clears_feed_and_its_scoped_tracks() {
        let owner = MetadataRequestOwner::with_windows(
            Duration::from_secs(3600),
            Duration::from_secs(3600),
        );
        let provider = endpoint_identity("invalidate");
        let feed_key = RequestKey::feed(provider.clone(), "f1", Some("L1"));
        let scoped_key = RequestKey::scoped_track(provider.clone(), "f1", "t1", Some("L1"));
        let unscoped_key = RequestKey::unscoped_track(provider.clone(), "t2", Some("L1"));

        let _ = owner.fetch_feed_with_receipts(feed_key.clone(), RefreshIntent::Normal, || {
            Ok((sample_feed("f1"), Vec::new()))
        });
        let _ = owner.fetch_track_with_receipts(scoped_key.clone(), RefreshIntent::Normal, || {
            Ok((sample_track("t1"), Vec::new()))
        });
        let _ =
            owner.fetch_track_with_receipts(unscoped_key.clone(), RefreshIntent::Normal, || {
                Ok((sample_track("t2"), Vec::new()))
            });
        assert_eq!(owner.retained_feed_count(), 1);
        assert_eq!(owner.retained_track_count(), 2);

        owner.invalidate_feed(&provider, "f1");
        assert_eq!(
            owner.retained_feed_count(),
            0,
            "the feed's own retained response is cleared"
        );
        assert_eq!(
            owner.retained_track_count(),
            1,
            "the scoped track is cleared; the unscoped track is not"
        );

        let feed_calls = Arc::new(AtomicUsize::new(0));
        let feed_calls_a = Arc::clone(&feed_calls);
        let _ = owner.fetch_feed_with_receipts(feed_key, RefreshIntent::Normal, move || {
            feed_calls_a.fetch_add(1, Ordering::SeqCst);
            Ok((sample_feed("f1"), Vec::new()))
        });
        assert_eq!(
            feed_calls.load(Ordering::SeqCst),
            1,
            "the feed is fetched again"
        );

        let scoped_calls = Arc::new(AtomicUsize::new(0));
        let scoped_calls_a = Arc::clone(&scoped_calls);
        let _ = owner.fetch_track_with_receipts(scoped_key, RefreshIntent::Normal, move || {
            scoped_calls_a.fetch_add(1, Ordering::SeqCst);
            Ok((sample_track("t1"), Vec::new()))
        });
        assert_eq!(
            scoped_calls.load(Ordering::SeqCst),
            1,
            "the scoped track of the invalidated feed is fetched again"
        );

        let unscoped_calls = Arc::new(AtomicUsize::new(0));
        let unscoped_calls_a = Arc::clone(&unscoped_calls);
        let _ = owner.fetch_track_with_receipts(unscoped_key, RefreshIntent::Normal, move || {
            unscoped_calls_a.fetch_add(1, Ordering::SeqCst);
            Ok((sample_track("t2"), Vec::new()))
        });
        assert_eq!(
            unscoped_calls.load(Ordering::SeqCst),
            0,
            "the unscoped track carries no feed identity, so it stays retained after this invalidation"
        );
    }

    /// R18B-07: a reused response carries the identifier of the
    /// observation that produced it, and creates no new observation.
    #[test]
    fn adr_0075_request_reuse_retained_response_carries_the_original_receipt() {
        let owner = MetadataRequestOwner::with_windows(
            Duration::from_secs(3600),
            Duration::from_secs(3600),
        );
        let key = RequestKey::feed(endpoint_identity("retained-receipt"), "f1", None);

        let calls = Arc::new(AtomicUsize::new(0));
        let calls_a = Arc::clone(&calls);
        let (first, _) =
            owner.fetch_feed_with_receipts(key.clone(), RefreshIntent::Normal, move || {
                calls_a.fetch_add(1, Ordering::SeqCst);
                Ok((sample_feed("f1"), vec![sample_receipt(7)]))
            });
        let (_, first_receipts) = first.unwrap();
        assert_eq!(first_receipts[0].observation_id, 7);

        let (second, _) = owner.fetch_feed_with_receipts(key, RefreshIntent::Normal, || {
            unreachable!("a retained response must not send a new request")
        });
        let (second_feed, second_receipts) = second.unwrap();
        assert_eq!(second_feed.feed_guid.as_deref(), Some("f1"));
        assert_eq!(second_receipts.len(), 1);
        assert_eq!(
            second_receipts[0].observation_id, 7,
            "R18B-07: the reused response names the observation that produced it"
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "the reused response creates no new observation"
        );
    }
}
