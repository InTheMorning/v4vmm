//! Provider snapshot schema, applied only by migration 12 (ADRs 0016, 0075).

#![warn(clippy::pedantic)]

use anyhow::{Context, Result};
use rusqlite::Connection;

pub(super) const COLUMNS: &[(&str, &[&str])] = &[
    ("metadata_providers", &["id", "kind", "identity"]),
    ("metadata_resources", &["id", "provider_id", "request_uri"]),
    (
        "metadata_subjects",
        &[
            "id",
            "subject_key",
            "kind",
            "feed_scope_kind",
            "feed_scope",
            "item_guid",
        ],
    ),
    ("metadata_bodies", &["sha256", "byte_length", "bytes"]),
    ("metadata_generation", &["singleton", "last_generation"]),
    (
        "metadata_request_slots",
        &[
            "request_key",
            "provider_id",
            "resource_id",
            "requested_subject_id",
            "requested_subject_json",
            "profile_json",
            "generation",
            "started_at_us",
            "state",
            "latest_observation_id",
            "latest_failure_id",
        ],
    ),
    (
        "metadata_observations",
        &[
            "id",
            "observation_key",
            "provider_id",
            "resource_id",
            "requested_subject_id",
            "requested_subject_json",
            "profile_json",
            "body_sha256",
            "http_status",
            "response_uri",
            "interpretation_metadata_json",
            "source_revision_json",
            "source_times_json",
            "contract_id",
            "decoder_version",
            "outcome",
            "failure_json",
            "first_generation",
            "last_generation",
            "first_started_at_us",
            "last_started_at_us",
            "first_finished_at_us",
            "last_finished_at_us",
            "first_fetched_at_us",
            "last_fetched_at_us",
            "first_occurrence_metadata_json",
            "last_occurrence_metadata_json",
            "occurrence_count",
            "superseded_count",
        ],
    ),
    (
        "metadata_coverage",
        &[
            "observation_id",
            "scope_ordinal",
            "collection",
            "target_subject_id",
            "target_owner_json",
            "request_intent",
            "presence",
            "completeness",
            "contract_id",
            "basis_json",
        ],
    ),
    (
        "metadata_facts",
        &[
            "id",
            "observation_id",
            "scope_ordinal",
            "transport_ordinal",
            "declared_subject_id",
            "declared_owner_json",
            "owner_basis_json",
            "fact_kind",
            "assertion_source",
            "source_position",
            "extraction_path",
            "source_observed_json",
            "representation",
            "validation",
            "value_json",
            "raw_member_json",
            "body_locator_json",
        ],
    ),
    (
        "metadata_snapshots",
        &[
            "id",
            "provider_id",
            "subject_id",
            "collection",
            "content_key",
            "first_observation_id",
            "first_scope_ordinal",
            "member_count",
        ],
    ),
    (
        "metadata_snapshot_members",
        &["snapshot_id", "member_ordinal", "fact_id"],
    ),
    (
        "metadata_collection_heads",
        &[
            "provider_id",
            "subject_id",
            "collection",
            "snapshot_id",
            "accepted_generation",
            "accepted_observation_id",
            "accepted_scope_ordinal",
            "accepted_fetched_at_us",
            "accepted_occurrence_metadata_json",
            "last_attempt_generation",
            "last_attempt_state",
            "last_attempt_observation_id",
            "latest_failure_id",
        ],
    ),
    (
        "metadata_field_selections",
        &[
            "subject_id",
            "field",
            "context_key",
            "context_json",
            "selected_provider_id",
            "selected_observation_id",
            "selected_scope_ordinal",
            "selected_snapshot_id",
            "selection_state",
            "value_json",
            "evidence_json",
            "policy_version",
            "selected_at_us",
        ],
    ),
    (
        "metadata_discrepancies",
        &[
            "id",
            "subject_id",
            "field",
            "rss_provider_id",
            "rss_resource_id",
            "index_provider_id",
            "index_resource_id",
            "state",
            "first_recorded_at_us",
            "last_recorded_at_us",
            "last_compared_json",
            "latest_transition_id",
        ],
    ),
    (
        "metadata_discrepancy_transitions",
        &[
            "id",
            "discrepancy_id",
            "sequence",
            "previous_state",
            "state",
            "reason",
            "comparison_version",
            "pair_content_key",
            "rss_observation_id",
            "index_observation_id",
            "rss_body_sha256",
            "index_body_sha256",
            "rss_evidence_json",
            "index_evidence_json",
            "recorded_at_us",
        ],
    ),
];

