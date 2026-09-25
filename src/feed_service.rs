use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use rusqlite::Connection;

use crate::api::{
    Client as MusicIndexClient, Contributor, Feed, SourceEntityId, SourceEntityLink, Track,
};
use crate::application::queries::stored_values::{self, TrackStoredValues};
use crate::audio_tags::{read_audio_tags, AudioTags, Id3v24Edit};
use crate::config;
use crate::db::{self, TrackRow};
use crate::identity_ingest;
use crate::library_service;
use crate::metadata::{
    sanitize_track_context_source_text, source_text_missing, MusicBrainzLookupResult, TrackContext,
};
use crate::metadata_service::musicbrainz_lookup_metadata;
use crate::musicbrainz::{lookup_recordings, MusicBrainzCandidate, MusicBrainzLookup};

#[derive(Clone, Debug)]
pub struct StaleFeed {
    pub feed_id: i64,
    pub feed_guid: String,
    pub title: Option<String>,
    pub new_updated_at: i64,
}

/// The result of one feed update. ADR 0076 Decision 8: a feed update
/// changes the database only, so the outcome counts stored track records and
/// no file write.
#[derive(Default, Debug, Clone)]
pub struct FeedApplyOutcome {
    /// The tracks whose `MusicIndex` record the update stored.
    pub tracks_refreshed: usize,
}

#[derive(Clone, Debug)]
pub struct StagedMusicBrainzLookup {
    pub lookup: MusicBrainzLookupResult,
    pub edit_count: usize,
}

pub fn fetch_library_track_context(
    track: &TrackRow,
    musicindex_endpoint: &crate::config::MusicIndexEndpoint,
) -> Result<TrackContext> {
    fetch_library_track_context_with_recorder(
        track,
        musicindex_endpoint,
        None,
        crate::application::request_reuse::RefreshIntent::Normal,
        &mut Vec::new(),
    )
}

/// `receipts_out` accumulates the track and feed detail receipts this call
/// produces or joins (packet 018 R18B-07, R18B-11), regardless of whether
/// this call ultimately succeeds. A caller with its own recorder-draining
/// convention (`assemble_provider_context`, `assemble_observed_query`)
/// keeps its existing "receipts survive a later failure" behavior only
/// when it reads the receipts from here, not from `TrackContext`: unlike
/// `recorder`, a `TrackContext` does not exist yet when this function
/// fails, so a receipt cannot ride inside one on that path.
pub(crate) fn fetch_library_track_context_with_recorder(
    track: &TrackRow,
    musicindex_endpoint: &crate::config::MusicIndexEndpoint,
    recorder: Option<Arc<crate::provider_observation::ProviderObservationRecorder>>,
    refresh: crate::application::request_reuse::RefreshIntent,
    receipts_out: &mut Vec<crate::provider_observation::ObservationReceipt>,
) -> Result<TrackContext> {
    let (fetched_track, fetched_feed) = fetch_library_track_detail_with_recorder(
        track,
        musicindex_endpoint,
        recorder.clone(),
        refresh,
        receipts_out,
    )?;
    merge_track_context_with_recorder(track, fetched_track, fetched_feed, recorder.as_deref())
}

/// Fetches a Library track's remote MusicIndex detail through the shared
/// request owner (ADR 0075 section 6, packet 018).
///
/// `refresh` travels with every request this function sends: `Normal` for
/// a passive read, which may join an active request or reuse a retained
/// response; `Explicit` for a caller that wants a fresh value, such as
/// `apply_feed_updates` or a manual comparison, which always sends its own
/// request and never reuses a retained response (R18A-09, R18A-10). The
/// owner never runs the fetch twice for one active identity, and it never
/// holds its registry lock or its retained-cache lock across the network
/// call.
///
/// Packet 018 Part B routes every sub-fetch through the owner's
/// receipts-carrying methods (R18B-07: a reused response must carry the
/// identifier of the observation that produced it). Each sub-fetch's
/// closure drains `recorder` immediately after its own call, before the
/// next sub-fetch can add anything else to it, so the drained value is
/// exactly that one call's own receipt — the same pattern
/// `library::hydrate_album_identity_facts` already uses for its one feed
/// fetch. This function accumulates every sub-fetch's receipts into
/// `receipts_out`, which the caller merges with whatever `recorder` holds
/// afterward (here, only a later RSS receipt remains undrained).
fn fetch_library_track_detail_with_recorder(
    track: &TrackRow,
    musicindex_endpoint: &crate::config::MusicIndexEndpoint,
    recorder: Option<Arc<crate::provider_observation::ProviderObservationRecorder>>,
    refresh: crate::application::request_reuse::RefreshIntent,
    receipts_out: &mut Vec<crate::provider_observation::ObservationReceipt>,
) -> Result<(Option<Track>, Option<Feed>)> {
    use crate::application::request_profiles::{
        LIBRARY_TRACK_DETAIL_FEED, LIBRARY_TRACK_DETAIL_SCOPED_TRACK,
        LIBRARY_TRACK_DETAIL_UNSCOPED_TRACK,
    };
    use crate::application::request_reuse::{self, RequestKey, SharedFetchError};
    use crate::provider_observation::{propagate_storage_failure, ProviderObservationRecorder};
    let client = MusicIndexClient::new_with_base_url(musicindex_endpoint.clone())
        .with_observation_recorder(recorder.clone());
    let owner = request_reuse::shared();
    let provider_identity = musicindex_endpoint
        .require()
        .map(str::to_owned)
        .unwrap_or_default();

    // Drains `recorder` right after one HTTP call, before the next
    // sub-fetch below can add anything else to it, so the drained value is
    // exactly that one call's own receipt. `library::hydrate_album_identity_facts`
    // uses the same pattern for its single feed fetch. Storing the
    // receipts on the owner's registry entry, rather than only in
    // `recorder`, is what lets a later reused response still name the
    // observation that produced it (R18B-07), and lets a caller that joins
    // an in-flight request receive that request's receipts (R18A-05,
    // R18B-11).
    let drain = |recorder: &Option<Arc<ProviderObservationRecorder>>| {
        recorder
            .as_deref()
            .map(ProviderObservationRecorder::take_receipts)
            .unwrap_or_default()
    };

    let mut fetched_track = match track.feed_guid.as_deref() {
        Some(feed_guid) => {
            let key = RequestKey::scoped_track(
                provider_identity.clone(),
                feed_guid,
                &track.item_guid,
                LIBRARY_TRACK_DETAIL_SCOPED_TRACK.include(),
            );
            let result = owner
                .fetch_track_with_receipts(key, refresh, || {
                    let fetched = client.fetch_feed_track_with_profile(
                        feed_guid,
                        &track.item_guid,
                        &LIBRARY_TRACK_DETAIL_SCOPED_TRACK,
                    )?;
                    Ok((fetched, drain(&recorder)))
                })
                .0
                .map_err(SharedFetchError::into_anyhow);
            propagate_storage_failure(result)?.map(|(fetched, receipts)| {
                receipts_out.extend(receipts);
                fetched
            })
        }
        None => None,
    };
    if fetched_track.is_none() {
        let key = RequestKey::unscoped_track(
            provider_identity.clone(),
            &track.item_guid,
            LIBRARY_TRACK_DETAIL_UNSCOPED_TRACK.include(),
        );
        let result = owner
            .fetch_track_with_receipts(key, refresh, || {
                let fetched = client.fetch_track_with_profile(
                    &track.item_guid,
                    &LIBRARY_TRACK_DETAIL_UNSCOPED_TRACK,
                )?;
                Ok((fetched, drain(&recorder)))
            })
            .0
            .map_err(SharedFetchError::into_anyhow);
        fetched_track = propagate_storage_failure(result)?.map(|(fetched, receipts)| {
            receipts_out.extend(receipts);
            fetched
        });
    }
    let feed_guid = fetched_track
        .as_ref()
        .and_then(|track| track.feed_guid.as_deref())
        .or(track.feed_guid.as_deref());
    let fetched_feed = match feed_guid {
        Some(guid) => {
            let key =
                RequestKey::feed(provider_identity, guid, LIBRARY_TRACK_DETAIL_FEED.include());
            let result = owner
                .fetch_feed_with_receipts(key, refresh, || {
                    let fetched =
                        client.fetch_feed_with_profile(guid, &LIBRARY_TRACK_DETAIL_FEED)?;
                    Ok((fetched, drain(&recorder)))
                })
                .0
                .map_err(SharedFetchError::into_anyhow);
            propagate_storage_failure(result)?.map(|(fetched, receipts)| {
                receipts_out.extend(receipts);
                fetched
            })
        }
        None => None,
    };
    if fetched_track.is_none() && fetched_feed.is_none() {
        return Err(anyhow!("MusicIndex metadata unavailable"));
    }
    Ok((fetched_track, fetched_feed))
}

