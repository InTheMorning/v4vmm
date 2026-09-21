//! Raw MusicIndex property evidence for ADR 0075. No completeness contract is inferred.

use serde_json::{json, Value};

use super::{
    CoverageEvidence, FactEvidence, ObservationOutcome, ObservationRetention, PropertyPresence,
    ProviderObservation, ProviderRequestSpec, SubjectKey,
};

const COLLECTIONS: &[&str] = &[
    "tracks",
    "source_contributors",
    "source_links",
    "source_ids",
    "source_release_claims",
    "source_enclosures",
    "source_transcripts",
    "source_platforms",
    "remote_items",
    "publisher",
    "value_time_splits",
    "payment_routes",
];
const SCALARS: &[&str] = &[
    "feed_guid",
    "track_guid",
    "created_at",
    "updated_at",
    "title",
    "name",
    "feed_url",
    "feed_title",
    "release_artist",
    "release_artist_sort",
    "raw_medium",
    "release_kind",
    "release_date",
    "publisher_text",
    "language",
    "explicit",
    "episode_count",
    "newest_item_at",
    "oldest_item_at",
    "description",
    "image_url",
    "duration_secs",
    "pub_date",
    "track_number",
    "enclosure_url",
    "enclosure_type",
    "enclosure_bytes",
    "track_artist",
    "track_artist_sort",
    "artist_credit",
];

