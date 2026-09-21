//! Verified direct RSS collection admission for ADR 0075.

use anyhow::{anyhow, ensure, Context, Result};
use quick_xml::{events::Event, Reader};
use roxmltree::Node;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{
    CoverageEvidence, ObservationOutcome, ProviderKind, ProviderObservation, ProviderRequestSpec,
    SubjectKey,
};

pub(crate) const RSS_DECODER: &str = "rss-dom-v2";
const RSS_CONTRACT: &str = "rss-direct-source-ids-v1";
const PODCAST: [&str; 2] = [
    "https://podcastindex.org/namespace/1.0",
    "https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/1.0.md",
];
const RSS1: &str = "http://purl.org/rss/1.0/";

/// The decode boundary binds XML text to retained response bytes.
pub(crate) struct DecodedRssBody {
    text: String,
    encoding: String,
    body_key: String,
}
impl DecodedRssBody {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }
    pub(crate) fn encoding(&self) -> &str {
        &self.encoding
    }
}

pub(crate) fn decode_rss_body(
    bytes: &[u8],
    retained_encoding: Option<&mut Value>,
) -> Result<DecodedRssBody> {
    let mut reader = Reader::from_reader(bytes);
    loop {
        match reader
            .read_event()
            .map_err(|_| anyhow!("RSS declaration is invalid"))?
        {
            Event::DocType(_) => return Err(anyhow!("RSS document contains a DTD")),
            Event::Start(_) | Event::Empty(_) | Event::Eof => break,
            _ => {}
        }
    }
    let decoder = reader.decoder();
    let encoding = decoder.encoding().name().to_owned();
    if let Some(retained_encoding) = retained_encoding {
        *retained_encoding = json!(encoding);
    }
    Ok(DecodedRssBody {
        text: decoder
            .decode(bytes)
            .context("decode RSS document")?
            .into_owned(),
        encoding,
        body_key: format!("{:x}", Sha256::digest(bytes)),
    })
}

/// Only this registry can construct a replacement proof.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct VerifiedCoverage {
    binding: Value,
}

impl std::fmt::Debug for DecodedRssBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecodedRssBody").finish_non_exhaustive()
    }
}
impl std::fmt::Debug for VerifiedCoverage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifiedCoverage").finish_non_exhaustive()
    }
}

pub(crate) fn rss_element_value(node: Node<'_, '_>) -> Value {
    let direct_text: String = node
        .children()
        .filter(|child| child.is_text())
        .filter_map(|child| child.text())
        .collect();
    json!({"name":node.tag_name().name(),"namespace":node.tag_name().namespace(),
        "attributes":node.attributes().map(|attribute|json!({"name":attribute.name(),"namespace":attribute.namespace(),"value":attribute.value()})).collect::<Vec<_>>(),
        "direct_text":direct_text})
}
impl VerifiedCoverage {
    pub(crate) fn contract(&self) -> &'static str {
        RSS_CONTRACT
    }
    pub(crate) fn evidence(&self) -> &Value {
        &self.binding
    }
}

pub(crate) fn rss_request(
    resource: &str,
    track_guid: Option<&str>,
    enclosure: Option<&str>,
) -> ProviderRequestSpec {
    ProviderRequestSpec {
        provider: ProviderKind::Rss,
        provider_identity: resource.into(),
        request_uri: resource.into(),
        requested_subject: None,
        requested_parameters: json!({"track_guid":track_guid,"enclosure_url":enclosure}),
        profile: json!({"version":1,"operation":"rss_track_enrichment"}),
        started_at_us: 0,
    }
}

fn ordinary(node: Node<'_, '_>, name: &str) -> bool {
    node.is_element()
        && node.tag_name().name() == name
        && matches!(node.tag_name().namespace(), None | Some(RSS1))
}
fn podcast(node: Node<'_, '_>, name: &str) -> bool {
    node.is_element()
        && node.tag_name().name() == name
        && node
            .tag_name()
            .namespace()
            .is_some_and(|ns| PODCAST.contains(&ns))
}
fn raw_guid(owner: Node<'_, '_>, feed: bool) -> Result<Option<String>, ()> {
    let declarations: Vec<_> = owner
        .children()
        .filter(|node| {
            if feed {
                podcast(*node, "guid")
            } else {
                ordinary(*node, "guid")
            }
        })
        .collect();
    match declarations.as_slice() {
        [] => Ok(None),
        [node] if !node.children().any(|child| child.is_element()) => {
            let text: String = node
                .children()
                .filter(|child| child.is_text())
                .filter_map(|child| child.text())
                .collect();
            if text.trim().is_empty() {
                Err(())
            } else {
                Ok(Some(text))
            }
        }
        _ => Err(()),
    }
}

