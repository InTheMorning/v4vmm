//! Playlist RSS check actor (ADR 0076 Decision 2, ADR 0040).
//!
//! The actor reads the RSS document of each distinct feed of one playlist.
//! It sends one conditional GET for each feed. It sends one request at a
//! time to each host and waits `MIN_HOST_INTERVAL` or more after each
//! response from that host. It runs at most `MAX_PARALLEL_HOSTS` host
//! queues at the same time. It obeys `Retry-After`. After HTTP 429 it sends
//! no more requests to that host during the run.
//!
//! Each request records one observation in the ADR 0075 store. For each
//! received RSS document, packet 002 of ADR 0076 compares the document with
//! the stored values of its feed and applies each difference in one
//! transaction (`rss::check_apply`). The actor writes no audio tag.
//!
//! The actor publishes one snapshot after each feed result. The playlist
//! page reads the snapshot and updates in place. When the session stops,
//! the actor stops each host queue, waits for the requests that are in
//! progress, and then exits.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::Connection;
use serde_json::Value;
use tokio::sync::{mpsc, watch, Semaphore};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::application::session_lifecycle::SessionLifecycle;
use crate::db::rss_check_runs::{self as store, PlaylistCheckFeed, StoredRun};
pub use crate::db::rss_check_runs::{RssCheckTrigger, RssFeedOutcome};
pub use crate::db::rss_field_holds::{DifferenceKind, RssField, StoredDifference};
use crate::provider_observation::{
    contracts, http, ObservationOutcome, ProviderObservation, ProviderObservationRecorder,
    RequestValidators,
};

/// ADR 0076 accepted value: the minimum interval between two requests to
/// one host.
pub(crate) const MIN_HOST_INTERVAL: Duration = Duration::from_secs(2);

/// ADR 0076 accepted value: the maximum number of hosts that the check
/// sends requests to at the same time.
pub(crate) const MAX_PARALLEL_HOSTS: usize = 4;

const INBOX_CAPACITY: usize = 16;

/// The HTTP layer of the check. Tests inject a fake, so no test sends a
/// network request.
pub(crate) trait RssDocumentFetcher: Send + Sync + 'static {
    /// Send one conditional GET and capture the response.
    fn fetch(&self, feed_url: &str, validators: Option<&RequestValidators>) -> ProviderObservation;
}

/// The production fetcher: the document client of `src/http_client.rs`
/// and the ADR 0075 capture.
struct HttpRssDocumentFetcher;

impl RssDocumentFetcher for HttpRssDocumentFetcher {
    fn fetch(&self, feed_url: &str, validators: Option<&RequestValidators>) -> ProviderObservation {
        http::capture_conditional(
            http::conditional_request(&crate::http_client::document(), feed_url, validators),
            contracts::RSS_DECODER,
        )
    }
}

/// A future that completes at a clock deadline.
pub(crate) type ClockSleep = Pin<Box<dyn Future<Output = ()> + Send>>;

/// The clock of the check. Tests inject a clock that advances without a
/// real wait.
pub(crate) trait CheckClock: Send + Sync + 'static {
    /// Monotonic time since an arbitrary origin.
    fn now(&self) -> Duration;
    /// Complete when `now()` reaches `deadline`.
    fn sleep_until(&self, deadline: Duration) -> ClockSleep;
    /// Wall time in microseconds since the Unix epoch, for stored records.
    fn wall_time_us(&self) -> i64;
}

struct SystemClock {
    origin: tokio::time::Instant,
}

impl CheckClock for SystemClock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }

    fn sleep_until(&self, deadline: Duration) -> ClockSleep {
        Box::pin(tokio::time::sleep_until(self.origin + deadline))
    }

    fn wall_time_us(&self) -> i64 {
        chrono::Utc::now().timestamp_micros()
    }
}

/// The check state of one feed of a run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RssFeedCheck {
    pub feed_id: i64,
    pub title: Option<String>,
    pub feed_url: Option<String>,
    pub host: Option<String>,
    /// `None` while the feed waits for its request.
    pub outcome: Option<RssFeedOutcome>,
    pub http_status: Option<u16>,
    pub observation_id: Option<i64>,
    pub message: Option<String>,
}

impl RssFeedCheck {
    fn waiting(feed: &PlaylistCheckFeed) -> Self {
        Self {
            feed_id: feed.feed_id,
            title: feed.title.clone(),
            feed_url: Some(feed.feed_url.clone()),
            host: feed_host(&feed.feed_url),
            outcome: None,
            http_status: None,
            observation_id: None,
            message: None,
        }
    }
}

/// The state of one run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RssCheckRunState {
    /// The check sends requests.
    Running,
    /// The check has a result for each feed.
    Finished,
    /// The app stopped before the check had a result for each feed.
    Interrupted,
    /// The check could not start. `PlaylistRssRun::error` gives the reason.
    Failed,
}

/// One run of the check for one playlist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaylistRssRun {
    pub run_id: Option<i64>,
    pub playlist_id: i64,
    pub trigger: RssCheckTrigger,
    pub started_at_us: i64,
    pub finished_at_us: Option<i64>,
    pub state: RssCheckRunState,
    pub feeds: Vec<RssFeedCheck>,
    /// Each host that the check stopped after HTTP 429, in stop order.
    pub stopped_hosts: Vec<String>,
    /// Each difference that the check applied, in record order (packet 002).
    pub differences: Vec<StoredDifference>,
    pub error: Option<String>,
}

impl PlaylistRssRun {
    fn starting(playlist_id: i64, trigger: RssCheckTrigger, started_at_us: i64) -> Self {
        Self {
            run_id: None,
            playlist_id,
            trigger,
            started_at_us,
            finished_at_us: None,
            state: RssCheckRunState::Running,
            feeds: Vec::new(),
            stopped_hosts: Vec::new(),
            differences: Vec::new(),
            error: None,
        }
    }

    fn from_stored(run: StoredRun, differences: Vec<StoredDifference>) -> Self {
        let feeds: Vec<RssFeedCheck> = run
            .results
            .into_iter()
            .map(|result| RssFeedCheck {
                feed_id: result.feed_id,
                host: result.feed_url.as_deref().and_then(feed_host),
                title: result.title,
                feed_url: result.feed_url,
                outcome: Some(result.outcome),
                http_status: result.http_status,
                observation_id: result.observation_id,
                message: result.message,
            })
            .collect();
        let mut stopped_hosts = Vec::new();
        for feed in &feeds {
            if feed.http_status == Some(429) {
                if let Some(host) = &feed.host {
                    if !stopped_hosts.contains(host) {
                        stopped_hosts.push(host.clone());
                    }
                }
            }
        }
        Self {
            run_id: Some(run.id),
            playlist_id: run.playlist_id,
            trigger: run.trigger,
            started_at_us: run.started_at_us,
            finished_at_us: run.finished_at_us,
            state: if run.finished_at_us.is_some() {
                RssCheckRunState::Finished
            } else {
                RssCheckRunState::Interrupted
            },
            feeds,
            stopped_hosts,
            differences,
            error: None,
        }
    }

    /// `true` while the check sends requests.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.state == RssCheckRunState::Running
    }

    /// The number of feeds with this outcome.
    #[must_use]
    pub fn count(&self, outcome: RssFeedOutcome) -> usize {
        self.feeds
            .iter()
            .filter(|feed| feed.outcome == Some(outcome))
            .count()
    }

    /// The number of feeds with a result.
    #[must_use]
    pub fn completed(&self) -> usize {
        self.feeds
            .iter()
            .filter(|feed| feed.outcome.is_some())
            .count()
    }
}