const DDL: &str = r"
CREATE TABLE metadata_providers (
    id INTEGER NOT NULL PRIMARY KEY,
    kind TEXT NOT NULL,
    identity TEXT NOT NULL,
    UNIQUE(kind, identity),
    CHECK(kind IN ('musicindex','rss')),
    CHECK(identity != '')
);

CREATE TABLE metadata_resources (
    id INTEGER NOT NULL PRIMARY KEY,
    provider_id INTEGER NOT NULL REFERENCES metadata_providers(id) ON DELETE RESTRICT,
    request_uri TEXT NOT NULL,
    UNIQUE(provider_id, request_uri),
    CHECK(request_uri != '')
);

CREATE TABLE metadata_subjects (
    id INTEGER NOT NULL PRIMARY KEY,
    subject_key TEXT NOT NULL CHECK(json_valid(subject_key)),
    kind TEXT NOT NULL,
    feed_scope_kind TEXT NOT NULL,
    feed_scope TEXT NOT NULL,
    item_guid TEXT,
    UNIQUE(subject_key),
    CHECK(kind IN ('feed','track')),
    CHECK(feed_scope_kind IN ('guid','resource')),
    CHECK(feed_scope != ''),
    CHECK((kind='feed' AND item_guid IS NULL) OR (kind='track' AND item_guid IS NOT NULL AND item_guid != ''))
);

CREATE TABLE metadata_bodies (
    sha256 TEXT NOT NULL PRIMARY KEY,
    byte_length INTEGER NOT NULL,
    bytes BLOB NOT NULL,
    CHECK(length(sha256)=64 AND sha256 NOT GLOB '*[^0-9a-f]*'),
    CHECK(byte_length >= 0 AND byte_length=length(bytes))
);

CREATE TABLE metadata_generation (
    singleton INTEGER NOT NULL PRIMARY KEY,
    last_generation INTEGER NOT NULL,
    CHECK(singleton=1),
    CHECK(last_generation>=0)
);

CREATE TABLE metadata_request_slots (
    request_key TEXT NOT NULL PRIMARY KEY,
    provider_id INTEGER NOT NULL REFERENCES metadata_providers(id) ON DELETE RESTRICT,
    resource_id INTEGER NOT NULL REFERENCES metadata_resources(id) ON DELETE RESTRICT,
    requested_subject_id INTEGER REFERENCES metadata_subjects(id) ON DELETE RESTRICT,
    requested_subject_json TEXT NOT NULL CHECK(json_valid(requested_subject_json)),
    profile_json TEXT NOT NULL CHECK(json_valid(profile_json)),
    generation INTEGER NOT NULL,
    started_at_us INTEGER NOT NULL,
    state TEXT NOT NULL,
    latest_observation_id INTEGER REFERENCES metadata_observations(id) ON DELETE RESTRICT,
    latest_failure_id INTEGER REFERENCES metadata_observations(id) ON DELETE RESTRICT,
    CHECK(generation>0),
    CHECK(state IN ('pending','success','partial','failed','abandoned'))
);