/// Resolve raw declarations without converting unusable declarations to absence.
pub(crate) fn rss_subject(owner: Node<'_, '_>, resource: &str) -> Option<SubjectKey> {
    let root = owner.document().root_element();
    let channels: Vec<_> = root
        .children()
        .filter(|node| ordinary(*node, "channel"))
        .collect();
    let [channel] = channels.as_slice() else {
        return None;
    };
    let feed = raw_guid(*channel, true).ok()?;
    let item_guid = if ordinary(owner, "item") {
        Some(raw_guid(owner, false).ok()??)
    } else if owner == *channel {
        None
    } else {
        return None;
    };
    Some(SubjectKey {
        kind: if item_guid.is_some() { "track" } else { "feed" }.into(),
        scope_kind: if feed.is_some() { "guid" } else { "resource" }.into(),
        scope: feed.unwrap_or_else(|| resource.into()),
        item_guid,
    })
}

fn binding(spec: &ProviderRequestSpec, observation: &ProviderObservation, scope: usize) -> Value {
    let coverage = &observation.coverage[scope];
    json!({
        "contract":RSS_CONTRACT,"provider":spec.provider.token(),"provider_identity":spec.provider_identity,
        "resource":spec.request_uri,"requested_subject":spec.requested_subject.as_ref().map(SubjectKey::json),
        "parameters":spec.requested_parameters,"profile":spec.profile,
        "body":observation.body.as_ref().map(|bytes|format!("{:x}",Sha256::digest(bytes))),
        "http_status":observation.http_status,"response_uri":observation.response_uri,
        "interpretation":observation.interpretation,"decoder":observation.decoder_version,
        "source_revision":observation.source_revision,"source_times":observation.source_times,"failure":observation.failure,
        "scope":scope,"coverage":coverage_value(coverage),
        "owner_rule":"one_raw_guid_or_absent_feed_guid","pagination":"complete_rss_document",
        "operation":"rss_track_enrichment","enumeration":"all_direct_podcast_txt"
    })
}

pub(crate) fn coverage_value(coverage: &CoverageEvidence) -> Value {
    json!({"collection":coverage.collection,"subject":coverage.target,"owner":coverage.target_owner,
        "intent":coverage.request_intent,"presence":coverage.presence.token(),"basis":coverage.basis,"facts":coverage.facts})
}