/// The latest run of each playlist that this session checked or loaded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlaylistRssCheckSnapshot {
    runs: BTreeMap<i64, PlaylistRssRun>,
    /// The tracks of each loaded playlist with the "removed from feed"
    /// mark, with the check time of the mark (ADR 0076 Decision 7).
    removed_marks: BTreeMap<i64, BTreeMap<i64, i64>>,
    /// Counts each applied document with one or more differences. The
    /// Library view reloads its stored values when this value changes.
    applied_revision: u64,
}

impl PlaylistRssCheckSnapshot {
    /// The latest run of the playlist.
    #[must_use]
    pub fn run(&self, playlist_id: i64) -> Option<&PlaylistRssRun> {
        self.runs.get(&playlist_id)
    }

    /// `true` while a check of the playlist runs.
    #[must_use]
    pub fn is_running(&self, playlist_id: i64) -> bool {
        self.run(playlist_id)
            .is_some_and(PlaylistRssRun::is_running)
    }

    /// Each playlist with a running check. The Library compares two
    /// snapshots with it to find a check that completed (packet 004).
    pub fn running_playlists(&self) -> impl Iterator<Item = i64> + '_ {
        self.runs
            .iter()
            .filter(|(_, run)| run.is_running())
            .map(|(playlist_id, _)| *playlist_id)
    }

    /// The check time of the "removed from feed" mark of a playlist track.
    #[must_use]
    pub fn removed_mark(&self, playlist_id: i64, track_id: i64) -> Option<i64> {
        self.removed_marks
            .get(&playlist_id)
            .and_then(|marks| marks.get(&track_id))
            .copied()
    }

    /// Counts each applied document with one or more differences.
    #[must_use]
    pub fn applied_revision(&self) -> u64 {
        self.applied_revision
    }
}

/// Inbox messages of the actor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaylistRssCheckMessage {
    /// Start a check. A start for a running playlist joins that check.
    Start {
        playlist_id: i64,
        trigger: RssCheckTrigger,
    },
    /// Read the latest stored run of a playlist into the snapshot.
    Load { playlist_id: i64 },
}

/// Caller-side handle of the actor.
#[derive(Clone, Debug)]
pub struct PlaylistRssCheckHandle {
    inbox: mpsc::Sender<PlaylistRssCheckMessage>,
    snapshot: watch::Receiver<PlaylistRssCheckSnapshot>,
}

impl PlaylistRssCheckHandle {
    /// Send `Start`. Returns `false` when the actor stopped or its inbox is full.
    #[must_use]
    pub fn start(&self, playlist_id: i64, trigger: RssCheckTrigger) -> bool {
        self.inbox
            .try_send(PlaylistRssCheckMessage::Start {
                playlist_id,
                trigger,
            })
            .is_ok()
    }

    /// Send `Load`. Returns `false` when the actor stopped or its inbox is full.
    #[must_use]
    pub fn load(&self, playlist_id: i64) -> bool {
        self.inbox
            .try_send(PlaylistRssCheckMessage::Load { playlist_id })
            .is_ok()
    }

    /// Subscribe to snapshots.
    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<PlaylistRssCheckSnapshot> {
        self.snapshot.clone()
    }

    /// The latest snapshot.
    #[must_use]
    pub fn latest(&self) -> PlaylistRssCheckSnapshot {
        self.snapshot.borrow().clone()
    }
}

/// Spawn the actor with the production HTTP layer and clock.
#[must_use]
pub fn spawn(conn: Arc<Mutex<Connection>>, session: &SessionLifecycle) -> PlaylistRssCheckHandle {
    spawn_with(
        conn,
        Arc::new(HttpRssDocumentFetcher),
        Arc::new(SystemClock {
            origin: tokio::time::Instant::now(),
        }),
        session,
    )
}

#[derive(Clone)]
struct CheckContext {
    conn: Arc<Mutex<Connection>>,
    fetcher: Arc<dyn RssDocumentFetcher>,
    clock: Arc<dyn CheckClock>,
    snapshot: Arc<watch::Sender<PlaylistRssCheckSnapshot>>,
}

impl CheckContext {
    fn update_run(&self, playlist_id: i64, update: impl FnOnce(&mut PlaylistRssRun)) {
        self.snapshot.send_modify(|snapshot| {
            if let Some(run) = snapshot.runs.get_mut(&playlist_id) {
                update(run);
            }
        });
    }
}

pub(crate) fn spawn_with(
    conn: Arc<Mutex<Connection>>,
    fetcher: Arc<dyn RssDocumentFetcher>,
    clock: Arc<dyn CheckClock>,
    session: &SessionLifecycle,
) -> PlaylistRssCheckHandle {
    let (snapshot_tx, snapshot_rx) = watch::channel(PlaylistRssCheckSnapshot::default());
    let (inbox_tx, mut inbox_rx) = mpsc::channel(INBOX_CAPACITY);
    let context = CheckContext {
        conn,
        fetcher,
        clock,
        snapshot: Arc::new(snapshot_tx),
    };
    let stop = session.stop_token();
    session.spawn_actor("Playlist RSS check", async move {
        let runs_stop = CancellationToken::new();
        let mut runs = JoinSet::new();
        loop {
            tokio::select! {
                biased;
                () = stop.cancelled() => break,
                message = inbox_rx.recv() => {
                    let Some(message) = message else { break };
                    handle_message(&context, message, &mut runs, &runs_stop);
                }
                Some(_) = runs.join_next(), if !runs.is_empty() => {}
            }
        }
        // Stop each host queue, and wait for the requests in progress. The
        // session drain thus waits until no check writes to the database.
        runs_stop.cancel();
        while runs.join_next().await.is_some() {}
    });
    PlaylistRssCheckHandle {
        inbox: inbox_tx,
        snapshot: snapshot_rx,
    }
}

fn handle_message(
    context: &CheckContext,
    message: PlaylistRssCheckMessage,
    runs: &mut JoinSet<()>,
    runs_stop: &CancellationToken,
) {
    match message {
        PlaylistRssCheckMessage::Start {
            playlist_id,
            trigger,
        } => {
            if context.snapshot.borrow().is_running(playlist_id) {
                // A second start joins the running check (R1-10).
                return;
            }
            let started_at_us = context.clock.wall_time_us();
            context.snapshot.send_modify(|snapshot| {
                snapshot.runs.insert(
                    playlist_id,
                    PlaylistRssRun::starting(playlist_id, trigger, started_at_us),
                );
            });
            runs.spawn(run_check(
                context.clone(),
                playlist_id,
                trigger,
                started_at_us,
                runs_stop.child_token(),
            ));
        }
        PlaylistRssCheckMessage::Load { playlist_id } => {
            if context.snapshot.borrow().run(playlist_id).is_some()
                && context
                    .snapshot
                    .borrow()
                    .removed_marks
                    .contains_key(&playlist_id)
            {
                return;
            }
            let context = context.clone();
            runs.spawn(async move {
                let conn = Arc::clone(&context.conn);
                let stored = blocking(move || {
                    let conn = lock(&conn)?;
                    let run = store::latest_run(&conn, playlist_id)
                        .map_err(|error| format!("{error:#}"))?;
                    let differences = match &run {
                        Some(run) => {
                            crate::db::rss_field_holds::run_differences(&conn, run.id, None)
                                .map_err(|error| format!("{error:#}"))?
                        }
                        None => Vec::new(),
                    };
                    let marks =
                        crate::db::rss_field_holds::playlist_removed_marks(&conn, playlist_id)
                            .map_err(|error| format!("{error:#}"))?;
                    Ok((run, differences, marks))
                })
                .await;
                if let Ok((stored, differences, marks)) = stored {
                    context.snapshot.send_if_modified(|snapshot| {
                        snapshot
                            .removed_marks
                            .insert(playlist_id, marks.into_iter().collect());
                        if let Some(stored) = stored {
                            snapshot.runs.entry(playlist_id).or_insert_with(|| {
                                PlaylistRssRun::from_stored(stored, differences)
                            });
                        }
                        true
                    });
                }
            });
        }
    }
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| "The background task of the RSS check stopped unexpectedly.".to_owned())?
}