fn merge_track_context_with_recorder(
    track_row: &TrackRow,
    fetched_track: Option<Track>,
    fetched_feed: Option<Feed>,
    recorder: Option<&crate::provider_observation::ProviderObservationRecorder>,
) -> Result<TrackContext> {
    let local_track = crate::subscribe_service::track_row_to_api_track(track_row);
    let local_feed = track_row_to_feed(track_row);
    let feed = feed_defaults(
        fetched_feed.unwrap_or_else(|| local_feed.clone()),
        &local_feed,
    );
    let track = crate::api::track_with_feed_defaults(
        track_defaults(
            fetched_track.unwrap_or_else(|| local_track.clone()),
            &local_track,
        ),
        Some(&feed),
    );
    let mut context = TrackContext::new(track, Some(feed));
    crate::subscribe_service::enrich_track_context_from_rss_with_recorder(&mut context, recorder)?;
    sanitize_track_context_source_text(&mut context);
    Ok(context)
}

pub fn track_row_to_feed(track: &TrackRow) -> Feed {
    Feed {
        // Identity column passes through verbatim.
        feed_guid: track.feed_guid.clone(),
        // Display facts are sanitized so polluted local rows cannot surface
        // as display strings.
        title: drop_placeholder(track.feed_title.clone()),
        image_url: drop_placeholder(track.album_image_href.clone()),
        ..Feed::default()
    }
}

/// Strip placeholder transport values (`...`, `\u{2026}`, whitespace-only)
/// so polluted local rows do not surface as display facts.
fn drop_placeholder(value: Option<String>) -> Option<String> {
    value.filter(|value| !source_text_missing(Some(value.as_str())))
}

fn track_defaults(mut track: Track, defaults: &Track) -> Track {
    if source_text_missing(track.track_guid.as_deref()) {
        track.track_guid = defaults.track_guid.clone();
    }
    if source_text_missing(track.feed_guid.as_deref()) {
        track.feed_guid = defaults.feed_guid.clone();
    }
    if source_text_missing(track.feed_title.as_deref()) {
        track.feed_title = defaults.feed_title.clone();
    }
    if source_text_missing(track.title.as_deref()) {
        track.title = defaults.title.clone();
    }
    if track.duration_secs.is_none() {
        track.duration_secs = defaults.duration_secs;
    }
    if track.track_number.is_none() {
        track.track_number = defaults.track_number;
    }
    if source_text_missing(track.enclosure_url.as_deref()) {
        track.enclosure_url = defaults.enclosure_url.clone();
    }
    if source_text_missing(track.image_url.as_deref()) {
        track.image_url = defaults.image_url.clone();
    }
    if source_text_missing(track.track_artist.as_deref()) {
        track.track_artist = defaults.track_artist.clone();
    }
    if source_text_missing(track.release_artist.as_deref()) {
        track.release_artist = defaults.release_artist.clone();
    }
    if source_text_missing(track.description.as_deref()) {
        track.description = defaults.description.clone();
    }
    if source_text_missing(track.publisher_text.as_deref()) {
        track.publisher_text = defaults.publisher_text.clone();
    }
    if track.source_contributors.is_none() {
        track.source_contributors = defaults.source_contributors.clone();
    }
    if track.source_links.is_none() {
        track.source_links = defaults.source_links.clone();
    }
    if track.source_ids.is_none() {
        track.source_ids = defaults.source_ids.clone();
    }
    if track.source_release_claims.is_none() {
        track.source_release_claims = defaults.source_release_claims.clone();
    }
    if track.payment_routes.is_none() {
        track.payment_routes = defaults.payment_routes.clone();
    }
    track
}

fn feed_defaults(mut feed: Feed, defaults: &Feed) -> Feed {
    if source_text_missing(feed.feed_guid.as_deref()) {
        feed.feed_guid = defaults.feed_guid.clone();
    }
    if source_text_missing(feed.title.as_deref()) {
        feed.title = defaults.title.clone();
    }
    if source_text_missing(feed.name.as_deref()) {
        feed.name = defaults.name.clone();
    }
    if source_text_missing(feed.feed_url.as_deref()) {
        feed.feed_url = defaults.feed_url.clone();
    }
    if source_text_missing(feed.image_url.as_deref()) {
        feed.image_url = defaults.image_url.clone();
    }
    if source_text_missing(feed.release_artist.as_deref()) {
        feed.release_artist = defaults.release_artist.clone();
    }
    if source_text_missing(feed.publisher_text.as_deref()) {
        feed.publisher_text = defaults.publisher_text.clone();
    }
    if source_text_missing(feed.language.as_deref()) {
        feed.language = defaults.language.clone();
    }
    if source_text_missing(feed.description.as_deref()) {
        feed.description = defaults.description.clone();
    }
    feed
}

pub fn ensure_feed_in_db(
    conn: &Arc<Mutex<Connection>>,
    feed_guid: &str,
    feed_url: Option<&str>,
    musicindex_endpoint: &crate::config::MusicIndexEndpoint,
) -> Result<i64> {
    {
        let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        if let Some(id) = db::find_feed_id_by_guid(&db, feed_guid)? {
            return Ok(id);
        }
    }
    let url = feed_url.ok_or_else(|| anyhow!("feed URL unknown; cannot auto-subscribe"))?;
    {
        let mut db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        crate::rss::subscribe_feed(&mut db, url, musicindex_endpoint)?;
    }
    let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
    db::find_feed_id_by_guid(&db, feed_guid)?
        .ok_or_else(|| anyhow!("subscribe completed but feed not found"))
}