CREATE TABLE metadata_observations (
    id INTEGER NOT NULL PRIMARY KEY,
    observation_key TEXT NOT NULL,
    provider_id INTEGER NOT NULL REFERENCES metadata_providers(id) ON DELETE RESTRICT,
    resource_id INTEGER NOT NULL REFERENCES metadata_resources(id) ON DELETE RESTRICT,
    requested_subject_id INTEGER REFERENCES metadata_subjects(id) ON DELETE RESTRICT,
    requested_subject_json TEXT NOT NULL CHECK(json_valid(requested_subject_json)),
    profile_json TEXT NOT NULL CHECK(json_valid(profile_json)),
    body_sha256 TEXT REFERENCES metadata_bodies(sha256) ON DELETE RESTRICT,
    http_status INTEGER,
    response_uri TEXT,
    interpretation_metadata_json TEXT NOT NULL CHECK(json_valid(interpretation_metadata_json)),
    source_revision_json TEXT CHECK(source_revision_json IS NULL OR json_valid(source_revision_json)),
    source_times_json TEXT NOT NULL CHECK(json_valid(source_times_json)),
    contract_id TEXT,
    decoder_version TEXT NOT NULL,
    outcome TEXT NOT NULL,
    failure_json TEXT CHECK(failure_json IS NULL OR json_valid(failure_json)),
    first_generation INTEGER NOT NULL,
    last_generation INTEGER NOT NULL,
    first_started_at_us INTEGER NOT NULL,
    last_started_at_us INTEGER NOT NULL,
    first_finished_at_us INTEGER NOT NULL,
    last_finished_at_us INTEGER NOT NULL,
    first_fetched_at_us INTEGER,
    last_fetched_at_us INTEGER,
    first_occurrence_metadata_json TEXT NOT NULL CHECK(json_valid(first_occurrence_metadata_json)),
    last_occurrence_metadata_json TEXT NOT NULL CHECK(json_valid(last_occurrence_metadata_json)),
    occurrence_count INTEGER NOT NULL,
    superseded_count INTEGER NOT NULL,
    UNIQUE(observation_key),
    CHECK(outcome IN ('success','partial','failed')),
    CHECK(first_generation>0 AND last_generation>0),
    CHECK(occurrence_count>0 AND superseded_count>=0)
);

CREATE TABLE metadata_coverage (
    observation_id INTEGER NOT NULL REFERENCES metadata_observations(id) ON DELETE RESTRICT,
    scope_ordinal INTEGER NOT NULL,
    collection TEXT NOT NULL,
    target_subject_id INTEGER REFERENCES metadata_subjects(id) ON DELETE RESTRICT,
    target_owner_json TEXT NOT NULL CHECK(json_valid(target_owner_json)),
    request_intent TEXT NOT NULL,
    presence TEXT NOT NULL,
    completeness TEXT NOT NULL,
    contract_id TEXT,
    basis_json TEXT NOT NULL CHECK(json_valid(basis_json)),
    PRIMARY KEY(observation_id, scope_ordinal),
    CHECK(scope_ordinal>=0),
    CHECK(request_intent IN ('requested','implicit','not_requested')),
    CHECK(presence IN ('missing','null','empty','populated','invalid')),
    CHECK(completeness IN ('complete','partial','unknown','failed'))
);

CREATE TABLE metadata_facts (
    id INTEGER NOT NULL PRIMARY KEY,
    observation_id INTEGER NOT NULL REFERENCES metadata_observations(id) ON DELETE RESTRICT,
    scope_ordinal INTEGER NOT NULL,
    transport_ordinal INTEGER NOT NULL,
    declared_subject_id INTEGER REFERENCES metadata_subjects(id) ON DELETE RESTRICT,
    declared_owner_json TEXT NOT NULL CHECK(json_valid(declared_owner_json)),
    owner_basis_json TEXT NOT NULL CHECK(json_valid(owner_basis_json)),
    fact_kind TEXT NOT NULL,
    assertion_source TEXT,
    source_position INTEGER,
    extraction_path TEXT,
    source_observed_json TEXT CHECK(source_observed_json IS NULL OR json_valid(source_observed_json)),
    representation TEXT NOT NULL,
    validation TEXT NOT NULL,
    value_json TEXT NOT NULL CHECK(json_valid(value_json)),
    raw_member_json TEXT CHECK(raw_member_json IS NULL OR json_valid(raw_member_json)),
    body_locator_json TEXT NOT NULL CHECK(json_valid(body_locator_json)),
    UNIQUE(observation_id, scope_ordinal, transport_ordinal),
    CHECK(scope_ordinal>=0 AND transport_ordinal>=0),
    CHECK(representation IN ('plain_text','html','structured','unknown')),
    CHECK(validation IN ('valid','unsupported','malformed','unresolved')),
    FOREIGN KEY(observation_id, scope_ordinal) REFERENCES metadata_coverage(observation_id, scope_ordinal) ON DELETE RESTRICT
);

CREATE TABLE metadata_snapshots (
    id INTEGER NOT NULL PRIMARY KEY,
    provider_id INTEGER NOT NULL REFERENCES metadata_providers(id) ON DELETE RESTRICT,
    subject_id INTEGER NOT NULL REFERENCES metadata_subjects(id) ON DELETE RESTRICT,
    collection TEXT NOT NULL,
    content_key TEXT NOT NULL,
    first_observation_id INTEGER NOT NULL,
    first_scope_ordinal INTEGER NOT NULL,
    member_count INTEGER NOT NULL,
    UNIQUE(provider_id, subject_id, collection, content_key),
    CHECK(member_count>=0),
    FOREIGN KEY(first_observation_id, first_scope_ordinal) REFERENCES metadata_coverage(observation_id, scope_ordinal) ON DELETE RESTRICT
);

