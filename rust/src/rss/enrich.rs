use std::io::Cursor;

use anyhow::{Context, Result};
use rss::extension::{Extension, ExtensionMap};
use rss::{Channel, Item};

use super::helpers::find_ext_attr;
use crate::api::{Feed, SourceEntityId, SourceEntityLink, Track};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RssTrackEnrichment {
    pub transcript_url: Option<String>,
    pub transcript_type: Option<String>,
    pub track_nostr: Option<String>,
    pub feed_nostr: Option<String>,
}

pub fn enrich_track_from_feed_rss(
    track: &mut Track,
    feed: Option<&mut Feed>,
    feed_url: &str,
) -> Result<bool> {
    let enrichment = fetch_track_enrichment_from_feed(
        feed_url,
        track.track_guid.as_deref(),
        track.enclosure_url.as_deref(),
    )?;
    let Some(enrichment) = enrichment else {
        return Ok(false);
    };

    let mut changed = false;
    if let Some(transcript_url) = enrichment.transcript_url {
        changed |= append_track_source_link(
            track,
            "transcript",
            &transcript_url,
            "podcast:transcript@url",
        );
    }
    if let Some(nostr) = enrichment.track_nostr {
        changed |= append_track_source_id(track, "nostr_npub", &nostr, "podcast:txt@purpose=nostr");
    }
    if let (Some(feed), Some(nostr)) = (feed, enrichment.feed_nostr) {
        changed |= append_feed_source_id(feed, "nostr_npub", &nostr, "podcast:txt@purpose=nostr");
    }

    Ok(changed)
}

pub fn fetch_track_enrichment_from_feed(
    feed_url: &str,
    track_guid: Option<&str>,
    enclosure_url: Option<&str>,
) -> Result<Option<RssTrackEnrichment>> {
    let body = reqwest::blocking::Client::new()
        .get(feed_url)
        .send()
        .with_context(|| format!("GET {feed_url}"))?
        .error_for_status()
        .with_context(|| format!("HTTP error for {feed_url}"))?
        .bytes()
        .with_context(|| format!("read body {feed_url}"))?;
    let feed = Channel::read_from(Cursor::new(body)).context("parse RSS")?;
    let Some(item) = feed
        .items()
        .iter()
        .find(|item| rss_item_matches_track(item, track_guid, enclosure_url))
    else {
        return Ok(None);
    };

    Ok(Some(RssTrackEnrichment {
        transcript_url: find_ext_attr(item.extensions(), "podcast", "transcript", "url"),
        transcript_type: find_ext_attr(item.extensions(), "podcast", "transcript", "type"),
        track_nostr: nostr_from_extensions(item.extensions()),
        feed_nostr: nostr_from_extensions(feed.extensions()),
    }))
}

fn rss_item_matches_track(
    item: &Item,
    track_guid: Option<&str>,
    enclosure_url: Option<&str>,
) -> bool {
    let guid_matches = track_guid
        .zip(item.guid().map(|guid| guid.value()))
        .is_some_and(|(expected, actual)| expected == actual);
    let enclosure_matches = enclosure_url
        .zip(item.enclosure().map(|enclosure| enclosure.url()))
        .is_some_and(|(expected, actual)| expected == actual);
    guid_matches || enclosure_matches
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
    extraction_path: &str,
) -> bool {
    let ids = track.source_ids.get_or_insert_with(Vec::new);
    if ids
        .iter()
        .any(|id| id.scheme.as_deref() == Some(scheme) && id.value.as_deref() == Some(value))
    {
        return false;
    }
    ids.push(SourceEntityId {
        entity_type: Some("track".into()),
        entity_id: track.track_guid.clone(),
        position: Some(ids.len() as i64),
        scheme: Some(scheme.into()),
        value: Some(value.into()),
        source: Some("rss".into()),
        extraction_path: Some(extraction_path.into()),
        observed_at: None,
    });
    true
}

fn append_feed_source_id(
    feed: &mut Feed,
    scheme: &str,
    value: &str,
    extraction_path: &str,
) -> bool {
    let ids = feed.source_ids.get_or_insert_with(Vec::new);
    if ids
        .iter()
        .any(|id| id.scheme.as_deref() == Some(scheme) && id.value.as_deref() == Some(value))
    {
        return false;
    }
    ids.push(SourceEntityId {
        entity_type: Some("feed".into()),
        entity_id: feed.feed_guid.clone(),
        position: Some(ids.len() as i64),
        scheme: Some(scheme.into()),
        value: Some(value.into()),
        source: Some("rss".into()),
        extraction_path: Some(extraction_path.into()),
        observed_at: None,
    });
    true
}

fn nostr_from_extensions(exts: &ExtensionMap) -> Option<String> {
    exts.get("podcast")?
        .values()
        .flat_map(|extensions| extensions.iter())
        .find_map(nostr_from_extension)
}

fn nostr_from_extension(ext: &Extension) -> Option<String> {
    let looks_like_nostr = ext.attrs.iter().any(|(key, value)| {
        let key = key.to_ascii_lowercase();
        let value = value.to_ascii_lowercase();
        (key.contains("purpose") || key.contains("type") || key.contains("name"))
            && value.contains("nostr")
    });
    if looks_like_nostr {
        if let Some(value) = ext.value.as_deref().and_then(extract_nostr_handle) {
            return Some(value);
        }
        for attr in ["value", "url", "href"] {
            if let Some(value) = ext
                .attrs
                .get(attr)
                .and_then(|value| extract_nostr_handle(value))
            {
                return Some(value);
            }
        }
    }

    ext.value
        .as_deref()
        .and_then(extract_nostr_handle)
        .or_else(|| {
            ext.attrs
                .values()
                .find_map(|value| extract_nostr_handle(value))
        })
        .or_else(|| {
            ext.children
                .values()
                .flat_map(|children| children.iter())
                .find_map(nostr_from_extension)
        })
}

fn extract_nostr_handle(value: &str) -> Option<String> {
    for prefix in ["npub1", "nprofile1"] {
        if let Some(start) = value.find(prefix) {
            let handle = value[start..]
                .chars()
                .take_while(char::is_ascii_alphanumeric)
                .collect::<String>();
            if handle.len() > prefix.len() {
                return Some(handle);
            }
        }
    }

    value
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '"' | '\'' | '<' | '>'))
        .map(|part| part.trim_matches(|ch: char| matches!(ch, ':' | '/' | ')' | '(')))
        .find(|part| part.starts_with("npub1") || part.starts_with("nprofile1"))
        .map(ToOwned::to_owned)
}