/// ADR 0075 records the feed-check request and response for one local feed.
///
/// Packet 018 Part B routes this request through `fetch_feed_with_receipts`,
/// not the plain `fetch_feed`, so a caller that joins this identity's
/// active slot (a concurrent `Normal` read of the same feed and include
/// list) receives this request's own receipts too (R18B-11). This
/// function's own caller still receives them through `receipts_out`,
/// because the closure below drains `recorder` for the owner's registry
/// entry before this function returns (see
/// `fetch_library_track_detail_with_recorder`'s documentation for why that
/// drain is needed and why it is safe).
///
/// # Errors
/// Returns the existing database or transport failure. A provider storage
/// failure keeps its typed capsule so the caller can classify it.
pub fn check_feed_staleness(
    conn: &Arc<Mutex<Connection>>,
    musicindex_endpoint: &crate::config::MusicIndexEndpoint,
    feed_id: i64,
    recorder: &Arc<crate::provider_observation::ProviderObservationRecorder>,
    receipts_out: &mut Vec<crate::provider_observation::ObservationReceipt>,
) -> Result<Option<StaleFeed>> {
    let stored = {
        let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        db::feed_stale_check_row(&db, feed_id)?
    };
    let Some(stored) = stored else {
        return Ok(None);
    };
    let client = MusicIndexClient::new_with_base_url(musicindex_endpoint.clone())
        .with_observation_recorder(Some(Arc::clone(recorder)));
    // ADR 0075 section 6, packet 018: an explicit "check for updates"
    // request always reaches the network. It never joins an active
    // request that started without that intent (R18A-09), and it never
    // reuses a retained response (Part B, R18B-01 applies to `Normal`
    // only).
    use crate::application::request_reuse::{self, RefreshIntent, RequestKey, SharedFetchError};
    let owner = request_reuse::shared();
    let provider_identity = musicindex_endpoint
        .require()
        .map(str::to_owned)
        .unwrap_or_default();
    let key = RequestKey::feed(provider_identity, &stored.feed_guid, None);
    let (api_feed, receipts) = owner
        .fetch_feed_with_receipts(key, RefreshIntent::Explicit, || {
            let feed = client.fetch_feed(&stored.feed_guid, None)?;
            Ok((feed, recorder.take_receipts()))
        })
        .0
        .map_err(SharedFetchError::into_anyhow)?;
    receipts_out.extend(receipts);
    let Some(api_updated_at) = api_feed.updated_at else {
        return Ok(None);
    };
    if stored
        .musicindex_updated_at
        .is_some_and(|stored_at| stored_at >= api_updated_at)
    {
        return Ok(None);
    }
    Ok(Some(StaleFeed {
        feed_id,
        feed_guid: stored.feed_guid,
        title: stored.title,
        new_updated_at: api_updated_at,
    }))
}

/// ADR 0075 records the feed and track requests that one feed update makes.
///
/// Packet 018 P18-7: an explicit feed update is the point where the app
/// knows a feed's content actually changed, so this function clears every
/// retained MusicIndex response of this feed and its scoped tracks, and
/// the feed's retained RSS document, before it sends any request. This
/// keeps a later passive read from reusing a response that predates the
/// update.
///
/// This function's own feed fetch, and each track's detail fetch inside
/// the loop below, route through the owner's receipts-carrying methods
/// (R18B-11). Both drain `recorder` for the owner's registry entry, so
/// `receipts_out` accumulates every one of them; whatever the per-track
/// RSS enrichment adds afterward stays on `recorder` for this function's
/// own caller to drain.
///
/// # Errors
/// Returns the existing database, merge or transport failure. A provider
/// storage failure stops the feed before legacy persistence, and keeps its
/// typed capsule for the caller.
///
/// ADR 0076 Decision 8 and ADR 0075 section 7: the update writes no audio
/// tag. It changes the database only.
pub fn apply_feed_updates(
    conn: &Arc<Mutex<Connection>>,
    musicindex_endpoint: &crate::config::MusicIndexEndpoint,
    stale: &StaleFeed,
    recorder: &Arc<crate::provider_observation::ProviderObservationRecorder>,
    receipts_out: &mut Vec<crate::provider_observation::ObservationReceipt>,
) -> Result<FeedApplyOutcome> {
    use crate::application::request_profiles::LIBRARY_FEED_UPDATE_FEED;
    use crate::application::request_reuse::{self, RefreshIntent, RequestKey, SharedFetchError};
    use crate::provider_observation::propagate_storage_failure;
    let client = MusicIndexClient::new_with_base_url(musicindex_endpoint.clone())
        .with_observation_recorder(Some(Arc::clone(recorder)));
    // ADR 0075 section 6, packet 018: an explicit feed update always
    // reaches the network for the feed itself (R18A-09, R18A-10).
    let owner = request_reuse::shared();
    let provider_identity = musicindex_endpoint
        .require()
        .map(str::to_owned)
        .unwrap_or_default();

    // P18-7: clear every retained entry of this feed before sending a new
    // request. A missing or unreadable feed URL leaves the RSS document
    // cache alone; it still ages out on its own window.
    owner.invalidate_feed(&provider_identity, &stale.feed_guid);
    let feed_url = conn
        .lock()
        .map_err(|_| anyhow!("database lock poisoned"))
        .and_then(|db| db::feed_url_by_id(&db, stale.feed_id))
        .ok()
        .flatten();
    if let Some(feed_url) = feed_url.as_deref() {
        crate::rss::invalidate_feed_document(feed_url);
    }

    let key = RequestKey::feed(
        provider_identity,
        &stale.feed_guid,
        LIBRARY_FEED_UPDATE_FEED.include(),
    );
    let feed_result = owner
        .fetch_feed_with_receipts(key, RefreshIntent::Explicit, || {
            let feed =
                client.fetch_feed_with_profile(&stale.feed_guid, &LIBRARY_FEED_UPDATE_FEED)?;
            Ok((feed, recorder.take_receipts()))
        })
        .0
        .map_err(SharedFetchError::into_anyhow);
    let feed_update = propagate_storage_failure(feed_result)?.map(|(feed, receipts)| {
        receipts_out.extend(receipts);
        feed
    });
    if let Some(feed) = feed_update.as_ref() {
        let mut db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        // ADR 0076 Decision 5: a held RSS description stays until
        // MusicIndex agrees or supplies a newer record.
        if !source_text_missing(feed.description.as_deref())
            && db::rss_field_holds::musicindex_gate(
                &db,
                db::rss_field_holds::MusicIndexClaim::feed_description(
                    stale.feed_id,
                    feed.description.as_deref(),
                    feed.updated_at,
                ),
            )?
            .writes()
        {
            db::set_feed_description(&db, stale.feed_id, feed.description.as_deref())?;
        }
        identity_ingest::persist_musicindex_feed(&mut db, stale.feed_id, feed)?;
    }

    let tracks = {
        let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        library_service::tracks_for_feed(&db, stale.feed_id)?
    };
    let mut outcome = FeedApplyOutcome::default();
    for track in &tracks {
        // The update reads the tracks with a downloaded file, as before.
        // ADR 0076 Decision 8: it writes no audio tag. The tag update scan
        // of packet 004 offers the file write after this update.
        if track.local_path.is_none() {
            continue;
        }
        let detail = propagate_storage_failure(fetch_library_track_detail_with_recorder(
            track,
            musicindex_endpoint,
            Some(Arc::clone(recorder)),
            crate::application::request_reuse::RefreshIntent::Explicit,
            receipts_out,
        ))?;
        let Some((fetched_track, fetched_feed)) = detail else {
            continue;
        };
        // ADR 0075 packet 039: the RSS enrichment of the merge retains its
        // observation. The merged context is not used for a tag write.
        merge_track_context_with_recorder(
            track,
            fetched_track.clone(),
            fetched_feed.clone(),
            Some(recorder.as_ref()),
        )?;
        let mut db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        if let Some(feed) = fetched_feed.as_ref() {
            identity_ingest::persist_musicindex_feed(&mut db, stale.feed_id, feed)?;
        }
        if let Some(fetched_track) = fetched_track.as_ref() {
            identity_ingest::persist_musicindex_track(&mut db, track.id, fetched_track)?;
            outcome.tracks_refreshed += 1;
        }
    }
    {
        let db = conn.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        db::set_feed_musicindex_updated_at(&db, stale.feed_id, stale.new_updated_at)?;
    }
    Ok(outcome)
}