CREATE TABLE metadata_snapshot_members (
    snapshot_id INTEGER NOT NULL REFERENCES metadata_snapshots(id) ON DELETE RESTRICT,
    member_ordinal INTEGER NOT NULL,
    fact_id INTEGER NOT NULL REFERENCES metadata_facts(id) ON DELETE RESTRICT,
    PRIMARY KEY(snapshot_id, member_ordinal),
    UNIQUE(snapshot_id, fact_id),
    CHECK(member_ordinal>=0)
);

CREATE TABLE metadata_collection_heads (
    provider_id INTEGER NOT NULL REFERENCES metadata_providers(id) ON DELETE RESTRICT,
    subject_id INTEGER NOT NULL REFERENCES metadata_subjects(id) ON DELETE RESTRICT,
    collection TEXT NOT NULL,
    snapshot_id INTEGER REFERENCES metadata_snapshots(id) ON DELETE RESTRICT,
    accepted_generation INTEGER NOT NULL,
    accepted_observation_id INTEGER REFERENCES metadata_observations(id) ON DELETE RESTRICT,
    accepted_scope_ordinal INTEGER,
    accepted_fetched_at_us INTEGER,
    accepted_occurrence_metadata_json TEXT CHECK(accepted_occurrence_metadata_json IS NULL OR json_valid(accepted_occurrence_metadata_json)),
    last_attempt_generation INTEGER NOT NULL,
    last_attempt_state TEXT NOT NULL,
    last_attempt_observation_id INTEGER REFERENCES metadata_observations(id) ON DELETE RESTRICT,
    latest_failure_id INTEGER REFERENCES metadata_observations(id) ON DELETE RESTRICT,
    PRIMARY KEY(provider_id, subject_id, collection),
    CHECK(accepted_generation>=0 AND last_attempt_generation>=0),
    CHECK(last_attempt_state IN ('none','pending','success','partial','failed','superseded','abandoned')),
    FOREIGN KEY(accepted_observation_id, accepted_scope_ordinal) REFERENCES metadata_coverage(observation_id, scope_ordinal) ON DELETE RESTRICT
);

CREATE TABLE metadata_field_selections (
    subject_id INTEGER NOT NULL REFERENCES metadata_subjects(id) ON DELETE RESTRICT,
    field TEXT NOT NULL,
    context_key TEXT NOT NULL,
    context_json TEXT NOT NULL CHECK(json_valid(context_json)),
    selected_provider_id INTEGER NOT NULL REFERENCES metadata_providers(id) ON DELETE RESTRICT,
    selected_observation_id INTEGER NOT NULL,
    selected_scope_ordinal INTEGER NOT NULL,
    selected_snapshot_id INTEGER NOT NULL REFERENCES metadata_snapshots(id) ON DELETE RESTRICT,
    selection_state TEXT NOT NULL,
    value_json TEXT CHECK(value_json IS NULL OR json_valid(value_json)),
    evidence_json TEXT NOT NULL CHECK(json_valid(evidence_json)),
    policy_version TEXT NOT NULL,
    selected_at_us INTEGER NOT NULL,
    PRIMARY KEY(subject_id, field, context_key),
    CHECK(field != ''),
    CHECK(selection_state IN ('value','absent')),
    CHECK((selection_state='value' AND value_json IS NOT NULL) OR (selection_state='absent' AND value_json IS NULL)),
    FOREIGN KEY(selected_observation_id, selected_scope_ordinal) REFERENCES metadata_coverage(observation_id, scope_ordinal) ON DELETE RESTRICT
);

