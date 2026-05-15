use std::io::Cursor;

use anyhow::{Context, Result};
use rss::extension::{Extension, ExtensionMap};
use rss::{Channel, Item};

use super::helpers::{
    clean_text, find_ext, find_ext_attr, find_ext_text, first_person_by_role, parse_itunes_duration,
};
use crate::api::{Feed, SourceEntityId, SourceEntityLink, Track};
use crate::metadata::source_text_missing;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PodrollEntry {
    pub feed_guid: Option<String>,
    pub feed_url: Option<String>,
}

pub fn fetch_feed_podroll(feed_url: &str) -> Result<Vec<PodrollEntry>> {
    let body = reqwest::blocking::Client::new()
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
    pub track_nostr: Option<String>,
    pub feed_nostr: Option<String>,
}

pub fn enrich_track_from_feed_rss(
    track: &mut Track,
    mut feed: Option<&mut Feed>,
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
    changed |= apply_track_enrichment(track, feed.as_deref_mut(), &enrichment);
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
        feed_title: clean_text(Some(feed.title())),
        feed_description: clean_text(Some(feed.description())),
        feed_artist: feed
            .itunes_ext()
            .and_then(|itunes| clean_text(itunes.author())),
        feed_image_url: feed
            .itunes_ext()
            .and_then(|itunes| itunes.image())
            .and_then(|value| clean_text(Some(value)))
            .or_else(|| feed.image().and_then(|image| clean_text(Some(image.url())))),
        feed_episode_count: feed.items().len().try_into().ok(),
        track_title: item.title().and_then(|value| clean_text(Some(value))),
        track_description: item.description().and_then(|value| clean_text(Some(value))),
        track_artist: item
            .itunes_ext()
            .and_then(|itunes| clean_text(itunes.author()))
            .or_else(|| clean_text(item.author()))
            .or_else(|| {
                first_person_by_role(
                    item.extensions(),
                    &["artist", "creator", "composer", "performer"],
                )
            }),
        track_image_url: item
            .itunes_ext()
            .and_then(|itunes| itunes.image())
            .and_then(|value| clean_text(Some(value))),
        track_number: find_ext_text(item.extensions(), "podcast", "episode")
            .and_then(|value| value.trim().parse::<i32>().ok()),
        duration_secs: item
            .itunes_ext()
            .and_then(|itunes| itunes.duration())
            .and_then(parse_itunes_duration)
            .and_then(|value| value.try_into().ok()),
        pub_date: item.pub_date().and_then(parse_rss_pub_date),
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