fn lock(conn: &Mutex<Connection>) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
    conn.lock()
        .map_err(|_| "The database connection is not available.".to_owned())
}

async fn run_check(
    context: CheckContext,
    playlist_id: i64,
    trigger: RssCheckTrigger,
    started_at_us: i64,
    stop: CancellationToken,
) {
    let conn = Arc::clone(&context.conn);
    let prepared = blocking(move || {
        let conn = lock(&conn)?;
        let feeds = store::playlist_check_feeds(&conn, playlist_id)
            .map_err(|error| format!("{error:#}"))?;
        let run_id = store::insert_run(&conn, playlist_id, trigger, started_at_us)
            .map_err(|error| format!("{error:#}"))?;
        Ok((run_id, feeds))
    })
    .await;
    let (run_id, feeds) = match prepared {
        Ok(prepared) => prepared,
        Err(error) => {
            let finished_at_us = context.clock.wall_time_us();
            context.update_run(playlist_id, |run| {
                run.state = RssCheckRunState::Failed;
                run.finished_at_us = Some(finished_at_us);
                run.error = Some(format!(
                    "The app could not start the RSS check, and it sent no request. Database: {error}"
                ));
            });
            return;
        }
    };
    context.update_run(playlist_id, |run| {
        run.run_id = Some(run_id);
        run.feeds = feeds.iter().map(RssFeedCheck::waiting).collect();
    });

    let mut queues: Vec<(String, Vec<PlaylistCheckFeed>)> = Vec::new();
    for feed in feeds {
        match feed_host(&feed.feed_url) {
            Some(host) => match queues.iter_mut().find(|(queued, _)| *queued == host) {
                Some((_, queue)) => queue.push(feed),
                None => queues.push((host, vec![feed])),
            },
            None => {
                record(
                    &context,
                    playlist_id,
                    run_id,
                    feed.feed_id,
                    FeedFetch::not_checked(
                        "Not checked. The feed has no HTTP or HTTPS address.".to_owned(),
                    ),
                )
                .await;
            }
        }
    }

    let permits = Arc::new(Semaphore::new(MAX_PARALLEL_HOSTS));
    let mut hosts = JoinSet::new();
    for (host, queue) in queues {
        hosts.spawn(host_queue(
            context.clone(),
            playlist_id,
            run_id,
            host,
            queue,
            Arc::clone(&permits),
            stop.clone(),
        ));
    }
    while hosts.join_next().await.is_some() {}

    if stop.is_cancelled() {
        context.update_run(playlist_id, |run| {
            run.state = RssCheckRunState::Interrupted;
        });
        return;
    }
    let finished_at_us = context.clock.wall_time_us();
    let conn = Arc::clone(&context.conn);
    let finished = blocking(move || {
        let conn = lock(&conn)?;
        store::finish_run(&conn, run_id, finished_at_us).map_err(|error| format!("{error:#}"))
    })
    .await;
    context.update_run(playlist_id, |run| {
        run.state = RssCheckRunState::Finished;
        run.finished_at_us = Some(finished_at_us.max(run.started_at_us));
        if let Err(error) = finished {
            run.error = Some(format!(
                "The app could not store the finish time of the RSS check. Database: {error}"
            ));
        }
    });
}

async fn host_queue(
    context: CheckContext,
    playlist_id: i64,
    run_id: i64,
    host: String,
    feeds: Vec<PlaylistCheckFeed>,
    permits: Arc<Semaphore>,
    stop: CancellationToken,
) {
    let permit = tokio::select! {
        biased;
        () = stop.cancelled() => return,
        permit = permits.acquire_owned() => permit,
    };
    let Ok(_permit) = permit else {
        return;
    };
    let mut next_request_at: Option<Duration> = None;
    let mut stopped = false;
    for feed in feeds {
        if stopped {
            record(
                &context,
                playlist_id,
                run_id,
                feed.feed_id,
                FeedFetch::not_checked(format!(
                    "Not checked. {host} sent HTTP 429 (Too Many Requests) earlier in this check. The check sent no more requests to {host}."
                )),
            )
            .await;
            continue;
        }
        if let Some(deadline) = next_request_at {
            tokio::select! {
                biased;
                () = stop.cancelled() => return,
                () = context.clock.sleep_until(deadline) => {}
            }
        }
        if stop.is_cancelled() {
            return;
        }
        let started_at_us = context.clock.wall_time_us();
        let conn = Arc::clone(&context.conn);
        let fetcher = Arc::clone(&context.fetcher);
        let url = feed.feed_url.clone();
        let fetch_host = host.clone();
        let feed_id = feed.feed_id;
        let fetch = blocking(move || {
            Ok(check_feed(
                &conn,
                fetcher.as_ref(),
                FeedRequest {
                    run_id,
                    feed_id,
                    feed_url: &url,
                    host: &fetch_host,
                    started_at_us,
                },
            ))
        })
        .await
        .unwrap_or_else(|error| FeedFetch::failed(None, None, error));
        let answered_at = context.clock.now();
        next_request_at =
            Some(answered_at + fetch.retry_after.unwrap_or_default().max(MIN_HOST_INTERVAL));
        if fetch.http_status == Some(429) {
            stopped = true;
            let stopped_host = host.clone();
            context.update_run(playlist_id, |run| {
                if !run.stopped_hosts.contains(&stopped_host) {
                    run.stopped_hosts.push(stopped_host);
                }
            });
        }
        record(&context, playlist_id, run_id, feed.feed_id, fetch).await;
    }
}

/// The result of one feed request.
#[derive(Clone, Debug, PartialEq, Eq)]
struct FeedFetch {
    outcome: RssFeedOutcome,
    http_status: Option<u16>,
    observation_id: Option<i64>,
    retry_after: Option<Duration>,
    message: Option<String>,
    /// The number of differences that the apply of packet 002 recorded.
    differences: usize,
}

impl FeedFetch {
    fn not_checked(message: String) -> Self {
        Self {
            outcome: RssFeedOutcome::NotChecked,
            http_status: None,
            observation_id: None,
            retry_after: None,
            message: Some(message),
            differences: 0,
        }
    }

    fn failed(http_status: Option<u16>, observation_id: Option<i64>, message: String) -> Self {
        Self {
            outcome: RssFeedOutcome::Failed,
            http_status,
            observation_id,
            retry_after: None,
            message: Some(message),
            differences: 0,
        }
    }
}

