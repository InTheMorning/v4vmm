//! RSS enrichment and observation capture for ADR 0075.

use std::collections::HashMap;
use std::fmt;
use std::io::Cursor;
use std::sync::atomic::AtomicI64;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use roxmltree::{Document, Node};
use rss::{extension::ExtensionMap, Channel};

use super::helpers::{clean_text, find_ext, parse_itunes_duration};
use super::{IdentityValidation, NostrIdentity};
use crate::api::{Feed, SourceEntityId, SourceEntityLink, Track};
use crate::application::request_reuse::{self, RefreshIntent, RetainedCache, SharedFetchError};
use crate::metadata::{source_text_missing, TrackContext};
use crate::provider_observation::ProviderObservation;

/// P18-3: a parsed RSS document stays reusable for this long, keyed by feed
/// URL. This is the one place this value appears in the code (R18B-03).
const RSS_DOCUMENT_REUSE_WINDOW: Duration = Duration::from_secs(15 * 60);

/// P18-5: the app holds at most this many parsed RSS documents.
const RSS_DOCUMENT_CAPACITY: usize = 32;

/// A fetched RSS response, retained so a later enrichment call for a
/// different track of the same feed does not repeat the HTTP request.
///
/// Re-parsing the retained bytes for a different track is cheap, in-memory
/// work: no network call. `parse_track_enrichment_document` already does
/// this from the outer response fields, whether or not the response came
/// from a retained entry.
#[derive(Clone)]
struct CachedRssDocument {
    response_url: String,
    fetched_at: DateTime<Utc>,
    response_bytes: Arc<[u8]>,
    /// The receipt of the fetch that produced these bytes, when a
    /// recorder observed it. A later observed call replays this receipt,
    /// so a reused document names the observation that produced it and
    /// creates no second observation (R18B-07). An unobserved fetch
    /// retains `None`, and a later observed call then fetches again
    /// rather than report evidence it does not have.
    receipt: Option<crate::provider_observation::ObservationReceipt>,
}

static RETAINED_DOCUMENTS: OnceLock<Mutex<RetainedCache<String, CachedRssDocument>>> =
    OnceLock::new();

fn retained_documents() -> &'static Mutex<RetainedCache<String, CachedRssDocument>> {
    RETAINED_DOCUMENTS.get_or_init(|| {
        Mutex::new(RetainedCache::new(
            RSS_DOCUMENT_REUSE_WINDOW,
            RSS_DOCUMENT_CAPACITY,
        ))
    })
}

fn retained_document(feed_url: &str) -> Option<CachedRssDocument> {
    retained_documents()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&feed_url.to_owned())
}

fn retain_document(feed_url: &str, document: CachedRssDocument) {
    retained_documents()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .store(feed_url.to_owned(), document);
}

/// P18-7: an explicit feed refresh removes that feed's retained RSS
/// document, so the next enrichment call fetches and parses it again.
/// `feed_service::apply_feed_updates` calls this before it sends any
/// request for an explicitly refreshed feed.
pub(crate) fn invalidate_feed_document(feed_url: &str) {
    retained_documents()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(&feed_url.to_owned());
}

/// ADR 0076 packet 001: check that a playlist-check response holds a
/// readable RSS document before the check records and retains it.
///
/// The check uses the decode and root rules of the track enrichment parse.
/// It compares nothing. A failure marks the observation `xml_decode`, as
/// the enrichment fetch does, so a malformed document never becomes the
/// retained document and never supplies a validator.
pub(crate) fn check_rss_document(observation: &mut ProviderObservation) -> Result<()> {
    observation.decoder_version = crate::provider_observation::contracts::RSS_DECODER.into();
    let result = match observation.body.clone() {
        None => Err(anyhow!("RSS response has no body")),
        Some(body) => crate::provider_observation::contracts::decode_rss_body(
            &body,
            Some(&mut observation.interpretation["charset"]),
        )
        .and_then(|_| {
            // The enrichment parse without a track applies the same XML and
            // root rules, and it keeps one DOM parser for RSS (ADR 0075).
            let url = observation.response_uri.clone().unwrap_or_default();
            parse_track_enrichment_document(&url, &url, Utc::now(), body, None, None).map(|_| ())
        }),
    };
    if result.is_err() {
        observation.fail("xml_decode");
    }
    result
}

/// ADR 0076 packet 001: a `200` response of the playlist check replaces the
/// retained document of its feed URL, with the receipt of that fetch. A
/// later enrichment call for a track of this feed then reuses the checked
/// bytes and replays this receipt (R18B-07).
///
/// The caller passes only an observation that `check_rss_document` accepted.
/// This function retains nothing for a failed, empty or incomplete response.
pub(crate) fn retain_checked_document(
    feed_url: &str,
    observation: &ProviderObservation,
    receipt: crate::provider_observation::ObservationReceipt,
) -> bool {
    use crate::provider_observation::ObservationOutcome;
    let (Some(body), Some(fetched_at)) = (
        observation.body.as_ref(),
        observation
            .fetched_at_us
            .and_then(DateTime::from_timestamp_micros),
    ) else {
        return false;
    };
    if observation.outcome != ObservationOutcome::Success
        || !observation
            .http_status
            .is_some_and(|status| (200..300).contains(&status))
        || observation.interpretation["body_state"] != "complete"
    {
        return false;
    }
    retain_document(
        feed_url,
        CachedRssDocument {
            response_url: observation
                .response_uri
                .clone()
                .unwrap_or_else(|| feed_url.to_owned()),
            fetched_at,
            response_bytes: Arc::clone(body),
            receipt: Some(receipt),
        },
    );
    true
}

/// Packet 018: shares an in-flight RSS GET across concurrent callers of the
/// same feed URL, keyed by feed URL like the completed-response cache
/// above. This closes the gap the packet's own concurrent measurement
/// found: two concurrent Library track details of one track used to send
/// two GETs for one feed, because only the completed-response cache
/// existed, with no active-request join for a request still in flight.
///
/// This reuses `application::request_reuse`'s generic `single_flight`
/// (packet 018 Part A's own machinery, generalized over its key type for
/// this reuse) instead of a second copy of it, so the same abandonment
/// guarantee applies here too: a panic in the fetch fails the slot, wakes
/// every joined caller, and removes the identity.
///
/// The shared value is the raw captured HTTP response (`ProviderObservation`,
/// before decoding or parsing), not a parsed, track-matched result: a
/// concurrent caller may want a different track's enrichment from the same
/// feed, so only the network fetch is shared. Each caller — the winner and
/// every joiner — still decodes and parses its own clone for its own track
/// GUID and enclosure, and still records its own observation, because an
/// RSS observation's outcome depends on which item matched that caller's
/// own request.
static RSS_FETCH_SEQUENCE: AtomicI64 = AtomicI64::new(0);
static RSS_FETCH_REGISTRY: OnceLock<request_reuse::Registry<String, ProviderObservation>> =
    OnceLock::new();

fn rss_fetch_registry() -> &'static request_reuse::Registry<String, ProviderObservation> {
    RSS_FETCH_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

const PODCAST_NAMESPACES: [&str; 2] = [
    "https://podcastindex.org/namespace/1.0",
    "https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/1.0.md",
];
const ITUNES_NAMESPACES: [&str; 1] = ["http://www.itunes.com/dtds/podcast-1.0.dtd"];
const RSS1_NAMESPACE: &str = "http://purl.org/rss/1.0/";
const RDF_ROOT_NAMESPACE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PodrollEntry {
    pub feed_guid: Option<String>,
    pub feed_url: Option<String>,
}

pub fn fetch_feed_podroll(feed_url: &str) -> Result<Vec<PodrollEntry>> {
    let body = crate::http_client::document()
        .get(feed_url)
        .send()
        .with_context(|| format!("GET {feed_url}"))?
        .error_for_status()
        .with_context(|| format!("HTTP error for {feed_url}"))?
        .bytes()
        .with_context(|| format!("read body {feed_url}"))?;
    let channel = Channel::read_from(Cursor::new(body)).context("parse RSS")?;
    Ok(podroll_entries(channel.extensions()))
}