CREATE TABLE metadata_discrepancies (
    id INTEGER NOT NULL PRIMARY KEY,
    subject_id INTEGER NOT NULL REFERENCES metadata_subjects(id) ON DELETE RESTRICT,
    field TEXT NOT NULL,
    rss_provider_id INTEGER NOT NULL REFERENCES metadata_providers(id) ON DELETE RESTRICT,
    rss_resource_id INTEGER NOT NULL REFERENCES metadata_resources(id) ON DELETE RESTRICT,
    index_provider_id INTEGER NOT NULL REFERENCES metadata_providers(id) ON DELETE RESTRICT,
    index_resource_id INTEGER NOT NULL REFERENCES metadata_resources(id) ON DELETE RESTRICT,
    state TEXT NOT NULL,
    first_recorded_at_us INTEGER NOT NULL,
    last_recorded_at_us INTEGER NOT NULL,
    last_compared_json TEXT NOT NULL CHECK(json_valid(last_compared_json)),
    latest_transition_id INTEGER REFERENCES metadata_discrepancy_transitions(id) ON DELETE RESTRICT,
    UNIQUE(subject_id, field, rss_provider_id, rss_resource_id, index_provider_id, index_resource_id),
    CHECK(state IN ('active','resolved')),
    CHECK(field IN ('description','website_page'))
);

CREATE TABLE metadata_discrepancy_transitions (
    id INTEGER NOT NULL PRIMARY KEY,
    discrepancy_id INTEGER NOT NULL REFERENCES metadata_discrepancies(id) ON DELETE RESTRICT,
    sequence INTEGER NOT NULL,
    previous_state TEXT,
    state TEXT NOT NULL,
    reason TEXT NOT NULL,
    comparison_version TEXT NOT NULL,
    pair_content_key TEXT NOT NULL,
    rss_observation_id INTEGER NOT NULL REFERENCES metadata_observations(id) ON DELETE RESTRICT,
    index_observation_id INTEGER NOT NULL REFERENCES metadata_observations(id) ON DELETE RESTRICT,
    rss_body_sha256 TEXT REFERENCES metadata_bodies(sha256) ON DELETE RESTRICT,
    index_body_sha256 TEXT REFERENCES metadata_bodies(sha256) ON DELETE RESTRICT,
    rss_evidence_json TEXT NOT NULL CHECK(json_valid(rss_evidence_json)),
    index_evidence_json TEXT NOT NULL CHECK(json_valid(index_evidence_json)),
    recorded_at_us INTEGER NOT NULL,
    UNIQUE(discrepancy_id, sequence),
    CHECK(sequence>0),
    CHECK(previous_state IS NULL OR previous_state IN ('active','resolved')),
    CHECK(state IN ('active','resolved')),
    CHECK(reason IN ('mismatch','changed_mismatch','resolved','reactivated'))
);
INSERT INTO metadata_generation(singleton, last_generation) VALUES(1, 0);
";

pub(super) fn apply(conn: &Connection) -> Result<()> {
    conn.execute_batch(DDL)
        .context("Create provider metadata snapshot schema")
}

#[cfg(test)]
pub(super) const RETAINED_ROWS: &str = r#"
INSERT INTO metadata_providers VALUES(1,'rss','https://rss.invalid/feed'),(2,'musicindex','https://index.invalid');
INSERT INTO metadata_resources VALUES(1,1,'https://rss.invalid/feed'),(2,2,'https://index.invalid/track'),(3,2,'https://index.invalid/other');
INSERT INTO metadata_subjects VALUES(1,'[1,"track","guid","feed","track"]','track','guid','feed','track');
INSERT INTO metadata_bodies VALUES('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',3,x'00ff42');
INSERT INTO metadata_observations(id, observation_key, provider_id, resource_id,
 requested_subject_id, requested_subject_json, profile_json, body_sha256,
 interpretation_metadata_json, source_times_json, decoder_version, outcome,
 first_generation, last_generation, first_started_at_us, last_started_at_us,
 first_finished_at_us, last_finished_at_us, first_occurrence_metadata_json,
 last_occurrence_metadata_json, occurrence_count, superseded_count)
 VALUES(1,'observation-1',1,1,1,'{}','{}','aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
 '{}','{}','v1','success',1,1,10,10,20,20,'{}','{}',1,0),
 (2,'observation-2',2,2,1,'{}','{}',NULL,'{}','{}','v1','success',2,2,30,30,40,40,'{}','{}',1,0);
INSERT INTO metadata_coverage VALUES(1,0,'field:description',1,'{}','requested','empty','complete',NULL,'{}'),
 (2,0,'field:description',1,'{}','requested','populated','complete',NULL,'{}');
INSERT INTO metadata_snapshots VALUES(1,1,1,'field:description','empty',1,0,0);
"#;