async fn record(
    context: &CheckContext,
    playlist_id: i64,
    run_id: i64,
    feed_id: i64,
    fetch: FeedFetch,
) {
    let conn = Arc::clone(&context.conn);
    let stored = {
        let fetch = fetch.clone();
        blocking(move || {
            let conn = lock(&conn)?;
            store::insert_feed_result(
                &conn,
                run_id,
                feed_id,
                fetch.outcome,
                fetch.http_status,
                fetch.observation_id,
                fetch.message.as_deref(),
            )
            .map_err(|error| format!("{error:#}"))?;
            if fetch.differences == 0 {
                return Ok(None);
            }
            let differences =
                crate::db::rss_field_holds::run_differences(&conn, run_id, Some(feed_id))
                    .map_err(|error| format!("{error:#}"))?;
            let marks = crate::db::rss_field_holds::playlist_removed_marks(&conn, playlist_id)
                .map_err(|error| format!("{error:#}"))?;
            Ok(Some((differences, marks)))
        })
        .await
    };
    context.snapshot.send_modify(|snapshot| {
        if let Ok(Some((_, marks))) = &stored {
            snapshot
                .removed_marks
                .insert(playlist_id, marks.iter().copied().collect());
            snapshot.applied_revision += 1;
        }
        let Some(run) = snapshot.runs.get_mut(&playlist_id) else {
            return;
        };
        if let Some(feed) = run.feeds.iter_mut().find(|feed| feed.feed_id == feed_id) {
            feed.outcome = Some(fetch.outcome);
            feed.http_status = fetch.http_status;
            feed.observation_id = fetch.observation_id;
            feed.message = fetch.message;
        }
        match stored {
            Ok(Some((differences, _))) => run.differences.extend(differences),
            Ok(None) => {}
            Err(error) => {
                run.error = Some(format!(
                    "The app could not store a feed result of the RSS check. Database: {error}"
                ));
            }
        }
    });
}

/// The inputs of one feed request.
#[derive(Clone, Copy)]
struct FeedRequest<'a> {
    run_id: i64,
    feed_id: i64,
    feed_url: &'a str,
    host: &'a str,
    started_at_us: i64,
}

/// Send one request, record its observation and apply a received
/// document (packet 002). Blocking.
fn check_feed(
    conn: &Arc<Mutex<Connection>>,
    fetcher: &dyn RssDocumentFetcher,
    request: FeedRequest<'_>,
) -> FeedFetch {
    let FeedRequest {
        run_id,
        feed_id,
        feed_url,
        host,
        started_at_us,
    } = request;
    let mut spec = contracts::rss_document(feed_url);
    spec.started_at_us = started_at_us;
    // A validator that the app cannot read only removes the condition from
    // the request. The request still runs.
    let validators = conn.lock().ok().and_then(|conn| {
        crate::db::provider_observations::read_request_validators(&conn, &spec)
            .ok()
            .flatten()
    });
    let recorder = ProviderObservationRecorder::new(Arc::clone(conn));
    let token = match recorder.begin(spec) {
        Ok(token) => token,
        Err(error) => return FeedFetch::failed(None, None, error.to_string()),
    };
    let mut observation = fetcher.fetch(feed_url, validators.as_ref());
    let http_status = observation.http_status;
    let retry_after = retry_after(&observation.occurrence);
    let success = observation.outcome == ObservationOutcome::Success;
    let not_modified = success && http_status == Some(304);
    let document = success && http_status.is_some_and(|status| (200..300).contains(&status));
    let document = document && crate::rss::check_rss_document(&mut observation).is_ok();
    let reason = observation
        .failure
        .as_ref()
        .and_then(|failure| failure["reason"].as_str())
        .map(str::to_owned);
    let retained = document.then(|| observation.clone());
    let body = document.then(|| observation.body.clone()).flatten();
    let receipt = match recorder.record(token, observation) {
        Ok(receipt) => receipt,
        Err(error) => {
            let mut fetch = FeedFetch::failed(
                http_status,
                None,
                format!("The app received a response, but it could not store it. {error}"),
            );
            fetch.retry_after = retry_after;
            return fetch;
        }
    };
    let observation_id = Some(receipt.observation_id);
    if let Some(observation) = retained {
        crate::rss::retain_checked_document(feed_url, &observation, receipt);
    }
    let mut fetch = if document {
        // ADR 0076 packet 002: compare the document with the stored values
        // of the feed and apply each difference in one transaction. A
        // failure changes no stored value.
        let applied = body.map_or_else(
            || Err("The response has no document body.".to_owned()),
            |body| {
                let conn = lock(conn)?;
                crate::rss::check_apply::apply_checked_document(
                    &conn,
                    run_id,
                    feed_id,
                    &body,
                    started_at_us,
                )
                .map_err(|error| format!("{error:#}"))
            },
        );
        let (differences, message) = match applied {
            Ok(applied) => (applied.differences, None),
            Err(error) => (
                0,
                Some(format!(
                    "The app received the RSS document, but it could not compare it with the stored values. No stored value changed. {error}"
                )),
            ),
        };
        FeedFetch {
            outcome: RssFeedOutcome::Document,
            http_status,
            observation_id,
            retry_after: None,
            message,
            differences,
        }
    } else if not_modified {
        FeedFetch {
            outcome: RssFeedOutcome::NotModified,
            http_status,
            observation_id,
            retry_after: None,
            message: None,
            differences: 0,
        }
    } else {
        FeedFetch::failed(
            http_status,
            observation_id,
            failure_message(reason.as_deref(), http_status, host),
        )
    };
    fetch.retry_after = retry_after;
    fetch
}

fn failure_message(reason: Option<&str>, http_status: Option<u16>, host: &str) -> String {
    match (reason, http_status) {
        (Some("transport"), _) => {
            format!("The app could not connect to {host}, or {host} did not reply in time.")
        }
        (Some("body_read"), _) => format!("The response from {host} stopped before its end."),
        (Some("xml_decode"), _) => {
            format!("{host} sent a response that is not a readable RSS document.")
        }
        (_, Some(429)) => format!(
            "{host} sent HTTP 429 (Too Many Requests). The check sent no more requests to {host}."
        ),
        (_, Some(status)) => format!("{host} sent HTTP {status}."),
        (_, None) => format!("The request to {host} failed."),
    }
}

/// Read `Retry-After` from the captured headers: a number of seconds, or
/// an HTTP date.
fn retry_after(occurrence: &Value) -> Option<Duration> {
    use base64::Engine;
    let encoded = occurrence["headers"]["retry-after"].get(0)?.as_str()?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    let text = String::from_utf8(bytes).ok()?;
    let text = text.trim();
    if let Ok(seconds) = text.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    let at = chrono::DateTime::parse_from_rfc2822(text).ok()?;
    let wait = at.with_timezone(&chrono::Utc) - chrono::Utc::now();
    Some(wait.to_std().unwrap_or_default())
}