fn podroll_entries(exts: &ExtensionMap) -> Vec<PodrollEntry> {
    let Some(podroll) = find_ext(exts, "podcast", "podroll") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (name, list) in &podroll.children {
        if name != "remoteItem" {
            continue;
        }
        for child in list {
            let entry = PodrollEntry {
                feed_guid: child.attrs.get("feedGuid").cloned(),
                feed_url: child.attrs.get("feedUrl").cloned(),
            };
            if entry.feed_guid.is_some() || entry.feed_url.is_some() {
                out.push(entry);
            }
        }
    }
    out
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RssTrackEnrichment {
    pub feed_title: Option<String>,
    pub feed_description: Option<String>,
    pub feed_artist: Option<String>,
    pub feed_image_url: Option<String>,
    pub feed_episode_count: Option<i32>,
    pub track_title: Option<String>,
    pub track_description: Option<String>,
    pub track_artist: Option<String>,
    pub track_image_url: Option<String>,
    pub track_number: Option<i32>,
    pub duration_secs: Option<i32>,
    pub pub_date: Option<i64>,
    pub transcript_url: Option<String>,
    pub transcript_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RssIdentityOwner {
    Feed,
    Track { item_index: usize },
}
#[derive(Clone, Debug, PartialEq, Eq)]
/// Position in the decoded XML character document.
pub struct RssSourcePosition {
    pub decoded_byte_offset: usize,
    pub line: u32,
    pub column: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RssTxtValidation {
    ValidPublicKey(NostrIdentity),
    ValidProfile(NostrIdentity),
    UnsupportedPurpose,
    UnsupportedEncoding,
    MalformedEncoding,
}
#[derive(Clone, PartialEq, Eq)]
pub struct RssTxtEvidence {
    pub owner: RssIdentityOwner,
    pub occurrence_position: usize,
    pub owner_guid: Option<String>,
    pub purpose: Option<String>,
    pub direct_text: Option<String>,
    pub element_xml: String,
    pub namespace_uri: Option<String>,
    pub source_position: RssSourcePosition,
    pub extraction_path: String,
    pub validation: RssTxtValidation,
}
impl fmt::Debug for RssTxtEvidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RssTxtEvidence")
            .field("owner", &self.owner)
            .field("occurrence_position", &self.occurrence_position)
            .field("namespace_uri", &self.namespace_uri)
            .field("source_position", &self.source_position)
            .field("extraction_path", &self.extraction_path)
            .field("validation", &self.validation)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RssExcludedElementKind {
    Contributor,
    NestedTxt,
    UnknownNamespaceTxt,
}
#[derive(Clone, PartialEq, Eq)]
pub struct RssExcludedElement {
    pub kind: RssExcludedElementKind,
    pub element_xml: String,
    pub namespace_uri: Option<String>,
    pub source_position: RssSourcePosition,
    pub extraction_path: String,
}
impl fmt::Debug for RssExcludedElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RssExcludedElement")
            .field("kind", &self.kind)
            .field("namespace_uri", &self.namespace_uri)
            .field("source_position", &self.source_position)
            .field("extraction_path", &self.extraction_path)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RssItemMatch {
    Matched {
        item_index: usize,
        observed_item_guid: Option<String>,
        observed_enclosure_url: Option<String>,
    },
    NoMatch,
}
#[derive(Clone, PartialEq, Eq)]
pub struct RssObservation {
    pub requested_url: String,
    pub response_url: String,
    pub fetched_at: DateTime<Utc>,
    pub response_bytes: Arc<[u8]>,
    pub character_encoding: String,
    pub requested_track_guid: Option<String>,
    pub requested_enclosure_url: Option<String>,
    pub observed_feed_guid: Option<String>,
    pub item_match: RssItemMatch,
    pub txt_evidence: Vec<RssTxtEvidence>,
    pub excluded_elements: Vec<RssExcludedElement>,
}
impl fmt::Debug for RssObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RssObservation")
            .field("response_byte_count", &self.response_bytes.len())
            .field("character_encoding", &self.character_encoding)
            .field(
                "has_matching_item",
                &matches!(self.item_match, RssItemMatch::Matched { .. }),
            )
            .field("txt_evidence_count", &self.txt_evidence.len())
            .field("excluded_element_count", &self.excluded_elements.len())
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug)]
pub struct RssFetchResult {
    pub observation: Arc<RssObservation>,
    pub enrichment: Option<RssTrackEnrichment>,
}

pub fn enrich_track_from_feed_rss(context: &mut TrackContext, feed_url: &str) -> Result<bool> {
    enrich_track_from_feed_rss_with_recorder(context, feed_url, None)
}

pub fn enrich_track_from_feed_rss_observed(
    context: &mut TrackContext,
    feed_url: &str,
    recorder: &crate::provider_observation::ProviderObservationRecorder,
) -> Result<bool> {
    enrich_track_from_feed_rss_with_recorder(context, feed_url, Some(recorder))
}

fn enrich_track_from_feed_rss_with_recorder(
    context: &mut TrackContext,
    feed_url: &str,
    recorder: Option<&crate::provider_observation::ProviderObservationRecorder>,
) -> Result<bool> {
    let result = fetch_track_enrichment_with_recorder(
        feed_url,
        context.track.track_guid.as_deref(),
        context.track.enclosure_url.as_deref(),
        recorder,
    )?;
    context.rss_observation = Some(Arc::clone(&result.observation));
    let Some(enrichment) = result.enrichment else {
        return Ok(false);
    };
    let mut changed =
        apply_track_enrichment(&mut context.track, context.feed.as_mut(), &enrichment);
    if let Some(url) = enrichment.transcript_url {
        changed |= append_track_source_link(
            &mut context.track,
            "transcript",
            &url,
            "podcast:transcript@url",
        );
    }
    changed |= apply_validated_ids(context, &result.observation);
    Ok(changed)
}

pub fn fetch_track_enrichment_from_feed(
    feed_url: &str,
    track_guid: Option<&str>,
    enclosure_url: Option<&str>,
) -> Result<RssFetchResult> {
    fetch_track_enrichment_with_recorder(feed_url, track_guid, enclosure_url, None)
}

/// Fetches, or reuses, a feed's RSS document and parses it for one track.
fn fetch_track_enrichment_with_recorder(
    feed_url: &str,
    track_guid: Option<&str>,
    enclosure_url: Option<&str>,
    recorder: Option<&crate::provider_observation::ProviderObservationRecorder>,
) -> Result<RssFetchResult> {
    // Packet 018 P18-3: a retained document reuses the last fetched bytes
    // for a different track of the same feed. Re-parsing them for this
    // track's own GUID and enclosure is cheap, in-memory work, and it
    // sends no request.
    //
    // A caller with a recorder replays the receipt of the fetch that
    // produced those bytes. A reused response names the observation that
    // produced it, and it creates no second observation of one fetch
    // (R18B-07). The feed and track caches in
    // `application::request_reuse` replay their receipts the same way.
    // The replayed receipt keeps the generation of that earlier fetch,
    // because the evidence is older than this call. The operator decided
    // this on 2026-09-22, against a new observation for each reuse.
    match (retained_document(feed_url), recorder) {
        (Some(cached), Some(recorder)) => {
            if let Some(receipt) = cached.receipt.clone() {
                let result = parse_track_enrichment_document(
                    feed_url,
                    &cached.response_url,
                    cached.fetched_at,
                    cached.response_bytes,
                    track_guid,
                    enclosure_url,
                );
                recorder.replay(receipt);
                return result;
            }
            // An unobserved fetch retained these bytes, so no receipt
            // exists to replay. This call fetches again and records its
            // own observation, rather than report evidence it cannot
            // name.
        }
        (Some(cached), None) => {
            return parse_track_enrichment_document(
                feed_url,
                &cached.response_url,
                cached.fetched_at,
                cached.response_bytes,
                track_guid,
                enclosure_url,
            );
        }
        _ => {}
    }
    if let Some(recorder) = recorder {
        use crate::provider_observation::{contracts, http, ObservationOutcome};
        let mut spec = contracts::rss_request(feed_url, track_guid, enclosure_url);
        spec.started_at_us = Utc::now().timestamp_micros();
        let token = recorder.begin(spec)?;
        // Packet 018: share the GET itself with a concurrent caller of the
        // same feed URL (see `rss_fetch_registry`'s own documentation for
        // why this shares only the fetch, not the parse or the
        // observation). Every caller passes `Normal`: an RSS document has
        // no explicit-refresh concept of its own at this level — P18-7's
        // explicit refresh instead invalidates the retained document
        // (`invalidate_feed_document`, called by `feed_service::apply_feed_updates`)
        // before this function runs, so the lookup above already misses
        // and this call reaches the network on its own.
        let (captured, _generation) = request_reuse::single_flight(
            rss_fetch_registry(),
            &RSS_FETCH_SEQUENCE,
            feed_url.to_owned(),
            RefreshIntent::Normal,
            || {
                Ok(http::capture(
                    crate::http_client::document().get(feed_url),
                    contracts::RSS_DECODER,
                ))
            },
        );
        let mut observation = captured.map_err(SharedFetchError::into_anyhow)?;
        let result = if observation.outcome == ObservationOutcome::Failed {
            Err(anyhow!("RSS request failed"))
        } else {
            parse_track_enrichment_document_observed(
                feed_url,
                observation
                    .response_uri
                    .clone()
                    .as_deref()
                    .unwrap_or(feed_url),
                DateTime::from_timestamp_micros(
                    observation
                        .fetched_at_us
                        .expect("completed response has fetch time"),
                )
                .expect("recorded UTC time is valid"),
                Arc::clone(observation.body.as_ref().expect("response has body")),
                track_guid,
                enclosure_url,
                Some(&mut observation),
            )
        };
        match &result {
            Err(_) if observation.outcome != ObservationOutcome::Failed => {
                observation.fail("xml_decode")
            }
            Ok(result) if result.enrichment.is_none() => {
                observation.outcome = ObservationOutcome::Partial;
                observation.failure = Some(serde_json::json!({"reason":"no_matching_item"}));
            }
            _ => {}
        }
        let response_url = observation
            .response_uri
            .clone()
            .unwrap_or_else(|| feed_url.to_owned());
        let fetched_at = observation
            .fetched_at_us
            .and_then(|micros| DateTime::from_timestamp_micros(micros).map(|time| (time, micros)));
        let response_bytes = observation.body.as_ref().map(Arc::clone);
        // The receipt of this fetch belongs with its retained bytes, so a
        // later observed call can replay it (R18B-07). The write happens
        // first, because the receipt exists only after it.
        let receipt = recorder.record(token, observation)?;
        // P18-4: never retain a failure. A malformed document still fails
        // `parse_track_enrichment_document_observed`, so it never reaches
        // `retain_document` either.
        if let (Ok(_), Some((fetched_at, _)), Some(response_bytes)) =
            (&result, fetched_at, response_bytes)
        {
            retain_document(
                feed_url,
                CachedRssDocument {
                    response_url,
                    fetched_at,
                    response_bytes,
                    receipt: Some(receipt),
                },
            );
        }
        return result;
    }
    let response = crate::http_client::document()
        .get(feed_url)
        .send()
        .with_context(|| format!("GET {feed_url}"))?
        .error_for_status()
        .with_context(|| format!("HTTP error for {feed_url}"))?;
    let response_url = response.url().to_string();
    let bytes: Arc<[u8]> = response
        .bytes()
        .with_context(|| format!("read body {feed_url}"))?
        .to_vec()
        .into();
    let fetched_at = Utc::now();
    let result = parse_track_enrichment_document(
        feed_url,
        &response_url,
        fetched_at,
        Arc::clone(&bytes),
        track_guid,
        enclosure_url,
    );
    if result.is_ok() {
        retain_document(
            feed_url,
            CachedRssDocument {
                response_url,
                fetched_at,
                response_bytes: bytes,
                // No recorder observed this fetch, so it has no receipt.
                receipt: None,
            },
        );
    }
    result
}

fn parse_track_enrichment_document(
    requested_url: &str,
    response_url: &str,
    fetched_at: DateTime<Utc>,
    response_bytes: Arc<[u8]>,
    track_guid: Option<&str>,
    enclosure_url: Option<&str>,
) -> Result<RssFetchResult> {
    parse_track_enrichment_document_observed(
        requested_url,
        response_url,
        fetched_at,
        response_bytes,
        track_guid,
        enclosure_url,
        None,
    )
}

fn parse_track_enrichment_document_observed(
    requested_url: &str,
    response_url: &str,
    fetched_at: DateTime<Utc>,
    response_bytes: Arc<[u8]>,
    track_guid: Option<&str>,
    enclosure_url: Option<&str>,
    mut retained: Option<&mut crate::provider_observation::ProviderObservation>,
) -> Result<RssFetchResult> {
    if let Some(observation) = retained.as_mut() {
        observation.decoder_version = crate::provider_observation::contracts::RSS_DECODER.into();
    }
    let decoded = crate::provider_observation::contracts::decode_rss_body(
        &response_bytes,
        retained
            .as_mut()
            .map(|observation| &mut observation.interpretation["charset"]),
    )?;
    let character_encoding = decoded.encoding().to_owned();
    count_observation_dom_parse();
    let document = Document::parse(decoded.text()).map_err(|error| {
        let position = error.pos();
        anyhow!(
            "RSS XML is invalid at line {}, column {}",
            position.row,
            position.col
        )
    })?;
    let root = document.root_element();
    let valid_root = (root.tag_name().name() == "rss" && root.tag_name().namespace().is_none())
        || (root.tag_name().name() == "RDF"
            && root.tag_name().namespace() == Some(RDF_ROOT_NAMESPACE));
    if !valid_root {
        return Err(anyhow!("RSS document has an unsupported root"));
    }
    let channel = children(root)
        .filter(|node| {
            node.tag_name().name() == "channel" && ordinary_namespace(node.tag_name().namespace())
        })
        .last()
        .ok_or_else(|| anyhow!("RSS document has no channel"))?;
    let feed_guid = extension(channel, &PODCAST_NAMESPACES, "guid")
        .map(scalar_text)
        .filter(|value| !value.is_empty());
    let mut feed_items = items(channel).collect::<Vec<_>>();
    feed_items.extend(items(root));
    let matched = feed_items
        .iter()
        .enumerate()
        .find(|(_, item)| item_matches(**item, track_guid, enclosure_url));
    let item_match = matched.map_or(RssItemMatch::NoMatch, |(item_index, item)| {
        RssItemMatch::Matched {
            item_index,
            observed_item_guid: raw_text_child(*item, "guid"),
            observed_enclosure_url: child(*item, "enclosure")
                .and_then(|node| node.attribute("url"))
                .map(ToOwned::to_owned),
        }
    });
    let mut evidence = owner_evidence(
        &document,
        channel,
        RssIdentityOwner::Feed,
        feed_guid.clone(),
        &channel_path(root),
    );
    let mut excluded_elements = excluded(&document, channel, &channel_path(root));
    for (item_index, item) in feed_items.iter().enumerate() {
        let path = item_path(root, *item);
        evidence.extend(owner_evidence(
            &document,
            *item,
            RssIdentityOwner::Track { item_index },
            raw_text_child(*item, "guid"),
            &path,
        ));
        excluded_elements.extend(excluded(&document, *item, &path));
    }
    let enrichment =
        matched.map(|(_, item)| enrichment_from_nodes(channel, *item, feed_items.len()));
    if let Some(retained) = retained {
        use crate::provider_observation::contracts;
        retained.decoder_version = contracts::RSS_DECODER.into();
        let request = contracts::rss_request(requested_url, track_guid, enclosure_url);
        for owner in std::iter::once(channel).chain(feed_items.iter().copied()) {
            let scope = retain_dom_evidence(retained, &document, owner, requested_url, &evidence);
            let proof = contracts::certify_rss(retained, &request, scope, owner, &decoded);
            retained.coverage[scope].proof = proof;
        }
    }
    Ok(RssFetchResult {
        observation: Arc::new(RssObservation {
            requested_url: requested_url.into(),
            response_url: response_url.into(),
            fetched_at,
            response_bytes,
            character_encoding,
            requested_track_guid: track_guid.map(str::to_owned),
            requested_enclosure_url: enclosure_url.map(str::to_owned),
            observed_feed_guid: feed_guid,
            item_match,
            txt_evidence: evidence,
            excluded_elements,
        }),
        enrichment,
    })
}

fn retain_dom_evidence(
    retained: &mut crate::provider_observation::ProviderObservation,
    document: &Document<'_>,
    owner: Node<'_, '_>,
    resource: &str,
    identities: &[RssTxtEvidence],
) -> usize {
    use crate::provider_observation::{
        contracts, CoverageEvidence, FactEvidence, ObservationRetention, PropertyPresence,
    };
    use serde_json::json;
    let subject = contracts::rss_subject(owner, resource);
    let declared_owner = json!({"subject":subject,"feed_resource":resource});
    let scope = retained.coverage.len();
    retained.coverage.push(CoverageEvidence {
        collection: "source_ids".into(),
        target: subject.clone(),
        target_owner: declared_owner.clone(),
        request_intent: "implicit".into(),
        presence: PropertyPresence::Empty,
        retention: ObservationRetention::Partial,
        basis: json!({"enumeration":"direct_podcast_txt","reason":"unverified_unless_registered_proof"}),
        facts: Vec::new(),
        proof: None,
    });
    for node in owner.descendants().filter(|n| {
        n.is_element()
            && *n != owner
            && n.ancestors()
                .skip(1)
                .find(|a| a.tag_name().name() == "item" || a.tag_name().name() == "channel")
                == Some(owner)
    }) {
        let position = source_position(document, node);
        let identity = identities
            .iter()
            .find(|i| i.source_position.decoded_byte_offset == position.decoded_byte_offset);
        let validation = identity.map_or("unresolved", |i| match i.validation {
            RssTxtValidation::ValidPublicKey(_) | RssTxtValidation::ValidProfile(_) => "valid",
            RssTxtValidation::MalformedEncoding => "malformed",
            RssTxtValidation::UnsupportedPurpose | RssTxtValidation::UnsupportedEncoding => {
                "unsupported"
            }
        });
        let direct_owner = node.parent() == Some(owner);
        let collection = if identity.is_some() {
            "source_ids".into()
        } else {
            format!(
                "field:xml:{{{}}}{}",
                node.tag_name().namespace().unwrap_or_default(),
                node.tag_name().name()
            )
        };
        let locator = json!({"decoded_byte_offset":position.decoded_byte_offset,"line":position.line,"column":position.column});
        let fact = FactEvidence {
            subject: if direct_owner { subject.clone() } else { None },
            declared_owner: declared_owner.clone(),
            owner_basis: json!({"basis":if direct_owner{"direct_xml_owner"}else{"nested_unresolved"}}),
            kind: collection.clone(),
            assertion_source: Some("rss".into()),
            source_position: identity.and_then(|i| i64::try_from(i.occurrence_position).ok()),
            extraction_path: identity.map(|i| i.extraction_path.clone()),
            source_observed: None,
            representation: "structured".into(),
            validation: validation.into(),
            value: contracts::rss_element_value(node),
            raw_member: Some(json!({"element_xml":document.input_text()[node.range()]})),
            body_locator: locator,
        };
        if identity.is_some() && direct_owner {
            let coverage = &mut retained.coverage[scope];
            coverage.presence = PropertyPresence::Populated;
            coverage.facts.push(fact);
            continue;
        }
        retained.coverage.push(CoverageEvidence {
            proof: None,
            collection,
            target: if direct_owner { subject.clone() } else { None },
            target_owner: declared_owner.clone(),
            request_intent: "implicit".into(),
            presence: PropertyPresence::Populated,
            retention: ObservationRetention::Partial,
            basis: json!({"reason":"no_registered_completeness_contract"}),
            facts: vec![fact],
        });
    }
    scope
}

/// The retained bytes and receipt of a feed URL, for tests of other modules.
#[cfg(test)]
pub(crate) fn retained_document_for_test(
    feed_url: &str,
) -> Option<(
    Arc<[u8]>,
    Option<crate::provider_observation::ObservationReceipt>,
)> {
    retained_document(feed_url).map(|document| (document.response_bytes, document.receipt))
}

#[cfg(test)]
fn decode_xml(
    bytes: &[u8],
    retained_encoding: Option<&mut serde_json::Value>,
) -> Result<(String, String)> {
    let decoded =
        crate::provider_observation::contracts::decode_rss_body(bytes, retained_encoding)?;
    Ok((decoded.text().to_owned(), decoded.encoding().to_owned()))
}
fn children<'a, 'input>(node: Node<'a, 'input>) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children().filter(|node| node.is_element())
}
fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    children(node)
        .filter(|child| {
            child.tag_name().name() == name && ordinary_namespace(child.tag_name().namespace())
        })
        .last()
}
fn direct_text(node: Node<'_, '_>) -> String {
    node.children()
        .filter(|child| child.is_text())
        .filter_map(|child| child.text())
        .collect()
}
fn scalar_text(node: Node<'_, '_>) -> String {
    // ADR 0075 preserves rss::element_text: skip nested elements, then trim direct text.
    direct_text(node).trim().to_owned()
}
fn raw_text_child(node: Node<'_, '_>, name: &str) -> Option<String> {
    child(node, name).map(scalar_text)
}
fn text_child(node: Node<'_, '_>, name: &str) -> Option<String> {
    raw_text_child(node, name).and_then(|value| clean_text(Some(&value)))
}
fn channel_text(node: Node<'_, '_>, name: &str) -> Option<String> {
    // Channel title and description retain the last XML-nonempty value before placeholder handling.
    children(node)
        .filter(|child| {
            child.tag_name().name() == name && ordinary_namespace(child.tag_name().namespace())
        })
        .map(scalar_text)
        .filter(|value| !value.is_empty())
        .last()
        .and_then(|value| clean_text(Some(&value)))
}
fn items<'a, 'input>(node: Node<'a, 'input>) -> impl Iterator<Item = Node<'a, 'input>> {
    children(node).filter(|child| {
        child.tag_name().name() == "item" && ordinary_namespace(child.tag_name().namespace())
    })
}
fn extension<'a, 'input>(
    node: Node<'a, 'input>,
    namespaces: &[&str],
    name: &str,
) -> Option<Node<'a, 'input>> {
    children(node).find(|child| {
        child.tag_name().name() == name
            && child
                .tag_name()
                .namespace()
                .is_some_and(|namespace| namespaces.contains(&namespace))
    })
}
fn item_matches(item: Node<'_, '_>, track_guid: Option<&str>, enclosure_url: Option<&str>) -> bool {
    track_guid
        .zip(raw_text_child(item, "guid"))
        .is_some_and(|(wanted, found)| wanted == found)
        || enclosure_url
            .zip(child(item, "enclosure").map(|node| node.attribute("url").unwrap_or_default()))
            .is_some_and(|(wanted, found)| wanted == found)
}

fn enrichment_from_nodes(
    channel: Node<'_, '_>,
    item: Node<'_, '_>,
    item_count: usize,
) -> RssTrackEnrichment {
    let itunes_text = |node, name| {
        extension(node, &ITUNES_NAMESPACES, name)
            .map(scalar_text)
            .and_then(|value| clean_text(Some(&value)))
    };
    let podcast_text = |node, name| extension(node, &PODCAST_NAMESPACES, name).map(scalar_text);
    RssTrackEnrichment {
        feed_title: channel_text(channel, "title"),
        feed_description: channel_text(channel, "description"),
        feed_artist: itunes_text(channel, "author"),
        feed_image_url: extension(channel, &ITUNES_NAMESPACES, "image")
            .and_then(|node| node.attribute("href"))
            .and_then(|value| clean_text(Some(value)))
            .or_else(|| {
                channel
                    .parent()
                    .and_then(|root| child(root, "image"))
                    .or_else(|| child(channel, "image"))
                    .and_then(|image| text_child(image, "url"))
            }),
        feed_episode_count: item_count.try_into().ok(),
        track_title: text_child(item, "title"),
        track_description: text_child(item, "description"),
        track_artist: itunes_text(item, "author")
            .or_else(|| text_child(item, "author"))
            .or_else(|| first_person(item)),
        track_image_url: extension(item, &ITUNES_NAMESPACES, "image")
            .and_then(|node| node.attribute("href"))
            .and_then(|value| clean_text(Some(value))),
        track_number: podcast_text(item, "episode").and_then(|value| value.trim().parse().ok()),
        duration_secs: itunes_text(item, "duration")
            .and_then(|value| parse_itunes_duration(&value))
            .and_then(|value| value.try_into().ok()),
        pub_date: text_child(item, "pubDate").and_then(|value| parse_rss_pub_date(&value)),
        transcript_url: extension(item, &PODCAST_NAMESPACES, "transcript")
            .and_then(|node| node.attribute("url"))
            .map(str::to_owned),
        transcript_type: extension(item, &PODCAST_NAMESPACES, "transcript")
            .and_then(|node| node.attribute("type"))
            .map(str::to_owned),
    }
}
fn first_person(item: Node<'_, '_>) -> Option<String> {
    children(item).find_map(|person| {
        let role = person
            .attribute("role")
            .unwrap_or_default()
            .to_ascii_lowercase();
        (person.tag_name().name() == "person"
            && podcast_namespace(person.tag_name().namespace())
            && ["artist", "creator", "composer", "performer"]
                .iter()
                .any(|name| role.contains(name)))
        .then(|| clean_text(Some(&scalar_text(person))))
        .flatten()
    })
}