/// Verify one owner through the DOM already used for RSS extraction.
pub(crate) fn certify_rss(
    observation: &ProviderObservation,
    spec: &ProviderRequestSpec,
    scope: usize,
    owner: Node<'_, '_>,
    decoded: &DecodedRssBody,
) -> Option<VerifiedCoverage> {
    if spec.provider != ProviderKind::Rss
        || spec.provider_identity != spec.request_uri
        || spec.profile["operation"] != "rss_track_enrichment"
        || observation.decoder_version != RSS_DECODER
        || observation.outcome != ObservationOutcome::Success
        || !observation
            .http_status
            .is_some_and(|status| (200..300).contains(&status))
        || observation.body.is_none()
        || observation.interpretation["body_state"] != "complete"
    {
        return None;
    }
    if decoded.text() != owner.document().input_text()
        || decoded.body_key != format!("{:x}", Sha256::digest(observation.body.as_ref()?))
    {
        return None;
    }
    let root = owner.document().root_element();
    if !((root.tag_name().name() == "rss" && root.tag_name().namespace().is_none())
        || (root.tag_name().name() == "RDF"
            && root.tag_name().namespace() == Some("http://www.w3.org/1999/02/22-rdf-syntax-ns#")))
    {
        return None;
    }
    let subject = rss_subject(owner, &spec.request_uri)?;
    let root = owner.document().root_element();
    let channel = root.children().find(|node| ordinary(*node, "channel"))?;
    let items: Vec<_> = channel
        .children()
        .chain(root.children())
        .filter(|node| ordinary(*node, "item"))
        .collect();
    let wanted = spec.requested_parameters["track_guid"].as_str();
    let enclosure = spec.requested_parameters["enclosure_url"].as_str();
    let matched: Vec<_> = items
        .iter()
        .filter(|item| {
            let guid = raw_guid(**item, false).ok().flatten();
            wanted.zip(guid.as_deref()).is_some_and(|(a, b)| a == b)
                || enclosure.is_some_and(|url| {
                    item.children()
                        .any(|n| ordinary(n, "enclosure") && n.attribute("url") == Some(url))
                })
        })
        .collect();
    let [matched] = matched.as_slice() else {
        return None;
    };
    if ordinary(owner, "item") {
        if owner != **matched {
            return None;
        }
        let guid = subject.item_guid.as_deref()?;
        if wanted.is_some_and(|wanted| wanted != guid) {
            return None;
        }
        if items
            .iter()
            .filter(|item| raw_guid(**item, false).ok().flatten().as_deref() == Some(guid))
            .count()
            != 1
        {
            return None;
        }
    }
    let coverage = observation.coverage.get(scope)?;
    if coverage.collection != "source_ids" || coverage.target.as_ref() != Some(&subject) {
        return None;
    }
    let candidates: Vec<_> = owner
        .children()
        .filter(|node| podcast(*node, "txt"))
        .collect();
    if candidates.len() != coverage.facts.len()
        || coverage
            .facts
            .iter()
            .any(|fact| fact.subject.as_ref() != Some(&subject) || fact.validation != "valid")
    {
        return None;
    }
    for (node, fact) in candidates.iter().zip(&coverage.facts) {
        if fact.value != rss_element_value(*node) {
            return None;
        }
        let candidate: String = node
            .children()
            .filter(|child| child.is_text())
            .filter_map(|child| child.text())
            .collect();
        if node.children().any(|child| child.is_element())
            || node
                .attribute("purpose")
                .map(|purpose| purpose.trim().to_ascii_lowercase())
                .as_deref()
                != Some("npub")
            || !matches!(
                crate::rss::validate_nostr_identity(candidate.trim()),
                crate::rss::IdentityValidation::Valid(_)
            )
        {
            return None;
        }
        if fact.raw_member.as_ref()?["element_xml"].as_str()?
            != &owner.document().input_text()[node.range()]
        {
            return None;
        }
        if fact.body_locator["decoded_byte_offset"].as_u64()?
            != u64::try_from(node.range().start).ok()?
        {
            return None;
        }
    }
    Some(VerifiedCoverage {
        binding: binding(spec, observation, scope),
    })
}

pub(crate) fn validate(
    spec: &ProviderRequestSpec,
    observation: &ProviderObservation,
) -> Result<Option<&'static str>> {
    let mut keys = std::collections::BTreeSet::new();
    let mut contract = None;
    for (scope, coverage) in observation.coverage.iter().enumerate() {
        if let Some(proof) = &coverage.proof {
            ensure!(
                spec.provider == ProviderKind::Rss
                    && observation.decoder_version == RSS_DECODER
                    && observation.outcome == ObservationOutcome::Success,
                "Unsupported completeness contract"
            );
            ensure!(
                proof.binding == binding(spec, observation, scope),
                "Completeness proof differs"
            );
            let target = coverage
                .target
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Completeness owner is unresolved"))?;
            ensure!(
                keys.insert((target.json().to_string(), coverage.collection.clone())),
                "Duplicate complete coverage scope"
            );
            contract = Some(proof.contract());
        }
    }
    Ok(contract)
}

#[cfg(test)]
pub(crate) fn rebind_test_proofs(
    spec: &ProviderRequestSpec,
    observation: &mut ProviderObservation,
) {
    assert_eq!(spec.provider, ProviderKind::Rss);
    for scope in 0..observation.coverage.len() {
        if observation.coverage[scope].proof.is_some() {
            observation.coverage[scope].proof = Some(VerifiedCoverage {
                binding: binding(spec, observation, scope),
            });
        }
    }
}