/// The tag frame context of a track row without a database connection. It
/// reads the column step of the stored value projection (ADR 0075 packet
/// 020). A caller with a connection uses
/// [`track_row_to_track_context_with_local_identity`], which reads the full
/// projection.
pub fn track_row_to_track_context(track: &TrackRow) -> TrackContext {
    track_context_with_values(track, &stored_values::track_values_from_columns(track))
}

/// ADR 0076 Decision 1: the tag frame context shows the stored value of each
/// field. The stored value projection selects it. This function selects no
/// source.
fn track_context_with_values(track: &TrackRow, values: &TrackStoredValues) -> TrackContext {
    let mut feed = track_row_to_feed(track);
    feed.title = drop_placeholder(values.album_title.value.clone());
    let mut api_track = crate::subscribe_service::track_row_to_api_track(track);
    api_track.title = drop_placeholder(values.title.value.clone());
    api_track.track_artist = drop_placeholder(values.artist.value.clone());
    api_track.release_artist = drop_placeholder(values.album_artist.value.clone());
    api_track.feed_title = drop_placeholder(values.album_title.value.clone());
    api_track.image_url = drop_placeholder(values.artwork.value.clone());
    api_track.pub_date = values.pub_date.value;
    api_track.explicit = values.explicit.value;
    api_track.description = values.description.value.clone();
    api_track.publisher_text = values.publisher_text.value.clone();
    let api_track = crate::api::track_with_feed_defaults(api_track, Some(&feed));
    let mut context = TrackContext::new(api_track, Some(feed));
    sanitize_track_context_source_text(&mut context);
    context
}

/// ADR 0075 resolves the local RSS resource without changing legacy DTO fields.
pub(crate) fn local_provider_request(
    conn: &Connection,
    track: &TrackRow,
    context: &TrackContext,
) -> Result<Option<crate::provider_observation::ProviderRequestSpec>> {
    Ok(db::feed_url_by_id(conn, track.feed_id)?
        .filter(|url| !source_text_missing(Some(url.as_str())))
        .map(|resource| {
            crate::provider_observation::contracts::rss_request(
                &resource,
                context.track.track_guid.as_deref(),
                context.track.enclosure_url.as_deref(),
            )
        }))
}

pub fn track_row_to_track_context_with_local_identity(
    conn: &Connection,
    track: &TrackRow,
) -> Result<TrackContext> {
    let values = stored_values::track_values(conn, track)?;
    let mut context = track_context_with_values(track, &values);
    context.feed = Some(hydrate_feed_identity(
        conn,
        track.feed_id,
        context.feed.take(),
    )?);
    context.track = hydrate_track_identity(conn, track.id, context.track)?;
    sanitize_track_context_source_text(&mut context);
    Ok(context)
}

fn hydrate_feed_identity(conn: &Connection, feed_id: i64, feed: Option<Feed>) -> Result<Feed> {
    let mut feed = feed.unwrap_or_default();
    feed.source_links = Some(
        db::local_identity_links(conn, db::LocalIdentityOwner::Feed(feed_id))?
            .into_iter()
            .map(source_link_from_local)
            .collect(),
    );
    feed.source_ids = Some(
        db::local_identity_ids(conn, db::LocalIdentityOwner::Feed(feed_id))?
            .into_iter()
            .map(source_id_from_local)
            .collect(),
    );
    feed.source_contributors = Some(
        db::local_contributors(conn, db::LocalEntityOwner::Feed(feed_id))?
            .into_iter()
            .map(contributor_from_local)
            .collect(),
    );
    Ok(feed)
}

fn hydrate_track_identity(conn: &Connection, track_id: i64, mut track: Track) -> Result<Track> {
    track.source_links = Some(
        db::local_identity_links(conn, db::LocalIdentityOwner::Track(track_id))?
            .into_iter()
            .map(source_link_from_local)
            .collect(),
    );
    track.source_ids = Some(
        db::local_identity_ids(conn, db::LocalIdentityOwner::Track(track_id))?
            .into_iter()
            .map(source_id_from_local)
            .collect(),
    );
    track.source_contributors = Some(
        db::local_contributors(conn, db::LocalEntityOwner::Track(track_id))?
            .into_iter()
            .map(contributor_from_local)
            .collect(),
    );
    Ok(track)
}

fn source_link_from_local(row: db::LocalIdentityLinkRow) -> SourceEntityLink {
    SourceEntityLink {
        entity_type: row.entity_type,
        entity_id: row.entity_id,
        position: row.position,
        link_type: row.link_type,
        url: row.url,
        source: Some(row.source),
        extraction_path: row.extraction_path,
        observed_at: row.observed_at,
    }
}

fn source_id_from_local(row: db::LocalIdentityIdRow) -> SourceEntityId {
    SourceEntityId {
        entity_type: row.entity_type,
        entity_id: row.entity_id,
        position: row.position,
        scheme: row.scheme,
        value: row.value,
        source: Some(row.source),
        extraction_path: row.extraction_path,
        observed_at: row.observed_at,
    }
}

fn contributor_from_local(row: db::LocalContributorRow) -> Contributor {
    // ADR 0075: the local row keeps no claim provenance, so these fields stay unknown.
    Contributor {
        name: row.name,
        role: row.role,
        href: row.href,
        img: row.image_url,
        npub: row.nostr_npub,
        group_name: row.group_name,
        entity_type: None,
        entity_id: None,
        position: None,
        role_norm: None,
        source: None,
        extraction_path: None,
        observed_at: None,
    }
}

pub fn lookup_musicbrainz_library_track(track: &TrackRow) -> Result<MusicBrainzLookupResult> {
    let cfg_path = config::config_path()?;
    let music_dir = config::ConfigSnapshot::read_existing(&cfg_path)?.music_dir?;
    let path = track
        .local_path
        .as_ref()
        .map(|path| path.resolve(&music_dir))
        .ok_or_else(|| anyhow!("library track has no local file"))?;
    let tags = read_audio_tags(&path)?;
    let context = track_row_to_track_context(track);
    let metadata = musicbrainz_lookup_metadata(&context.track, &tags);
    let musicbrainz_client = crate::http_client::document_builder()
        .user_agent(format!(
            "v4vmm/{} (MusicBrainz metadata lookup)",
            env!("CARGO_PKG_VERSION")
        ))
        .build()?;
    let lookup = lookup_recordings(&musicbrainz_client, &metadata, 5)?;
    let image = lookup
        .candidates
        .first()
        .and_then(|candidate| candidate.release_id.as_deref())
        .and_then(|release_id| {
            let url = format!("https://coverartarchive.org/release/{release_id}/front-250");
            crate::subscribe_service::download_image(&url)
        });
    Ok(MusicBrainzLookupResult { lookup, image })
}

