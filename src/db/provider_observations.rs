//! Atomic request allocation and evidence retention for ADR 0075.

use std::sync::{Arc, Mutex};

use anyhow::{bail, ensure, Result};
use rusqlite::{params, params_from_iter, types::Value as SqlValue, Connection, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::provider_observation::{
    contracts, AcceptedSnapshot, CollectionOutcome, CollectionRefresh, CollectionRetention,
    CollectionState, CoverageEvidence, FactEvidence, ObservationOutcome, ObservationReceipt,
    ObservationRetention, ObservationStorageError, ObservationWriteFailure, ProviderBinding,
    ProviderCollection, ProviderObservation, ProviderRequestSpec, ProviderTrackState, RefreshState,
    RequestRefresh, RequestToken, RequestWriteState, StorageRetry, StoredFact, SubjectKey,
};

fn identity(domain: &str, parts: &[String]) -> String {
    let mut hash = Sha256::new();
    for part in std::iter::once(domain).chain(parts.iter().map(String::as_str)) {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    format!("{:x}", hash.finalize())
}
fn text(value: impl Into<String>) -> SqlValue {
    SqlValue::Text(value.into())
}
fn optional_text(value: Option<&str>) -> SqlValue {
    value.map_or(SqlValue::Null, text)
}
fn optional_integer(value: Option<i64>) -> SqlValue {
    value.map_or(SqlValue::Null, SqlValue::Integer)
}
fn json_text(value: &Value) -> SqlValue {
    text(value.to_string())
}
fn optional_json(value: Option<&Value>) -> SqlValue {
    value.map_or(SqlValue::Null, json_text)
}

fn subject_id(conn: &Connection, subject: Option<&SubjectKey>) -> Result<Option<i64>> {
    let Some(subject) = subject else {
        return Ok(None);
    };
    ensure!(
        !subject.scope.is_empty() && matches!(subject.scope_kind.as_str(), "guid" | "resource"),
        "Invalid subject scope"
    );
    ensure!(
        match subject.kind.as_str() {
            "feed" => subject.item_guid.is_none(),
            "track" => subject
                .item_guid
                .as_ref()
                .is_some_and(|item| !item.is_empty()),
            _ => false,
        },
        "Invalid subject kind"
    );
    let key = subject.json().to_string();
    conn.execute("INSERT INTO metadata_subjects(subject_key,kind,feed_scope_kind,feed_scope,item_guid) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(subject_key) DO NOTHING", params![key, subject.kind, subject.scope_kind, subject.scope, subject.item_guid])?;
    let (id, kind, scope_kind, scope, item): (i64,String,String,String,Option<String>) = conn.query_row("SELECT id,kind,feed_scope_kind,feed_scope,item_guid FROM metadata_subjects WHERE subject_key=?1", [&key], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
    ensure!(
        (kind, scope_kind, scope, item)
            == (
                subject.kind.clone(),
                subject.scope_kind.clone(),
                subject.scope.clone(),
                subject.item_guid.clone()
            ),
        "Stored subject differs"
    );
    Ok(Some(id))
}

fn request_identity(spec: &ProviderRequestSpec) -> (String, String, String) {
    let descriptor = json!({"parameters":spec.requested_parameters,"subject":spec.requested_subject.as_ref().map(SubjectKey::json)}).to_string();
    let profile = spec.profile.to_string();
    let key = identity(
        "provider-request-v1",
        &[
            spec.provider.token().into(),
            spec.provider_identity.clone(),
            spec.request_uri.clone(),
            descriptor.clone(),
            profile.clone(),
        ],
    );
    (key, descriptor, profile)
}

pub fn begin_provider_request(
    conn: &Connection,
    spec: ProviderRequestSpec,
) -> Result<RequestToken, ObservationStorageError> {
    begin(conn, spec).map_err(|_| ObservationStorageError::RequestAllocation)
}
fn begin(conn: &Connection, spec: ProviderRequestSpec) -> Result<RequestToken> {
    ensure!(
        !spec.provider_identity.is_empty() && !spec.request_uri.is_empty(),
        "Missing provider resource"
    );
    let tx = conn.unchecked_transaction()?;
    tx.execute("INSERT INTO metadata_providers(kind,identity) VALUES(?1,?2) ON CONFLICT(kind,identity) DO NOTHING", params![spec.provider.token(),spec.provider_identity])?;
    let provider_id: i64 = tx.query_row(
        "SELECT id FROM metadata_providers WHERE kind=?1 AND identity=?2",
        params![spec.provider.token(), spec.provider_identity],
        |r| r.get(0),
    )?;
    tx.execute("INSERT INTO metadata_resources(provider_id,request_uri) VALUES(?1,?2) ON CONFLICT(provider_id,request_uri) DO NOTHING", params![provider_id,spec.request_uri])?;
    let resource_id: i64 = tx.query_row(
        "SELECT id FROM metadata_resources WHERE provider_id=?1 AND request_uri=?2",
        params![provider_id, spec.request_uri],
        |r| r.get(0),
    )?;
    let subject_id = subject_id(&tx, spec.requested_subject.as_ref())?;
    let (key, descriptor, profile) = request_identity(&spec);
    let expected = vec![
        SqlValue::Integer(provider_id),
        SqlValue::Integer(resource_id),
        optional_integer(subject_id),
        text(&descriptor),
        text(&profile),
    ];
    if let Some(stored) = read_values(&tx,"SELECT provider_id,resource_id,requested_subject_id,requested_subject_json,profile_json FROM metadata_request_slots WHERE request_key=?1", &[text(&key)], expected.len())? { ensure!(stored == expected, "Request identity collision"); }
    tx.execute("UPDATE metadata_generation SET last_generation=last_generation+1 WHERE singleton=1 AND last_generation<9223372036854775807", [])?;
    ensure!(tx.changes() == 1, "Generation allocation failed");
    let generation = tx.query_row(
        "SELECT last_generation FROM metadata_generation WHERE singleton=1",
        [],
        |r| r.get(0),
    )?;
    tx.execute("INSERT INTO metadata_request_slots(request_key,provider_id,resource_id,requested_subject_id,requested_subject_json,profile_json,generation,started_at_us,state) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,'pending') ON CONFLICT(request_key) DO UPDATE SET generation=excluded.generation,started_at_us=excluded.started_at_us,state='pending'",params![key,provider_id,resource_id,subject_id,descriptor,profile,generation,spec.started_at_us])?;
    tx.commit()?;
    Ok(RequestToken {
        key,
        generation,
        provider_id,
        resource_id,
        subject_id,
        spec: Arc::new(spec),
        write_state: Arc::new(Mutex::new(RequestWriteState::Ready)),
    })
}

pub fn record_provider_observation(
    conn: &Connection,
    token: RequestToken,
    observation: Arc<ProviderObservation>,
) -> Result<ObservationReceipt, ObservationWriteFailure> {
    let result = record_once(conn, &token, &observation);
    result.map_err(|retry| ObservationWriteFailure {
        token,
        observation,
        operation: ObservationStorageError::ResponseWrite,
        retry,
    })
}

const IMMUTABLE: &str = "provider_id,resource_id,requested_subject_id,requested_subject_json,profile_json,body_sha256,http_status,response_uri,interpretation_metadata_json,source_revision_json,source_times_json,contract_id,decoder_version,outcome,failure_json";

fn record_once(
    conn: &Connection,
    token: &RequestToken,
    observation: &Arc<ProviderObservation>,
) -> Result<ObservationReceipt, StorageRetry> {
    let mut state = token
        .write_state
        .lock()
        .map_err(|_| StorageRetry::Blocked)?;
    match &*state {
        RequestWriteState::Committed(original, receipt) if original == observation => {
            return Ok((**receipt).clone())
        }
        RequestWriteState::Committed(..) | RequestWriteState::Uncertain => {
            return Err(StorageRetry::Blocked)
        }
        RequestWriteState::Retry(original) if original != observation => {
            return Err(StorageRetry::Blocked)
        }
        RequestWriteState::Ready | RequestWriteState::Retry(_) => {}
    }
    *state = RequestWriteState::Uncertain;
    let tx = conn
        .unchecked_transaction()
        .map_err(|_| StorageRetry::Blocked)?;
    match record(&tx, token, observation) {
        Ok(receipt) => match tx.commit() {
            Ok(()) => {
                *state = RequestWriteState::Committed(
                    Arc::clone(observation),
                    Box::new(receipt.clone()),
                );
                Ok(receipt)
            }
            Err(_) => Err(StorageRetry::Blocked),
        },
        Err(_) => {
            if tx.rollback().is_ok() && conn.is_autocommit() {
                *state = RequestWriteState::Retry(Arc::clone(observation));
                Err(StorageRetry::VerifiedRollback)
            } else {
                Err(StorageRetry::Blocked)
            }
        }
    }
}

fn record(
    tx: &Connection,
    token: &RequestToken,
    observation: &ProviderObservation,
) -> Result<ObservationReceipt> {
    let contract = contracts::validate(&token.spec, observation)?;
    let provider: (String,String,String,i64) = tx.query_row("SELECT p.kind,p.identity,r.request_uri,r.provider_id FROM metadata_providers p JOIN metadata_resources r ON r.id=?2 WHERE p.id=?1", params![token.provider_id,token.resource_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
    ensure!(
        provider
            == (
                token.spec.provider.token().into(),
                token.spec.provider_identity.clone(),
                token.spec.request_uri.clone(),
                token.provider_id
            ),
        "Provider resource differs"
    );
    ensure!(
        subject_id(tx, token.spec.requested_subject.as_ref())? == token.subject_id,
        "Requested subject differs"
    );
    let slot_generation: i64 = tx.query_row(
        "SELECT generation FROM metadata_request_slots WHERE request_key=?1",
        [&token.key],
        |r| r.get(0),
    )?;
    ensure!(
        slot_generation >= token.generation,
        "Unknown request generation"
    );
    let superseded = slot_generation > token.generation;
    let body_key = observation
        .body
        .as_ref()
        .map(|body| format!("{:x}", Sha256::digest(body)));
    if let (Some(key), Some(body)) = (&body_key, &observation.body) {
        let expected = vec![
            SqlValue::Integer(i64::try_from(body.len())?),
            SqlValue::Blob(body.to_vec()),
        ];
        if let Some(stored) = read_values(
            tx,
            "SELECT byte_length,bytes FROM metadata_bodies WHERE sha256=?1",
            &[text(key)],
            2,
        )? {
            ensure!(stored == expected, "Body identity collision");
        } else {
            tx.execute(
                "INSERT INTO metadata_bodies(sha256,byte_length,bytes) VALUES(?1,?2,?3)",
                params![key, i64::try_from(body.len())?, body.as_ref()],
            )?;
        }
    }
    let descriptor = json!({"parameters":token.spec.requested_parameters,"subject":token.spec.requested_subject.as_ref().map(SubjectKey::json)}).to_string();
    let immutable = vec![
        SqlValue::Integer(token.provider_id),
        SqlValue::Integer(token.resource_id),
        optional_integer(token.subject_id),
        text(descriptor),
        json_text(&token.spec.profile),
        optional_text(body_key.as_deref()),
        optional_integer(observation.http_status.map(i64::from)),
        optional_text(observation.response_uri.as_deref()),
        json_text(&observation.interpretation),
        optional_json(observation.source_revision.as_ref()),
        json_text(&observation.source_times),
        optional_text(contract),
        text(&observation.decoder_version),
        text(observation.outcome.token()),
        optional_json(observation.failure.as_ref()),
    ];
    let key = identity(
        "provider-observation-v1",
        &[
            token.key.clone(),
            json!([
                body_key,
                observation.http_status,
                observation.response_uri,
                observation.interpretation,
                observation.source_revision,
                observation.source_times,
                contract,
                observation.decoder_version,
                observation.outcome.token(),
                observation.failure
            ])
            .to_string(),
        ],
    );
    let stored = read_values(
        tx,
        &format!("SELECT id,{IMMUTABLE} FROM metadata_observations WHERE observation_key=?1"),
        &[text(&key)],
        immutable.len() + 1,
    )?;
    let repeated = stored.is_some();
    let id = if let Some(stored) = stored {
        ensure!(stored[1..] == immutable, "Observation identity collision");
        let SqlValue::Integer(id) = stored[0] else {
            bail!("Invalid observation ID")
        };
        id
    } else {
        let mut values = vec![text(&key)];
        values.extend(immutable);
        values.extend([
            SqlValue::Integer(token.generation),
            SqlValue::Integer(token.spec.started_at_us),
            SqlValue::Integer(observation.finished_at_us),
            optional_integer(observation.fetched_at_us),
            json_text(&observation.occurrence),
            SqlValue::Integer(i64::from(superseded)),
        ]);
        tx.execute(&format!("INSERT INTO metadata_observations(observation_key,{IMMUTABLE},first_generation,last_generation,first_started_at_us,last_started_at_us,first_finished_at_us,last_finished_at_us,first_fetched_at_us,last_fetched_at_us,first_occurrence_metadata_json,last_occurrence_metadata_json,occurrence_count,superseded_count) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?17,?18,?18,?19,?19,?20,?20,?21,?21,1,?22)"),params_from_iter(values))?;
        tx.last_insert_rowid()
    };
    write_evidence(tx, id, observation, repeated)?;
    let collections = replace_collections(tx, token, observation, id, superseded)?;
    let head_superseded = collections
        .iter()
        .any(|result| result.retention == CollectionRetention::RejectedSuperseded);
    if head_superseded && !superseded && !repeated {
        tx.execute(
            "UPDATE metadata_observations SET superseded_count=superseded_count+1 WHERE id=?1",
            [id],
        )?;
    }
    if repeated {
        tx.execute("UPDATE metadata_observations SET first_started_at_us=CASE WHEN ?2<first_generation THEN ?3 ELSE first_started_at_us END,first_finished_at_us=CASE WHEN ?2<first_generation THEN ?4 ELSE first_finished_at_us END,first_fetched_at_us=CASE WHEN ?2<first_generation THEN ?5 ELSE first_fetched_at_us END,first_occurrence_metadata_json=CASE WHEN ?2<first_generation THEN ?6 ELSE first_occurrence_metadata_json END,last_started_at_us=CASE WHEN ?2>last_generation THEN ?3 ELSE last_started_at_us END,last_finished_at_us=CASE WHEN ?2>last_generation THEN ?4 ELSE last_finished_at_us END,last_fetched_at_us=CASE WHEN ?2>last_generation THEN ?5 ELSE last_fetched_at_us END,last_occurrence_metadata_json=CASE WHEN ?2>last_generation THEN ?6 ELSE last_occurrence_metadata_json END,first_generation=min(first_generation,?2),last_generation=max(last_generation,?2),occurrence_count=occurrence_count+1,superseded_count=superseded_count+?7 WHERE id=?1",params![id,token.generation,token.spec.started_at_us,observation.finished_at_us,observation.fetched_at_us,observation.occurrence.to_string(),i64::from(superseded || head_superseded)])?;
    }
    tx.execute("UPDATE metadata_request_slots SET state=?3,latest_observation_id=CASE WHEN ?3='failed' THEN latest_observation_id ELSE ?4 END,latest_failure_id=CASE WHEN ?3='failed' THEN ?4 ELSE NULL END WHERE request_key=?1 AND generation=?2",params![token.key,token.generation,observation.outcome.token(),id])?;
    Ok(ObservationReceipt {
        observation_id: id,
        generation: token.generation,
        provider_id: token.provider_id,
        resource_id: token.resource_id,
        request_uri: token.spec.request_uri.clone(),
        response_uri: observation.response_uri.clone(),
        body_key,
        started_at_us: token.spec.started_at_us,
        finished_at_us: observation.finished_at_us,
        fetched_at_us: observation.fetched_at_us,
        occurrence: observation.occurrence.clone(),
        outcome: observation.outcome,
        retention: if superseded {
            ObservationRetention::SupersededAttempt
        } else if repeated {
            ObservationRetention::Repeated
        } else {
            match observation.outcome {
                ObservationOutcome::Success => ObservationRetention::Unknown,
                ObservationOutcome::Partial => ObservationRetention::Partial,
                ObservationOutcome::Failed => ObservationRetention::Failed,
            }
        },
        collections,
    })
}

fn read_values(
    conn: &Connection,
    query: &str,
    parameters: &[SqlValue],
    count: usize,
) -> Result<Option<Vec<SqlValue>>> {
    Ok(conn
        .query_row(query, params_from_iter(parameters), |r| {
            (0..count).map(|i| r.get(i)).collect()
        })
        .optional()?)
}
fn immutable_row(
    conn: &Connection,
    table: &str,
    keys: &[(&str, SqlValue)],
    fields: &[(&str, SqlValue)],
    repeated: bool,
) -> Result<()> {
    let all = keys.iter().chain(fields).collect::<Vec<_>>();
    let names = all
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join(",");
    let conditions = keys
        .iter()
        .enumerate()
        .map(|(i, (name, _))| format!("{name}=?{}", i + 1))
        .collect::<Vec<_>>()
        .join(" AND ");
    let key_values = keys.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>();
    let values = all.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>();
    if repeated {
        let stored = read_values(
            conn,
            &format!("SELECT {names} FROM {table} WHERE {conditions}"),
            &key_values,
            values.len(),
        )?;
        ensure!(
            stored.as_ref() == Some(&values),
            "Stored interpretation differs"
        );
    } else {
        let placeholders = (1..=all.len())
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(",");
        conn.execute(
            &format!("INSERT INTO {table}({names}) VALUES({placeholders})"),
            params_from_iter(values),
        )?;
    }
    Ok(())
}
fn write_evidence(
    conn: &Connection,
    id: i64,
    observation: &ProviderObservation,
    repeated: bool,
) -> Result<()> {
    for (scope, coverage) in observation.coverage.iter().enumerate() {
        let scope = i64::try_from(scope)?;
        immutable_row(
            conn,
            "metadata_coverage",
            &[
                ("observation_id", SqlValue::Integer(id)),
                ("scope_ordinal", SqlValue::Integer(scope)),
            ],
            &[
                ("collection", text(&coverage.collection)),
                (
                    "target_subject_id",
                    optional_integer(subject_id(conn, coverage.target.as_ref())?),
                ),
                ("target_owner_json", json_text(&coverage.target_owner)),
                ("request_intent", text(&coverage.request_intent)),
                ("presence", text(coverage.presence.token())),
                (
                    "completeness",
                    text(if coverage.proof.is_some() {
                        "complete"
                    } else {
                        coverage.retention.completeness()
                    }),
                ),
                (
                    "contract_id",
                    optional_text(
                        coverage
                            .proof
                            .as_ref()
                            .map(contracts::VerifiedCoverage::contract),
                    ),
                ),
                (
                    "basis_json",
                    json_text(&coverage.proof.as_ref().map_or_else(
                        || coverage.basis.clone(),
                        |proof| json!({"evidence":coverage.basis,"verified":proof.evidence()}),
                    )),
                ),
            ],
            repeated,
        )?;
        for (ordinal, fact) in coverage.facts.iter().enumerate() {
            immutable_row(
                conn,
                "metadata_facts",
                &[
                    ("observation_id", SqlValue::Integer(id)),
                    ("scope_ordinal", SqlValue::Integer(scope)),
                    (
                        "transport_ordinal",
                        SqlValue::Integer(i64::try_from(ordinal)?),
                    ),
                ],
                &[
                    (
                        "declared_subject_id",
                        optional_integer(subject_id(conn, fact.subject.as_ref())?),
                    ),
                    ("declared_owner_json", json_text(&fact.declared_owner)),
                    ("owner_basis_json", json_text(&fact.owner_basis)),
                    ("fact_kind", text(&fact.kind)),
                    (
                        "assertion_source",
                        optional_text(fact.assertion_source.as_deref()),
                    ),
                    ("source_position", optional_integer(fact.source_position)),
                    (
                        "extraction_path",
                        optional_text(fact.extraction_path.as_deref()),
                    ),
                    (
                        "source_observed_json",
                        optional_json(fact.source_observed.as_ref()),
                    ),
                    ("representation", text(&fact.representation)),
                    ("validation", text(&fact.validation)),
                    ("value_json", json_text(&fact.value)),
                    ("raw_member_json", optional_json(fact.raw_member.as_ref())),
                    ("body_locator_json", json_text(&fact.body_locator)),
                ],
                repeated,
            )?;
        }
    }
    for (table, expected) in [
        ("metadata_coverage", observation.coverage.len()),
        (
            "metadata_facts",
            observation.coverage.iter().map(|c| c.facts.len()).sum(),
        ),
    ] {
        let count: i64 = conn.query_row(
            &format!("SELECT count(*) FROM {table} WHERE observation_id=?1"),
            [id],
            |r| r.get(0),
        )?;
        ensure!(
            count == i64::try_from(expected)?,
            "Stored evidence count differs"
        );
    }
    Ok(())
}

fn replace_collections(
    conn: &Connection,
    token: &RequestToken,
    observation: &ProviderObservation,
    observation_id: i64,
    superseded: bool,
) -> Result<Vec<CollectionOutcome>> {
    let mut outcomes = Vec::new();
    for (scope, coverage) in observation.coverage.iter().enumerate() {
        let mut retention = match coverage.retention {
            ObservationRetention::Partial => CollectionRetention::KeptPartial,
            ObservationRetention::Failed => CollectionRetention::KeptFailed,
            _ => CollectionRetention::KeptUnknown,
        };
        if let Some(subject) = &coverage.target {
            let subject_id = subject_id(conn, Some(subject))?
                .ok_or_else(|| anyhow::anyhow!("Missing collection subject"))?;
            let prior =
                read_provider_collection(conn, token.provider_id, subject, &coverage.collection)?;
            let (accepted_generation, prior_snapshot) = match &prior.state {
                CollectionState::NoSnapshot => (0, None),
                CollectionState::CompleteEmpty(snapshot)
                | CollectionState::CompletePopulated(snapshot) => {
                    (snapshot.generation, Some(snapshot.snapshot_id))
                }
            };
            let rejected = superseded || accepted_generation > token.generation;
            if coverage.proof.is_some() {
                if rejected {
                    retention = CollectionRetention::RejectedSuperseded;
                } else {
                    let snapshot_id = store_snapshot(
                        conn,
                        token.provider_id,
                        subject_id,
                        observation_id,
                        scope,
                        coverage,
                    )?;
                    conn.execute("INSERT INTO metadata_collection_heads(provider_id,subject_id,collection,snapshot_id,accepted_generation,accepted_observation_id,accepted_scope_ordinal,accepted_fetched_at_us,accepted_occurrence_metadata_json,last_attempt_generation,last_attempt_state) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,0,'none') ON CONFLICT(provider_id,subject_id,collection) DO UPDATE SET snapshot_id=excluded.snapshot_id,accepted_generation=excluded.accepted_generation,accepted_observation_id=excluded.accepted_observation_id,accepted_scope_ordinal=excluded.accepted_scope_ordinal,accepted_fetched_at_us=excluded.accepted_fetched_at_us,accepted_occurrence_metadata_json=excluded.accepted_occurrence_metadata_json", params![token.provider_id,subject_id,coverage.collection,snapshot_id,token.generation,observation_id,i64::try_from(scope)?,observation.fetched_at_us,observation.occurrence.to_string()])?;
                    retention = if prior_snapshot == Some(snapshot_id) {
                        CollectionRetention::Repeated
                    } else if coverage.facts.is_empty() {
                        CollectionRetention::ReplacedEmpty
                    } else {
                        CollectionRetention::ReplacedPopulated
                    };
                }
            }
            if !superseded {
                let attempt_state = if rejected {
                    "superseded"
                } else if coverage.proof.is_some() {
                    "success"
                } else if observation.outcome == ObservationOutcome::Failed {
                    "failed"
                } else {
                    "partial"
                };
                conn.execute("INSERT INTO metadata_collection_heads(provider_id,subject_id,collection,accepted_generation,last_attempt_generation,last_attempt_state,last_attempt_observation_id,latest_failure_id) VALUES(?1,?2,?3,0,?4,?5,?6,?7) ON CONFLICT(provider_id,subject_id,collection) DO UPDATE SET last_attempt_generation=excluded.last_attempt_generation,last_attempt_state=excluded.last_attempt_state,last_attempt_observation_id=excluded.last_attempt_observation_id,latest_failure_id=excluded.latest_failure_id WHERE excluded.last_attempt_generation>=metadata_collection_heads.last_attempt_generation", params![token.provider_id,subject_id,coverage.collection,token.generation,attempt_state,observation_id,(attempt_state == "failed").then_some(observation_id)])?;
            }
        }
        outcomes.push(CollectionOutcome {
            provider_id: token.provider_id,
            subject: coverage.target.clone(),
            collection: coverage.collection.clone(),
            scope_ordinal: scope,
            retention,
        });
    }
    Ok(outcomes)
}

fn snapshot_content(coverage: &CoverageEvidence) -> Value {
    json!([
        coverage
            .proof
            .as_ref()
            .map(contracts::VerifiedCoverage::contract),
        coverage.collection,
        coverage.target,
        coverage.target_owner,
        coverage.basis,
        coverage.facts
    ])
}

fn store_snapshot(
    conn: &Connection,
    provider: i64,
    subject: i64,
    observation: i64,
    scope: usize,
    coverage: &CoverageEvidence,
) -> Result<i64> {
    let content = snapshot_content(coverage);
    let key = identity("provider-snapshot-v1", &[content.to_string()]);
    let stored: Option<i64> = conn.query_row("SELECT id FROM metadata_snapshots WHERE provider_id=?1 AND subject_id=?2 AND collection=?3 AND content_key=?4",params![provider,subject,coverage.collection,key],|row|row.get(0)).optional()?;
    if let Some(id) = stored {
        let (contract, owner, basis, members) =
            snapshot_inputs(conn, id, provider, subject, &coverage.collection)?;
        ensure!(
            json!([
                contract,
                coverage.collection,
                coverage.target,
                owner,
                basis,
                members
                    .iter()
                    .map(|member| &member.evidence)
                    .collect::<Vec<_>>()
            ]) == content,
            "Snapshot identity collision"
        );
        return Ok(id);
    }
    conn.execute("INSERT INTO metadata_snapshots(provider_id,subject_id,collection,content_key,first_observation_id,first_scope_ordinal,member_count) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![provider,subject,coverage.collection,key,observation,i64::try_from(scope)?,i64::try_from(coverage.facts.len())?])?;
    let id = conn.last_insert_rowid();
    conn.execute("INSERT INTO metadata_snapshot_members(snapshot_id,member_ordinal,fact_id) SELECT ?1,transport_ordinal,id FROM metadata_facts WHERE observation_id=?2 AND scope_ordinal=?3 ORDER BY transport_ordinal",params![id,observation,i64::try_from(scope)?])?;
    snapshot_inputs(conn, id, provider, subject, &coverage.collection)?;
    Ok(id)
}

fn parse_json(raw: String) -> rusqlite::Result<Value> {
    serde_json::from_str(&raw).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}
fn row_json(row: &rusqlite::Row<'_>, column: usize) -> rusqlite::Result<Value> {
    parse_json(row.get(column)?)
}
fn row_optional_json(row: &rusqlite::Row<'_>, column: usize) -> rusqlite::Result<Option<Value>> {
    row.get::<_, Option<String>>(column)?
        .map(parse_json)
        .transpose()
}

struct SnapshotCoverageRow {
    stored_provider: i64,
    stored_subject: i64,
    stored_collection: String,
    count: i64,
    coverage_provider: i64,
    coverage_subject: Option<i64>,
    coverage_collection: String,
    completeness: String,
    contract: Option<String>,
    owner: Value,
    basis: Value,
}

struct CollectionHeadRow {
    snapshot: Option<i64>,
    generation: i64,
    observation: Option<i64>,
    scope: Option<i64>,
    fetched_at: Option<i64>,
    occurrence: Option<Value>,
    attempt_generation: i64,
    attempt_state: String,
    attempt_observation: Option<i64>,
    failure: Option<i64>,
}

struct RequestRefreshRow {
    kind: String,
    identity: String,
    uri: String,
    stored_descriptor: String,
    stored_profile: String,
    generation: i64,
    started: i64,
    state: String,
    observation_id: Option<i64>,
    failure_id: Option<i64>,
}

fn snapshot_inputs(
    conn: &Connection,
    id: i64,
    provider: i64,
    subject: i64,
    collection: &str,
) -> Result<(String, Value, Value, Vec<StoredFact>)> {
    let SnapshotCoverageRow {stored_provider,stored_subject,stored_collection,count,coverage_provider,coverage_subject,coverage_collection,completeness,contract,owner,basis} = conn.query_row("SELECT s.provider_id,s.subject_id,s.collection,s.member_count,o.provider_id,c.target_subject_id,c.collection,c.completeness,c.contract_id,c.target_owner_json,c.basis_json FROM metadata_snapshots s JOIN metadata_coverage c ON c.observation_id=s.first_observation_id AND c.scope_ordinal=s.first_scope_ordinal JOIN metadata_observations o ON o.id=c.observation_id WHERE s.id=?1",[id],|r|Ok(SnapshotCoverageRow {stored_provider:r.get(0)?,stored_subject:r.get(1)?,stored_collection:r.get(2)?,count:r.get(3)?,coverage_provider:r.get(4)?,coverage_subject:r.get(5)?,coverage_collection:r.get(6)?,completeness:r.get(7)?,contract:r.get(8)?,owner:row_json(r,9)?,basis:row_json(r,10)?}))?;
    ensure!(
        stored_provider == provider
            && stored_subject == subject
            && stored_collection == collection
            && coverage_provider == provider
            && coverage_subject == Some(subject)
            && coverage_collection == collection
            && completeness == "complete",
        "Snapshot coverage differs"
    );
    let contract = contract.ok_or_else(|| anyhow::anyhow!("Snapshot contract is absent"))?;
    let (first_observation,first_scope,stored_key):(i64,i64,String) = conn.query_row("SELECT first_observation_id,first_scope_ordinal,content_key FROM metadata_snapshots WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
    let mut statement = conn.prepare("SELECT m.member_ordinal,f.id,f.observation_id,f.scope_ordinal,f.transport_ordinal,s.kind,s.feed_scope_kind,s.feed_scope,s.item_guid,f.declared_owner_json,f.owner_basis_json,f.fact_kind,f.assertion_source,f.source_position,f.extraction_path,f.source_observed_json,f.representation,f.validation,f.value_json,f.raw_member_json,f.body_locator_json,f.declared_subject_id FROM metadata_snapshot_members m JOIN metadata_facts f ON f.id=m.fact_id LEFT JOIN metadata_subjects s ON s.id=f.declared_subject_id WHERE m.snapshot_id=?1 ORDER BY m.member_ordinal")?;
    let rows = statement
        .query_map([id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Option<i64>>(21)?,
                StoredFact {
                    fact_id: r.get(1)?,
                    observation_id: r.get(2)?,
                    scope_ordinal: r.get(3)?,
                    transport_ordinal: r.get(4)?,
                    evidence: FactEvidence {
                        subject: Some(SubjectKey {
                            kind: r.get(5)?,
                            scope_kind: r.get(6)?,
                            scope: r.get(7)?,
                            item_guid: r.get(8)?,
                        }),
                        declared_owner: row_json(r, 9)?,
                        owner_basis: row_json(r, 10)?,
                        kind: r.get(11)?,
                        assertion_source: r.get(12)?,
                        source_position: r.get(13)?,
                        extraction_path: r.get(14)?,
                        source_observed: row_optional_json(r, 15)?,
                        representation: r.get(16)?,
                        validation: r.get(17)?,
                        value: row_json(r, 18)?,
                        raw_member: row_optional_json(r, 19)?,
                        body_locator: row_json(r, 20)?,
                    },
                },
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ensure!(
        i64::try_from(rows.len())? == count,
        "Snapshot member count differs"
    );
    let mut members = Vec::with_capacity(rows.len());
    for (ordinal, (stored_ordinal, owner, member)) in rows.into_iter().enumerate() {
        ensure!(
            stored_ordinal == i64::try_from(ordinal)?
                && owner == Some(subject)
                && member.evidence.kind == collection
                && member.evidence.validation == "valid"
                && member.observation_id == first_observation
                && member.scope_ordinal == first_scope
                && member.transport_ordinal == stored_ordinal,
            "Snapshot member differs"
        );
        members.push(member);
    }
    let evidence = basis
        .get("evidence")
        .ok_or_else(|| anyhow::anyhow!("Snapshot proof is absent"))?
        .clone();
    ensure!(
        basis["verified"]["contract"] == contract,
        "Snapshot contract differs"
    );
    let subject_key: SubjectKey = conn.query_row(
        "SELECT kind,feed_scope_kind,feed_scope,item_guid FROM metadata_subjects WHERE id=?1",
        [subject],
        |r| {
            Ok(SubjectKey {
                kind: r.get(0)?,
                scope_kind: r.get(1)?,
                scope: r.get(2)?,
                item_guid: r.get(3)?,
            })
        },
    )?;
    let facts: Vec<_> = members.iter().map(|member| &member.evidence).collect();
    let content = json!([contract, collection, subject_key, owner, evidence, facts]);
    ensure!(
        identity("provider-snapshot-v1", &[content.to_string()]) == stored_key,
        "Snapshot content key differs"
    );
    ensure!(
        basis["verified"]["coverage"]["facts"] == json!(facts),
        "Snapshot proof members differ"
    );
    ensure!(
        scope_fact_values(conn, first_observation, first_scope)?.len() == members.len(),
        "Snapshot coverage member count differs"
    );
    Ok((contract, owner, evidence, members))
}

fn scope_fact_values(
    conn: &Connection,
    observation: i64,
    scope: i64,
) -> Result<Vec<Vec<SqlValue>>> {
    let mut statement = conn.prepare("SELECT transport_ordinal,declared_subject_id,declared_owner_json,owner_basis_json,fact_kind,assertion_source,source_position,extraction_path,source_observed_json,representation,validation,value_json,raw_member_json,body_locator_json FROM metadata_facts WHERE observation_id=?1 AND scope_ordinal=?2 ORDER BY transport_ordinal")?;
    let rows = statement
        .query_map(params![observation, scope], |row| {
            (0..14).map(|column| row.get(column)).collect()
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

fn refresh_state(value: &str) -> Result<RefreshState> {
    Ok(match value {
        "none" => RefreshState::None,
        "pending" => RefreshState::Pending,
        "success" => RefreshState::Success,
        "partial" => RefreshState::Partial,
        "failed" => RefreshState::Failed,
        "superseded" => RefreshState::Superseded,
        "abandoned" => RefreshState::Abandoned,
        _ => bail!("Stored refresh state is invalid"),
    })
}

/// Read an exact provider collection without writing inferred ownership.
pub fn read_provider_collection(
    conn: &Connection,
    provider: i64,
    subject: &SubjectKey,
    collection: &str,
) -> Result<ProviderCollection> {
    let mut result = ProviderCollection {
        provider_id: provider,
        subject: subject.clone(),
        collection: collection.into(),
        state: CollectionState::NoSnapshot,
        refresh: None,
    };
    let subject_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM metadata_subjects WHERE subject_key=?1",
            [subject.json().to_string()],
            |r| r.get(0),
        )
        .optional()?;
    let Some(subject_id) = subject_id else {
        return Ok(result);
    };
    let head: Option<CollectionHeadRow> = conn.query_row("SELECT snapshot_id,accepted_generation,accepted_observation_id,accepted_scope_ordinal,accepted_fetched_at_us,accepted_occurrence_metadata_json,last_attempt_generation,last_attempt_state,last_attempt_observation_id,latest_failure_id FROM metadata_collection_heads WHERE provider_id=?1 AND subject_id=?2 AND collection=?3",params![provider,subject_id,collection],|r|Ok(CollectionHeadRow {snapshot:r.get(0)?,generation:r.get(1)?,observation:r.get(2)?,scope:r.get(3)?,fetched_at:r.get(4)?,occurrence:row_optional_json(r,5)?,attempt_generation:r.get(6)?,attempt_state:r.get(7)?,attempt_observation:r.get(8)?,failure:r.get(9)?})).optional()?;
    let Some(CollectionHeadRow {
        snapshot,
        generation,
        observation,
        scope,
        fetched_at,
        occurrence,
        attempt_generation,
        attempt_state,
        attempt_observation,
        failure,
    }) = head
    else {
        return Ok(result);
    };
    result.refresh = Some(CollectionRefresh {
        generation: attempt_generation,
        state: refresh_state(&attempt_state)?,
        observation_id: attempt_observation,
        failure_id: failure,
    });
    let Some(snapshot_id) = snapshot else {
        ensure!(
            generation == 0 && observation.is_none() && scope.is_none() && occurrence.is_none(),
            "Unaccepted head contains acceptance evidence"
        );
        return Ok(result);
    };
    let observation_id =
        observation.ok_or_else(|| anyhow::anyhow!("Accepted observation is absent"))?;
    let scope_ordinal = scope.ok_or_else(|| anyhow::anyhow!("Accepted scope is absent"))?;
    let occurrence = occurrence.ok_or_else(|| anyhow::anyhow!("Accepted occurrence is absent"))?;
    ensure!(generation > 0, "Accepted generation is invalid");
    let (accepted_provider,accepted_subject,accepted_collection,completeness,accepted_contract): (i64,Option<i64>,String,String,Option<String>) = conn.query_row("SELECT o.provider_id,c.target_subject_id,c.collection,c.completeness,c.contract_id FROM metadata_coverage c JOIN metadata_observations o ON o.id=c.observation_id WHERE c.observation_id=?1 AND c.scope_ordinal=?2",params![observation_id,scope_ordinal],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
    let (contract, snapshot_owner, snapshot_basis, members) =
        snapshot_inputs(conn, snapshot_id, provider, subject_id, collection)?;
    ensure!(
        accepted_provider == provider
            && accepted_subject == Some(subject_id)
            && accepted_collection == collection
            && completeness == "complete"
            && accepted_contract.as_deref() == Some(&contract),
        "Accepted head differs from coverage"
    );
    let (first_observation, first_scope): (i64, i64) = conn.query_row(
        "SELECT first_observation_id,first_scope_ordinal FROM metadata_snapshots WHERE id=?1",
        [snapshot_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let (accepted_owner,accepted_basis):(Value,Value) = conn.query_row("SELECT target_owner_json,basis_json FROM metadata_coverage WHERE observation_id=?1 AND scope_ordinal=?2",params![observation_id,scope_ordinal],|r|Ok((row_json(r,0)?,row_json(r,1)?)))?;
    ensure!(
        accepted_owner == snapshot_owner
            && accepted_basis["evidence"] == snapshot_basis
            && scope_fact_values(conn, observation_id, scope_ordinal)?
                == scope_fact_values(conn, first_observation, first_scope)?,
        "Accepted coverage content differs from snapshot"
    );
    let snapshot = AcceptedSnapshot {
        snapshot_id,
        generation,
        observation_id,
        scope_ordinal,
        fetched_at_us: fetched_at,
        occurrence,
        members,
    };
    result.state = if snapshot.members.is_empty() {
        CollectionState::CompleteEmpty(snapshot)
    } else {
        CollectionState::CompletePopulated(snapshot)
    };
    Ok(result)
}

/// Read the exact request slot, including its profile and null input distinctions.
pub fn read_request_refresh(
    conn: &Connection,
    spec: &ProviderRequestSpec,
) -> Result<Option<RequestRefresh>> {
    let (key, descriptor, profile) = request_identity(spec);
    let row: Option<RequestRefreshRow> = conn.query_row("SELECT p.kind,p.identity,r.request_uri,s.requested_subject_json,s.profile_json,s.generation,s.started_at_us,s.state,s.latest_observation_id,s.latest_failure_id FROM metadata_request_slots s JOIN metadata_providers p ON p.id=s.provider_id JOIN metadata_resources r ON r.id=s.resource_id AND r.provider_id=s.provider_id WHERE s.request_key=?1",[key],|r|Ok(RequestRefreshRow {kind:r.get(0)?,identity:r.get(1)?,uri:r.get(2)?,stored_descriptor:r.get(3)?,stored_profile:r.get(4)?,generation:r.get(5)?,started:r.get(6)?,state:r.get(7)?,observation_id:r.get(8)?,failure_id:r.get(9)?})).optional()?;
    let Some(RequestRefreshRow {
        kind,
        identity,
        uri,
        stored_descriptor,
        stored_profile,
        generation,
        started,
        state,
        observation_id,
        failure_id,
    }) = row
    else {
        return Ok(None);
    };
    ensure!(
        kind == spec.provider.token()
            && identity == spec.provider_identity
            && uri == spec.request_uri
            && stored_descriptor == descriptor
            && stored_profile == profile,
        "Request identity collision"
    );
    let mut request = spec.clone();
    request.started_at_us = started;
    Ok(Some(RequestRefresh {
        request,
        generation,
        state: refresh_state(&state)?,
        observation_id,
        failure_id,
    }))
}

/// Bind collections only through exact retained RSS request evidence.
pub fn read_track_provider_state(
    conn: &Connection,
    request: Option<&ProviderRequestSpec>,
) -> Result<ProviderTrackState> {
    let Some(request) = request else {
        return Ok(ProviderTrackState::default());
    };
    let mut result = ProviderTrackState {
        binding: ProviderBinding::RequestOnly,
        request_refresh: read_request_refresh(conn, request)?,
        ..ProviderTrackState::default()
    };
    let (_, descriptor, profile) = request_identity(request);
    let mut statement = conn.prepare("SELECT DISTINCT p.id,s.kind,s.feed_scope_kind,s.feed_scope,s.item_guid FROM metadata_observations o JOIN metadata_providers p ON p.id=o.provider_id JOIN metadata_resources r ON r.id=o.resource_id JOIN metadata_coverage c ON c.observation_id=o.id JOIN metadata_subjects s ON s.id=c.target_subject_id WHERE p.kind=?1 AND p.identity=?2 AND r.request_uri=?3 AND o.requested_subject_json=?4 AND o.profile_json=?5 AND c.collection='source_ids' AND c.completeness='complete' ORDER BY p.id,s.subject_key")?;
    let scopes = statement
        .query_map(
            params![
                request.provider.token(),
                request.provider_identity,
                request.request_uri,
                descriptor,
                profile
            ],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    SubjectKey {
                        kind: r.get(1)?,
                        scope_kind: r.get(2)?,
                        scope: r.get(3)?,
                        item_guid: r.get(4)?,
                    },
                ))
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (provider, subject) in scopes {
        result.collections.push(read_provider_collection(
            conn,
            provider,
            &subject,
            "source_ids",
        )?);
    }
    if !result.collections.is_empty() {
        result.binding = ProviderBinding::Proven;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_observation::{musicindex, ProviderKind};

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::upgrades::create_fixture(&conn, 12).unwrap();
        conn
    }
    fn spec() -> ProviderRequestSpec {
        ProviderRequestSpec {
            provider: ProviderKind::MusicIndex,
            provider_identity: "https://private.invalid".into(),
            request_uri: "https://private.invalid/v1/feeds/f/tracks/t".into(),
            requested_subject: Some(SubjectKey::guid("f", Some("t"))),
            requested_parameters: json!({"path":["v1","feeds","f","tracks","t"]}),
            profile: json!({"version":1,"query":[["include","source_ids,source_links,source_contributors"]]}),
            started_at_us: 100,
        }
    }
    fn observation(spec: &ProviderRequestSpec) -> Arc<ProviderObservation> {
        let body=br#"{"data":{"feed_guid":"f","track_guid":"t","title":"...","source_ids":[{"entity_type":"track","entity_id":"t","position":7,"value":"PRIVATE BODY","unknown_claim":{"a":1}}],"source_links":null,"source_contributors":[],"source_enclosures":false,"remote_items":[{"future":"kept"}]}}"#;
        let mut observation = ProviderObservation {
            body: Some(Arc::from(body.as_slice())),
            http_status: Some(200),
            response_uri: Some(spec.request_uri.clone()),
            interpretation: json!({"version":1,"media_type":"application/json","charset":"UTF-8","retained_body_codings":[],"body_state":"complete","effective_base_uri":null}),
            source_revision: None,
            source_times: json!({}),
            decoder_version: "musicindex-json-v1".into(),
            outcome: ObservationOutcome::Success,
            failure: None,
            finished_at_us: 200,
            fetched_at_us: Some(190),
            occurrence: json!({"version":1,"headers":{"etag":["UFJJVkFURSBFVEFH"]}}),
            coverage: Vec::new(),
        };
        musicindex::extract(&mut observation, spec, std::str::from_utf8(body).unwrap());
        Arc::new(observation)
    }
    fn counts(conn: &Connection) -> Vec<i64> {
        [
            "metadata_bodies",
            "metadata_observations",
            "metadata_coverage",
            "metadata_facts",
        ]
        .iter()
        .map(|table| {
            conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap()
        })
        .collect()
    }
    #[test]
    fn adr_0075_observation_atomic_failures_keep_capsules_and_retry_without_network() {
        for table in [
            "metadata_bodies",
            "metadata_observations",
            "metadata_coverage",
            "metadata_facts",
        ] {
            let conn = setup();
            let spec = spec();
            let observation = observation(&spec);
            let token = begin_provider_request(&conn, spec).unwrap();
            conn.execute_batch(&format!("CREATE TEMP TRIGGER reject_write AFTER INSERT ON {table} BEGIN SELECT RAISE(ABORT,'fixture rejection'); END")).unwrap();
            let failure =
                record_provider_observation(&conn, token, Arc::clone(&observation)).unwrap_err();
            assert_eq!(failure.retry, StorageRetry::VerifiedRollback);
            assert!(Arc::ptr_eq(&failure.observation, &observation));
            assert_eq!(counts(&conn), vec![0, 0, 0, 0]);
            assert_eq!(
                conn.query_row("SELECT state FROM metadata_request_slots", [], |r| r
                    .get::<_, String>(0))
                    .unwrap(),
                "pending"
            );
            conn.execute_batch("DROP TRIGGER reject_write").unwrap();
            let receipt =
                record_provider_observation(&conn, failure.token, failure.observation).unwrap();
            assert_eq!(receipt.generation, 1);
            assert_eq!(counts(&conn)[..2], [1, 1]);
        }
    }

    #[test]
    fn adr_0075_observation_token_replay_is_idempotent_and_uncertain_commit_blocks_retry() {
        let conn = setup();
        let spec = spec();
        let original = observation(&spec);
        let token = begin_provider_request(&conn, spec.clone()).unwrap();
        let receipt =
            record_provider_observation(&conn, token.clone(), Arc::clone(&original)).unwrap();
        let mutations = conn.total_changes();
        let replay =
            record_provider_observation(&conn, token.clone(), Arc::clone(&original)).unwrap();
        assert_eq!(receipt, replay);
        assert_eq!(conn.total_changes(), mutations);
        let mut changed = (*original).clone();
        changed.finished_at_us += 1;
        let failure = record_provider_observation(&conn, token, Arc::new(changed)).unwrap_err();
        assert_eq!(failure.retry, StorageRetry::Blocked);
        assert_eq!(conn.total_changes(), mutations);
        let token = begin_provider_request(&conn, spec).unwrap();
        conn.commit_hook(Some(|| true)).unwrap();
        let failure =
            record_provider_observation(&conn, token.clone(), Arc::clone(&original)).unwrap_err();
        assert_eq!(failure.retry, StorageRetry::Blocked);
        conn.commit_hook(None::<fn() -> bool>).unwrap();
        let mutations = conn.total_changes();
        assert!(record_provider_observation(&conn, token, original).is_err());
        assert_eq!(conn.total_changes(), mutations);
        assert_eq!(
            conn.query_row(
                "SELECT occurrence_count FROM metadata_observations",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
    }

    #[test]
    fn adr_0075_observation_redirect_retains_request_identity_and_final_resource() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let destination = format!("{base}/response");
        let final_uri = destination.clone();
        let worker = std::thread::spawn(move || {
            for status in [302, 200] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0; 4096];
                stream.read(&mut request).unwrap();
                let body = if status == 200 {
                    r#"{"data":{"feed_guid":"f","track_guid":"t"}}"#
                } else {
                    ""
                };
                let location = if status == 302 {
                    format!("Location: {destination}\r\n")
                } else {
                    String::new()
                };
                write!(stream,"HTTP/1.1 {status} Fixture\r\n{location}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            }
        });
        let conn = Arc::new(Mutex::new(setup()));
        let recorder = Arc::new(
            crate::provider_observation::ProviderObservationRecorder::new(Arc::clone(&conn)),
        );
        let client = crate::api::Client::new_with_base_url(base.clone())
            .with_observation_recorder(Some(Arc::clone(&recorder)));
        client.fetch_feed_track("f", "t", None).unwrap();
        worker.join().unwrap();
        let receipts = recorder.take_receipts();
        assert_eq!(receipts.len(), 1);
        assert_eq!(
            receipts[0].request_uri,
            format!("{base}/v1/feeds/f/tracks/t")
        );
        assert_eq!(
            receipts[0].response_uri.as_deref(),
            Some(final_uri.as_str())
        );
        assert_eq!(
            conn.lock()
                .unwrap()
                .query_row("SELECT identity FROM metadata_providers", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            base
        );
    }
    #[test]
    fn adr_0075_observation_repeated_headers_and_generation_order_preserve_identity() {
        let conn = setup();
        let spec = spec();
        let body = observation(&spec);
        let older = begin_provider_request(&conn, spec.clone()).unwrap();
        let mut later_spec = spec.clone();
        later_spec.started_at_us = -100;
        let later = begin_provider_request(&conn, later_spec).unwrap();
        let mut new = (*body).clone();
        new.finished_at_us = -10;
        new.fetched_at_us = Some(-20);
        new.occurrence = json!({"version":1,"headers":{"date":["bmV3"],"etag":["bmV3"],"last-modified":["bmV3"],"cache-control":["bmV3"]}});
        let first = record_provider_observation(&conn, later, Arc::new(new.clone())).unwrap();
        let before = counts(&conn);
        let old = record_provider_observation(&conn, older, Arc::clone(&body)).unwrap();
        assert_eq!(old.retention, ObservationRetention::SupersededAttempt);
        assert_eq!(old.observation_id, first.observation_id);
        assert_eq!(counts(&conn), before);
        let retained:(i64,i64,i64,i64,String,i64,i64)=conn.query_row("SELECT first_generation,last_generation,first_started_at_us,last_started_at_us,last_occurrence_metadata_json,occurrence_count,superseded_count FROM metadata_observations",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).unwrap();
        assert_eq!(
            retained,
            (1, 2, 100, -100, new.occurrence.to_string(), 2, 1)
        );
        let mut failed = (*body).clone();
        failed.fail("transport");
        let token = begin_provider_request(&conn, spec.clone()).unwrap();
        let fail_receipt = record_provider_observation(&conn, token, Arc::new(failed)).unwrap();
        let old_token = RequestToken {
            generation: 1,
            ..begin_provider_request(&conn, spec).unwrap()
        };
        let _receipt = record_provider_observation(&conn, old_token, body).unwrap();
        assert_eq!(
            conn.query_row(
                "SELECT latest_failure_id FROM metadata_request_slots",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            fail_receipt.observation_id
        );
    }
    #[test]
    fn adr_0075_observation_reopen_connections_preserve_pending_and_original_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("observations.sqlite");
        let conn = Connection::open(&path).unwrap();
        crate::db::upgrades::create_fixture(&conn, 12).unwrap();
        let spec = spec();
        let original = observation(&spec);
        let first = begin_provider_request(&conn, spec.clone()).unwrap();
        let second = Connection::open(&path).unwrap();
        let second_token = begin_provider_request(&second, spec.clone()).unwrap();
        assert_eq!(second_token.generation, first.generation + 1);
        let _receipt = record_provider_observation(&conn, first, Arc::clone(&original)).unwrap();
        drop(conn);
        drop(second);
        let reopened = Connection::open(&path).unwrap();
        assert_eq!(
            reopened
                .query_row("SELECT state FROM metadata_request_slots", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            "pending"
        );
        let next = begin_provider_request(&reopened, spec).unwrap();
        assert_eq!(next.generation, 3);
        assert_eq!(
            reopened
                .query_row("SELECT bytes FROM metadata_bodies", [], |r| r
                    .get::<_, Vec<u8>>(0))
                .unwrap(),
            original.body.as_ref().unwrap().as_ref()
        );
        let mut stmt=reopened.prepare("SELECT collection,presence FROM metadata_coverage WHERE collection IN ('source_ids','source_links','source_contributors','source_enclosures','source_transcripts') ORDER BY collection").unwrap();
        let states = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(
            states,
            vec![
                ("source_contributors".into(), "empty".into()),
                ("source_enclosures".into(), "invalid".into()),
                ("source_ids".into(), "populated".into()),
                ("source_links".into(), "null".into()),
                ("source_transcripts".into(), "missing".into())
            ]
        );
        assert!(reopened
            .query_row(
                "SELECT raw_member_json FROM metadata_facts WHERE fact_kind='source_ids'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap()
            .contains("unknown_claim"));
    }
    #[test]
    fn adr_0075_observation_collision_and_interpretation_mismatch_fail_closed() {
        let conn = setup();
        let spec = spec();
        let original = observation(&spec);
        let token = begin_provider_request(&conn, spec.clone()).unwrap();
        let _receipt = record_provider_observation(&conn, token, Arc::clone(&original)).unwrap();
        conn.execute("UPDATE metadata_bodies SET bytes=zeroblob(byte_length)", [])
            .unwrap();
        let token = begin_provider_request(&conn, spec.clone()).unwrap();
        assert!(record_provider_observation(&conn, token, Arc::clone(&original)).is_err());
        conn.execute(
            "UPDATE metadata_bodies SET bytes=?1",
            [original.body.as_ref().unwrap().as_ref()],
        )
        .unwrap();
        conn.execute(
            "UPDATE metadata_facts SET validation='malformed' WHERE fact_kind='source_ids'",
            [],
        )
        .unwrap();
        let token = begin_provider_request(&conn, spec).unwrap();
        assert!(record_provider_observation(&conn, token, original).is_err());
        assert_eq!(
            conn.query_row(
                "SELECT occurrence_count FROM metadata_observations",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
    }
    #[test]
    fn adr_0075_observation_provider_scope_decoder_and_source_revisions_isolate_identity() {
        let conn = setup();
        let mut spec = spec();
        let original = observation(&spec);
        let token = begin_provider_request(&conn, spec.clone()).unwrap();
        let one = record_provider_observation(&conn, token, Arc::clone(&original)).unwrap();
        spec.provider_identity = "https://other.invalid".into();
        spec.request_uri = "https://other.invalid/v1/feeds/f/tracks/t".into();
        let token = begin_provider_request(&conn, spec.clone()).unwrap();
        let two = record_provider_observation(&conn, token, Arc::clone(&original)).unwrap();
        assert_ne!(one.provider_id, two.provider_id);
        for change in ["revision", "time", "decoder"] {
            let mut changed = (*original).clone();
            match change {
                "revision" => changed.source_revision = Some(json!(2)),
                "time" => changed.source_times = json!({"updated_at":123}),
                _ => changed.interpretation["charset"] = json!("windows-1252"),
            };
            let token = begin_provider_request(&conn, spec.clone()).unwrap();
            let receipt = record_provider_observation(&conn, token, Arc::new(changed)).unwrap();
            assert_ne!(receipt.observation_id, two.observation_id);
        }
        assert_eq!(counts(&conn)[..2], [1, 5]);
    }
    #[test]
    fn adr_0075_observation_preserves_all_legacy_and_snapshot_evidence_tables() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
        conn.execute_batch(include_str!("fixtures/adr-0075-schema-11.sql"))
            .unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        crate::db::migrate_schema(&conn).unwrap();
        conn.execute_batch(super::super::provider_snapshot_schema::RETAINED_ROWS)
            .unwrap();
        conn.execute_batch("INSERT INTO metadata_facts(id,observation_id,scope_ordinal,transport_ordinal,declared_subject_id,declared_owner_json,owner_basis_json,fact_kind,representation,validation,value_json,body_locator_json) VALUES(1,2,0,0,1,'{}','{}','description','plain_text','valid','\"kept\"','{}'); INSERT INTO metadata_snapshots VALUES(2,2,1,'field:description','populated',2,0,1); INSERT INTO metadata_snapshot_members VALUES(2,0,1); INSERT INTO metadata_collection_heads VALUES(2,1,'field:description',2,2,2,0,40,'{}',2,'success',2,NULL);").unwrap();
        let names = [
            "artist_source_facts",
            "artist_source_ids",
            "artist_source_links",
            "broadcast_event_selection",
            "broadcast_events",
            "entity_contributors",
            "entity_identity_ids",
            "entity_identity_links",
            "entity_metadata_facts",
            "feeds",
            "local_files",
            "local_path_repairs",
            "playback_sessions",
            "playlist_tracks",
            "playlists",
            "schema_migrations",
            "schema_version",
            "track_artist_source_bindings",
            "tracks",
            "metadata_providers",
            "metadata_resources",
            "metadata_subjects",
            "metadata_bodies",
            "metadata_request_slots",
            "metadata_observations",
            "metadata_coverage",
            "metadata_facts",
            "metadata_snapshots",
            "metadata_snapshot_members",
            "metadata_collection_heads",
            "metadata_field_selections",
            "metadata_discrepancies",
            "metadata_discrepancy_transitions",
        ];
        let limits = names
            .iter()
            .map(|name| {
                conn.query_row(
                    &format!("SELECT coalesce(max(rowid),0) FROM {name}"),
                    [],
                    |r| r.get::<_, i64>(0),
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let snapshot = |conn: &Connection| {
            names
                .iter()
                .zip(&limits)
                .map(|(name, limit)| {
                    let mut stmt = conn
                        .prepare(&format!(
                            "SELECT * FROM {name} WHERE rowid<=?1 ORDER BY rowid"
                        ))
                        .unwrap();
                    let count = stmt.column_count();
                    stmt.query_map([limit], |r| {
                        (0..count)
                            .map(|i| r.get::<_, SqlValue>(i))
                            .collect::<rusqlite::Result<Vec<_>>>()
                    })
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .unwrap()
                })
                .collect::<Vec<_>>()
        };
        let before = snapshot(&conn);
        let spec = spec();
        let original = observation(&spec);
        let token = begin_provider_request(&conn, spec.clone()).unwrap();
        let _receipt = record_provider_observation(&conn, token, Arc::clone(&original)).unwrap();
        let mut failed = (*original).clone();
        failed.fail("json_decode");
        let token = begin_provider_request(&conn, spec).unwrap();
        let _receipt = record_provider_observation(&conn, token, Arc::new(failed)).unwrap();
        assert_eq!(snapshot(&conn), before);
    }
}