/// The lowercase host of an HTTP or HTTPS feed URL.
pub(crate) fn feed_host(feed_url: &str) -> Option<String> {
    let url = reqwest::Url::parse(feed_url.trim()).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    url.host_str()
        .filter(|host| !host.is_empty())
        .map(str::to_ascii_lowercase)
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Injected clock and HTTP layer for tests of the check.

    use super::*;
    use std::sync::Condvar;

    /// A snapshot with one running run of the playlist: one feed has a
    /// result and one feed waits.
    pub(crate) fn running_snapshot(playlist_id: i64) -> PlaylistRssCheckSnapshot {
        let mut run =
            PlaylistRssRun::starting(playlist_id, RssCheckTrigger::Button, 1_790_000_000_000_000);
        run.run_id = Some(1);
        run.feeds = vec![RssFeedCheck {
            feed_id: 1,
            title: Some("Album".into()),
            feed_url: Some("https://feed.test/album.xml".into()),
            host: Some("feed.test".into()),
            outcome: None,
            http_status: None,
            observation_id: None,
            message: None,
        }];
        let mut snapshot = PlaylistRssCheckSnapshot::default();
        snapshot.runs.insert(playlist_id, run);
        snapshot
    }

    /// A snapshot with one finished run of the playlist, its differences
    /// and the "removed from feed" marks (ADR 0076 packet 002).
    pub(crate) fn snapshot_with_differences(
        playlist_id: i64,
        differences: Vec<StoredDifference>,
        marks: &[(i64, i64)],
    ) -> PlaylistRssCheckSnapshot {
        let mut snapshot = running_snapshot(playlist_id);
        if let Some(run) = snapshot.runs.get_mut(&playlist_id) {
            run.state = RssCheckRunState::Finished;
            run.finished_at_us = Some(run.started_at_us + 1);
            run.differences = differences;
        }
        snapshot
            .removed_marks
            .insert(playlist_id, marks.iter().copied().collect());
        snapshot
    }

    /// A clock that jumps to each deadline without a real wait.
    pub(crate) struct FakeClock {
        now: Mutex<Duration>,
    }

    impl FakeClock {
        pub(crate) fn new() -> Arc<Self> {
            Arc::new(Self {
                now: Mutex::new(Duration::ZERO),
            })
        }
    }

    impl CheckClock for FakeClock {
        fn now(&self) -> Duration {
            *self.now.lock().unwrap()
        }

        fn sleep_until(&self, deadline: Duration) -> ClockSleep {
            {
                let mut now = self.now.lock().unwrap();
                if deadline > *now {
                    *now = deadline;
                }
            }
            Box::pin(tokio::task::yield_now())
        }

        fn wall_time_us(&self) -> i64 {
            1_790_000_000_000_000 + i64::try_from(self.now().as_micros()).unwrap_or(i64::MAX / 2)
        }
    }

    /// One request that the fake HTTP layer received.
    #[derive(Clone, Debug)]
    pub(crate) struct SentRequest {
        pub(crate) url: String,
        pub(crate) validators: Option<RequestValidators>,
        pub(crate) at: Duration,
    }

    /// A fake HTTP layer. Each response comes from `respond`. An optional
    /// gate holds each request until the test opens it.
    pub(crate) struct FakeFetcher {
        clock: Arc<FakeClock>,
        respond: Box<dyn Fn(&str) -> ProviderObservation + Send + Sync>,
        pub(crate) sent: Mutex<Vec<SentRequest>>,
        gate: Mutex<bool>,
        opened: Condvar,
        active: Mutex<(usize, usize)>,
    }

    impl FakeFetcher {
        pub(crate) fn new(
            clock: Arc<FakeClock>,
            respond: impl Fn(&str) -> ProviderObservation + Send + Sync + 'static,
        ) -> Arc<Self> {
            Arc::new(Self {
                clock,
                respond: Box::new(respond),
                sent: Mutex::new(Vec::new()),
                gate: Mutex::new(true),
                opened: Condvar::new(),
                active: Mutex::new((0, 0)),
            })
        }

        pub(crate) fn close_gate(&self) {
            *self.gate.lock().unwrap() = false;
        }

        pub(crate) fn open_gate(&self) {
            *self.gate.lock().unwrap() = true;
            self.opened.notify_all();
        }

        /// The number of requests in progress, and the maximum number.
        pub(crate) fn active(&self) -> (usize, usize) {
            *self.active.lock().unwrap()
        }

        pub(crate) fn sent(&self) -> Vec<SentRequest> {
            self.sent.lock().unwrap().clone()
        }
    }

    impl RssDocumentFetcher for FakeFetcher {
        fn fetch(
            &self,
            feed_url: &str,
            validators: Option<&RequestValidators>,
        ) -> ProviderObservation {
            self.sent.lock().unwrap().push(SentRequest {
                url: feed_url.to_owned(),
                validators: validators.cloned(),
                at: self.clock.now(),
            });
            {
                let mut active = self.active.lock().unwrap();
                active.0 += 1;
                active.1 = active.1.max(active.0);
            }
            // The gate opens itself after 10 seconds, so a failed test
            // cannot hold the runtime shutdown for ever.
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            let mut open = self.gate.lock().unwrap();
            while !*open && std::time::Instant::now() < deadline {
                open = self
                    .opened
                    .wait_timeout(open, Duration::from_millis(100))
                    .unwrap()
                    .0;
            }
            drop(open);
            let observation = (self.respond)(feed_url);
            self.active.lock().unwrap().0 -= 1;
            observation
        }
    }

    /// A captured response with a status, headers and a body.
    pub(crate) fn response(
        url: &str,
        status: u16,
        headers: &[(&str, &str)],
        body: Option<&[u8]>,
    ) -> ProviderObservation {
        use base64::Engine;
        let mut captured = serde_json::Map::new();
        for (name, value) in headers {
            captured.insert(
                (*name).to_owned(),
                serde_json::json!([base64::engine::general_purpose::STANDARD.encode(value)]),
            );
        }
        let success = (200..300).contains(&status) || status == 304;
        ProviderObservation {
            body: body.map(Arc::from),
            http_status: Some(status),
            response_uri: Some(url.to_owned()),
            interpretation: serde_json::json!({"version":1,"media_type":"application/rss+xml","charset":null,"retained_body_codings":[],"body_state":if body.is_some() {"complete"} else {"absent"},"effective_base_uri":null}),
            source_revision: None,
            source_times: serde_json::json!({}),
            decoder_version: contracts::RSS_DECODER.into(),
            outcome: if success {
                ObservationOutcome::Success
            } else {
                ObservationOutcome::Failed
            },
            failure: (!success).then(|| serde_json::json!({"reason":"http_status"})),
            finished_at_us: 1_790_000_000_000_000,
            fetched_at_us: Some(1_790_000_000_000_000),
            occurrence: serde_json::json!({"version":1,"headers":captured}),
            coverage: Vec::new(),
        }
    }

    /// A transport failure: no response.
    pub(crate) fn no_response() -> ProviderObservation {
        let mut observation = response("https://unused.invalid/", 200, &[], None);
        observation.http_status = None;
        observation.response_uri = None;
        observation.fetched_at_us = None;
        observation.fail("transport");
        observation
    }

    pub(crate) const RSS: &[u8] =
        b"<?xml version=\"1.0\"?><rss version=\"2.0\"><channel><title>A</title></channel></rss>";

    /// A database with one playlist and the given feed URLs. Each URL gets
    /// one track; `repeat` gives the number of tracks of each feed.
    pub(crate) fn database(feeds: &[(&str, usize)]) -> (Arc<Mutex<Connection>>, i64, Vec<i64>) {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        crate::db::init_schema(&conn).unwrap();
        crate::db::migrate_schema(&conn).unwrap();
        let playlist_id = crate::db::playlist_create(&conn, "Show").unwrap();
        let mut feed_ids = Vec::new();
        for (index, (url, repeat)) in feeds.iter().enumerate() {
            conn.execute(
                "INSERT INTO feeds(feed_url, feed_guid, title) VALUES (?1, ?2, ?3)",
                rusqlite::params![url, format!("feed-guid-{index}"), format!("Album {index}")],
            )
            .unwrap();
            let feed_id = conn.last_insert_rowid();
            feed_ids.push(feed_id);
            for item in 0..*repeat {
                conn.execute(
                    "INSERT INTO tracks(feed_id, item_guid, track_title) VALUES (?1, ?2, ?3)",
                    rusqlite::params![feed_id, format!("{index}-{item}"), format!("Track {item}")],
                )
                .unwrap();
                let track_id = conn.last_insert_rowid();
                crate::db::playlist_append(&conn, playlist_id, track_id).unwrap();
            }
        }
        (Arc::new(Mutex::new(conn)), playlist_id, feed_ids)
    }

    /// Wait until the run of the playlist is not running.
    pub(crate) async fn finished(
        handle: &PlaylistRssCheckHandle,
        playlist_id: i64,
    ) -> PlaylistRssRun {
        finished_after(handle, playlist_id, None).await
    }

    /// Wait until a run other than `previous` is not running.
    pub(crate) async fn finished_after(
        handle: &PlaylistRssCheckHandle,
        playlist_id: i64,
        previous: Option<i64>,
    ) -> PlaylistRssRun {
        let mut receiver = handle.subscribe();
        loop {
            if let Some(run) = receiver.borrow().run(playlist_id) {
                if !run.is_running() && (previous.is_none() || run.run_id != previous) {
                    return run.clone();
                }
            }
            tokio::time::timeout(Duration::from_secs(10), receiver.changed())
                .await
                .expect("the check finishes")
                .expect("the actor keeps its snapshot channel");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;

    fn start(
        conn: &Arc<Mutex<Connection>>,
        fetcher: &Arc<FakeFetcher>,
        clock: &Arc<FakeClock>,
        session: &SessionLifecycle,
    ) -> PlaylistRssCheckHandle {
        spawn_with(
            Arc::clone(conn),
            Arc::clone(fetcher) as Arc<dyn RssDocumentFetcher>,
            Arc::clone(clock) as Arc<dyn CheckClock>,
            session,
        )
    }

    fn ok(url: &str) -> ProviderObservation {
        response(url, 200, &[], Some(RSS))
    }

    fn url_count(sent: &[SentRequest], url: &str) -> usize {
        sent.iter().filter(|request| request.url == url).count()
    }

    fn observation_count(conn: &Arc<Mutex<Connection>>) -> i64 {
        conn.lock()
            .unwrap()
            .query_row("SELECT count(*) FROM metadata_observations", [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    #[tokio::test(flavor = "current_thread")]
    async fn adr_0076_playlist_check_three_feeds_one_repeated_send_three_requests() {
        let urls = [
            "https://a.test/r101/feed.xml",
            "https://b.test/r101/feed.xml",
            "https://c.test/r101/feed.xml",
        ];
        let (conn, playlist_id, _) = database(&[(urls[0], 2), (urls[1], 1), (urls[2], 1)]);
        let clock = FakeClock::new();
        let fetcher = FakeFetcher::new(Arc::clone(&clock), ok);
        let session = SessionLifecycle::new();
        let handle = start(&conn, &fetcher, &clock, &session);
        assert!(handle.start(playlist_id, RssCheckTrigger::Button));
        let run = finished(&handle, playlist_id).await;
        let sent = fetcher.sent();
        assert_eq!(sent.len(), 3);
        for url in urls {
            assert_eq!(url_count(&sent, url), 1);
        }
        assert_eq!(run.feeds.len(), 3);
        assert_eq!(run.count(RssFeedOutcome::Document), 3);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn adr_0076_playlist_check_same_host_requests_wait_two_seconds() {
        let (conn, playlist_id, _) = database(&[
            ("https://host.test/r102/one.xml", 1),
            ("https://host.test/r102/two.xml", 1),
        ]);
        let clock = FakeClock::new();
        let fetcher = FakeFetcher::new(Arc::clone(&clock), ok);
        let session = SessionLifecycle::new();
        let handle = start(&conn, &fetcher, &clock, &session);
        assert!(handle.start(playlist_id, RssCheckTrigger::Button));
        finished(&handle, playlist_id).await;
        let sent = fetcher.sent();
        assert_eq!(sent.len(), 2);
        assert!(sent[1].at >= sent[0].at + MIN_HOST_INTERVAL);
        assert_eq!(MIN_HOST_INTERVAL, Duration::from_secs(2));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn adr_0076_playlist_check_six_hosts_run_at_most_four_requests() {
        let urls: Vec<String> = (0..6)
            .map(|index| format!("https://host{index}.test/r103/feed.xml"))
            .collect();
        let feeds: Vec<(&str, usize)> = urls.iter().map(|url| (url.as_str(), 1)).collect();
        let (conn, playlist_id, _) = database(&feeds);
        let clock = FakeClock::new();
        let fetcher = FakeFetcher::new(Arc::clone(&clock), ok);
        fetcher.close_gate();
        let session = SessionLifecycle::new();
        let handle = start(&conn, &fetcher, &clock, &session);
        assert!(handle.start(playlist_id, RssCheckTrigger::Button));
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while fetcher.active().0 < MAX_PARALLEL_HOSTS {
            assert!(std::time::Instant::now() < deadline, "four requests start");
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        // The two other hosts wait for a free host slot.
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(fetcher.active().0, MAX_PARALLEL_HOSTS);
        assert_eq!(fetcher.sent().len(), MAX_PARALLEL_HOSTS);
        fetcher.open_gate();
        let run = finished(&handle, playlist_id).await;
        assert_eq!(fetcher.sent().len(), 6);
        assert_eq!(fetcher.active().1, MAX_PARALLEL_HOSTS);
        assert_eq!(MAX_PARALLEL_HOSTS, 4);
        assert_eq!(run.count(RssFeedOutcome::Document), 6);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn adr_0076_playlist_check_retry_after_delays_the_next_request() {
        let first = "https://slow.test/r104/one.xml";
        let (conn, playlist_id, _) = database(&[(first, 1), ("https://slow.test/r104/two.xml", 1)]);
        let clock = FakeClock::new();
        let fetcher = FakeFetcher::new(Arc::clone(&clock), move |url| {
            if url == first {
                response(url, 200, &[("retry-after", "30")], Some(RSS))
            } else {
                ok(url)
            }
        });
        let session = SessionLifecycle::new();
        let handle = start(&conn, &fetcher, &clock, &session);
        assert!(handle.start(playlist_id, RssCheckTrigger::Button));
        finished(&handle, playlist_id).await;
        let sent = fetcher.sent();
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[1].at - sent[0].at, Duration::from_secs(30));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn adr_0076_playlist_check_http_429_stops_the_host() {
        let limited = [
            "https://limited.test/r105/one.xml",
            "https://limited.test/r105/two.xml",
            "https://limited.test/r105/three.xml",
        ];
        let other = "https://other.test/r105/feed.xml";
        let (conn, playlist_id, feed_ids) = database(&[
            (limited[0], 1),
            (limited[1], 1),
            (limited[2], 1),
            (other, 1),
        ]);
        let clock = FakeClock::new();
        let fetcher = FakeFetcher::new(Arc::clone(&clock), |url| {
            if url.contains("limited.test") {
                response(url, 429, &[("retry-after", "300")], Some(b""))
            } else {
                ok(url)
            }
        });
        let session = SessionLifecycle::new();
        let handle = start(&conn, &fetcher, &clock, &session);
        assert!(handle.start(playlist_id, RssCheckTrigger::Button));
        let run = finished(&handle, playlist_id).await;
        let sent = fetcher.sent();
        assert_eq!(
            sent.iter()
                .filter(|request| request.url.contains("limited.test"))
                .count(),
            1
        );
        assert_eq!(url_count(&sent, other), 1);
        assert_eq!(run.stopped_hosts, ["limited.test"]);
        let outcome = |feed_id: i64| {
            run.feeds
                .iter()
                .find(|feed| feed.feed_id == feed_id)
                .unwrap()
                .outcome
        };
        assert_eq!(outcome(feed_ids[0]), Some(RssFeedOutcome::Failed));
        assert_eq!(outcome(feed_ids[1]), Some(RssFeedOutcome::NotChecked));
        assert_eq!(outcome(feed_ids[2]), Some(RssFeedOutcome::NotChecked));
        assert_eq!(outcome(feed_ids[3]), Some(RssFeedOutcome::Document));
    }

    #[test]
    fn adr_0076_playlist_check_request_carries_stored_validators_only() {
        let client = crate::http_client::document();
        let url = "https://validators.test/r106/feed.xml";
        let validators = RequestValidators {
            etag: Some("\"v1\"".into()),
            last_modified: Some("Wed, 23 Sep 2026 10:00:00 GMT".into()),
        };
        let request = http::conditional_request(&client, url, Some(&validators))
            .build()
            .unwrap();
        assert_eq!(request.headers()["if-none-match"], "\"v1\"");
        assert_eq!(
            request.headers()["if-modified-since"],
            "Wed, 23 Sep 2026 10:00:00 GMT"
        );
        let request = http::conditional_request(&client, url, None)
            .build()
            .unwrap();
        assert!(request.headers().get("if-none-match").is_none());
        assert!(request.headers().get("if-modified-since").is_none());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn adr_0076_playlist_check_stored_validators_reach_the_second_request() {
        let url = "https://validators.test/r106/stored.xml";
        let (conn, playlist_id, _) = database(&[(url, 1)]);
        let clock = FakeClock::new();
        let fetcher = FakeFetcher::new(Arc::clone(&clock), |url| {
            response(
                url,
                200,
                &[
                    ("etag", "\"v1\""),
                    ("last-modified", "Wed, 23 Sep 2026 10:00:00 GMT"),
                ],
                Some(RSS),
            )
        });
        let session = SessionLifecycle::new();
        let handle = start(&conn, &fetcher, &clock, &session);
        assert!(handle.start(playlist_id, RssCheckTrigger::Button));
        let first = finished(&handle, playlist_id).await;
        assert!(handle.start(playlist_id, RssCheckTrigger::Button));
        finished_after(&handle, playlist_id, first.run_id).await;
        let sent = fetcher.sent();
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[0].validators, None);
        assert_eq!(
            sent[1].validators,
            Some(RequestValidators {
                etag: Some("\"v1\"".into()),
                last_modified: Some("Wed, 23 Sep 2026 10:00:00 GMT".into()),
            })
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn adr_0076_playlist_check_not_modified_keeps_validators() {
        let url = "https://cdn.test/r107/feed.xml";
        let (conn, playlist_id, _) = database(&[(url, 1)]);
        let clock = FakeClock::new();
        let calls = Arc::new(Mutex::new(0_usize));
        let counter = Arc::clone(&calls);
        let fetcher = FakeFetcher::new(Arc::clone(&clock), move |url| {
            let mut calls = counter.lock().unwrap();
            *calls += 1;
            if *calls == 1 {
                response(
                    url,
                    200,
                    &[
                        ("etag", "\"v1\""),
                        ("last-modified", "Wed, 23 Sep 2026 10:00:00 GMT"),
                    ],
                    Some(RSS),
                )
            } else {
                // A 304 without Last-Modified and with a different ETag.
                let mut observation = response(url, 304, &[("etag", "\"other\"")], None);
                observation.fetched_at_us = Some(1_790_000_000_000_001);
                observation
            }
        });
        let session = SessionLifecycle::new();
        let handle = start(&conn, &fetcher, &clock, &session);
        assert!(handle.start(playlist_id, RssCheckTrigger::Button));
        let first = finished(&handle, playlist_id).await;
        let spec = contracts::rss_document(url);
        let before =
            crate::db::provider_observations::read_request_validators(&conn.lock().unwrap(), &spec)
                .unwrap();
        assert!(handle.start(playlist_id, RssCheckTrigger::Button));
        let run = finished_after(&handle, playlist_id, first.run_id).await;
        let feed = &run.feeds[0];
        assert_eq!(feed.outcome, Some(RssFeedOutcome::NotModified));
        assert_eq!(feed.http_status, Some(304));
        let observation_id = feed.observation_id.unwrap();
        let (status, body, outcome): (Option<i64>, Option<String>, String) = conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT http_status, body_sha256, outcome FROM metadata_observations WHERE id = ?1",
                [observation_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(status, Some(304));
        assert_eq!(body, None);
        assert_eq!(outcome, "success");
        let after =
            crate::db::provider_observations::read_request_validators(&conn.lock().unwrap(), &spec)
                .unwrap();
        assert!(before.is_some());
        assert_eq!(after, before);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn adr_0076_playlist_check_document_replaces_the_retained_document() {
        let url = "https://fresh.test/r108/feed.xml";
        let (conn, playlist_id, _) = database(&[(url, 1)]);
        let clock = FakeClock::new();
        let fetcher = FakeFetcher::new(Arc::clone(&clock), ok);
        let session = SessionLifecycle::new();
        let handle = start(&conn, &fetcher, &clock, &session);
        assert!(handle.start(playlist_id, RssCheckTrigger::Button));
        let run = finished(&handle, playlist_id).await;
        let feed = &run.feeds[0];
        assert_eq!(feed.outcome, Some(RssFeedOutcome::Document));
        let observation_id = feed.observation_id.unwrap();
        let (status, body): (Option<i64>, Option<Vec<u8>>) = conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT o.http_status, b.bytes FROM metadata_observations o LEFT JOIN metadata_bodies b ON b.sha256 = o.body_sha256 WHERE o.id = ?1",
                [observation_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, Some(200));
        assert_eq!(body.as_deref(), Some(RSS));
        let (bytes, receipt) = crate::rss::retained_document_for_test(url).unwrap();
        assert_eq!(&*bytes, RSS);
        assert_eq!(receipt.unwrap().observation_id, observation_id);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn adr_0076_playlist_check_failure_changes_no_feed_or_track_column() {
        let refused = "https://down.test/r109/feed.xml";
        let broken = "https://broken.test/r109/feed.xml";
        let (conn, playlist_id, _) = database(&[(refused, 1), (broken, 1)]);
        let snapshot = |conn: &Arc<Mutex<Connection>>| -> Vec<String> {
            let conn = conn.lock().unwrap();
            let mut rows = Vec::new();
            for table in ["feeds", "tracks"] {
                let mut statement = conn
                    .prepare(&format!("SELECT * FROM {table} ORDER BY id"))
                    .unwrap();
                let columns = statement.column_count();
                let mut query = statement.query([]).unwrap();
                while let Some(row) = query.next().unwrap() {
                    rows.push(
                        (0..columns)
                            .map(|index| format!("{:?}", row.get_ref(index).unwrap()))
                            .collect::<Vec<_>>()
                            .join("|"),
                    );
                }
            }
            rows
        };
        let before = snapshot(&conn);
        let clock = FakeClock::new();
        let fetcher = FakeFetcher::new(Arc::clone(&clock), move |url| {
            if url == refused {
                no_response()
            } else {
                response(url, 200, &[], Some(b"<html>not rss</html>"))
            }
        });
        let session = SessionLifecycle::new();
        let handle = start(&conn, &fetcher, &clock, &session);
        assert!(handle.start(playlist_id, RssCheckTrigger::Button));
        let run = finished(&handle, playlist_id).await;
        assert_eq!(run.count(RssFeedOutcome::Failed), 2);
        for feed in &run.feeds {
            let observation_id = feed.observation_id.expect("failed request is observed");
            let outcome: String = conn
                .lock()
                .unwrap()
                .query_row(
                    "SELECT outcome FROM metadata_observations WHERE id = ?1",
                    [observation_id],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(outcome, "failed");
            assert!(feed.message.is_some());
        }
        assert_eq!(snapshot(&conn), before);
        assert!(crate::rss::retained_document_for_test(broken).is_none());
    }

    /// R2-08: a failed request writes no slot, no hold and no difference.
    /// A received document of the same run applies its differences, and the
    /// snapshot carries them for the mounted report.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn adr_0076_rss_comparison_failed_request_writes_nothing() {
        let refused = "https://down.test/r208/feed.xml";
        let failing = "https://error.test/r208/feed.xml";
        let working = "https://band.test/r208/feed.xml";
        let (conn, playlist_id, feed_ids) = database(&[(refused, 1), (failing, 1), (working, 1)]);
        let body = crate::rss::check_apply::test_support::document(
            &crate::rss::check_apply::test_support::first(),
        );
        let clock = FakeClock::new();
        let fetcher = FakeFetcher::new(Arc::clone(&clock), move |url| {
            if url == refused {
                no_response()
            } else if url == failing {
                response(url, 500, &[], Some(b"server error"))
            } else {
                response(url, 200, &[], Some(&body))
            }
        });
        let session = SessionLifecycle::new();
        let handle = start(&conn, &fetcher, &clock, &session);
        assert!(handle.start(playlist_id, RssCheckTrigger::Button));
        let run = finished(&handle, playlist_id).await;
        assert_eq!(run.count(RssFeedOutcome::Failed), 2);
        assert_eq!(run.count(RssFeedOutcome::Document), 1);
        let conn = conn.lock().unwrap();
        for failed in &feed_ids[..2] {
            for table in ["rss_field_holds", "rss_check_differences"] {
                let count: i64 = conn
                    .query_row(
                        &format!("SELECT count(*) FROM {table} WHERE feed_id = ?1"),
                        [failed],
                        |row| row.get(0),
                    )
                    .unwrap();
                assert_eq!(count, 0, "{table} of feed {failed}");
            }
            let title: String = conn
                .query_row("SELECT title FROM feeds WHERE id = ?1", [failed], |row| {
                    row.get(0)
                })
                .unwrap();
            assert!(title.starts_with("Album "), "{title}");
        }
        let applied: i64 = conn
            .query_row(
                "SELECT count(*) FROM rss_check_differences WHERE feed_id = ?1",
                [feed_ids[2]],
                |row| row.get(0),
            )
            .unwrap();
        assert!(applied > 0);
        assert_eq!(
            i64::try_from(run.differences.len()).unwrap(),
            applied,
            "the snapshot carries each applied difference"
        );
        assert!(handle.latest().applied_revision() > 0);
        let removed =
            crate::db::rss_field_holds::playlist_removed_marks(&conn, playlist_id).unwrap();
        assert_eq!(removed.len(), 1, "the one stored item of the feed left RSS");
        assert_eq!(
            handle.latest().removed_mark(playlist_id, removed[0].0),
            Some(removed[0].1)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn adr_0076_playlist_check_second_start_joins_the_running_check() {
        let url = "https://join.test/r110/feed.xml";
        let (conn, playlist_id, _) = database(&[(url, 1)]);
        let clock = FakeClock::new();
        let fetcher = FakeFetcher::new(Arc::clone(&clock), ok);
        fetcher.close_gate();
        let session = SessionLifecycle::new();
        let handle = start(&conn, &fetcher, &clock, &session);
        assert!(handle.start(playlist_id, RssCheckTrigger::Button));
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while fetcher.active().0 == 0 {
            assert!(std::time::Instant::now() < deadline, "the request starts");
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(handle.start(playlist_id, RssCheckTrigger::PlaybackStart));
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(handle.latest().is_running(playlist_id));
        fetcher.open_gate();
        let run = finished(&handle, playlist_id).await;
        assert_eq!(fetcher.sent().len(), 1);
        assert_eq!(run.trigger, RssCheckTrigger::Button);
        let runs: i64 = conn
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM rss_check_runs", [], |row| row.get(0))
            .unwrap();
        assert_eq!(runs, 1);
        assert_eq!(observation_count(&conn), 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn adr_0076_playlist_check_run_rows_match_the_results() {
        let (conn, playlist_id, _) = database(&[
            ("https://rows.test/r111/document.xml", 1),
            ("https://cdn.test/r111/same.xml", 1),
            ("https://down.test/r111/feed.xml", 1),
            ("ftp://rows.test/r111/feed.xml", 1),
        ]);
        let clock = FakeClock::new();
        let fetcher = FakeFetcher::new(Arc::clone(&clock), |url| {
            if url.contains("cdn.test") {
                response(url, 304, &[], None)
            } else if url.contains("down.test") {
                response(url, 503, &[], Some(b""))
            } else {
                ok(url)
            }
        });
        let session = SessionLifecycle::new();
        let handle = start(&conn, &fetcher, &clock, &session);
        assert!(handle.start(playlist_id, RssCheckTrigger::PlaybackStart));
        let run = finished(&handle, playlist_id).await;
        assert_eq!(run.state, RssCheckRunState::Finished);
        let stored = crate::db::rss_check_runs::latest_run(&conn.lock().unwrap(), playlist_id)
            .unwrap()
            .unwrap();
        assert_eq!(Some(stored.id), run.run_id);
        assert_eq!(stored.trigger, RssCheckTrigger::PlaybackStart);
        assert!(stored.finished_at_us.is_some());
        assert_eq!(stored.results.len(), 4);
        let count = |outcome| {
            i64::try_from(
                stored
                    .results
                    .iter()
                    .filter(|result| result.outcome == outcome)
                    .count(),
            )
            .unwrap()
        };
        assert_eq!(stored.document_count, count(RssFeedOutcome::Document));
        assert_eq!(
            stored.not_modified_count,
            count(RssFeedOutcome::NotModified)
        );
        assert_eq!(stored.failed_count, count(RssFeedOutcome::Failed));
        assert_eq!(stored.not_checked_count, count(RssFeedOutcome::NotChecked));
        assert_eq!(
            (
                stored.document_count,
                stored.not_modified_count,
                stored.failed_count,
                stored.not_checked_count
            ),
            (1, 1, 1, 1)
        );
        // A later session reads the stored run into its snapshot.
        let reload_session = SessionLifecycle::new();
        let reload = start(&conn, &fetcher, &clock, &reload_session);
        assert!(reload.load(playlist_id));
        let loaded = finished(&reload, playlist_id).await;
        assert_eq!(loaded.run_id, run.run_id);
        assert_eq!(loaded.feeds.len(), 4);
        assert_eq!(loaded.state, RssCheckRunState::Finished);
    }

    #[test]
    fn adr_0076_playlist_check_feed_host_and_retry_after_parse() {
        assert_eq!(
            feed_host("https://Feed.Wavlake.com/feed/music/1").as_deref(),
            Some("feed.wavlake.com")
        );
        assert_eq!(feed_host("ftp://example.test/feed"), None);
        assert_eq!(feed_host(""), None);
        let occurrence =
            response("https://a.test/", 200, &[("retry-after", "30")], None).occurrence;
        assert_eq!(retry_after(&occurrence), Some(Duration::from_secs(30)));
        let occurrence = response(
            "https://a.test/",
            200,
            &[("retry-after", "Wed, 21 Oct 2015 07:28:00 GMT")],
            None,
        )
        .occurrence;
        assert_eq!(retry_after(&occurrence), Some(Duration::ZERO));
    }
}