pub(crate) fn extract(
    observation: &mut ProviderObservation,
    spec: &ProviderRequestSpec,
    text: &str,
) {
    let parsed = serde_json::from_str::<Value>(text);
    let data = parsed.as_ref().ok().and_then(|v| v.get("data"));
    let reason = if parsed.is_err() {
        "raw_extraction_incomplete"
    } else {
        "no_registered_completeness_contract"
    };
    let owner = data.map_or(
        Value::Null,
        |v| json!({"feed_guid":v.get("feed_guid"),"track_guid":v.get("track_guid")}),
    );
    let target = data.and_then(declared_target).filter(|target| {
        let path = &spec.requested_parameters["path"];
        let track_request = path
            .as_array()
            .is_some_and(|parts| parts.iter().any(|p| p == "tracks"));
        (!track_request || target.kind == "track")
            && spec
                .requested_subject
                .as_ref()
                .is_none_or(|requested| requested == target)
    });
    let includes = spec.profile["query"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|p| p[0] == "include")
        .and_then(|p| p[1].as_str())
        .unwrap_or("");
    for (field, collection) in COLLECTIONS
        .iter()
        .map(|s| (*s, true))
        .chain(SCALARS.iter().map(|s| (*s, false)))
    {
        let value = data.and_then(|v| v.get(field));
        let token = if collection {
            field.into()
        } else {
            format!("field:{field}")
        };
        let mut coverage = CoverageEvidence {
            proof: None,
            collection: token,
            target: if collection { target.clone() } else { None },
            target_owner: owner.clone(),
            request_intent: if !collection {
                "implicit"
            } else if includes.split(',').any(|v| v == field) {
                "requested"
            } else {
                "not_requested"
            }
            .into(),
            presence: if field == "publisher" {
                match value {
                    Some(Value::Object(v)) => {
                        if v.is_empty() {
                            PropertyPresence::Empty
                        } else {
                            PropertyPresence::Populated
                        }
                    }
                    None => PropertyPresence::Missing,
                    Some(Value::Null) => PropertyPresence::Null,
                    Some(_) => PropertyPresence::Invalid,
                }
            } else {
                PropertyPresence::from_json(value, collection)
            },
            retention: if observation.outcome == ObservationOutcome::Failed {
                ObservationRetention::Failed
            } else if value.is_some() {
                ObservationRetention::Partial
            } else {
                ObservationRetention::Unknown
            },
            basis: json!({"reason":reason}),
            facts: Vec::new(),
        };
        if let Some(value) = value {
            let members = if collection {
                value
                    .as_array()
                    .map_or_else(|| vec![value], |v| v.iter().collect())
            } else {
                vec![value]
            };
            for (ordinal, member) in members.into_iter().enumerate() {
                let declared_owner = if collection {
                    json!({"entity_type":member.get("entity_type"),"entity_id":member.get("entity_id"),"feed_guid":member.get("feed_guid")})
                } else {
                    json!({"container_identity": owner, "declared_owner": null})
                };
                let subject = if collection {
                    member_subject(member, target.as_ref())
                } else {
                    None
                };
                coverage.facts.push(FactEvidence { owner_basis:json!({"basis":if subject.is_some(){"declared_response_owner"}else{"unresolved"}}),subject,declared_owner,kind:field.into(),assertion_source:member.get("source").and_then(Value::as_str).map(str::to_owned),source_position:member.get("position").and_then(Value::as_i64),extraction_path:member.get("extraction_path").and_then(Value::as_str).map(str::to_owned),source_observed:member.get("observed_at").cloned(),representation:if member.is_string(){"plain_text"}else{"structured"}.into(),validation:"unresolved".into(),value:member.clone(),raw_member:Some(member.clone()),body_locator:json!({"json_pointer":if collection && value.is_array() {format!("/data/{field}/{ordinal}")}else{format!("/data/{field}")}}) });
            }
        }
        observation.coverage.push(coverage);
    }
    observation.source_times = json!({"created_at":data.and_then(|v|v.get("created_at")),"updated_at":data.and_then(|v|v.get("updated_at"))});
    observation.source_revision = data.and_then(|v| v.get("revision")).cloned();
}
fn declared_target(value: &Value) -> Option<SubjectKey> {
    let feed = value.get("feed_guid")?.as_str().filter(|s| !s.is_empty())?;
    let item = match value.get("track_guid") {
        None => None,
        Some(value) => Some(value.as_str().filter(|s| !s.is_empty())?),
    };
    Some(SubjectKey::guid(feed, item))
}
fn member_subject(member: &Value, envelope: Option<&SubjectKey>) -> Option<SubjectKey> {
    let id = member
        .get("entity_id")?
        .as_str()
        .filter(|s| !s.is_empty())?;
    match member.get("entity_type")?.as_str()? {
        "feed" => Some(SubjectKey::guid(id, None)),
        "track" => {
            let envelope = envelope?;
            if envelope.kind != "track" || envelope.item_guid.as_deref() != Some(id) {
                return None;
            }
            if member
                .get("feed_guid")
                .is_some_and(|feed| feed.as_str() != Some(envelope.scope.as_str()))
            {
                return None;
            }
            Some(envelope.clone())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_observation::ProviderKind;
    #[test]
    fn adr_0075_observation_declared_ownership_separates_feeds_and_rejects_contradictions() {
        let one = declared_target(&json!({"feed_guid":"one","track_guid":"same"})).unwrap();
        let two = declared_target(&json!({"feed_guid":"two","track_guid":"same"})).unwrap();
        assert_ne!(one, two);
        assert!(declared_target(&json!({"track_guid":"same"})).is_none());
        for value in [json!(""), json!(null), json!(false)] {
            assert!(declared_target(&json!({"feed_guid":"one","track_guid":value})).is_none());
        }
        assert!(member_subject(
            &json!({"entity_type":"track","entity_id":"different"}),
            Some(&one)
        )
        .is_none());
        assert!(member_subject(
            &json!({"entity_type":"track","entity_id":"same","feed_guid":"two"}),
            Some(&one)
        )
        .is_none());
        assert!(member_subject(&json!({"entity_type":"track","entity_id":"same"}), None).is_none());
        assert_eq!(
            member_subject(&json!({"entity_type":"feed","entity_id":"two"}), Some(&one)),
            Some(SubjectKey::guid("two", None))
        );
        let spec = ProviderRequestSpec {
            provider: ProviderKind::MusicIndex,
            provider_identity: "x".into(),
            request_uri: "y".into(),
            requested_subject: Some(one),
            requested_parameters: json!({"path":["v1","feeds","one","tracks","same"]}),
            profile: json!({}),
            started_at_us: 0,
        };
        let mut observation = ProviderObservation {
            body: None,
            http_status: None,
            response_uri: None,
            interpretation: json!({}),
            source_revision: None,
            source_times: json!({}),
            decoder_version: "test".into(),
            outcome: ObservationOutcome::Success,
            failure: None,
            finished_at_us: 0,
            fetched_at_us: None,
            occurrence: json!({}),
            coverage: Vec::new(),
        };
        extract(
            &mut observation,
            &spec,
            r#"{"data":{"feed_guid":"two","track_guid":"same"}}"#,
        );
        assert!(observation.coverage.iter().all(|c| c.target.is_none()));
        observation.coverage.clear();
        extract(
            &mut observation,
            &spec,
            r#"{"data":{"feed_guid":"one","track_guid":"same","image_url":"image","release_artist":"artist","publisher_text":"publisher","language":"en","explicit":false,"publisher":{"remote_feed_guid":"publisher-feed"},"source_ids":false}}"#,
        );
        for field in [
            "image_url",
            "release_artist",
            "publisher_text",
            "language",
            "explicit",
        ] {
            let coverage = observation
                .coverage
                .iter()
                .find(|c| c.collection == format!("field:{field}"))
                .unwrap();
            assert!(coverage.target.is_none());
            assert!(coverage.facts[0].subject.is_none());
            assert_eq!(coverage.facts[0].owner_basis["basis"], "unresolved");
            assert_eq!(
                coverage.facts[0].declared_owner["container_identity"]["track_guid"],
                "same"
            );
        }
        let publisher = observation
            .coverage
            .iter()
            .find(|c| c.collection == "publisher")
            .unwrap();
        assert_eq!(publisher.presence, PropertyPresence::Populated);
        assert_eq!(
            publisher.facts[0].body_locator["json_pointer"],
            "/data/publisher"
        );
        for value in [json!([]), json!([{}]), json!(false)] {
            observation.coverage.clear();
            extract(
                &mut observation,
                &spec,
                &json!({"data":{"publisher":value,"source_ids":false}}).to_string(),
            );
            assert_eq!(
                observation
                    .coverage
                    .iter()
                    .find(|c| c.collection == "publisher")
                    .unwrap()
                    .presence,
                PropertyPresence::Invalid
            );
        }
        let invalid = observation
            .coverage
            .iter()
            .find(|c| c.collection == "source_ids")
            .unwrap();
        assert_eq!(invalid.presence, PropertyPresence::Invalid);
        assert_eq!(
            invalid.facts[0].body_locator["json_pointer"],
            "/data/source_ids"
        );
    }
}