/// Rows of the three tables that migration 16 drops (ADR 0076 packet 002).
/// Use them only on a schema before version 16.
#[cfg(test)]
pub(super) const SUPERSEDED_ROWS: &str = r#"
INSERT INTO metadata_field_selections VALUES(1,'description','context','{}',1,1,0,1,'absent',NULL,'{"original_value":null}','v1',20);
INSERT INTO metadata_discrepancies VALUES(1,1,'description',1,1,2,2,'active',40,40,'{}',NULL);
INSERT INTO metadata_discrepancy_transitions VALUES(1,1,1,NULL,'active','mismatch','v1','pair',1,2,
 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',NULL,'{"coverage":"known_absent"}','{"coverage":"known_value"}',40);
UPDATE metadata_discrepancies SET latest_transition_id=1;
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn populated() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::upgrades::create_fixture(&conn, 12).unwrap();
        conn.execute_batch(RETAINED_ROWS).unwrap();
        conn.execute_batch(SUPERSEDED_ROWS).unwrap();
        conn
    }

    #[test]
    fn adr_0075_migration_request_fact_member_and_head_constraints_are_enforced() {
        let conn = populated();
        conn.execute_batch(r#"
INSERT INTO metadata_request_slots VALUES('request',1,1,1,'{}','{}',1,10,'success',1,NULL);
INSERT INTO metadata_facts VALUES(1,1,0,0,1,'{}','{}','description',NULL,NULL,NULL,NULL,'plain_text','valid','"value"','{}','{}');
INSERT INTO metadata_snapshot_members VALUES(1,0,1);
INSERT INTO metadata_collection_heads VALUES(1,1,'field:description',1,1,1,0,20,'{}',1,'success',1,NULL);
"#).unwrap();
        for invalid in [
            "UPDATE metadata_request_slots SET generation=0",
            "UPDATE metadata_request_slots SET state='unknown'",
            "UPDATE metadata_request_slots SET latest_failure_id=99",
            "INSERT INTO metadata_request_slots SELECT * FROM metadata_request_slots",
            "UPDATE metadata_facts SET scope_ordinal=99",
            "UPDATE metadata_facts SET transport_ordinal=-1",
            "UPDATE metadata_facts SET representation='markdown'",
            "UPDATE metadata_facts SET validation='unknown'",
            "UPDATE metadata_facts SET declared_subject_id=99",
            "INSERT INTO metadata_facts SELECT 2,observation_id,scope_ordinal,transport_ordinal,declared_subject_id,declared_owner_json,owner_basis_json,fact_kind,assertion_source,source_position,extraction_path,source_observed_json,representation,validation,value_json,raw_member_json,body_locator_json FROM metadata_facts",
            "UPDATE metadata_snapshot_members SET member_ordinal=-1",
            "UPDATE metadata_snapshot_members SET fact_id=99",
            "INSERT INTO metadata_snapshot_members VALUES(1,1,1)",
            "INSERT INTO metadata_snapshots SELECT 2,provider_id,subject_id,collection,content_key,first_observation_id,first_scope_ordinal,member_count FROM metadata_snapshots",
            "UPDATE metadata_collection_heads SET accepted_generation=-1",
            "UPDATE metadata_collection_heads SET last_attempt_generation=-1",
            "UPDATE metadata_collection_heads SET last_attempt_state='unknown'",
            "UPDATE metadata_collection_heads SET accepted_scope_ordinal=99",
            "UPDATE metadata_collection_heads SET accepted_occurrence_metadata_json='invalid'",
            "INSERT INTO metadata_collection_heads SELECT * FROM metadata_collection_heads",
            "INSERT INTO metadata_snapshot_members SELECT * FROM metadata_snapshot_members",
            "INSERT INTO metadata_field_selections SELECT * FROM metadata_field_selections",
            "INSERT INTO metadata_coverage SELECT * FROM metadata_coverage",
            "INSERT INTO metadata_discrepancy_transitions SELECT 2,discrepancy_id,sequence,previous_state,state,reason,comparison_version,pair_content_key,rss_observation_id,index_observation_id,rss_body_sha256,index_body_sha256,rss_evidence_json,index_evidence_json,recorded_at_us FROM metadata_discrepancy_transitions",
        ] {
            assert!(conn.execute_batch(invalid).is_err(), "accepted invalid SQL: {invalid}");
        }
    }

    #[test]
    fn adr_0075_migration_reviewed_columns_keys_json_and_constraints() {
        let conn = populated();
        for (table, columns) in COLUMNS {
            let query = conn.prepare(&format!("SELECT * FROM {table}")).unwrap();
            assert_eq!(query.column_names(), *columns);
            let actions: Vec<String> = conn
                .prepare(&format!(
                    "SELECT on_delete FROM pragma_foreign_key_list('{table}')"
                ))
                .unwrap()
                .query_map([], |row| row.get(0))
                .unwrap()
                .map(Result::unwrap)
                .collect();
            assert!(actions.iter().all(|action| action == "RESTRICT"));
        }
        for invalid in [
            "INSERT INTO metadata_providers VALUES(3,'other','identity')",
            "INSERT INTO metadata_providers VALUES(3,'rss','')",
            "INSERT INTO metadata_providers VALUES(3,'rss','https://rss.invalid/feed')",
            "INSERT INTO metadata_resources VALUES(4,1,'')",
            "INSERT INTO metadata_resources VALUES(4,99,'uri')",
            "INSERT INTO metadata_resources VALUES(4,1,'https://rss.invalid/feed')",
            "INSERT INTO metadata_subjects VALUES(2,'invalid','feed','guid','scope',NULL)",
            "INSERT INTO metadata_subjects VALUES(2,'[]','feed','guid','scope','item')",
            "INSERT INTO metadata_subjects VALUES(2,'[]','track','guid','scope','')",
            "INSERT INTO metadata_subjects VALUES(2,'[]','feed','other','scope',NULL)",
            "INSERT INTO metadata_bodies VALUES('bad-hash',0,x'')",
            "UPDATE metadata_bodies SET byte_length=4",
            "UPDATE metadata_bodies SET sha256=upper(sha256)",
            "INSERT INTO metadata_generation VALUES(2,0)",
            "UPDATE metadata_generation SET last_generation=-1",
            "UPDATE metadata_observations SET first_generation=0",
            "UPDATE metadata_observations SET last_generation=0",
            "UPDATE metadata_observations SET occurrence_count=0",
            "UPDATE metadata_observations SET superseded_count=-1",
            "UPDATE metadata_observations SET outcome='unknown'",
            "UPDATE metadata_observations SET profile_json='invalid'",
            "UPDATE metadata_observations SET source_revision_json='invalid'",
            "UPDATE metadata_coverage SET scope_ordinal=-1",
            "UPDATE metadata_coverage SET request_intent='unknown'",
            "UPDATE metadata_coverage SET presence='absent'",
            "UPDATE metadata_coverage SET completeness='empty'",
            "UPDATE metadata_snapshots SET member_count=-1",
            "UPDATE metadata_field_selections SET field=''",
            "UPDATE metadata_field_selections SET value_json='{}'",
            "UPDATE metadata_field_selections SET selection_state='value'",
            "UPDATE metadata_field_selections SET selected_scope_ordinal=999",
            "UPDATE metadata_discrepancies SET field='artwork'",
            "UPDATE metadata_discrepancies SET state='unknown'",
            "UPDATE metadata_discrepancy_transitions SET sequence=0",
            "UPDATE metadata_discrepancy_transitions SET reason='other'",
            "DELETE FROM metadata_bodies",
            "DELETE FROM metadata_observations",
            "DELETE FROM metadata_coverage",
        ] {
            assert!(
                conn.execute_batch(invalid).is_err(),
                "accepted invalid SQL: {invalid}"
            );
        }
        conn.execute("INSERT INTO metadata_field_selections SELECT subject_id,'future_registered_field',context_key,context_json,selected_provider_id,selected_observation_id,selected_scope_ordinal,selected_snapshot_id,selection_state,value_json,evidence_json,policy_version,selected_at_us FROM metadata_field_selections", []).unwrap();
        conn.execute("INSERT INTO metadata_discrepancies VALUES(2,1,'description',1,1,2,3,'active',40,40,'{}',NULL)", []).unwrap();
        assert!(conn.execute("INSERT INTO metadata_discrepancies VALUES(3,1,'description',1,1,2,3,'active',40,40,'{}',NULL)", []).is_err());
        assert_eq!(
            conn.query_row("SELECT count(*) FROM metadata_discrepancies", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
    }
}