fn owner_evidence(
    document: &Document<'_>,
    owner_node: Node<'_, '_>,
    owner: RssIdentityOwner,
    owner_guid: Option<String>,
    path: &str,
) -> Vec<RssTxtEvidence> {
    children(owner_node)
        .filter(|node| {
            node.tag_name().name() == "txt" && podcast_namespace(node.tag_name().namespace())
        })
        .enumerate()
        .map(|(occurrence_position, node)| {
            let purpose = node.attribute("purpose").map(str::to_owned);
            let direct_text = direct_text(node);
            let candidate =
                (!node.children().any(|child| child.is_element())).then_some(direct_text.as_str());
            let validation = validate_txt(purpose.as_deref(), candidate);
            RssTxtEvidence {
                owner: owner.clone(),
                occurrence_position,
                owner_guid: owner_guid.clone(),
                purpose: purpose.clone(),
                direct_text: Some(direct_text),
                element_xml: document.input_text()[node.range()].to_owned(),
                namespace_uri: node.tag_name().namespace().map(str::to_owned),
                source_position: source_position(document, node),
                extraction_path: format!("{path}/podcast:txt[{}]/text()", occurrence_position + 1),
                validation,
            }
        })
        .collect()
}
fn validate_txt(purpose: Option<&str>, direct_text: Option<&str>) -> RssTxtValidation {
    if purpose.map(|value| value.trim().to_ascii_lowercase()) != Some("npub".to_owned()) {
        return RssTxtValidation::UnsupportedPurpose;
    }
    let Some(candidate) = direct_text.map(str::trim).filter(|value| !value.is_empty()) else {
        return RssTxtValidation::MalformedEncoding;
    };
    match super::identity::validate_nostr_identity(candidate) {
        IdentityValidation::Valid(value @ NostrIdentity::PublicKey { .. }) => {
            RssTxtValidation::ValidPublicKey(value)
        }
        IdentityValidation::Valid(value @ NostrIdentity::Profile { .. }) => {
            RssTxtValidation::ValidProfile(value)
        }
        IdentityValidation::UnsupportedEncoding => RssTxtValidation::UnsupportedEncoding,
        IdentityValidation::Malformed(_) => RssTxtValidation::MalformedEncoding,
    }
}
fn excluded(document: &Document<'_>, owner: Node<'_, '_>, path: &str) -> Vec<RssExcludedElement> {
    owner
        .descendants()
        .skip(1)
        .filter(|node| node.is_element())
        .filter_map(|node| {
            if owner.tag_name().name() == "channel"
                && node.ancestors().skip(1).any(|ancestor| {
                    ancestor.parent() == Some(owner)
                        && ancestor.tag_name().name() == "item"
                        && ordinary_namespace(ancestor.tag_name().namespace())
                })
            {
                return None;
            }
            let direct_txt = node.parent() == Some(owner)
                && node.tag_name().name() == "txt"
                && podcast_namespace(node.tag_name().namespace());
            if direct_txt {
                return None;
            }
            let kind = if node.tag_name().name() == "person"
                && podcast_namespace(node.tag_name().namespace())
            {
                RssExcludedElementKind::Contributor
            } else if node.tag_name().name() == "txt"
                && podcast_namespace(node.tag_name().namespace())
            {
                RssExcludedElementKind::NestedTxt
            } else if node.tag_name().name() == "txt" {
                RssExcludedElementKind::UnknownNamespaceTxt
            } else {
                return None;
            };
            Some(RssExcludedElement {
                kind,
                element_xml: document.input_text()[node.range()].to_owned(),
                namespace_uri: node.tag_name().namespace().map(str::to_owned),
                source_position: source_position(document, node),
                extraction_path: format!("{path}/{}", node.tag_name().name()),
            })
        })
        .collect()
}
fn podcast_namespace(namespace: Option<&str>) -> bool {
    namespace.is_some_and(|namespace| PODCAST_NAMESPACES.contains(&namespace))
}
fn ordinary_namespace(namespace: Option<&str>) -> bool {
    matches!(namespace, None | Some(RSS1_NAMESPACE))
}
fn source_position(document: &Document<'_>, node: Node<'_, '_>) -> RssSourcePosition {
    let position = document.text_pos_at(node.range().start);
    RssSourcePosition {
        decoded_byte_offset: node.range().start,
        line: position.row,
        column: position.col,
    }
}
fn item_path(root: Node<'_, '_>, item: Node<'_, '_>) -> String {
    let parent = item.parent().filter(|node| node.is_element());
    let ordinal = parent
        .map(|parent| {
            children(parent)
                .filter(|sibling| {
                    sibling.tag_name().name() == "item"
                        && ordinary_namespace(sibling.tag_name().namespace())
                })
                .take_while(|sibling| *sibling != item)
                .count()
                + 1
        })
        .unwrap_or(1);
    if item.parent() == Some(root) {
        if root.tag_name().namespace() == Some(RDF_ROOT_NAMESPACE) {
            format!("rdf:RDF/item[{ordinal}]")
        } else {
            format!("rss/item[{ordinal}]")
        }
    } else {
        format!("{}/item[{ordinal}]", channel_path(root))
    }
}
fn channel_path(root: Node<'_, '_>) -> String {
    if root.tag_name().namespace() == Some(RDF_ROOT_NAMESPACE) {
        "rdf:RDF/channel".into()
    } else {
        "rss/channel".into()
    }
}