pub fn lookup_musicbrainz_stage_for_track(track: &TrackRow) -> Result<StagedMusicBrainzLookup> {
    let cfg_path = config::config_path()?;
    let music_dir = config::ConfigSnapshot::read_existing(&cfg_path)?.music_dir?;
    let path = track
        .local_path
        .as_ref()
        .map(|path| path.resolve(&music_dir))
        .ok_or_else(|| anyhow!("no local file"))?;
    let tags = read_audio_tags(&path)?;
    let api_track = crate::subscribe_service::track_row_to_api_track(track);
    let metadata = musicbrainz_lookup_metadata(&api_track, &tags);
    let musicbrainz_client = crate::http_client::document_builder()
        .user_agent(format!(
            "v4vmm/{} (MusicBrainz metadata lookup)",
            env!("CARGO_PKG_VERSION")
        ))
        .build()?;
    let lookup = lookup_recordings(&musicbrainz_client, &metadata, 3)?;
    let candidate = lookup
        .candidates
        .first()
        .ok_or_else(|| anyhow!("no MusicBrainz results"))?;
    Ok(StagedMusicBrainzLookup {
        edit_count: mb_edits_for_missing_fields(&tags, candidate).len(),
        lookup: MusicBrainzLookupResult {
            lookup,
            image: None,
        },
    })
}

pub fn stage_candidate_for_track(
    track: &TrackRow,
    candidate: &MusicBrainzCandidate,
) -> Result<StagedMusicBrainzLookup> {
    let cfg_path = config::config_path()?;
    let music_dir = config::ConfigSnapshot::read_existing(&cfg_path)?.music_dir?;
    let path = track
        .local_path
        .as_ref()
        .map(|path| path.resolve(&music_dir))
        .ok_or_else(|| anyhow!("no local file"))?;
    let tags = read_audio_tags(&path)?;
    Ok(StagedMusicBrainzLookup {
        edit_count: mb_edits_for_missing_fields(&tags, candidate).len(),
        lookup: MusicBrainzLookupResult {
            lookup: MusicBrainzLookup {
                query: "batch release lookup".into(),
                candidates: vec![candidate.clone()],
            },
            image: None,
        },
    })
}

fn mb_edits_for_missing_fields(
    tags: &AudioTags,
    candidate: &MusicBrainzCandidate,
) -> Vec<Id3v24Edit> {
    let mut edits = Vec::new();
    let trck_value = match (candidate.track_position, candidate.total_tracks) {
        (Some(pos), Some(total)) => Some(format!("{pos}/{total}")),
        _ => candidate.track_number.clone(),
    };
    let checks: Vec<(&str, bool, Option<String>)> = vec![
        ("TIT2", tags.title.is_some(), Some(candidate.title.clone())),
        ("TPE1", tags.artist.is_some(), candidate.artist.clone()),
        (
            "TALB",
            tags.album.is_some(),
            candidate.release_title.clone(),
        ),
        ("TRCK", tags.track_number.is_some(), trck_value),
        ("TDRC", tags.date.is_some(), candidate.release_date.clone()),
        (
            "TPUB",
            tag_has_frame(tags, "TPUB"),
            candidate.labels.first().cloned(),
        ),
        (
            "TSRC",
            tag_has_frame(tags, "TSRC"),
            candidate.isrcs.first().cloned(),
        ),
        (
            "TMED",
            tag_has_frame(tags, "TMED"),
            candidate.format.clone(),
        ),
        (
            "TPOS",
            tag_has_frame(tags, "TPOS"),
            candidate.medium_position.map(|p| p.to_string()),
        ),
        (
            "TSST",
            tag_has_frame(tags, "TSST"),
            candidate.medium_title.clone(),
        ),
        (
            "TLEN",
            tag_has_frame(tags, "TLEN"),
            candidate.track_length_ms.map(|ms| ms.to_string()),
        ),
        (
            "TXXX:MusicBrainz Album Id",
            tags.custom.contains_key("MusicBrainz Album Id"),
            candidate.release_id.clone(),
        ),
        (
            "TXXX:MusicBrainz Release Group Id",
            tags.custom.contains_key("MusicBrainz Release Group Id"),
            candidate.release_group_id.clone(),
        ),
        (
            "TXXX:BARCODE",
            tags.custom.contains_key("BARCODE"),
            candidate.release_barcode.clone(),
        ),
        (
            "UFID:http://musicbrainz.org",
            tag_has_frame(tags, "UFID"),
            if candidate.recording_id.is_empty() {
                None
            } else {
                Some(candidate.recording_id.clone())
            },
        ),
    ];

    for (frame_label, has_existing, mb_value) in checks {
        if has_existing {
            continue;
        }
        if let Some(value) = mb_value {
            if !value.is_empty() {
                edits.push(Id3v24Edit {
                    frame_label: frame_label.to_string(),
                    value,
                });
            }
        }
    }
    edits
}

