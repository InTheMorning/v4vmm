# ADR 0075 Metadata Contract Review

## Status

Initial audit recorded - 2026-09-19. ADR 0075 is Proposed. The first packet is Draft.
Implementation has not started. No migration, deployment or new visual acceptance is recorded.

## Evidence And Limits

At the audit, the app checkout was clean at commit `7b95b25`.
The Stophammer checkout was clean at commit `a220f44`.
The agent did not inspect the deployed MusicIndex revision or current RSS contents.
The agent did not launch the app or a browser.

The operator supplied the [MoeFactz API evidence](adr-0037-review-checklist.md#task-002-contributor-source-check--2026-09-19).
MusicIndex supplies three contributor credits attached to the track. Two carry
HeyCitizen's npub. The host credit carries `https://www.moefactz.com/`.
Those facts do not establish a website or Nostr identity owned by the track itself.
The origin of nine stored local track Nostr facts remains unverified.

## Findings From Source Inspection

### Contributor Fields Are Lost

The app's `src/api.rs::Contributor` omits fields that Stophammer's
`src/query.rs::SourceContributorClaimResponse` supplies. These include the owner,
position, normalised role, source, extraction path and observation time.
Packet 001 preserves these fields when decoding and serialising API data.

Storage has a separate problem. `src/identity_ingest.rs::persist_contributors`
assigns positions from list order and sets observation time to `None`.
The types in `src/local_identity.rs` do not retain typed claim ownership.
The storage packet must preserve the supplied evidence.

### Returned Credits Can Belong To The Feed

Stophammer's `src/db.rs::get_effective_source_contributor_claims_for_track`
can return feed credits for a track. Each credit must retain its declared owner.
Contract tests must cover this case.

### RSS Extraction Can Assign The Wrong Owner

The app's `src/rss/enrich.rs::nostr_from_extension` scans arbitrary nested values.
`enrich_track_from_feed_rss` can assign a person's npub to the track with an
invented `podcast:txt` extraction path. The parser packet must correct this rule.

### Website Rules Differ

Stophammer's parser `extract_links` emits item links as `web_page`.
The app's `src/rss/subscribe.rs::rss_track_link_inputs` stores `item.link` as a
single metadata value, without a corresponding page identity fact.
The shared field rules must also cover `src/views.rs::website_url_from_links`.

### Feed Defaults Can Appear As Track Facts

The app's `src/api.rs::track_with_feed_defaults` can place feed identity,
credits, description and image in the track's API data object.
The refactor must keep stored source facts separate from values selected for display.

### An Empty Refresh Can Leave Old Index Facts

The app's `src/identity_ingest.rs::persist_source_links` and `persist_source_ids`
use source labels to select replacement groups.
`src/db.rs::replace_local_identity_links` removes only the `musicindex` group
when an Index collection is empty. Earlier `rss_link` and `podcast_txt` groups can remain.
The storage packet must replace the complete collection from that provider.

### Source Order Can Select The Displayed Identity

The app's `src/db.rs::local_identity_links` orders facts by source label.
The selectors `src/views.rs::website_url_from_links` and `nostr_npub_from_ids`
can use that alphabetical order to select an identity.
The shared display rules must define source selection and conflict handling explicitly.

### Repeated Requests Can Still Omit Required Facts

The app's `src/application/queries/search.rs::fetch_index_track_result_rows`
requests full details one track at a time.
Its `fetch_index_track_detail` calls do not request the identity collections.
The request packet must define required collections and measure request counts.

### Index Details Omit Contributor Sections

The app's `src/app/search_dispatch.rs::index_track_detail_slots` omits contributor sections.
Preserving API fields alone will not add contributor actions.
The later presentation packet must provide shared actions with owner labels.

### The Tag Summary Is Incomplete For Display

The app's `src/metadata.rs::musicindex_contributors_id3_value` keeps contributor names and roles only.
That tag summary cannot represent complete contributor facts.
The refactor must keep tag export separate from data prepared for display.

### RSS Errors Are Ignored

The app's `src/subscribe_service.rs::enrich_track_context_from_rss` ignores enrichment errors.
Missing data and failed requests can therefore appear alike.
The request result must distinguish those states.

These findings come from source inspection. They do not establish corruption
in production data or the contents of every Index record.

## Contract Cases Required Before Completion

- [ ] Identity facts retain their feed, track or contributor owner.
- [ ] Item credits replace feed credits for display without deleting feed facts.
- [ ] Matching names or keys do not merge separate credits from a source.
- [ ] `website`, `web_page`, transcript and enclosure links keep distinct kinds.
- [ ] Valid npub, nprofile, invalid values and unsupported syntax stay distinguishable.
- [ ] Namespace prefixes do not determine RSS semantics.
- [ ] Not requested, absent/null, empty, populated and failed responses stay distinct.
- [ ] An empty refresh replaces only the matching collection from that provider.
- [ ] Older request completion cannot replace a newer snapshot.
- [ ] Persisted facts retain meaning after restart and endpoint changes.
- [ ] Feed-scoped track IDs prevent cross-feed collisions.
- [ ] Each field has an explicit fallback rule that preserves stored ownership.
- [ ] Same facts produce the same actions through local and Index routes.
- [ ] Request counts establish the claimed performance improvement.
- [ ] Repair reports preserve unresolved records and do not modify audio files.
- [ ] Upstream parser changes include recrawl, signed-event and replica verification.

## Open Product Decisions

The operator's placement preference remains open. One option shows related
identities on the track page with explicit owner labels. The other keeps the
header limited to track identities and puts related identities in separate sections.
The working proposal uses the first option. The operator has not accepted it.

The parser packet must specify supported Nostr purpose values from the contract
and checked examples. Later packets must define fallback for each field, source
selection and the report for repairing old records before implementation.

## Review Disposition

The audit establishes the need to correct metadata handling. It does not establish
acceptance of every proposed rule. ADR 0075 remains Proposed. The first packet
is Draft. It only preserves fields in the app's type for API data.
Existing acceptance gates remain open.

## Operator Visual Check

The operator paused visual checks on 2026-09-19. Do not request another visual batch until
the metadata work is ready and the operator resumes those checks.
The current fixture is `/tmp/v4vmm-governance.ie6k8TQf`. Cleanup is unconfirmed.

## References

- [Proposed ADR](../adr/0075-metadata-ownership-and-completeness.md)
- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md)
- [First packet](../tasks/adr-0075-task-001-contributor-claim-transport.md)
- [Podcast person semantics](https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/person.md)
- [Podcast txt semantics](https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/txt.md)
- [NIP-19](https://github.com/nostr-protocol/nips/blob/master/19.md)