fn apply_validated_ids(context: &mut TrackContext, observation: &RssObservation) -> bool {
    let item_eligible = matches!(&observation.item_match, RssItemMatch::Matched { observed_item_guid, .. } if observation.requested_track_guid.as_ref().zip(observed_item_guid.as_ref()).is_none_or(|(requested, observed)| requested == observed.trim()));
    let track_feed_eligible = context
        .track
        .feed_guid
        .as_ref()
        .zip(observation.observed_feed_guid.as_ref())
        .is_none_or(|(requested, observed)| requested == observed.trim());
    let context_feed_eligible = context
        .feed
        .as_ref()
        .and_then(|feed| feed.feed_guid.as_ref())
        .zip(observation.observed_feed_guid.as_ref())
        .is_none_or(|(requested, observed)| requested == observed.trim());
    let context_scope_eligible = context
        .track
        .feed_guid
        .as_ref()
        .zip(
            context
                .feed
                .as_ref()
                .and_then(|feed| feed.feed_guid.as_ref()),
        )
        .is_none_or(|(track_feed_guid, context_feed_guid)| track_feed_guid == context_feed_guid);
    let feed_eligible = track_feed_eligible && context_feed_eligible && context_scope_eligible;
    let mut changed = false;
    for evidence in &observation.txt_evidence {
        let (scheme, value) = match &evidence.validation {
            RssTxtValidation::ValidPublicKey(value) | RssTxtValidation::ValidProfile(value) => {
                (value.scheme(), value.original())
            }
            _ => continue,
        };
        match evidence.owner {
            RssIdentityOwner::Feed if feed_eligible => {
                if let Some(feed) = context.feed.as_mut() {
                    changed |= append_feed_source_id(feed, scheme, value, evidence);
                }
            }
            RssIdentityOwner::Track { item_index }
                if feed_eligible
                    && item_eligible
                    && matches!(&observation.item_match, RssItemMatch::Matched { item_index: matched_index, .. } if item_index == *matched_index) =>
            {
                changed |= append_track_source_id(&mut context.track, scheme, value, evidence)
            }
            _ => {}
        }
    }
    changed
}
fn apply_track_enrichment(
    track: &mut Track,
    feed: Option<&mut Feed>,
    enrichment: &RssTrackEnrichment,
) -> bool {
    let mut changed = false;
    changed |= set_text_if_missing(&mut track.title, enrichment.track_title.clone());
    changed |= set_text_if_missing(&mut track.description, enrichment.track_description.clone());
    changed |= set_text_if_missing(&mut track.track_artist, enrichment.track_artist.clone());
    changed |= set_text_if_missing(&mut track.image_url, enrichment.track_image_url.clone());
    changed |= set_text_if_missing(&mut track.feed_title, enrichment.feed_title.clone());
    changed |= set_text_if_missing(&mut track.release_artist, enrichment.feed_artist.clone());
    if track.track_number.is_none() {
        track.track_number = enrichment.track_number;
        changed |= track.track_number.is_some();
    }
    if track.duration_secs.is_none() {
        track.duration_secs = enrichment.duration_secs;
        changed |= track.duration_secs.is_some();
    }
    if track.pub_date.is_none() {
        track.pub_date = enrichment.pub_date;
        changed |= track.pub_date.is_some();
    }
    if let Some(feed) = feed {
        changed |= set_text_if_missing(&mut feed.title, enrichment.feed_title.clone());
        changed |= set_text_if_missing(&mut feed.name, enrichment.feed_title.clone());
        changed |= set_text_if_missing(&mut feed.description, enrichment.feed_description.clone());
        changed |= set_text_if_missing(&mut feed.release_artist, enrichment.feed_artist.clone());
        changed |= set_text_if_missing(&mut feed.image_url, enrichment.feed_image_url.clone());
        if feed.episode_count.is_none() {
            feed.episode_count = enrichment.feed_episode_count;
            changed |= feed.episode_count.is_some();
        }
    }
    changed
}
fn set_text_if_missing(target: &mut Option<String>, value: Option<String>) -> bool {
    if !source_text_missing(target.as_deref()) {
        return false;
    }
    let Some(value) = value.filter(|value| !source_text_missing(Some(value.as_str()))) else {
        return false;
    };
    *target = Some(value);
    true
}
fn parse_rss_pub_date(value: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc2822(value)
        .ok()
        .map(|date| date.timestamp())
}
fn append_track_source_link(
    track: &mut Track,
    link_type: &str,
    url: &str,
    extraction_path: &str,
) -> bool {
    let links = track.source_links.get_or_insert_with(Vec::new);
    if links.iter().any(|link| {
        link.link_type.as_deref() == Some(link_type) && link.url.as_deref() == Some(url)
    }) {
        return false;
    }
    links.push(SourceEntityLink {
        entity_type: Some("track".into()),
        entity_id: track.track_guid.clone(),
        position: Some(links.len() as i64),
        link_type: Some(link_type.into()),
        url: Some(url.into()),
        source: Some("rss".into()),
        extraction_path: Some(extraction_path.into()),
        observed_at: None,
    });
    true
}
fn append_track_source_id(
    track: &mut Track,
    scheme: &str,
    value: &str,
    evidence: &RssTxtEvidence,
) -> bool {
    let ids = track.source_ids.get_or_insert_with(Vec::new);
    if ids.iter().any(|id| {
        id.entity_type.as_deref() == Some("track")
            && id.entity_id == evidence.owner_guid
            && id.position == Some(evidence.occurrence_position as i64)
            && id.scheme.as_deref() == Some(scheme)
            && id.value.as_deref() == Some(value)
            && id.source.as_deref() == Some("rss")
            && id.extraction_path.as_deref() == Some(&evidence.extraction_path)
    }) {
        return false;
    }
    ids.push(SourceEntityId {
        entity_type: Some("track".into()),
        entity_id: evidence.owner_guid.clone(),
        position: Some(evidence.occurrence_position as i64),
        scheme: Some(scheme.into()),
        value: Some(value.into()),
        source: Some("rss".into()),
        extraction_path: Some(evidence.extraction_path.clone()),
        observed_at: None,
    });
    true
}
fn append_feed_source_id(
    feed: &mut Feed,
    scheme: &str,
    value: &str,
    evidence: &RssTxtEvidence,
) -> bool {
    let ids = feed.source_ids.get_or_insert_with(Vec::new);
    if ids.iter().any(|id| {
        id.entity_type.as_deref() == Some("feed")
            && id.entity_id == evidence.owner_guid
            && id.position == Some(evidence.occurrence_position as i64)
            && id.scheme.as_deref() == Some(scheme)
            && id.value.as_deref() == Some(value)
            && id.source.as_deref() == Some("rss")
            && id.extraction_path.as_deref() == Some(&evidence.extraction_path)
    }) {
        return false;
    }
    ids.push(SourceEntityId {
        entity_type: Some("feed".into()),
        entity_id: evidence.owner_guid.clone(),
        position: Some(evidence.occurrence_position as i64),
        scheme: Some(scheme.into()),
        value: Some(value.into()),
        source: Some("rss".into()),
        extraction_path: Some(evidence.extraction_path.clone()),
        observed_at: None,
    });
    true
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::{Duration, Instant};

    use super::*;
    const NPUB: &str = "npub10elfcs4fr0l0r8af98jlmgdh9c8tcxjvz9qkw038js35mp4dma8qzvjptg";
    const OTHER_NPUB: &str = "npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkwsyjh6w6";
    const NPROFILE: &str = "nprofile1qqsrhuxx8l9ex335q7he0f09aej04zpazpl0ne2cgukyawd24mayt8gpp4mhxue69uhhytnc9e3k7mgpz4mhxue69uhkg6nzv9ejuumpv34kytnrdaksjlyr9p";

    fn document(channel: &str) -> String {
        format!(
            "<rss xmlns:podcast=\"{}\" xmlns:itunes=\"{}\"><channel>{channel}</channel></rss>",
            PODCAST_NAMESPACES[0], ITUNES_NAMESPACES[0]
        )
    }

    fn parse(xml: &str, guid: Option<&str>, enclosure: Option<&str>) -> Result<RssFetchResult> {
        parse_track_enrichment_document(
            "https://example.test/request",
            "https://example.test/response",
            Utc::now(),
            Arc::from(xml.as_bytes()),
            guid,
            enclosure,
        )
    }

    fn context() -> TrackContext {
        TrackContext::new(
            Track {
                track_guid: Some("track".into()),
                feed_guid: Some("feed".into()),
                ..Default::default()
            },
            Some(Feed {
                feed_guid: Some("feed".into()),
                ..Default::default()
            }),
        )
    }

    // Each response closes its connection. The bounded worker records every requested path.
    fn server(responses: Vec<(String, Vec<u8>)>) -> (String, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind the RSS fixture");
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/feed", listener.local_addr().unwrap());
        let worker = thread::spawn(move || {
            let mut paths = Vec::new();
            for (headers, body) in responses {
                let deadline = Instant::now() + Duration::from_secs(10);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(Instant::now() < deadline, "RSS fixture request timed out");
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(error) => panic!("RSS fixture accept failed: {error}"),
                    }
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                let mut buffer = [0; 1024];
                while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                    let count = stream.read(&mut buffer).unwrap();
                    assert_ne!(count, 0, "RSS fixture request ended before its headers");
                    request.extend_from_slice(&buffer[..count]);
                }
                paths.push(
                    String::from_utf8(request)
                        .unwrap()
                        .lines()
                        .next()
                        .unwrap()
                        .to_owned(),
                );
                write!(
                    stream,
                    "HTTP/1.1 {headers}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(&body).unwrap();
            }
            paths
        });
        (url, worker)
    }

    fn assert_scalar_compatibility(
        xml: &[u8],
        guid: Option<&str>,
        enclosure: Option<&str>,
    ) -> Result<()> {
        use super::super::helpers::{find_ext_attr, find_ext_text, first_person_by_role};
        let old = Channel::read_from(Cursor::new(xml))?;
        let expected = old
            .items()
            .iter()
            .find(|item| {
                guid.zip(item.guid().map(rss::Guid::value))
                    .is_some_and(|(wanted, found)| wanted == found)
                    || enclosure
                        .zip(item.enclosure().map(rss::Enclosure::url))
                        .is_some_and(|(wanted, found)| wanted == found)
            })
            .map(|item| RssTrackEnrichment {
                feed_title: clean_text(Some(old.title())),
                feed_description: clean_text(Some(old.description())),
                feed_artist: old
                    .itunes_ext()
                    .and_then(|value| clean_text(value.author())),
                feed_image_url: old
                    .itunes_ext()
                    .and_then(|value| clean_text(value.image()))
                    .or_else(|| old.image().and_then(|value| clean_text(Some(value.url())))),
                feed_episode_count: old.items().len().try_into().ok(),
                track_title: clean_text(item.title()),
                track_description: clean_text(item.description()),
                track_artist: item
                    .itunes_ext()
                    .and_then(|value| clean_text(value.author()))
                    .or_else(|| clean_text(item.author()))
                    .or_else(|| {
                        first_person_by_role(
                            item.extensions(),
                            &["artist", "creator", "composer", "performer"],
                        )
                    }),
                track_image_url: item
                    .itunes_ext()
                    .and_then(|value| clean_text(value.image())),
                track_number: find_ext_text(item.extensions(), "podcast", "episode")
                    .and_then(|value| value.parse().ok()),
                duration_secs: item
                    .itunes_ext()
                    .and_then(|value| value.duration())
                    .and_then(parse_itunes_duration)
                    .and_then(|value| value.try_into().ok()),
                pub_date: item.pub_date().and_then(parse_rss_pub_date),
                transcript_url: find_ext_attr(item.extensions(), "podcast", "transcript", "url"),
                transcript_type: find_ext_attr(item.extensions(), "podcast", "transcript", "type"),
            });
        let actual = parse_track_enrichment_document(
            "request",
            "response",
            Utc::now(),
            Arc::from(xml),
            guid,
            enclosure,
        )?;
        assert_eq!(
            actual.enrichment, expected,
            "scalar compatibility changed for {guid:?} / {enclosure:?}"
        );
        Ok(())
    }

    #[test]
    fn adr_0075_rss_preserves_all_existing_scalar_fields_and_artist_priority() -> Result<()> {
        let xml = document(
            r#"
            <title>Feed &amp; title</title><description><![CDATA[<p>Feed description</p>]]></description>
            <itunes:author>Feed artist</itunes:author><itunes:image href="itunes-feed.png"/>
            <image><url>ordinary-feed.png</url></image>
            <item><guid>track</guid><title>Track title</title><description>Track description</description>
                <itunes:author>Track artist</itunes:author><author>Ordinary artist</author>
                <podcast:person role="artist">Contributor artist</podcast:person>
                <itunes:image href="track.png"/><podcast:episode>7</podcast:episode>
                <itunes:duration>1:02:03</itunes:duration><pubDate>Sun, 20 Sep 2026 12:00:00 +0000</pubDate>
                <podcast:transcript url="first.vtt" type="text/vtt"/><podcast:transcript url="second.srt"/>
            </item>
            <item><guid>author</guid><author>Ordinary artist</author><podcast:person role="artist">Contributor</podcast:person></item>
            <item><guid>person</guid><podcast:person role="host">Host</podcast:person>
                <podcast:person role="composer">First composer</podcast:person><podcast:person role="artist">Second artist</podcast:person></item>
        "#,
        );
        for guid in ["track", "author", "person"] {
            assert_scalar_compatibility(xml.as_bytes(), Some(guid), None)?;
        }
        let fields = parse(&xml, Some("track"), None)?.enrichment.unwrap();
        assert_eq!(fields.duration_secs, Some(3723));
        assert_eq!(fields.track_number, Some(7));
        assert_eq!(fields.feed_episode_count, Some(3));
        assert!(fields.pub_date.is_some());
        Ok(())
    }

    #[test]
    fn adr_0075_rss_preserves_repeated_ordinary_and_itunes_selection() -> Result<()> {
        let xml = document(
            r#"
            <title>first feed</title><title>last feed</title><title/>
            <description>first feed description</description><description>last feed description</description><description/>
            <itunes:author>first feed artist</itunes:author><itunes:author>last feed artist</itunes:author>
            <itunes:image href="first.png"/><itunes:image href="last.png"/>
            <item><guid>first</guid><guid>last</guid><title>first track</title><title>last track</title>
                <description>first description</description><description>last description</description>
                <enclosure url="first.mp3"/><enclosure url="last.mp3"/>
                <itunes:author>first artist</itunes:author><itunes:author>last artist</itunes:author>
                <itunes:image href="first-track.png"/><itunes:image href="last-track.png"/>
                <itunes:duration>01:01</itunes:duration><itunes:duration>02:02</itunes:duration>
            </item>"#,
        );
        for (guid, enclosure) in [
            (Some("first"), None),
            (Some("last"), None),
            (None, Some("first.mp3")),
            (None, Some("last.mp3")),
        ] {
            assert_scalar_compatibility(xml.as_bytes(), guid, enclosure)?;
        }
        for last in ["", "...", " "] {
            let xml = document(&format!("<title>first</title><title>{last}</title><description>first</description><description>{last}</description><item><guid>track</guid><title>first</title><title>{last}</title><description>first</description><description>{last}</description><author>first</author><author>{last}</author></item>"));
            assert_scalar_compatibility(xml.as_bytes(), Some("track"), None)?;
        }
        let xml = document(
            r#"<itunes:author/><itunes:author>later</itunes:author><itunes:image/><itunes:image href="later.png"/>
            <item><guid>track</guid><itunes:author/><itunes:author>later</itunes:author><itunes:image/><itunes:image href="later.png"/>
            <itunes:duration/><itunes:duration>01:00</itunes:duration></item>"#,
        );
        assert_scalar_compatibility(xml.as_bytes(), Some("track"), None)?;
        Ok(())
    }

    #[test]
    fn adr_0075_rss_preserves_direct_text_and_guid_whitespace() -> Result<()> {
        for value in [
            " A <!--comment--> B ",
            " A <![CDATA[ B & C ]]> D ",
            " A <b>nested</b> B ",
            "<b>nested only</b>",
            "A &amp; B",
        ] {
            let xml = document(&format!("<title>{value}</title><description>{value}</description><itunes:author>{value}</itunes:author><item><guid> \n\u{2003}track\t </guid><title>{value}</title><description>{value}</description><itunes:author>{value}</itunes:author><podcast:person role=\"artist\">{value}</podcast:person></item>"));
            assert_scalar_compatibility(xml.as_bytes(), Some("track"), None)?;
        }
        let xml = document("<item><guid>previous</guid><guid/><enclosure/></item>");
        assert_scalar_compatibility(xml.as_bytes(), Some(""), None)?;
        assert_scalar_compatibility(xml.as_bytes(), None, Some(""))?;
        Ok(())
    }

    #[test]
    fn adr_0075_rss_preserves_rdf_artwork_and_root_item_order() -> Result<()> {
        for root in [
            format!("<rdf:RDF xmlns:rdf=\"{RDF_ROOT_NAMESPACE}\" xmlns=\"{RSS1_NAMESPACE}\">"),
            "<rss>".into(),
        ] {
            let end = if root.contains("rdf:RDF") {
                "</rdf:RDF>"
            } else {
                "</rss>"
            };
            let xml = format!("{root}<item><guid>track</guid><title>root first</title></item><channel><title>feed</title><image><url>channel.png</url></image><item><guid>track</guid><title>channel first</title></item></channel><image><url>root-first.png</url></image><image><url>root-last.png</url></image><item><guid>root-last</guid><title>root last</title></item>{end}");
            assert_scalar_compatibility(xml.as_bytes(), Some("track"), None)?;
            assert_scalar_compatibility(xml.as_bytes(), Some("root-last"), None)?;
            let result = parse(&xml, Some("track"), None)?;
            let fields = result.enrichment.unwrap();
            assert_eq!(fields.track_title.as_deref(), Some("channel first"));
            assert_eq!(fields.feed_image_url.as_deref(), Some("root-last.png"));
            assert_eq!(fields.feed_episode_count, Some(3));
            assert!(matches!(
                result.observation.item_match,
                RssItemMatch::Matched { item_index: 0, .. }
            ));
        }
        Ok(())
    }

    #[test]
    fn adr_0075_rss_preserves_legacy_encoding_and_original_bytes() -> Result<()> {
        let mut bytes =
            b"<?xml version=\"1.0\" encoding=\"windows-1252\"?><rss><channel><title>Caf".to_vec();
        bytes.push(0xe9);
        bytes.extend_from_slice(b"</title><item><guid>track</guid><description>Cr");
        bytes.push(0xe8);
        bytes.extend_from_slice(b"me</description></item></channel></rss>");
        assert_scalar_compatibility(&bytes, Some("track"), None)?;
        let now = Utc::now();
        let result = parse_track_enrichment_document(
            "requested",
            "response",
            now,
            Arc::from(bytes.clone()),
            Some("track"),
            None,
        )?;
        assert_eq!(result.observation.response_bytes.as_ref(), bytes);
        assert_eq!(result.observation.character_encoding, "windows-1252");
        assert_eq!(result.observation.fetched_at, now);
        assert_eq!(
            result.enrichment.unwrap().feed_title.as_deref(),
            Some("Café")
        );
        Ok(())
    }

    #[test]
    fn adr_0075_rss_keeps_duplicate_occurrences_and_independent_index_facts() -> Result<()> {
        let xml = document(&format!("<podcast:guid>feed</podcast:guid><podcast:txt purpose=\"npub\">{OTHER_NPUB}</podcast:txt><podcast:txt purpose=\"npub\">{OTHER_NPUB}</podcast:txt><item><guid>track</guid><podcast:txt purpose=\"npub\">{NPUB}</podcast:txt><podcast:txt purpose=\"npub\">{NPUB}</podcast:txt></item>"));
        let result = parse(&xml, Some("track"), None)?;
        let mut context = context();
        context.track.source_ids = Some(vec![SourceEntityId {
            entity_type: Some("track".into()),
            entity_id: Some("track".into()),
            source: Some("musicindex".into()),
            scheme: Some("nostr_npub".into()),
            value: Some(NPUB.into()),
            ..Default::default()
        }]);
        assert!(apply_validated_ids(&mut context, &result.observation));
        assert!(!apply_validated_ids(&mut context, &result.observation));
        let ids = context.track.source_ids.unwrap();
        assert_eq!(ids.len(), 3);
        assert_eq!(ids[0].source.as_deref(), Some("musicindex"));
        for (position, id) in ids[1..].iter().enumerate() {
            assert_eq!(id.position, Some(position as i64));
            assert_eq!(id.entity_id.as_deref(), Some("track"));
            assert_eq!(id.value.as_deref(), Some(NPUB));
            assert_eq!(
                id.extraction_path.as_deref(),
                Some(format!("rss/channel/item[1]/podcast:txt[{}]/text()", position + 1).as_str())
            );
            assert!(id.observed_at.is_none());
        }
        let feed_ids = context.feed.unwrap().source_ids.unwrap();
        assert_eq!(feed_ids.len(), 2);
        assert!(feed_ids
            .iter()
            .all(|id| id.entity_id.as_deref() == Some("feed")
                && id.value.as_deref() == Some(OTHER_NPUB)));
        Ok(())
    }

    #[test]
    fn adr_0075_rss_contributors_nested_claims_and_attributes_stay_evidence_only() -> Result<()> {
        let xml = document(&format!("<podcast:person role=\"artist\" href=\"{NPUB}\">{NPUB}<podcast:txt purpose=\"npub\">{NPUB}</podcast:txt></podcast:person><item><guid>track</guid><podcast:person role=\"artist\" href=\"{NPUB}\">{NPUB}</podcast:person><podcast:transcript url=\"{NPUB}\"/><podcast:value><podcast:txt purpose=\"npub\">{NPUB}</podcast:txt></podcast:value></item>"));
        let result = parse(&xml, Some("track"), None)?;
        let mut context = context();
        assert!(!apply_validated_ids(&mut context, &result.observation));
        assert!(result.observation.txt_evidence.is_empty());
        assert_eq!(
            result
                .observation
                .excluded_elements
                .iter()
                .filter(|entry| entry.kind == RssExcludedElementKind::Contributor)
                .count(),
            2
        );
        assert_eq!(
            result
                .observation
                .excluded_elements
                .iter()
                .filter(|entry| entry.kind == RssExcludedElementKind::NestedTxt)
                .count(),
            2
        );
        assert_eq!(result.observation.response_bytes.as_ref(), xml.as_bytes());
        assert!(context.track.source_ids.is_none());
        assert!(context.feed.unwrap().source_ids.is_none());
        Ok(())
    }

    #[test]
    fn adr_0075_rss_namespace_aliases_defaults_and_rebinding_keep_owner_scope() -> Result<()> {
        let xml = format!("<rss xmlns:pc=\"{}\" xmlns:alias=\"{}\"><channel><item><guid>track</guid><pc:txt purpose=\"npub\">{NPUB}</pc:txt><alias:txt purpose=\"npub\">{NPUB}</alias:txt><txt xmlns=\"{}\" purpose=\"npub\">{NPUB}</txt><pc:txt xmlns:pc=\"urn:other\" purpose=\"npub\">{NPUB}</pc:txt><txt purpose=\"npub\">{NPUB}</txt></item></channel></rss>", PODCAST_NAMESPACES[0], PODCAST_NAMESPACES[1], PODCAST_NAMESPACES[0]);
        let result = parse(&xml, Some("track"), None)?;
        assert_eq!(result.observation.txt_evidence.len(), 3);
        assert!(result.observation.txt_evidence.iter().all(|entry| matches!(
            entry.validation,
            RssTxtValidation::ValidPublicKey(_)
        ) && entry.owner
            == RssIdentityOwner::Track { item_index: 0 }));
        assert_eq!(result.observation.excluded_elements.len(), 2);
        assert!(result
            .observation
            .excluded_elements
            .iter()
            .all(|entry| entry.kind == RssExcludedElementKind::UnknownNamespaceTxt));
        Ok(())
    }

    #[test]
    fn adr_0075_rss_does_not_add_an_itunes_namespace_alias() -> Result<()> {
        let xml = document("<itunes:author>feed artist</itunes:author><item><guid>track</guid><itunes:author>track artist</itunes:author><itunes:duration>01:00</itunes:duration></item>")
            .replace(ITUNES_NAMESPACES[0], "https://www.itunes.com/dtds/podcast-1.0.dtd");
        assert_scalar_compatibility(xml.as_bytes(), Some("track"), None)
    }

    #[test]
    fn adr_0075_rss_purpose_and_exact_candidate_rules_preserve_rejected_text() -> Result<()> {
        let mut claims = String::new();
        for purpose in [None, Some(""), Some(" "), Some("nostr"), Some("unrelated")] {
            let attribute =
                purpose.map_or(String::new(), |purpose| format!(" purpose=\"{purpose}\""));
            claims.push_str(&format!("<podcast:txt{attribute}>{NPUB}</podcast:txt>"));
        }
        for value in [
            format!("nostr:{NPUB}"),
            format!("https://example.test/{NPUB}"),
            format!("{NPUB} {OTHER_NPUB}"),
            format!("{NPUB}<b>nested</b>"),
            "".into(),
        ] {
            claims.push_str(&format!(
                "<podcast:txt purpose=\"npub\">{value}</podcast:txt>"
            ));
        }
        claims.push_str(&format!(
            "<podcast:txt purpose=\" NPUB \"> \n{NPUB}\t </podcast:txt>"
        ));
        let xml = document(&format!("<item><guid>track</guid>{claims}</item>"));
        let result = parse(&xml, Some("track"), None)?;
        let entries = &result.observation.txt_evidence;
        assert_eq!(entries.len(), 11);
        assert!(entries[..5]
            .iter()
            .all(|entry| entry.validation == RssTxtValidation::UnsupportedPurpose));
        assert!(entries[5..10]
            .iter()
            .all(|entry| entry.validation == RssTxtValidation::MalformedEncoding));
        assert_eq!(entries[0].purpose, None);
        assert_eq!(entries[1].purpose.as_deref(), Some(""));
        assert_eq!(entries[8].direct_text.as_deref(), Some(NPUB));
        assert_eq!(entries[9].direct_text.as_deref(), Some(""));
        assert_eq!(entries[10].purpose.as_deref(), Some(" NPUB "));
        assert_eq!(
            entries[10].direct_text.as_deref(),
            Some(format!(" \n{NPUB}\t ").as_str())
        );
        let mut context = context();
        assert!(apply_validated_ids(&mut context, &result.observation));
        let ids = context.track.source_ids.unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0].position, Some(10));
        assert_eq!(ids[0].value.as_deref(), Some(NPUB));
        Ok(())
    }

    #[test]
    fn adr_0075_rss_checks_item_and_each_feed_disagreement_independently() -> Result<()> {
        for (item_guid, observed_feed, track_feed, context_feed, expect_feed_id) in [
            ("other", "feed", "feed", "feed", true),
            ("track", "feed", "other", "feed", false),
            ("track", "feed", "feed", "other", false),
        ] {
            let xml = document(&format!("<podcast:guid>{observed_feed}</podcast:guid><podcast:txt purpose=\"npub\">{NPUB}</podcast:txt><item><guid>{item_guid}</guid><enclosure url=\"audio.mp3\"/><podcast:txt purpose=\"npub\">{NPUB}</podcast:txt></item>"));
            let result = parse(&xml, Some("track"), Some("audio.mp3"))?;
            assert!(
                result.enrichment.is_some(),
                "scalar matching must stay unchanged"
            );
            let mut context = context();
            context.track.feed_guid = Some(track_feed.into());
            context.feed.as_mut().unwrap().feed_guid = Some(context_feed.into());
            apply_validated_ids(&mut context, &result.observation);
            assert!(context.track.source_ids.is_none());
            assert_eq!(context.feed.unwrap().source_ids.is_some(), expect_feed_id);
        }
        let xml = document(&format!("<podcast:txt purpose=\"npub\">{NPUB}</podcast:txt><item><enclosure url=\"audio.mp3\"/><podcast:txt purpose=\"npub\">{NPUB}</podcast:txt></item>"));
        let result = parse(&xml, Some("requested-only"), Some("audio.mp3"))?;
        let mut context = context();
        assert!(apply_validated_ids(&mut context, &result.observation));
        assert!(context.track.source_ids.unwrap()[0].entity_id.is_none());
        assert!(context.feed.unwrap().source_ids.unwrap()[0]
            .entity_id
            .is_none());
        Ok(())
    }

    #[test]
    fn adr_0075_rss_records_exact_source_positions_and_owner_paths() -> Result<()> {
        let xml = format!("<rss xmlns:podcast=\"{}\" xmlns:other=\"urn:other\"><channel><title>Café</title>\n<other:item/><item><guid>track</guid>\n<podcast:txt purpose=\"npub\">{NPUB}</podcast:txt>\n</item></channel></rss>", PODCAST_NAMESPACES[0]);
        let result = parse(&xml, Some("track"), None)?;
        let evidence = &result.observation.txt_evidence[0];
        let start = evidence.source_position.decoded_byte_offset;
        assert_eq!(
            &xml[start..start + evidence.element_xml.len()],
            evidence.element_xml
        );
        assert_eq!(evidence.source_position.line, 3);
        assert_eq!(evidence.source_position.column, 1);
        assert_eq!(
            evidence.extraction_path,
            "rss/channel/item[1]/podcast:txt[1]/text()"
        );
        assert_eq!(evidence.owner_guid.as_deref(), Some("track"));
        assert_eq!(evidence.occurrence_position, 0);
        Ok(())
    }

    #[test]
    fn adr_0075_rss_live_rejected_evidence_survives_clone_and_sanitization() -> Result<()> {
        let unsupported = "nsec180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkwsgyumg0";
        let xml = document(&format!("<item><guid>track</guid><podcast:txt purpose=\"nostr\">{NPUB}</podcast:txt><podcast:txt purpose=\"npub\">npub1notavalidkey</podcast:txt><podcast:txt purpose=\"npub\">{unsupported}</podcast:txt></item>"));
        let (url, worker) = server(vec![("200 OK".into(), xml.as_bytes().to_vec())]);
        let mut context = context();
        context.track.feed_url = Some(url.clone());
        let before = Utc::now();
        crate::subscribe_service::enrich_track_context_from_rss(&mut context);
        let after = Utc::now();
        assert_eq!(worker.join().unwrap(), ["GET /feed HTTP/1.1"]);
        let observation = Arc::clone(
            context
                .rss_observation
                .as_ref()
                .expect("retain the fetched observation"),
        );
        assert_eq!(observation.requested_url, url);
        assert_eq!(observation.response_url, url);
        assert!(observation.fetched_at >= before && observation.fetched_at <= after);
        assert_eq!(observation.response_bytes.as_ref(), xml.as_bytes());
        assert_eq!(observation.requested_track_guid.as_deref(), Some("track"));
        assert_eq!(
            observation.txt_evidence[0].validation,
            RssTxtValidation::UnsupportedPurpose
        );
        assert_eq!(
            observation.txt_evidence[1].validation,
            RssTxtValidation::MalformedEncoding
        );
        assert_eq!(
            observation.txt_evidence[2].validation,
            RssTxtValidation::UnsupportedEncoding
        );
        let mut cloned = context.clone();
        cloned.track.title = Some("...".into());
        crate::metadata::sanitize_track_context_source_text(&mut cloned);
        let retained = cloned.rss_observation.as_ref().unwrap();
        assert!(Arc::ptr_eq(&observation, retained));
        assert!(Arc::ptr_eq(
            &observation.response_bytes,
            &retained.response_bytes
        ));
        assert_eq!(observation.as_ref(), retained.as_ref());
        assert!(cloned.track.source_ids.is_none());
        assert!(crate::metadata::track_nostr(&cloned.track).is_none());
        let view = crate::views::EntityIdentityLinks::from_source_facts(
            None,
            vec![],
            cloned
                .track
                .source_ids
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
        );
        assert!(view.nostr_npub.is_none());
        let debug = format!("{retained:?} {:?}", retained.txt_evidence);
        assert!(!debug.contains(NPUB));
        assert!(!debug.contains("npub1notavalidkey"));
        assert!(!debug.contains(unsupported));
        Ok(())
    }

    #[test]
    fn adr_0075_rss_live_profile_stays_typed_and_outside_npub_selectors() -> Result<()> {
        let xml = document(&format!(
            "<item><guid>track</guid><podcast:txt purpose=\"npub\">{NPROFILE}</podcast:txt></item>"
        ));
        let (url, worker) = server(vec![("200 OK".into(), xml.into_bytes())]);
        let mut context = context();
        assert!(enrich_track_from_feed_rss(&mut context, &url)?);
        assert_eq!(worker.join().unwrap().len(), 1);
        let ids = context.track.source_ids.as_ref().unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0].scheme.as_deref(), Some("nostr_nprofile"));
        assert_eq!(ids[0].value.as_deref(), Some(NPROFILE));
        assert!(crate::metadata::track_nostr(&context.track).is_none());
        let view = crate::views::EntityIdentityLinks::from_source_facts(
            None,
            vec![],
            ids.clone().into_iter().map(Into::into).collect(),
        );
        assert!(view.nostr_npub.is_none());
        let evidence = &context.rss_observation.as_ref().unwrap().txt_evidence[0];
        assert!(
            matches!(&evidence.validation, RssTxtValidation::ValidProfile(NostrIdentity::Profile { relay_hints, tlvs, .. }) if relay_hints.len() == 2 && tlvs.len() == 3)
        );
        Ok(())
    }

    #[test]
    fn adr_0075_rss_live_no_match_keeps_observation_and_existing_facts() -> Result<()> {
        let xml = document(&format!("<title>new feed</title><podcast:txt purpose=\"npub\">{NPUB}</podcast:txt><item><guid>other</guid><title>new track</title></item>"));
        let (url, worker) = server(vec![("200 OK".into(), xml.as_bytes().to_vec())]);
        let mut context = context();
        context.track.title = Some("existing track".into());
        context.feed.as_mut().unwrap().title = Some("existing feed".into());
        let original = serde_json::to_value((&context.track, &context.feed))?;
        assert!(!enrich_track_from_feed_rss(&mut context, &url)?);
        assert_eq!(worker.join().unwrap().len(), 1);
        assert_eq!(
            serde_json::to_value((&context.track, &context.feed))?,
            original
        );
        let observation = context.rss_observation.as_ref().unwrap();
        assert_eq!(observation.item_match, RssItemMatch::NoMatch);
        assert_eq!(observation.response_bytes.as_ref(), xml.as_bytes());
        assert_eq!(observation.txt_evidence.len(), 1);
        Ok(())
    }

    #[test]
    fn adr_0075_rss_request_and_parse_failures_keep_prior_observation_and_facts() -> Result<()> {
        let xml = document(&format!("<item><guid>track</guid><title>retained track</title><podcast:txt purpose=\"npub\">{NPUB}</podcast:txt></item>"));
        let responses = vec![
            ("200 OK".into(), xml.into_bytes()),
            ("200 OK".into(), b"<rss><channel><item>".to_vec()),
            (
                "503 Service Unavailable".into(),
                b"failed response".to_vec(),
            ),
        ];
        let (url, worker) = server(responses);
        let mut context = context();
        assert!(enrich_track_from_feed_rss(&mut context, &url)?);
        let observation = Arc::clone(context.rss_observation.as_ref().unwrap());
        let original = serde_json::to_value((&context.track, &context.feed))?;
        for feed_url in [url.as_str(), url.as_str(), "not a valid URL"] {
            // Packet 018 P18-3 would otherwise reuse the first, successful
            // response for the rest of this loop. Each iteration here
            // means a genuinely new attempt that must reach the fixture's
            // next queued (malformed, then failing) response.
            invalidate_feed_document(feed_url);
            assert!(enrich_track_from_feed_rss(&mut context, feed_url).is_err());
            assert!(Arc::ptr_eq(
                &observation,
                context.rss_observation.as_ref().unwrap()
            ));
            assert_eq!(
                serde_json::to_value((&context.track, &context.feed))?,
                original
            );
        }
        assert_eq!(worker.join().unwrap().len(), 3);
        Ok(())
    }

    /// R18B-03: the RSS document window matches the accepted 15-minute
    /// policy. This is the one place this value appears in the code,
    /// beside the track and feed windows named in
    /// `application::request_reuse`.
    #[test]
    fn adr_0075_rss_document_window_matches_the_accepted_policy() {
        assert_eq!(RSS_DOCUMENT_REUSE_WINDOW, Duration::from_secs(15 * 60));
    }

    /// P18-5: the app holds at most this many parsed RSS documents. The
    /// eviction rule itself (least recently used first) is proven once, at
    /// the shared `RetainedCache` this module reuses
    /// (`adr_0075_request_reuse_retained_cache_capacity_evicts_least_recently_used`
    /// in `application::request_reuse`), rather than repeated here with 33
    /// local servers.
    #[test]
    fn adr_0075_rss_document_capacity_matches_the_accepted_policy() {
        assert_eq!(RSS_DOCUMENT_CAPACITY, 32);
    }

    /// R18B-01/P18-3: a retained RSS document is reused for a different
    /// track of the same feed, inside its window, without a second
    /// request. Re-parsing the retained bytes for the second track's own
    /// GUID is what makes the reuse correct, not just cheap: each track
    /// gets its own item match from the one fetched document.
    #[test]
    fn adr_0075_rss_document_reused_within_window_for_a_different_track() -> Result<()> {
        let xml = document(
            "<item><guid>track-a</guid><title>Track A</title></item>\
<item><guid>track-b</guid><title>Track B</title></item>",
        );
        let (url, worker) = server(vec![("200 OK".into(), xml.into_bytes())]);
        let first = fetch_track_enrichment_from_feed(&url, Some("track-a"), None)?;
        assert_eq!(
            first
                .enrichment
                .as_ref()
                .and_then(|e| e.track_title.clone()),
            Some("Track A".to_owned())
        );
        let second = fetch_track_enrichment_from_feed(&url, Some("track-b"), None)?;
        assert_eq!(
            second
                .enrichment
                .as_ref()
                .and_then(|e| e.track_title.clone()),
            Some("Track B".to_owned())
        );
        assert_eq!(
            worker.join().unwrap().len(),
            1,
            "R18B-01: the second ask reuses the retained document and sends no request"
        );
        Ok(())
    }

    /// Packet 018 Job 3 (Part B follow-up): a reused document still
    /// records this call's own observation from its retained bytes, with
    /// no network request, so a repeated read of a different track from
    /// the same feed still reports its own receipt.
    #[test]
    fn adr_0075_request_reuse_rss_document_reuse_replays_the_original_receipt() -> Result<()> {
        use rusqlite::Connection;

        let xml = document(
            "<item><guid>track-a</guid><title>Track A</title></item>\
<item><guid>track-b</guid><title>Track B</title></item>",
        );
        let (url, worker) = server(vec![("200 OK".into(), xml.into_bytes())]);
        let conn = Connection::open_in_memory().unwrap();
        crate::db::upgrades::create_fixture(&conn, 12).unwrap();
        let recorder = crate::provider_observation::ProviderObservationRecorder::new(Arc::new(
            Mutex::new(conn),
        ));

        // Every real caller of the observed path drains `recorder` itself
        // once its own operation finishes (`feed_service.rs`,
        // `library.rs`); this test does the same, one call at a time, so
        // each drain holds exactly that one call's own receipt.
        let mut first_context = context();
        first_context.track.track_guid = Some("track-a".into());
        assert!(enrich_track_from_feed_rss_observed(
            &mut first_context,
            &url,
            &recorder
        )?);
        let first_receipts = recorder.take_receipts();
        assert_eq!(
            first_receipts.len(),
            1,
            "a fresh, observed fetch records one receipt"
        );

        let mut second_context = context();
        second_context.track.track_guid = Some("track-b".into());
        assert!(enrich_track_from_feed_rss_observed(
            &mut second_context,
            &url,
            &recorder
        )?);
        let second_receipts = recorder.take_receipts();
        assert_eq!(
            second_receipts.len(),
            1,
            "the reused document still reports its evidence for a different track"
        );
        // R18B-07: a reused response names the observation that produced
        // it, and it creates no second observation of one fetch. The feed
        // and track caches replay their receipts the same way. The
        // operator decided this on 2026-09-22, against a new observation
        // for each reuse.
        assert_eq!(
            second_receipts[0].observation_id, first_receipts[0].observation_id,
            "the reuse replays the original fetch's observation"
        );
        assert_eq!(
            worker.join().unwrap().len(),
            1,
            "the reused document sends no request"
        );
        Ok(())
    }

    /// Packet 018 Job 2 (Part B follow-up): two concurrent callers of one
    /// feed URL share the GET, closing the gap the packet's own concurrent
    /// measurement found. The fixture accepts connections until this test
    /// releases them, so a caller that did not join the shared fetch would
    /// show up as a second accepted connection, not just a slower one.
    #[test]
    fn adr_0075_request_reuse_rss_concurrent_callers_share_one_fetch() -> Result<()> {
        use rusqlite::Connection;
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind the RSS fixture");
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/feed", listener.local_addr().unwrap());
        let xml = document("<item><guid>track</guid><title>Shared</title></item>");
        let release = Arc::new(AtomicBool::new(false));
        let accepted = Arc::new(AtomicUsize::new(0));
        let release_worker = Arc::clone(&release);
        let accepted_worker = Arc::clone(&accepted);
        let worker = thread::spawn(move || {
            let mut streams = Vec::new();
            loop {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(5)))
                            .unwrap();
                        let mut request = Vec::new();
                        let mut buffer = [0; 1024];
                        while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                            let count = stream.read(&mut buffer).unwrap();
                            assert_ne!(count, 0, "RSS fixture request ended before its headers");
                            request.extend_from_slice(&buffer[..count]);
                        }
                        accepted_worker.fetch_add(1, Ordering::SeqCst);
                        streams.push(stream);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if release_worker.load(Ordering::SeqCst) {
                            break;
                        }
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("RSS fixture accept failed: {error}"),
                }
            }
            for mut stream in streams {
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    xml.len()
                )
                .unwrap();
                stream.write_all(xml.as_bytes()).unwrap();
            }
        });

        let recorder_a = {
            let conn = Connection::open_in_memory().unwrap();
            crate::db::upgrades::create_fixture(&conn, 12).unwrap();
            crate::provider_observation::ProviderObservationRecorder::new(Arc::new(Mutex::new(
                conn,
            )))
        };
        let recorder_b = {
            let conn = Connection::open_in_memory().unwrap();
            crate::db::upgrades::create_fixture(&conn, 12).unwrap();
            crate::provider_observation::ProviderObservationRecorder::new(Arc::new(Mutex::new(
                conn,
            )))
        };

        let url_a = url.clone();
        let first = thread::spawn(move || {
            let mut context = context();
            let changed = enrich_track_from_feed_rss_observed(&mut context, &url_a, &recorder_a);
            (changed, context)
        });

        let start = Instant::now();
        while accepted.load(Ordering::SeqCst) == 0 {
            assert!(
                start.elapsed() < Duration::from_secs(3),
                "the first request did not reach the fixture"
            );
            thread::sleep(Duration::from_millis(2));
        }

        let url_b = url.clone();
        let second = thread::spawn(move || {
            let mut context = context();
            let changed = enrich_track_from_feed_rss_observed(&mut context, &url_b, &recorder_b);
            (changed, context)
        });

        // Give the second caller time to reach the shared fetch and join
        // it before this test releases the held response.
        thread::sleep(Duration::from_millis(80));
        release.store(true, Ordering::SeqCst);

        let (first_changed, first_context) = first.join().unwrap();
        let (second_changed, second_context) = second.join().unwrap();
        assert!(first_changed?, "the winning caller applies the enrichment");
        assert!(second_changed?, "the joining caller applies it too");
        assert_eq!(
            first_context.track.title.as_deref(),
            second_context.track.title.as_deref()
        );
        worker.join().unwrap();
        assert_eq!(
            accepted.load(Ordering::SeqCst),
            1,
            "R18A-03's rule, applied to RSS: one identity, one request"
        );
        Ok(())
    }

    /// R18B-04: a failed RSS request is never retained. The next ask sends
    /// its own request.
    #[test]
    fn adr_0075_rss_document_failure_is_never_retained() -> Result<()> {
        let (url, worker) = server(vec![
            ("503 Service Unavailable".into(), b"failed".to_vec()),
            (
                "200 OK".into(),
                document("<item><guid>track</guid><title>Recovered</title></item>").into_bytes(),
            ),
        ]);
        let first = fetch_track_enrichment_from_feed(&url, Some("track"), None);
        assert!(first.is_err(), "the fixture's first response is a failure");
        let second = fetch_track_enrichment_from_feed(&url, Some("track"), None)?;
        assert_eq!(
            second
                .enrichment
                .as_ref()
                .and_then(|e| e.track_title.clone()),
            Some("Recovered".to_owned())
        );
        assert_eq!(
            worker.join().unwrap().len(),
            2,
            "R18B-04: the failed first request was never retained, so the second ask sends its own request"
        );
        Ok(())
    }

    /// R18B-05/P18-7: an explicit refresh removes the retained RSS
    /// document of the named feed, so the next ask sends a new request.
    #[test]
    fn adr_0075_rss_document_explicit_refresh_clears_the_retained_document() -> Result<()> {
        let (url, worker) = server(vec![
            (
                "200 OK".into(),
                document("<item><guid>track</guid><title>First</title></item>").into_bytes(),
            ),
            (
                "200 OK".into(),
                document("<item><guid>track</guid><title>Second</title></item>").into_bytes(),
            ),
        ]);
        let first = fetch_track_enrichment_from_feed(&url, Some("track"), None)?;
        assert_eq!(
            first
                .enrichment
                .as_ref()
                .and_then(|e| e.track_title.clone()),
            Some("First".to_owned())
        );
        invalidate_feed_document(&url);
        let second = fetch_track_enrichment_from_feed(&url, Some("track"), None)?;
        assert_eq!(
            second
                .enrichment
                .as_ref()
                .and_then(|e| e.track_title.clone()),
            Some("Second".to_owned())
        );
        assert_eq!(
            worker.join().unwrap().len(),
            2,
            "P18-7: the explicit refresh clears the retained document, so the second ask sends its own request"
        );
        Ok(())
    }

    #[test]
    fn adr_0075_rss_records_final_redirect_url_and_only_final_response_bytes() -> Result<()> {
        let xml = document("<item><guid>track</guid></item>");
        let (url, worker) = server(vec![
            (
                "302 Found\r\nLocation: /final".into(),
                b"redirect body".to_vec(),
            ),
            ("200 OK".into(), xml.as_bytes().to_vec()),
        ]);
        let result = fetch_track_enrichment_from_feed(&url, Some("track"), None)?;
        assert_eq!(
            worker.join().unwrap(),
            ["GET /feed HTTP/1.1", "GET /final HTTP/1.1"]
        );
        assert_eq!(result.observation.requested_url, url);
        assert_eq!(
            result.observation.response_url,
            url.replace("/feed", "/final")
        );
        assert_eq!(result.observation.response_bytes.as_ref(), xml.as_bytes());
        Ok(())
    }

    #[test]
    fn adr_0075_rss_rejects_dtd_invalid_encoding_and_xml_without_raw_error_text() {
        for bytes in [
            b"<!DOCTYPE rss [<!ENTITY x SYSTEM 'file:///must-not-read'>]><rss><channel/></rss>"
                .as_slice(),
            b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><rss><channel>\xff</channel></rss>",
            b"<rss><channel><candidate-canary:txt/></channel></rss>",
        ] {
            let error = parse_track_enrichment_document(
                "request",
                "response",
                Utc::now(),
                Arc::from(bytes),
                Some("track"),
                None,
            )
            .unwrap_err();
            let error = format!("{error:#}");
            assert!(!error.contains("must-not-read"));
            assert!(!error.contains("candidate-canary"));
            assert!(!error.contains("<rss>"));
        }
    }
    #[test]
    fn rss_enrichment_replaces_placeholder_core_fields() {
        let mut track = Track {
            title: Some("\u{2026}".into()),
            feed_title: Some("...".into()),
            track_artist: Some("...".into()),
            release_artist: Some("...".into()),
            description: Some("...\n...\n...".into()),
            image_url: Some("...".into()),
            ..Default::default()
        };
        let mut feed = Feed {
            title: Some("...".into()),
            name: Some("...".into()),
            description: Some("...\n...\n...".into()),
            release_artist: Some("...".into()),
            image_url: Some("...".into()),
            ..Default::default()
        };
        let enrichment = RssTrackEnrichment {
            feed_title: Some("Way to Go".into()),
            feed_description: Some("Feed description".into()),
            feed_artist: Some("Survival Guide".into()),
            feed_image_url: Some("https://example.test/feed.png".into()),
            feed_episode_count: Some(10),
            track_title: Some("Lantern Tide".into()),
            track_description: Some("Track description".into()),
            track_artist: Some("Max DjK".into()),
            track_image_url: Some("https://example.test/track.png".into()),
            track_number: Some(2),
            duration_secs: Some(343),
            pub_date: Some(1_777_777_777),
            ..Default::default()
        };

        assert!(apply_track_enrichment(
            &mut track,
            Some(&mut feed),
            &enrichment
        ));

        assert_eq!(track.title.as_deref(), Some("Lantern Tide"));
        assert_eq!(track.feed_title.as_deref(), Some("Way to Go"));
        assert_eq!(track.track_artist.as_deref(), Some("Max DjK"));
        assert_eq!(track.release_artist.as_deref(), Some("Survival Guide"));
        assert_eq!(track.description.as_deref(), Some("Track description"));
        assert_eq!(
            track.image_url.as_deref(),
            Some("https://example.test/track.png")
        );
        assert_eq!(track.track_number, Some(2));
        assert_eq!(track.duration_secs, Some(343));
        assert_eq!(track.pub_date, Some(1_777_777_777));
        assert_eq!(feed.title.as_deref(), Some("Way to Go"));
        assert_eq!(feed.description.as_deref(), Some("Feed description"));
        assert_eq!(feed.release_artist.as_deref(), Some("Survival Guide"));
        assert_eq!(feed.episode_count, Some(10));
    }
    #[test]
    fn rss_enrichment_preserves_existing_source_facts() {
        let mut track = Track {
            title: Some("Existing".into()),
            track_number: Some(4),
            ..Default::default()
        };
        let enrichment = RssTrackEnrichment {
            track_title: Some("Replacement".into()),
            track_number: Some(9),
            ..Default::default()
        };

        assert!(!apply_track_enrichment(&mut track, None, &enrichment));

        assert_eq!(track.title.as_deref(), Some("Existing"));
        assert_eq!(track.track_number, Some(4));
    }
    #[test]
    fn adr_0075_rss_direct_owner_npub_creates_separate_ids() -> Result<()> {
        let xml = format!("<rss xmlns:podcast=\"{}\"><channel><podcast:guid>feed</podcast:guid><podcast:txt purpose=\"npub\">{NPUB}</podcast:txt><item><guid>track</guid><podcast:txt purpose=\"npub\">{NPUB}</podcast:txt></item></channel></rss>", PODCAST_NAMESPACES[0]);
        let result = parse_track_enrichment_document(
            "request",
            "response",
            Utc::now(),
            Arc::from(xml.as_bytes()),
            Some("track"),
            None,
        )?;
        let mut context = TrackContext::new(
            Track {
                track_guid: Some("track".into()),
                feed_guid: Some("feed".into()),
                ..Default::default()
            },
            Some(Feed {
                feed_guid: Some("feed".into()),
                ..Default::default()
            }),
        );
        assert!(apply_validated_ids(&mut context, &result.observation));
        assert_eq!(context.track.source_ids.as_ref().map(Vec::len), Some(1));
        assert_eq!(
            context
                .feed
                .as_ref()
                .and_then(|feed| feed.source_ids.as_ref())
                .map(Vec::len),
            Some(1)
        );
        Ok(())
    }
    #[test]
    fn adr_0075_rss_keeps_no_match_observation() -> Result<()> {
        let xml =
            "<rss><channel><title>feed</title><item><guid>other</guid></item></channel></rss>";
        let result = parse_track_enrichment_document(
            "request",
            "response",
            Utc::now(),
            Arc::from(xml.as_bytes()),
            Some("wanted"),
            None,
        )?;
        assert!(matches!(
            result.observation.item_match,
            RssItemMatch::NoMatch
        ));
        assert!(result.enrichment.is_none());
        Ok(())
    }

    #[test]
    fn adr_0075_rss_namespace_and_purpose_rules_retain_evidence() -> Result<()> {
        let xml = format!("<rss xmlns:pc=\"{}\" xmlns:bad=\"https://example.test\"><channel><item><guid>track</guid><pc:txt purpose=\" npub \">{NPUB}</pc:txt><pc:txt purpose=\"nostr\">{NPUB}</pc:txt><bad:txt purpose=\"npub\">{NPUB}</bad:txt><pc:txt purpose=\"npub\">npub1notavalidkey</pc:txt></item></channel></rss>", PODCAST_NAMESPACES[0]);
        let result = parse_track_enrichment_document(
            "request",
            "response",
            Utc::now(),
            Arc::from(xml.as_bytes()),
            Some("track"),
            None,
        )?;
        assert_eq!(result.observation.txt_evidence.len(), 3);
        assert!(matches!(
            result.observation.txt_evidence[0].validation,
            RssTxtValidation::ValidPublicKey(_)
        ));
        assert!(matches!(
            result.observation.txt_evidence[1].validation,
            RssTxtValidation::UnsupportedPurpose
        ));
        assert!(matches!(
            result.observation.txt_evidence[2].validation,
            RssTxtValidation::MalformedEncoding
        ));
        assert!(result
            .observation
            .excluded_elements
            .iter()
            .any(|element| element.kind == RssExcludedElementKind::UnknownNamespaceTxt));
        Ok(())
    }

    #[test]
    fn adr_0075_rss_match_and_feed_disagreements_block_track_ids() -> Result<()> {
        let xml = format!("<rss xmlns:podcast=\"{}\"><channel><podcast:guid>observed-feed</podcast:guid><item><guid>other-track</guid><enclosure url=\"https://example.test/audio\"/><podcast:txt purpose=\"npub\">{NPUB}</podcast:txt></item></channel></rss>", PODCAST_NAMESPACES[0]);
        let result = parse_track_enrichment_document(
            "request",
            "response",
            Utc::now(),
            Arc::from(xml.as_bytes()),
            Some("requested-track"),
            Some("https://example.test/audio"),
        )?;
        let mut context = TrackContext::new(
            Track {
                track_guid: Some("requested-track".into()),
                feed_guid: Some("requested-feed".into()),
                ..Default::default()
            },
            Some(Feed {
                feed_guid: Some("requested-feed".into()),
                ..Default::default()
            }),
        );
        assert!(!apply_validated_ids(&mut context, &result.observation));
        assert!(context.track.source_ids.is_none());
        Ok(())
    }

    #[test]
    fn adr_0075_rss_rdf_paths_and_scalar_text_are_exact() -> Result<()> {
        let xml = format!("<rdf:RDF xmlns:rdf=\"{RDF_ROOT_NAMESPACE}\" xmlns=\"{RSS1_NAMESPACE}\" xmlns:podcast=\"{}\"><channel><title>One<!-- split -->Two</title></channel><item><guid>track</guid><podcast:txt purpose=\"npub\">{NPUB}</podcast:txt></item></rdf:RDF>", PODCAST_NAMESPACES[0]);
        let result = parse_track_enrichment_document(
            "request",
            "response",
            Utc::now(),
            Arc::from(xml.as_bytes()),
            Some("track"),
            None,
        )?;
        assert_eq!(
            result
                .enrichment
                .as_ref()
                .and_then(|value| value.feed_title.as_deref()),
            Some("OneTwo")
        );
        assert_eq!(
            result.observation.txt_evidence[0].extraction_path,
            "rdf:RDF/item[1]/podcast:txt[1]/text()"
        );
        Ok(())
    }
}