fn tag_has_frame(tags: &AudioTags, frame_id: &str) -> bool {
    tags.fields.iter().any(|field| field.frame_id == frame_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Contributor, Feed, PaymentRoute, SourceEntityId, Track};
    use crate::metadata_service::id3_edits_for_track_context;

    fn setup_test_db() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        db::init_schema(&conn)?;
        db::migrate_schema(&conn)?;
        Ok(conn)
    }

    fn insert_track(conn: &Connection) -> Result<TrackRow> {
        conn.execute(
            "INSERT INTO feeds (feed_url, feed_guid, title)
             VALUES (?1, ?2, ?3)",
            rusqlite::params!["https://example.test/feed.xml", "feed-guid", "Feed"],
        )?;
        let feed_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO tracks (feed_id, item_guid, track_title, artist_name)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![feed_id, "track-guid", "Track", "Artist"],
        )?;
        let track_id = conn.last_insert_rowid();
        Ok(TrackRow {
            id: track_id,
            feed_id,
            feed_guid: Some("feed-guid".into()),
            item_guid: "track-guid".into(),
            track_title: Some("Track".into()),
            artist_name: Some("Artist".into()),
            feed_title: Some("Feed".into()),
            ..TrackRow::default()
        })
    }

    #[test]
    fn library_track_context_preserves_feed_guid_for_id3_provenance() {
        let track = TrackRow {
            id: 1,
            feed_id: 2,
            feed_guid: Some("feed-guid".into()),
            item_guid: "track-guid".into(),
            track_title: Some("Song".into()),
            artist_name: None,
            album_title: None,
            album_artist_name: None,
            track_number: None,
            disc_number: None,
            duration_seconds: None,
            enclosure_url: None,
            enclosure_type: None,
            track_image_href: None,
            is_in_library: true,
            feed_title: Some("Feed".into()),
            album_image_href: None,
            local_path: None,
            pub_date: None,
            explicit: None,
            transcript_url: None,
        };

        let context = track_row_to_track_context(&track);
        let edits = id3_edits_for_track_context(&context);

        assert!(edits.iter().any(|edit| {
            edit.frame_label == "TXXX:MusicIndex Feed Guid" && edit.value == "feed-guid"
        }));
    }

    #[test]
    fn library_track_context_inherits_feed_level_musicindex_metadata() {
        let track_row = TrackRow {
            id: 1,
            feed_id: 2,
            feed_guid: Some("feed-guid".into()),
            item_guid: "track-guid".into(),
            track_title: Some("Song".into()),
            artist_name: Some("Artist".into()),
            album_title: None,
            album_artist_name: None,
            track_number: Some(4),
            disc_number: None,
            duration_seconds: Some(223),
            enclosure_url: Some("https://example.test/track.mp3".into()),
            enclosure_type: None,
            track_image_href: None,
            is_in_library: true,
            feed_title: Some("Feed".into()),
            album_image_href: None,
            local_path: None,
            pub_date: None,
            explicit: None,
            transcript_url: None,
        };
        let track = Track {
            track_guid: Some("track-guid".into()),
            feed_guid: Some("feed-guid".into()),
            title: Some("Song".into()),
            ..Default::default()
        };
        let feed = Feed {
            feed_guid: Some("feed-guid".into()),
            title: Some("Feed".into()),
            publisher_text: Some("HeyCitizen".into()),
            description: Some("Feed description".into()),
            source_ids: Some(vec![SourceEntityId {
                scheme: Some("nostr_npub".into()),
                value: Some("npub1heycitizen".into()),
                ..Default::default()
            }]),
            source_contributors: Some(vec![Contributor {
                name: Some("HeyCitizen".into()),
                role: Some("musician".into()),
                ..Default::default()
            }]),
            payment_routes: Some(vec![PaymentRoute {
                recipient_name: Some("HeyCitizen".into()),
                split: Some(100.0),
                ..Default::default()
            }]),
            ..Default::default()
        };

        let context =
            merge_track_context_with_recorder(&track_row, Some(track), Some(feed), None).unwrap();
        assert_eq!(context.track.publisher_text.as_deref(), Some("HeyCitizen"));
        assert_eq!(
            context.track.source_contributors.as_ref().map(Vec::len),
            Some(1)
        );
        assert_eq!(context.track.source_ids.as_ref().map(Vec::len), Some(1));
        assert_eq!(context.track.payment_routes.as_ref().map(Vec::len), Some(1));
        assert_eq!(
            context
                .feed
                .as_ref()
                .and_then(|feed| feed.description.as_deref()),
            Some("Feed description")
        );
    }

    #[test]
    fn library_track_context_rejects_placeholder_source_text_at_boundary() {
        let track_row = TrackRow {
            id: 1,
            feed_id: 2,
            feed_guid: Some("feed-guid".into()),
            item_guid: "track-guid".into(),
            track_title: Some("Lantern Tide".into()),
            artist_name: Some("Max DjK".into()),
            album_title: None,
            album_artist_name: Some("Max DjK".into()),
            track_number: Some(2),
            disc_number: None,
            duration_seconds: Some(343),
            enclosure_url: Some("https://example.test/lantern.mp3".into()),
            enclosure_type: None,
            track_image_href: None,
            is_in_library: true,
            feed_title: Some("Orient Express".into()),
            album_image_href: None,
            local_path: None,
            pub_date: None,
            explicit: None,
            transcript_url: None,
        };
        let track = Track {
            track_guid: Some("\u{2026}".into()),
            feed_guid: Some("...".into()),
            title: Some("...".into()),
            track_artist: Some("...".into()),
            release_artist: Some("...".into()),
            feed_title: Some("...".into()),
            description: Some("...\n...\n...".into()),
            ..Default::default()
        };
        let feed = Feed {
            feed_guid: Some("...".into()),
            title: Some("...".into()),
            description: Some("...\n...\n...".into()),
            ..Default::default()
        };

        let context =
            merge_track_context_with_recorder(&track_row, Some(track), Some(feed), None).unwrap();

        assert_eq!(context.track.track_guid.as_deref(), Some("track-guid"));
        assert_eq!(context.track.feed_guid.as_deref(), Some("feed-guid"));
        assert_eq!(context.track.title.as_deref(), Some("Lantern Tide"));
        assert_eq!(context.track.track_artist.as_deref(), Some("Max DjK"));
        assert_eq!(context.track.release_artist.as_deref(), Some("Max DjK"));
        assert_eq!(context.track.feed_title.as_deref(), Some("Orient Express"));
        assert_ne!(
            context.track.description.as_deref(),
            Some("..."),
            "placeholder source text must not become a display fact"
        );
        assert_eq!(
            context
                .feed
                .as_ref()
                .and_then(|feed| feed.feed_guid.as_deref()),
            Some("feed-guid")
        );
        assert_eq!(
            context.feed.as_ref().and_then(|feed| feed.title.as_deref()),
            Some("Orient Express")
        );
    }

    #[test]
    fn local_track_row_strips_placeholder_text_at_projection_boundary() {
        let polluted_row = TrackRow {
            id: 1,
            feed_id: 2,
            feed_guid: Some("feed-guid".into()),
            item_guid: "track-guid".into(),
            track_title: Some("...".into()),
            artist_name: Some("\u{2026}".into()),
            album_title: Some("...".into()),
            album_artist_name: Some("... \u{2026}".into()),
            track_number: Some(4),
            disc_number: None,
            duration_seconds: Some(149),
            enclosure_url: Some("...".into()),
            enclosure_type: Some("\u{2026}".into()),
            track_image_href: Some("...".into()),
            is_in_library: true,
            feed_title: Some("...".into()),
            album_image_href: Some("...".into()),
            local_path: None,
            pub_date: None,
            explicit: None,
            transcript_url: Some("...".into()),
        };

        let api_track = crate::subscribe_service::track_row_to_api_track(&polluted_row);
        // Identity columns pass through; merge boundary owns their semantics.
        assert_eq!(api_track.track_guid.as_deref(), Some("track-guid"));
        assert_eq!(api_track.feed_guid.as_deref(), Some("feed-guid"));
        // Display facts collapse to None so the metadata grid never renders
        // placeholder transport values as if they were real source facts.
        assert_eq!(api_track.title, None);
        assert_eq!(api_track.track_artist, None);
        assert_eq!(api_track.release_artist, None);
        assert_eq!(api_track.feed_title, None);
        assert_eq!(api_track.enclosure_url, None);
        assert_eq!(api_track.enclosure_type, None);
        assert_eq!(api_track.image_url, None);
        assert!(
            api_track
                .source_links
                .as_ref()
                .is_none_or(|links| links.is_empty()),
            "placeholder transcript URL must not become a source link"
        );

        let local_feed = super::track_row_to_feed(&polluted_row);
        assert_eq!(local_feed.feed_guid.as_deref(), Some("feed-guid"));
        assert_eq!(local_feed.title, None);
        assert_eq!(local_feed.image_url, None);
    }

    #[test]
    fn local_track_context_hydrates_persisted_source_facts() -> Result<()> {
        let mut conn = setup_test_db()?;
        let track = insert_track(&conn)?;
        db::replace_local_identity_ids(
            &mut conn,
            db::LocalIdentityOwner::Feed(track.feed_id),
            "musicindex",
            &[db::LocalIdentityIdInput {
                scheme: Some("nostr_npub".into()),
                value: Some("npub1feed".into()),
                ..db::LocalIdentityIdInput::default()
            }],
        )?;
        db::replace_local_identity_links(
            &mut conn,
            db::LocalIdentityOwner::Track(track.id),
            "musicindex",
            &[db::LocalIdentityLinkInput {
                link_type: Some("website".into()),
                url: Some("https://example.test/track".into()),
                ..db::LocalIdentityLinkInput::default()
            }],
        )?;
        db::replace_local_contributors(
            &mut conn,
            db::LocalEntityOwner::Track(track.id),
            "musicindex",
            &[db::LocalContributorInput {
                position: 0,
                name: Some("Track Contributor".into()),
                image_url: Some("https://example.test/contributor.jpg".into()),
                nostr_npub: Some("npub1contributor".into()),
                ..db::LocalContributorInput::default()
            }],
        )?;

        let context = track_row_to_track_context_with_local_identity(&conn, &track)?;

        assert_eq!(
            context
                .feed
                .as_ref()
                .and_then(|feed| feed.source_ids.as_ref())
                .and_then(|ids| ids.first())
                .and_then(|id| id.value.as_deref()),
            Some("npub1feed")
        );
        assert_eq!(
            context
                .track
                .source_links
                .as_ref()
                .and_then(|links| links.first())
                .and_then(|link| link.url.as_deref()),
            Some("https://example.test/track")
        );
        assert_eq!(
            context
                .track
                .source_contributors
                .as_ref()
                .and_then(|contributors| contributors.first())
                .and_then(|contributor| contributor.img.as_deref()),
            Some("https://example.test/contributor.jpg")
        );

        Ok(())
    }

    #[test]
    fn local_track_context_hydrates_persisted_metadata_facts() -> Result<()> {
        let mut conn = setup_test_db()?;
        let mut track = insert_track(&conn)?;
        track.pub_date = Some(1);
        track.explicit = Some(false);
        db::replace_local_metadata_facts(
            &mut conn,
            db::LocalMetadataOwner::Track(track.id),
            "musicindex",
            &[
                db::LocalMetadataFactInput {
                    fact_key: "publisher_text".into(),
                    value: db::LocalMetadataValue::Text("Example Publisher".into()),
                    extraction_path: None,
                    observed_at: None,
                    raw_json: None,
                },
                db::LocalMetadataFactInput {
                    fact_key: "description".into(),
                    value: db::LocalMetadataValue::Text("Track description".into()),
                    extraction_path: None,
                    observed_at: None,
                    raw_json: None,
                },
                db::LocalMetadataFactInput {
                    fact_key: "pub_date".into(),
                    value: db::LocalMetadataValue::Integer(1_700_000_000),
                    extraction_path: None,
                    observed_at: None,
                    raw_json: None,
                },
                db::LocalMetadataFactInput {
                    fact_key: "explicit".into(),
                    value: db::LocalMetadataValue::Boolean(true),
                    extraction_path: None,
                    observed_at: None,
                    raw_json: None,
                },
            ],
        )?;

        let context = track_row_to_track_context_with_local_identity(&conn, &track)?;

        assert_eq!(
            context.track.publisher_text.as_deref(),
            Some("Example Publisher")
        );
        assert_eq!(
            context.track.description.as_deref(),
            Some("Track description")
        );
        assert_eq!(context.track.pub_date, Some(1_700_000_000));
        assert_eq!(context.track.explicit, Some(true));
        Ok(())
    }
}