#[cfg(not(test))]
fn count_observation_dom_parse() {}
#[cfg(test)]
thread_local! { static OBSERVATION_DOM_PARSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
#[cfg(test)]
fn count_observation_dom_parse() {
    OBSERVATION_DOM_PARSES.with(|count| count.set(count.get() + 1));
}
#[cfg(test)]
mod observation_tests {
    use super::*;
    use crate::provider_observation::{
        ObservationOutcome, ProviderKind, ProviderObservation, ProviderRequestSpec,
    };
    use serde_json::json;

    #[test]
    fn adr_0075_observation_rss_single_parse_scoped_originals_survive_reopen() {
        let mut encoding = serde_json::Value::Null;
        assert!(decode_xml(b"<rss>\xff</rss>", Some(&mut encoding)).is_err());
        assert_eq!(encoding, "UTF-8");
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("rss.sqlite");
        let conn = rusqlite::Connection::open(&path).unwrap();
        crate::db::upgrades::create_fixture(&conn, 12).unwrap();
        let body:Arc<[u8]>=Arc::from(br#"<?xml version="1.0" encoding="UTF-8"?><rss xmlns:p="https://podcastindex.org/namespace/1.0"><channel><p:guid>feed-owner</p:guid><title>...</title><p:txt purpose="future">retained-feed-text</p:txt><item><guid>actual-item</guid><enclosure url="https://audio.invalid/shared"/><p:txt purpose="npub">malformed-nostr</p:txt><p:person><p:txt purpose="npub">nested-key</p:txt></p:person></item><item><guid/><title>Empty GUID evidence</title></item></channel></rss>"#.as_slice());
        let count = OBSERVATION_DOM_PARSES.with(std::cell::Cell::get);
        for requested in ["requested-one", "requested-two"] {
            let mut retained = ProviderObservation {
                body: Some(Arc::clone(&body)),
                http_status: Some(200),
                response_uri: Some("https://redirect.invalid/feed".into()),
                interpretation: json!({"version":1,"media_type":"application/rss+xml","charset":null,"retained_body_codings":[],"body_state":"complete","effective_base_uri":null}),
                source_revision: None,
                source_times: json!({}),
                decoder_version: "rss-dom-v1".into(),
                outcome: ObservationOutcome::Success,
                failure: None,
                finished_at_us: 30,
                fetched_at_us: Some(20),
                occurrence: json!({"version":1,"headers":{}}),
                coverage: Vec::new(),
            };
            let result = parse_track_enrichment_document_observed(
                "https://requested.invalid/feed",
                "https://redirect.invalid/feed",
                Utc::now(),
                Arc::clone(&body),
                Some(requested),
                Some("https://audio.invalid/shared"),
                Some(&mut retained),
            )
            .unwrap();
            assert!(
                matches!(result.observation.item_match,RssItemMatch::Matched {ref observed_item_guid,..} if observed_item_guid.as_deref()==Some("actual-item"))
            );
            assert!(Arc::ptr_eq(&result.observation.response_bytes, &body));
            assert!(retained
                .coverage
                .iter()
                .filter_map(|c| c.target.as_ref())
                .filter(|s| s.kind == "track")
                .all(|s| s.item_guid.as_deref() == Some("actual-item")));
            let spec = ProviderRequestSpec {
                provider: ProviderKind::Rss,
                provider_identity: "https://requested.invalid/feed".into(),
                request_uri: "https://requested.invalid/feed".into(),
                requested_subject: None,
                requested_parameters: json!({"track_guid":requested,"enclosure_url":"https://audio.invalid/shared"}),
                profile: json!({"version":1,"operation":"rss_track_enrichment"}),
                started_at_us: 10,
            };
            let token =
                crate::db::provider_observations::begin_provider_request(&conn, spec).unwrap();
            let receipt = crate::db::provider_observations::record_provider_observation(
                &conn,
                token,
                Arc::new(retained),
            )
            .unwrap();
            assert_eq!(
                receipt.response_uri.as_deref(),
                Some("https://redirect.invalid/feed")
            );
        }
        assert_eq!(OBSERVATION_DOM_PARSES.with(std::cell::Cell::get) - count, 2);
        drop(conn);
        let reopened = rusqlite::Connection::open(path).unwrap();
        assert_eq!(
            reopened
                .query_row("SELECT count(*) FROM metadata_bodies", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            reopened
                .query_row("SELECT bytes FROM metadata_bodies", [], |r| r
                    .get::<_, Vec<u8>>(0))
                .unwrap(),
            body.as_ref()
        );
        let evidence: String = reopened
            .query_row(
                "SELECT value_json FROM metadata_facts WHERE validation='unsupported' LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(evidence.contains("retained-feed-text"));
        assert_eq!(
            reopened
                .query_row(
                    "SELECT count(*) FROM metadata_facts WHERE validation='malformed'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            2
        );
        let locator: String = reopened
            .query_row(
                "SELECT body_locator_json FROM metadata_facts WHERE validation='malformed' LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let locator: serde_json::Value = serde_json::from_str(&locator).unwrap();
        assert!(locator["decoded_byte_offset"].as_u64().unwrap() > 0);
        assert_eq!(locator["line"], 1);
        let decoder: String = reopened
            .query_row(
                "SELECT interpretation_metadata_json FROM metadata_observations LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(decoder.contains("UTF-8"));
    }
    const SNAPSHOT_NPUB: &str = "npub1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzqujme";

    fn snapshot_document(
        feed_guid: &str,
        track_guid: &str,
        feed_txt: &str,
        track_txt: &str,
    ) -> String {
        format!("<rss xmlns:p=\"https://podcastindex.org/namespace/1.0\"><channel>{feed_guid}{feed_txt}<item>{track_guid}{track_txt}</item></channel></rss>")
    }
    fn snapshot_observation(spec: &ProviderRequestSpec, xml: &str) -> ProviderObservation {
        let body: Arc<[u8]> = Arc::from(xml.as_bytes());
        let mut observation = ProviderObservation {
            body: Some(Arc::clone(&body)),
            http_status: Some(200),
            response_uri: Some(spec.request_uri.clone()),
            interpretation: json!({"body_state":"complete","charset":null}),
            source_revision: None,
            source_times: json!({}),
            decoder_version: "rss-dom-v1".into(),
            outcome: ObservationOutcome::Success,
            failure: None,
            finished_at_us: 30,
            fetched_at_us: Some(20),
            occurrence: json!({"headers":{}}),
            coverage: Vec::new(),
        };
        let parsed = parse_track_enrichment_document_observed(
            &spec.request_uri,
            &spec.request_uri,
            Utc::now(),
            body,
            spec.requested_parameters["track_guid"].as_str(),
            spec.requested_parameters["enclosure_url"].as_str(),
            Some(&mut observation),
        );
        match parsed {
            Err(_) => observation.fail("xml_decode"),
            Ok(result) if result.enrichment.is_none() => {
                observation.outcome = ObservationOutcome::Partial;
                observation.failure = Some(json!({"reason":"no_matching_item"}));
            }
            _ => {}
        }
        crate::provider_observation::contracts::rebind_test_proofs(spec, &mut observation);
        observation
    }
    fn snapshot_connection() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::upgrades::create_fixture(&conn, 12).unwrap();
        conn
    }
    fn snapshot_request(resource: &str) -> ProviderRequestSpec {
        crate::provider_observation::contracts::rss_request(resource, Some("track"), None)
    }
    fn record_snapshot(
        conn: &rusqlite::Connection,
        spec: ProviderRequestSpec,
        observation: ProviderObservation,
    ) -> crate::provider_observation::ObservationReceipt {
        let token = crate::db::provider_observations::begin_provider_request(conn, spec).unwrap();
        crate::db::provider_observations::record_provider_observation(
            conn,
            token,
            Arc::new(observation),
        )
        .unwrap()
    }
    fn snapshot_state(
        conn: &rusqlite::Connection,
        provider: i64,
        subject: &crate::provider_observation::SubjectKey,
    ) -> crate::provider_observation::CollectionState {
        crate::db::provider_observations::read_provider_collection(
            conn,
            provider,
            subject,
            "source_ids",
        )
        .unwrap()
        .state
    }
    fn snapshot_counts(conn: &rusqlite::Connection) -> Vec<i64> {
        [
            "metadata_bodies",
            "metadata_observations",
            "metadata_coverage",
            "metadata_facts",
            "metadata_snapshots",
            "metadata_snapshot_members",
            "metadata_collection_heads",
        ]
        .map(|table| {
            conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap()
        })
        .to_vec()
    }

    #[test]
    fn adr_0075_snapshot_populated_empty_reopen_and_provider_subject_isolation() {
        use crate::provider_observation::{CollectionState, SubjectKey};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snapshot.sqlite");
        let conn = rusqlite::Connection::open(&path).unwrap();
        crate::db::upgrades::create_fixture(&conn, 12).unwrap();
        let txt = format!("<p:txt purpose=\"npub\">{SNAPSHOT_NPUB}</p:txt>");
        let xml = snapshot_document("<p:guid>feed</p:guid>", "<guid>track</guid>", &txt, &txt);
        let spec = snapshot_request("https://rss.test/one");
        let observation = snapshot_observation(&spec, &xml);
        assert_eq!(
            observation
                .coverage
                .iter()
                .filter(|c| c.proof.is_some())
                .count(),
            2
        );
        let receipt = record_snapshot(&conn, spec.clone(), observation);
        let other_spec = snapshot_request("https://rss.test/two");
        let other = record_snapshot(
            &conn,
            other_spec.clone(),
            snapshot_observation(&other_spec, &xml),
        );
        let other_state = snapshot_state(
            &conn,
            other.provider_id,
            &SubjectKey::guid("feed", Some("track")),
        );
        assert!(matches!(other_state, CollectionState::CompletePopulated(_)));
        let changed_feed = snapshot_document(
            "<p:guid>other-feed</p:guid>",
            "<guid>track</guid>",
            &txt,
            &txt,
        );
        let _ = record_snapshot(
            &conn,
            spec.clone(),
            snapshot_observation(&spec, &changed_feed),
        );
        let scoped = snapshot_document("", "<guid>track</guid>", &txt, &txt);
        let _ = record_snapshot(&conn, spec.clone(), snapshot_observation(&spec, &scoped));
        let empty = snapshot_document("<p:guid>feed</p:guid>", "<guid>track</guid>", "", "");
        let _ = record_snapshot(&conn, spec.clone(), snapshot_observation(&spec, &empty));
        assert_eq!(
            snapshot_state(
                &conn,
                other.provider_id,
                &SubjectKey::guid("feed", Some("track"))
            ),
            other_state
        );
        assert!(matches!(
            snapshot_state(
                &conn,
                receipt.provider_id,
                &SubjectKey::guid("other-feed", Some("track"))
            ),
            CollectionState::CompletePopulated(_)
        ));
        let resource_subject = SubjectKey {
            kind: "track".into(),
            scope_kind: "resource".into(),
            scope: spec.request_uri.clone(),
            item_guid: Some("track".into()),
        };
        assert!(matches!(
            snapshot_state(&conn, receipt.provider_id, &resource_subject),
            CollectionState::CompletePopulated(_)
        ));
        drop(conn);
        let conn = rusqlite::Connection::open(path).unwrap();
        for subject in [
            SubjectKey::guid("feed", None),
            SubjectKey::guid("feed", Some("track")),
        ] {
            assert!(matches!(
                snapshot_state(&conn, receipt.provider_id, &subject),
                CollectionState::CompleteEmpty(_)
            ));
        }
        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM metadata_snapshots WHERE member_count=0",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            2
        );
    }

    #[test]
    fn adr_0075_snapshot_rejected_identity_and_owner_declarations_preserve_heads() {
        use crate::provider_observation::SubjectKey;
        let conn = snapshot_connection();
        let spec = snapshot_request("https://rss.test/feed");
        let txt = format!("<p:txt purpose=\"npub\">{SNAPSHOT_NPUB}</p:txt>");
        let seed = snapshot_document("<p:guid>feed</p:guid>", "<guid>track</guid>", &txt, &txt);
        let receipt = record_snapshot(&conn, spec.clone(), snapshot_observation(&spec, &seed));
        let prior_feed =
            snapshot_state(&conn, receipt.provider_id, &SubjectKey::guid("feed", None));
        let prior_track = snapshot_state(
            &conn,
            receipt.provider_id,
            &SubjectKey::guid("feed", Some("track")),
        );
        for candidate in [
            "<p:txt purpose=\"npub\">malformed</p:txt>",
            "<p:txt purpose=\"future\">retained</p:txt>",
            "<p:txt purpose=\"npub\"><nested/></p:txt>",
        ] {
            let xml = snapshot_document(
                "<p:guid>feed</p:guid>",
                "<guid>track</guid>",
                candidate,
                candidate,
            );
            let observation = snapshot_observation(&spec, &xml);
            assert!(observation.coverage.iter().all(|c| c.proof.is_none()));
            let _ = record_snapshot(&conn, spec.clone(), observation);
            assert_eq!(
                snapshot_state(&conn, receipt.provider_id, &SubjectKey::guid("feed", None)),
                prior_feed
            );
            assert_eq!(
                snapshot_state(
                    &conn,
                    receipt.provider_id,
                    &SubjectKey::guid("feed", Some("track"))
                ),
                prior_track
            );
        }
        let resource = snapshot_document("", "<guid>track</guid>", &txt, &txt);
        let _ = record_snapshot(&conn, spec.clone(), snapshot_observation(&spec, &resource));
        let resource_count = conn
            .query_row(
                "SELECT count(*) FROM metadata_subjects WHERE feed_scope_kind='resource'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap();
        for declaration in [
            "<p:guid/>",
            "<p:guid>   </p:guid>",
            "<p:guid><nested>feed</nested></p:guid>",
            "<p:guid>feed</p:guid><p:guid>feed</p:guid>",
        ] {
            let xml = snapshot_document(declaration, "<guid>track</guid>", "", "");
            let observation = snapshot_observation(&spec, &xml);
            assert!(observation
                .coverage
                .iter()
                .all(|c| c.proof.is_none() && c.target.is_none()));
            let _ = record_snapshot(&conn, spec.clone(), observation);
            assert_eq!(
                snapshot_state(&conn, receipt.provider_id, &SubjectKey::guid("feed", None)),
                prior_feed
            );
        }
        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM metadata_subjects WHERE feed_scope_kind='resource'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            resource_count
        );
        for declaration in [
            "<guid/>",
            "<guid> </guid>",
            "<guid><nested>track</nested></guid>",
            "<guid>track</guid><guid>track</guid>",
        ] {
            let xml = snapshot_document("<p:guid>feed</p:guid>", declaration, "", "");
            let _ = record_snapshot(&conn, spec.clone(), snapshot_observation(&spec, &xml));
            assert_eq!(
                snapshot_state(
                    &conn,
                    receipt.provider_id,
                    &SubjectKey::guid("feed", Some("track"))
                ),
                prior_track
            );
        }
    }

    #[test]
    fn adr_0075_snapshot_ambiguous_matches_and_nested_evidence_cannot_replace_track() {
        use crate::provider_observation::SubjectKey;
        let conn = snapshot_connection();
        let mut spec = snapshot_request("https://rss.test/feed");
        spec.requested_parameters["enclosure_url"] = json!("https://audio.test/shared");
        let txt = format!("<p:txt purpose=\"npub\">{SNAPSHOT_NPUB}</p:txt>");
        let seed = snapshot_document("<p:guid>feed</p:guid>", "<guid>track</guid>", &txt, &txt);
        let receipt = record_snapshot(&conn, spec.clone(), snapshot_observation(&spec, &seed));
        let prior = snapshot_state(
            &conn,
            receipt.provider_id,
            &SubjectKey::guid("feed", Some("track")),
        );
        for xml in [
            seed.replace("</channel>","<item><guid>track</guid></item></channel>"),
            seed.replace("</rss>","<channel><p:guid>feed</p:guid><item><guid>track</guid></item></channel></rss>"),
            snapshot_document("<p:guid>feed</p:guid>","<guid>other</guid><enclosure url=\"https://audio.test/shared\"/>","",""),
            snapshot_document("<p:guid>feed</p:guid>","<guid>missing</guid>","",""),
            seed.replace("</channel>","<item><guid>other</guid><enclosure url=\"https://audio.test/shared\"/></item></channel>").replace("<guid>track</guid>","<guid>another</guid><enclosure url=\"https://audio.test/shared\"/>"),
        ] {
            let _ = record_snapshot(&conn,spec.clone(),snapshot_observation(&spec,&xml));
            assert_eq!(snapshot_state(&conn,receipt.provider_id,&SubjectKey::guid("feed",Some("track"))),prior);
        }
        let nested = snapshot_document("<p:guid>feed</p:guid>","<guid>track</guid>","",&format!("<p:person>{txt}</p:person><nested>{txt}</nested><foreign:txt xmlns:foreign=\"https://foreign.test\">{SNAPSHOT_NPUB}</foreign:txt>"));
        let observation = snapshot_observation(&spec, &nested);
        assert!(observation
            .coverage
            .iter()
            .filter(|c| c.collection == "source_ids")
            .all(|c| c.facts.is_empty()));
        assert!(observation
            .coverage
            .iter()
            .flat_map(|c| &c.facts)
            .filter(|f| f.value["name"] == "txt")
            .all(|f| f.validation == "unresolved"));
        let _ = record_snapshot(&conn, spec.clone(), observation);
        let xml = seed.replace(
            "</channel>",
            &format!("<item><guid>other</guid>{txt}</item></channel>"),
        );
        let observation = snapshot_observation(&spec, &xml);
        assert!(observation
            .coverage
            .iter()
            .filter(|c| c.target.as_ref().and_then(|s| s.item_guid.as_deref()) == Some("other"))
            .all(|c| c.proof.is_none()));
    }

    #[test]
    fn adr_0075_snapshot_repeated_content_replay_and_generation_order() {
        use crate::db::provider_observations::{
            begin_provider_request, read_request_refresh, record_provider_observation,
        };
        use crate::provider_observation::{
            CollectionRetention, CollectionState, StorageRetry, SubjectKey,
        };
        let conn = snapshot_connection();
        let spec = snapshot_request("https://rss.test/feed");
        let xml = snapshot_document("<p:guid>feed</p:guid>", "<guid>track</guid>", "", "");
        let observation = Arc::new(snapshot_observation(&spec, &xml));
        let token = begin_provider_request(&conn, spec.clone()).unwrap();
        let first =
            record_provider_observation(&conn, token.clone(), Arc::clone(&observation)).unwrap();
        let counts = snapshot_counts(&conn);
        let mut repeated = (*observation).clone();
        repeated.finished_at_us = 100;
        repeated.fetched_at_us = Some(90);
        repeated.occurrence = json!({"headers":{"etag":["new"]}});
        let latest = record_snapshot(&conn, spec.clone(), repeated);
        assert_eq!(snapshot_counts(&conn), counts);
        assert!(latest
            .collections
            .iter()
            .filter(|c| c.collection == "source_ids")
            .all(|c| c.retention == CollectionRetention::Repeated));
        let head = snapshot_state(
            &conn,
            first.provider_id,
            &SubjectKey::guid("feed", Some("track")),
        );
        assert!(
            matches!(&head,CollectionState::CompleteEmpty(snapshot) if snapshot.fetched_at_us == Some(90) && snapshot.generation == latest.generation)
        );
        let changes = conn.total_changes();
        assert_eq!(
            record_provider_observation(&conn, token.clone(), Arc::clone(&observation)).unwrap(),
            first
        );
        assert_eq!(conn.total_changes(), changes);
        let mut changed = (*observation).clone();
        changed.finished_at_us += 1;
        assert_eq!(
            record_provider_observation(&conn, token, Arc::new(changed))
                .unwrap_err()
                .retry,
            StorageRetry::Blocked
        );
        let mut other_profile = spec.clone();
        other_profile.profile["case"] = json!("other");
        let old_token = begin_provider_request(&conn, other_profile.clone()).unwrap();
        let mut older = snapshot_observation(&other_profile, &xml);
        // The request profile participates in proof, but not snapshot identity.
        let recent = record_snapshot(&conn, spec.clone(), snapshot_observation(&spec, &xml));
        older.occurrence = json!({"headers":{"old":true}});
        let old = record_provider_observation(&conn, old_token, Arc::new(older)).unwrap();
        assert!(old
            .collections
            .iter()
            .filter(|c| c.collection == "source_ids")
            .all(|c| c.retention == CollectionRetention::RejectedSuperseded));
        let mut failure = snapshot_observation(&spec, "<broken");
        failure.fail("xml_decode");
        let failed = record_snapshot(&conn, spec.clone(), failure);
        let old_slot_token = begin_provider_request(&conn, spec.clone()).unwrap();
        let new_slot_token = begin_provider_request(&conn, spec.clone()).unwrap();
        let mut failed_again = snapshot_observation(&spec, "<broken");
        failed_again.fail("xml_decode");
        let newest_failure =
            record_provider_observation(&conn, new_slot_token, Arc::new(failed_again)).unwrap();
        let rejected = record_provider_observation(
            &conn,
            old_slot_token,
            Arc::new(snapshot_observation(&spec, &xml)),
        )
        .unwrap();
        assert!(rejected
            .collections
            .iter()
            .filter(|c| c.collection == "source_ids")
            .all(|c| c.retention == CollectionRetention::RejectedSuperseded));
        assert!(recent.generation < failed.generation);
        assert_eq!(
            read_request_refresh(&conn, &spec)
                .unwrap()
                .unwrap()
                .failure_id,
            Some(newest_failure.observation_id)
        );
    }

    #[test]
    fn adr_0075_snapshot_transaction_failures_rollback_and_verified_retry_once() {
        use crate::db::provider_observations::{
            begin_provider_request, record_provider_observation,
        };
        use crate::provider_observation::{StorageRetry, SubjectKey};
        for trigger in [
            "CREATE TEMP TRIGGER reject_response AFTER INSERT ON metadata_facts BEGIN SELECT RAISE(ABORT,'after evidence'); END",
            "CREATE TEMP TRIGGER reject_response AFTER INSERT ON metadata_collection_heads BEGIN SELECT RAISE(ABORT,'after first head'); END",
            "CREATE TEMP TRIGGER reject_response BEFORE UPDATE OF state ON metadata_request_slots BEGIN SELECT RAISE(ABORT,'before commit'); END",
        ] {
            let conn = snapshot_connection();
            let spec = snapshot_request("https://rss.test/feed");
            let xml = snapshot_document("<p:guid>feed</p:guid>","<guid>track</guid>","","");
            let token = begin_provider_request(&conn,spec.clone()).unwrap();
            let observation = Arc::new(snapshot_observation(&spec,&xml));
            let before = snapshot_counts(&conn);
            conn.execute_batch(trigger).unwrap();
            let failure = record_provider_observation(&conn,token.clone(),Arc::clone(&observation)).unwrap_err();
            assert_eq!(failure.retry,StorageRetry::VerifiedRollback);
            assert_eq!(snapshot_counts(&conn),before);
            conn.execute_batch("DROP TRIGGER reject_response").unwrap();
            let receipt = record_provider_observation(&conn,token,observation).unwrap();
            assert_eq!(receipt.generation,failure.token.generation());
            assert!(matches!(snapshot_state(&conn,receipt.provider_id,&SubjectKey::guid("feed",Some("track"))),crate::provider_observation::CollectionState::CompleteEmpty(_)));
            assert_eq!(conn.query_row("SELECT last_generation FROM metadata_generation",[],|r|r.get::<_,i64>(0)).unwrap(),1);
        }
    }

    #[test]
    fn adr_0075_snapshot_proof_mismatch_and_decoder_versions_preserve_old_interpretations() {
        use crate::db::provider_observations::{
            begin_provider_request, record_provider_observation,
        };
        let conn = snapshot_connection();
        let spec = snapshot_request("https://rss.test/feed");
        for xml in [
            snapshot_document("<p:guid>feed</p:guid>", "<guid>track</guid>", "", ""),
            snapshot_document("<p:guid/>", "<guid>track</guid>", "", ""),
            snapshot_document(
                "<p:guid>feed</p:guid>",
                "<guid>track</guid>",
                "<p:txt purpose=\"other\">raw</p:txt>",
                "",
            ),
            snapshot_document("<p:guid>feed</p:guid>", "<guid>other</guid>", "", ""),
            "<broken".into(),
        ] {
            let current = snapshot_observation(&spec, &xml);
            assert_eq!(current.decoder_version, "rss-dom-v2");
            let mut old = current.clone();
            old.decoder_version = "rss-dom-v1".into();
            for coverage in &mut old.coverage {
                coverage.proof = None;
            }
            let old_receipt = record_snapshot(&conn, spec.clone(), old);
            let new_receipt = record_snapshot(&conn, spec.clone(), current);
            assert_ne!(old_receipt.observation_id, new_receipt.observation_id);
            let old_contract: Option<String> = conn
                .query_row(
                    "SELECT contract_id FROM metadata_observations WHERE id=?1",
                    [old_receipt.observation_id],
                    |r| r.get(0),
                )
                .unwrap();
            assert!(old_contract.is_none());
        }
        let xml = snapshot_document("<p:guid>feed</p:guid>", "<guid>track</guid>", "", "");
        let valid = snapshot_observation(&spec, &xml);
        for mutation in 0..4 {
            let mut forged = valid.clone();
            match mutation {
                0 => forged.coverage[0].target.as_mut().unwrap().scope = "other".into(),
                1 => forged.decoder_version = "unknown-contract".into(),
                2 => forged.body = Some(Arc::from(b"other body".as_slice())),
                _ => forged.coverage.push(forged.coverage[0].clone()),
            }
            let token = begin_provider_request(&conn, spec.clone()).unwrap();
            let before = snapshot_counts(&conn);
            assert!(record_provider_observation(&conn, token, Arc::new(forged)).is_err());
            assert_eq!(snapshot_counts(&conn), before);
        }
    }

    #[test]
    fn adr_0075_snapshot_exact_refresh_keys_keep_profiles_null_and_empty_separate() {
        use crate::db::provider_observations::{begin_provider_request, read_request_refresh};
        use crate::provider_observation::RefreshState;
        let conn = snapshot_connection();
        let original = snapshot_request("https://rss.test/feed");
        let mut variants = vec![original.clone()];
        let mut profile = original.clone();
        profile.profile["variant"] = json!(2);
        variants.push(profile);
        let mut empty = original.clone();
        empty.requested_parameters["enclosure_url"] = json!("");
        variants.push(empty);
        let mut guid = original.clone();
        guid.requested_parameters["track_guid"] = json!("other");
        variants.push(guid);
        let mut resource = original.clone();
        resource.request_uri.push_str("/other");
        resource.provider_identity = resource.request_uri.clone();
        variants.push(resource);
        for (index, spec) in variants.iter().enumerate() {
            let token = begin_provider_request(&conn, spec.clone()).unwrap();
            let refresh = read_request_refresh(&conn, spec).unwrap().unwrap();
            assert_eq!(refresh.generation, token.generation());
            assert_eq!(refresh.generation, i64::try_from(index + 1).unwrap());
            assert_eq!(refresh.state, RefreshState::Pending);
        }
        assert_eq!(
            read_request_refresh(&conn, &original)
                .unwrap()
                .unwrap()
                .generation,
            1
        );
        let changes = conn.total_changes();
        let state =
            crate::db::provider_observations::read_track_provider_state(&conn, Some(&original))
                .unwrap();
        assert!(state.collections.is_empty());
        assert_eq!(
            state.binding,
            crate::provider_observation::ProviderBinding::RequestOnly
        );
        assert_eq!(conn.total_changes(), changes);
    }
    #[test]
    fn adr_0075_snapshot_registry_rejects_unrelated_dom_and_conflicting_values() {
        use crate::provider_observation::contracts;
        let spec = snapshot_request("https://private-resource.test/feed");
        let txt = format!("<p:txt purpose=\"npub\">{SNAPSHOT_NPUB}</p:txt>");
        for text in ["", txt.as_str()] {
            let xml = snapshot_document("<p:guid>feed</p:guid>", "<guid>track</guid>", text, text);
            let observation = snapshot_observation(&spec, &xml);
            let decoded = contracts::decode_rss_body(xml.as_bytes(), None).unwrap();
            let document = Document::parse(decoded.text()).unwrap();
            let owner = document
                .root_element()
                .children()
                .find(|node| node.is_element())
                .unwrap();
            let proof = contracts::certify_rss(&observation, &spec, 0, owner, &decoded).unwrap();
            for rendered in [format!("{decoded:?}"), format!("{proof:?}")] {
                assert!(!rendered.contains("private-resource"));
                assert!(!rendered.contains("<rss"));
                assert!(!rendered.contains(SNAPSHOT_NPUB));
            }
            let unrelated = xml.replace("feed</p:guid>", "other</p:guid>");
            let other_decoded = contracts::decode_rss_body(unrelated.as_bytes(), None).unwrap();
            let other_document = Document::parse(other_decoded.text()).unwrap();
            let other_owner = other_document
                .root_element()
                .children()
                .find(|node| node.is_element())
                .unwrap();
            assert!(
                contracts::certify_rss(&observation, &spec, 0, other_owner, &decoded).is_none()
            );
            assert!(
                contracts::certify_rss(&observation, &spec, 0, other_owner, &other_decoded)
                    .is_none()
            );
            let unsupported = xml
                .replace("<rss ", "<unsupported ")
                .replace("</rss>", "</unsupported>");
            let unsupported_decoded =
                contracts::decode_rss_body(unsupported.as_bytes(), None).unwrap();
            let unsupported_document = Document::parse(unsupported_decoded.text()).unwrap();
            let unsupported_owner = unsupported_document
                .root_element()
                .children()
                .find(|node| node.is_element())
                .unwrap();
            let mut wrong_root = observation.clone();
            wrong_root.body = Some(Arc::from(unsupported.as_bytes()));
            assert!(contracts::certify_rss(
                &wrong_root,
                &spec,
                0,
                unsupported_owner,
                &unsupported_decoded
            )
            .is_none());
            if !text.is_empty() {
                for field in ["direct_text", "name", "namespace", "attributes"] {
                    let mut altered = observation.clone();
                    altered.coverage[0].facts[0].value[field] = json!("conflicting-assertion");
                    assert!(contracts::certify_rss(&altered, &spec, 0, owner, &decoded).is_none());
                }
            }
        }
    }

    #[test]
    fn adr_0075_snapshot_read_rejects_cross_provider_member_and_head_content_substitution() {
        use crate::db::provider_observations::read_provider_collection;
        use crate::provider_observation::{CollectionState, SubjectKey};
        let conn = snapshot_connection();
        let spec = snapshot_request("https://rss.test/one");
        let other_spec = snapshot_request("https://rss.test/two");
        let txt = format!("<p:txt purpose=\"npub\">{SNAPSHOT_NPUB}</p:txt>");
        let populated =
            snapshot_document("<p:guid>feed</p:guid>", "<guid>track</guid>", &txt, &txt);
        let first = record_snapshot(&conn, spec.clone(), snapshot_observation(&spec, &populated));
        let other = record_snapshot(
            &conn,
            other_spec.clone(),
            snapshot_observation(&other_spec, &populated),
        );
        let subject = SubjectKey::guid("feed", Some("track"));
        let CollectionState::CompletePopulated(first_state) =
            snapshot_state(&conn, first.provider_id, &subject)
        else {
            panic!("populated snapshot required");
        };
        let CollectionState::CompletePopulated(other_state) =
            snapshot_state(&conn, other.provider_id, &subject)
        else {
            panic!("other populated snapshot required");
        };
        conn.execute(
            "UPDATE metadata_snapshot_members SET fact_id=?1 WHERE snapshot_id=?2",
            rusqlite::params![other_state.members[0].fact_id, first_state.snapshot_id],
        )
        .unwrap();
        assert!(
            read_provider_collection(&conn, first.provider_id, &subject, "source_ids").is_err()
        );
        conn.execute(
            "UPDATE metadata_snapshot_members SET fact_id=?1 WHERE snapshot_id=?2",
            rusqlite::params![first_state.members[0].fact_id, first_state.snapshot_id],
        )
        .unwrap();
        let empty = snapshot_document("<p:guid>feed</p:guid>", "<guid>track</guid>", "", "");
        let _ = record_snapshot(&conn, spec.clone(), snapshot_observation(&spec, &empty));
        conn.execute("UPDATE metadata_collection_heads SET snapshot_id=?1 WHERE provider_id=?2 AND subject_id=(SELECT id FROM metadata_subjects WHERE subject_key=?3) AND collection='source_ids'",rusqlite::params![first_state.snapshot_id,first.provider_id,subject.json().to_string()]).unwrap();
        assert!(
            read_provider_collection(&conn, first.provider_id, &subject, "source_ids").is_err()
        );
        assert_eq!(
            snapshot_state(&conn, other.provider_id, &subject),
            CollectionState::CompletePopulated(other_state)
        );
    }

    #[test]
    fn adr_0075_snapshot_source_labels_replace_together_and_uncertain_commit_blocks_replay() {
        use crate::db::provider_observations::{
            begin_provider_request, record_provider_observation,
        };
        use crate::provider_observation::{contracts, CollectionState, StorageRetry, SubjectKey};
        let conn = snapshot_connection();
        let spec = snapshot_request("https://rss.test/feed");
        let txt = format!("<p:txt purpose=\"npub\">{SNAPSHOT_NPUB}</p:txt>");
        let xml = snapshot_document(
            "<p:guid>feed</p:guid>",
            "<guid>track</guid>",
            &txt,
            &format!("{txt}{txt}"),
        );
        let mut populated = snapshot_observation(&spec, &xml);
        let track = populated
            .coverage
            .iter_mut()
            .find(|c| {
                c.collection == "source_ids" && c.target.as_ref().is_some_and(|s| s.kind == "track")
            })
            .unwrap();
        track.facts[0].assertion_source = Some("rss-first".into());
        track.facts[1].assertion_source = Some("rss-second".into());
        contracts::rebind_test_proofs(&spec, &mut populated);
        let receipt = record_snapshot(&conn, spec.clone(), populated);
        let subject = SubjectKey::guid("feed", Some("track"));
        let CollectionState::CompletePopulated(snapshot) =
            snapshot_state(&conn, receipt.provider_id, &subject)
        else {
            panic!("populated snapshot required");
        };
        assert_eq!(snapshot.members.len(), 2);
        let empty = snapshot_document("<p:guid>feed</p:guid>", "<guid>track</guid>", "", "");
        let empty_observation = Arc::new(snapshot_observation(&spec, &empty));
        let token = begin_provider_request(&conn, spec.clone()).unwrap();
        conn.commit_hook(Some(|| true)).unwrap();
        let failure =
            record_provider_observation(&conn, token.clone(), Arc::clone(&empty_observation))
                .unwrap_err();
        assert_eq!(failure.retry, StorageRetry::Blocked);
        conn.commit_hook(None::<fn() -> bool>).unwrap();
        assert_eq!(
            snapshot_state(&conn, receipt.provider_id, &subject),
            CollectionState::CompletePopulated(snapshot)
        );
        assert_eq!(
            record_provider_observation(&conn, token, empty_observation)
                .unwrap_err()
                .retry,
            StorageRetry::Blocked
        );
        let _ = record_snapshot(&conn, spec.clone(), snapshot_observation(&spec, &empty));
        assert!(matches!(
            snapshot_state(&conn, receipt.provider_id, &subject),
            CollectionState::CompleteEmpty(_)
        ));
        assert_eq!(conn.query_row("SELECT count(DISTINCT assertion_source) FROM metadata_facts WHERE assertion_source LIKE 'rss-%'",[],|r|r.get::<_,i64>(0)).unwrap(),2);
    }
    #[test]
    fn adr_0075_snapshot_unverified_index_presence_matrix_preserves_seeded_heads() {
        use crate::provider_observation::{musicindex, SubjectKey};
        let conn = snapshot_connection();
        let rss_spec = snapshot_request("https://index.test");
        let txt = format!("<p:txt purpose=\"npub\">{SNAPSHOT_NPUB}</p:txt>");
        let xml = snapshot_document("<p:guid>feed</p:guid>", "<guid>track</guid>", &txt, &txt);
        let first = record_snapshot(
            &conn,
            rss_spec.clone(),
            snapshot_observation(&rss_spec, &xml),
        );
        // Synthetic seeded Index heads test preservation without enabling an Index contract.
        conn.execute(
            "UPDATE metadata_providers SET kind='musicindex' WHERE id=?1",
            [first.provider_id],
        )
        .unwrap();
        let subject = SubjectKey::guid("feed", Some("track"));
        let prior = snapshot_state(&conn, first.provider_id, &subject);
        let mut spec = rss_spec.clone();
        spec.provider = ProviderKind::MusicIndex;
        spec.requested_subject = Some(subject.clone());
        spec.requested_parameters = json!({"path":["v1","feeds","feed","tracks","track"]});
        spec.profile = json!({"version":1,"query":[["include","source_ids"]]});
        let mut presences = std::collections::BTreeSet::new();
        for (index,payload) in [
            json!({"data":{"feed_guid":"feed","track_guid":"track"}}),
            json!({"data":{"feed_guid":"feed","track_guid":"track","source_ids":null}}),
            json!({"data":{"feed_guid":"feed","track_guid":"track","source_ids":[]}}),
            json!({"data":{"feed_guid":"feed","track_guid":"track","source_ids":false}}),
            json!({"data":{"feed_guid":"feed","track_guid":"track","source_ids":[{"entity_type":"track","entity_id":"track","value":"kept","scheme":"unknown"}]}}),
        ].iter().enumerate() {
            let mut observation = snapshot_observation(&rss_spec,"<invalid");
            observation.body = Some(Arc::from(payload.to_string().into_bytes()));
            observation.decoder_version = "musicindex-json-v1".into();
            observation.outcome = ObservationOutcome::Success;observation.failure = None;observation.coverage.clear();
            musicindex::extract(&mut observation,&spec,&payload.to_string());
            let coverage = observation.coverage.iter().find(|coverage|coverage.collection == "source_ids").unwrap();
            presences.insert(coverage.presence.token().to_owned());
            assert!(coverage.proof.is_none());
            let _ = record_snapshot(&conn,spec.clone(),observation);
            assert_eq!(snapshot_state(&conn,first.provider_id,&subject),prior,"matrix case {index}");
        }
        assert_eq!(presences.len(), 5);
        let mut failed = snapshot_observation(&rss_spec, "<broken");
        failed.decoder_version = "musicindex-json-v1".into();
        let _ = record_snapshot(&conn, spec.clone(), failed);
        assert_eq!(snapshot_state(&conn, first.provider_id, &subject), prior);
        let mut omitted = spec.clone();
        omitted.profile = json!({"version":1,"query":[]});
        let payload = json!({"data":{"feed_guid":"feed","track_guid":"track"}});
        let mut summary = snapshot_observation(&rss_spec, "<broken");
        summary.decoder_version = "musicindex-json-v1".into();
        summary.outcome = ObservationOutcome::Success;
        summary.failure = None;
        summary.body = Some(Arc::from(payload.to_string().into_bytes()));
        summary.coverage.clear();
        musicindex::extract(&mut summary, &omitted, &payload.to_string());
        assert!(summary
            .coverage
            .iter()
            .all(|coverage| coverage.proof.is_none()));
        let _ = record_snapshot(&conn, omitted, summary);
        assert_eq!(snapshot_state(&conn, first.provider_id, &subject), prior);
    }

    #[test]
    fn adr_0075_snapshot_cross_resource_order_keeps_newer_head_and_failure() {
        use crate::db::provider_observations::{
            begin_provider_request, read_request_refresh, record_provider_observation,
        };
        use crate::provider_observation::{contracts, CollectionRetention, SubjectKey};
        let conn = snapshot_connection();
        let spec = snapshot_request("https://rss.test/one");
        let xml = snapshot_document("<p:guid>feed</p:guid>", "<guid>track</guid>", "", "");
        let mut other = spec.clone();
        other.request_uri = "https://rss.test/two".into();
        let old_token = begin_provider_request(&conn, other.clone()).unwrap();
        let mut old = snapshot_observation(&spec, &xml);
        // Test-only proof binds another resource within the same synthetic provider.
        contracts::rebind_test_proofs(&other, &mut old);
        let latest = record_snapshot(&conn, spec.clone(), snapshot_observation(&spec, &xml));
        let prior = snapshot_state(
            &conn,
            latest.provider_id,
            &SubjectKey::guid("feed", Some("track")),
        );
        let mut failure = snapshot_observation(&spec, "<broken");
        failure.fail("xml_decode");
        let failed = record_snapshot(&conn, spec.clone(), failure);
        let rejected = record_provider_observation(&conn, old_token, Arc::new(old)).unwrap();
        assert!(rejected
            .collections
            .iter()
            .filter(|c| c.collection == "source_ids")
            .all(|c| c.retention == CollectionRetention::RejectedSuperseded));
        assert_eq!(
            snapshot_state(
                &conn,
                latest.provider_id,
                &SubjectKey::guid("feed", Some("track"))
            ),
            prior
        );
        assert_eq!(
            read_request_refresh(&conn, &spec)
                .unwrap()
                .unwrap()
                .failure_id,
            Some(failed.observation_id)
        );
    }
    #[test]
    fn adr_0075_snapshot_missing_binding_missing_head_and_empty_are_distinct() {
        use crate::db::provider_observations::{
            read_provider_collection, read_track_provider_state,
        };
        use crate::provider_observation::{CollectionState, ProviderBinding, SubjectKey};
        let conn = snapshot_connection();
        let spec = snapshot_request("https://rss.test/feed");
        assert_eq!(
            read_track_provider_state(&conn, None).unwrap().binding,
            ProviderBinding::Unavailable
        );
        assert_eq!(
            read_track_provider_state(&conn, Some(&spec))
                .unwrap()
                .binding,
            ProviderBinding::RequestOnly
        );
        let xml = snapshot_document("<p:guid>feed</p:guid>", "<guid>track</guid>", "", "");
        let receipt = record_snapshot(&conn, spec.clone(), snapshot_observation(&spec, &xml));
        let subject = SubjectKey::guid("feed", Some("track"));
        assert!(matches!(
            read_provider_collection(&conn, receipt.provider_id, &subject, "source_ids")
                .unwrap()
                .state,
            CollectionState::CompleteEmpty(_)
        ));
        conn.execute("UPDATE metadata_collection_heads SET snapshot_id=NULL,accepted_generation=0,accepted_observation_id=NULL,accepted_scope_ordinal=NULL,accepted_fetched_at_us=NULL,accepted_occurrence_metadata_json=NULL",[]).unwrap();
        let state = read_track_provider_state(&conn, Some(&spec)).unwrap();
        assert_eq!(state.binding, ProviderBinding::Proven);
        assert!(state.collections.iter().all(|collection| matches!(
            collection.state,
            CollectionState::NoSnapshot
        ) && collection.refresh.is_some()));
        conn.execute("DELETE FROM metadata_collection_heads", [])
            .unwrap();
        let missing =
            read_provider_collection(&conn, receipt.provider_id, &subject, "source_ids").unwrap();
        assert!(matches!(missing.state, CollectionState::NoSnapshot));
        assert!(missing.refresh.is_none());
    }
}