/// ADR 0075 packet 017: request-profile behavior at the Library track
/// detail and Library feed update request sites.
#[cfg(test)]
mod adr_0075_request_profile_tests {
    use super::*;
    use crate::application::request_profiles;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::Duration;

    /// A minimal local HTTP server that records each request path.
    struct Fixture {
        endpoint: crate::config::MusicIndexEndpoint,
        address: String,
        requests: Arc<Mutex<Vec<String>>>,
        mode: Arc<AtomicUsize>,
        stop: Arc<AtomicBool>,
        worker: Option<std::thread::JoinHandle<()>>,
    }

    impl Fixture {
        fn start() -> Self {
            // Packet 018 Part B: the shared owner is one process-wide value
            // (P18-6), so this and every other test share it. A fresh
            // ephemeral port makes each fixture's `RequestKey` distinct
            // (the key carries the endpoint), so tests never collide on a
            // retained entry; nothing here clears the shared owner, which
            // would race a concurrently running test's own state.
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap().to_string();
            let endpoint: crate::config::MusicIndexEndpoint = format!("http://{address}").into();
            let requests = Arc::new(Mutex::new(Vec::new()));
            let mode = Arc::new(AtomicUsize::new(0));
            let stop = Arc::new(AtomicBool::new(false));
            let received = Arc::clone(&requests);
            let selected = Arc::clone(&mode);
            let stopped = Arc::clone(&stop);
            let worker = std::thread::spawn(move || {
                while !stopped.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            stream
                                .set_read_timeout(Some(Duration::from_secs(2)))
                                .unwrap();
                            let mut bytes = Vec::new();
                            let mut buffer = [0; 4096];
                            while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                                match stream.read(&mut buffer) {
                                    Ok(0) | Err(_) => break,
                                    Ok(read) => bytes.extend_from_slice(&buffer[..read]),
                                }
                            }
                            let request = String::from_utf8_lossy(&bytes);
                            let Some(path) = request
                                .lines()
                                .next()
                                .and_then(|line| line.split_whitespace().nth(1))
                            else {
                                continue;
                            };
                            received.lock().unwrap().push(path.to_string());
                            let (status, body) = response(path, selected.load(Ordering::SeqCst));
                            write!(
                                stream,
                                "HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                                body.len()
                            )
                            .unwrap();
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(2));
                        }
                        Err(error) => panic!("fixture listener: {error}"),
                    }
                }
            });
            Self {
                endpoint,
                address,
                requests,
                mode,
                stop,
                worker: Some(worker),
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            let _ = std::net::TcpStream::connect(&self.address);
            self.worker.take().unwrap().join().unwrap();
        }
    }

    /// `mode` 0: the scoped track request succeeds. `mode` 1: the scoped
    /// track request fails, so the caller must fall back to the unscoped
    /// track request.
    fn response(path: &str, mode: usize) -> (&'static str, String) {
        let bare = path.split('?').next().unwrap_or(path);
        if mode == 1 && bare == "/v1/feeds/f1/tracks/t1" {
            return (
                "503 Service Unavailable",
                "{\"error\":\"induced failure\"}".into(),
            );
        }
        if bare.contains("/tracks/") {
            // `mode` 2: the track response carries a MusicIndex payment
            // route that differs from the stored route (ADR 0076 packet 003).
            let routes = if mode == 2 {
                serde_json::json!([{
                    "recipient_name": "MusicIndex", "route_type": "node",
                    "split": 100.0, "fee": false, "address": "03abcdef"
                }])
            } else {
                serde_json::Value::Null
            };
            return (
                "200 OK",
                serde_json::json!({"data": {
                    "track_guid": bare.rsplit('/').next(),
                    "feed_guid": "f1",
                    "source_links": [], "source_ids": [], "source_contributors": [],
                    "payment_routes": routes
                }})
                .to_string(),
            );
        }
        (
            "200 OK",
            serde_json::json!({"data": {
                "feed_guid": "f1", "title": "Feed",
                "source_links": [], "source_ids": [], "source_contributors": []
            }})
            .to_string(),
        )
    }

    fn scoped_track_row() -> TrackRow {
        TrackRow {
            id: 1,
            feed_id: 1,
            feed_guid: Some("f1".into()),
            item_guid: "t1".into(),
            ..TrackRow::default()
        }
    }

    /// Percent-encodes an include list's commas the way the `url` crate
    /// encodes a comma-separated query value.
    fn encoded_include(profile: crate::application::request_profiles::RequestProfile) -> String {
        profile
            .include()
            .map(|include| include.replace(',', "%2C"))
            .unwrap_or_default()
    }

    /// R17-04: when the scoped track request succeeds, the Library track
    /// detail route sends the scoped track request first, then the feed
    /// request. It sends no unscoped track request.
    #[test]
    fn adr_0075_request_profile_library_track_detail_scoped_success_skips_unscoped_request() {
        let fixture = Fixture::start();
        let track = scoped_track_row();

        let context = fetch_library_track_context(&track, &fixture.endpoint).unwrap();

        assert_eq!(context.track.track_guid.as_deref(), Some("t1"));
        let requests = fixture.requests.lock().unwrap().clone();
        assert_eq!(
            requests,
            vec![
                format!(
                    "/v1/feeds/f1/tracks/t1?include={}",
                    encoded_include(request_profiles::LIBRARY_TRACK_DETAIL_SCOPED_TRACK)
                ),
                format!(
                    "/v1/feeds/f1?include={}",
                    encoded_include(request_profiles::LIBRARY_TRACK_DETAIL_FEED)
                ),
            ],
            "R17-04: a successful scoped request must not fall back to the unscoped request"
        );
    }

    /// R17-04: when the scoped track request fails, the route falls back
    /// to the unscoped track request, then sends the feed request.
    #[test]
    fn adr_0075_request_profile_library_track_detail_falls_back_to_unscoped_then_feed() {
        let fixture = Fixture::start();
        fixture.mode.store(1, Ordering::SeqCst);
        let track = scoped_track_row();

        let context = fetch_library_track_context(&track, &fixture.endpoint).unwrap();

        assert_eq!(context.track.track_guid.as_deref(), Some("t1"));
        let requests = fixture.requests.lock().unwrap().clone();
        assert_eq!(
            requests,
            vec![
                format!(
                    "/v1/feeds/f1/tracks/t1?include={}",
                    encoded_include(request_profiles::LIBRARY_TRACK_DETAIL_SCOPED_TRACK)
                ),
                format!(
                    "/v1/tracks/t1?include={}",
                    encoded_include(request_profiles::LIBRARY_TRACK_DETAIL_UNSCOPED_TRACK)
                ),
                format!(
                    "/v1/feeds/f1?include={}",
                    encoded_include(request_profiles::LIBRARY_TRACK_DETAIL_FEED)
                ),
            ],
            "R17-04: a failed scoped request must fall back to the unscoped request, then the feed request"
        );
    }

    /// R17-07: the Library feed update request sends L1.
    #[test]
    fn adr_0075_request_profile_library_feed_update_sends_l1() {
        let fixture = Fixture::start();
        let db_conn = Connection::open_in_memory().unwrap();
        db::upgrades::create_fixture(&db_conn, db::CURRENT_VERSION).unwrap();
        db_conn
            .execute(
                "INSERT INTO feeds(feed_url,feed_guid,title,is_subscribed) \
                 VALUES('http://fixture.invalid/feed.xml','f1','Feed',1)",
                [],
            )
            .unwrap();
        let conn = Arc::new(Mutex::new(db_conn));
        let recorder = Arc::new(
            crate::provider_observation::ProviderObservationRecorder::new(Arc::clone(&conn)),
        );
        let stale = StaleFeed {
            feed_id: 1,
            feed_guid: "f1".into(),
            title: None,
            new_updated_at: 100,
        };

        let outcome =
            apply_feed_updates(&conn, &fixture.endpoint, &stale, &recorder, &mut Vec::new())
                .unwrap();

        assert_eq!(outcome.tracks_refreshed, 0);
        let requests = fixture.requests.lock().unwrap().clone();
        assert_eq!(
            requests,
            vec![format!(
                "/v1/feeds/f1?include={}",
                encoded_include(request_profiles::LIBRARY_FEED_UPDATE_FEED)
            )],
            "R17-07: the Library feed update request must send L1"
        );
    }

    /// ADR 0076 Decision 8 (packet 004): a feed update changes the database
    /// only. It stores the `MusicIndex` track record and writes no tag. The
    /// file bytes and the stored route stay the same.
    #[test]
    fn adr_0076_tag_update_feed_update_writes_no_tag() {
        let fixture = Fixture::start();
        fixture.mode.store(2, Ordering::SeqCst);
        let music = tempfile::tempdir().unwrap();
        let path = music.path().join("song.mp3");
        std::fs::write(&path, b"not really an mp3").unwrap();
        let before = std::fs::read(&path).unwrap();
        let stored = serde_json::json!([{
            "recipient_name": "Stored", "route_type": "node",
            "split": 100.0, "fee": false, "address": "03abcdef"
        }])
        .to_string();
        let db_conn = Connection::open_in_memory().unwrap();
        db::upgrades::create_fixture(&db_conn, db::CURRENT_VERSION).unwrap();
        db_conn
            .execute_batch(
                "INSERT INTO feeds(id,feed_url,feed_guid,title,is_subscribed) \
                 VALUES(1,'http://fixture.invalid/feed.xml','f1','Feed',1);",
            )
            .unwrap();
        db_conn
            .execute(
                "INSERT INTO tracks(id,feed_id,item_guid,track_title,is_in_library,payment_routes_json) \
                 VALUES(1,1,'t1','Song',1,?1)",
                [&stored],
            )
            .unwrap();
        db_conn
            .execute(
                "INSERT INTO local_files(path,track_id) VALUES('song.mp3',1)",
                [],
            )
            .unwrap();
        let conn = Arc::new(Mutex::new(db_conn));
        let recorder = Arc::new(
            crate::provider_observation::ProviderObservationRecorder::new(Arc::clone(&conn)),
        );
        let stale = StaleFeed {
            feed_id: 1,
            feed_guid: "f1".into(),
            title: None,
            new_updated_at: 100,
        };

        let outcome =
            apply_feed_updates(&conn, &fixture.endpoint, &stale, &recorder, &mut Vec::new())
                .unwrap();

        assert_eq!(outcome.tracks_refreshed, 1);
        assert!(fixture
            .requests
            .lock()
            .unwrap()
            .iter()
            .any(|request| request.contains("/tracks/t1")));
        assert_eq!(
            std::fs::read(&path).unwrap(),
            before,
            "ADR 0076 Decision 8: a feed update writes no audio tag"
        );
        let route: String = conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT payment_routes_json FROM tracks WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(route, stored, "the stored route stays the same");
    }
}
