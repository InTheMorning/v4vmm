# ADR 0075 Metadata Example Corpus

## Scope

This corpus holds 21 original cases and five correction scenarios for RSS and MusicIndex metadata.
Each case records the declared owner, the current result and the required result.
[ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) governs each required result.
The cited source functions establish each current result.

The corpus separates current results from the results that ADR 0075 requires.
A current result is a statement about code. A required result is a statement about the accepted rule.
The corpus does not claim that production data is correct.
The corpus does not claim that a required result is present in the code today.

Each source form comes from supplied evidence or from a constructed example of a cited rule.
A constructed example carries the label "Constructed example".
A constructed example is not a captured production response.
Only case C01 uses supplied evidence.

This corpus changes no code, no schema and no acceptance gate.
Later packets use the recorded differences to identify each necessary correction.

## Evidence

| Source | Revision | Use |
|---|---|---|
| This repository, working tree | After ADR 0075 packet 001 | Current app behavior |
| `/home/citizen/build/stophammer` | Commit `a220f44` | Current upstream parser and API behavior |
| `rss` crate | Version 2.0.12, as pinned by `Cargo.lock` | Current XML extension behavior |
| [Task 001 supplied response](../tasks/adr-0075-task-001-contributor-claim-transport.md#supplied-response) | Supplied on 2026-09-19 | The C01 source form |

The enabled `rss` crate features in this build are `atom_syndication`, `builders`,
`default`, `derive_builder` and `never`. The `atom` feature is not enabled.
`cargo tree --offline -e features -i rss` reports these features.

This corpus records no live network request. No agent launched the app.

## Summary Index

| Case | Scenario | ADR 0075 rule |
|---|---|---|
| [C01](#c01) | The three supplied MoeFactz contributor credits | Decision 1, contributor occurrence |
| [C02](#c02) | A Nostr identity that belongs only to a feed | Decision 3, direct channel `podcast:txt` |
| [C03](#c03) | A Nostr identity that belongs only to a track | Decision 3, direct item `podcast:txt` |
| [C04](#c04) | Duplicate roles for one contributor name | Decision 1, contributor occurrence |
| [C05](#c05) | Two names for one Nostr key | Decision 1, no identity from a matching key |
| [C06](#c06) | Two Nostr keys for one name | Decision 1, no identity from a matching name |
| [C07](#c07) | A channel RSS `link` | Decision 3, feed `website` |
| [C08a](#c08a) | An item RSS `link` | Decision 3, track `web_page` |
| [C08b](#c08b) | An item Atom alternate link | Decision 3, track `web_page` |
| [C09](#c09) | A malformed Nostr identifier | Decision 3, unknown syntax stays evidence |
| [C10](#c10) | An alternate XML namespace prefix | Decision 3, unknown syntax stays evidence |
| [C11](#c11) | A collection that the request did not ask for | Decision 2, not requested |
| [C12](#c12) | A collection returned as JSON `null` | Decision 2, not returned |
| [C13](#c13) | A collection returned as an empty array | Decision 2, returned empty |
| [C14](#c14) | A collection returned with entries | Decision 2, returned populated |
| [C15](#c15) | A failed request | Decision 2, request failed |
| [C16](#c16) | A malformed payload with HTTP 200 | Decision 2, request failed |
| [C17](#c17) | One track GUID under two feeds | Decision 1, subject |
| [C18a](#c18a) | A track with its own credits | Decision 4, item credits replace channel credits |
| [C18b](#c18b) | The same track with feed credits only | Decision 4, preserve the original owner |
| [C18c](#c18c) | The same track with no credits | Decision 2, returned empty |

## Cases

### C01

**Source Form**
Supplied evidence. See the
[task 001 supplied response](../tasks/adr-0075-task-001-contributor-claim-transport.md#supplied-response).
It holds three `source_contributors` entries. Two entries name HeyCitizen and carry one npub.
The third entry names Moe Factz, carries `https://www.moefactz.com/` as `href`, and carries no npub.

**Declared Owner**
Each credit declares `entity_type` `track` and `entity_id` `d489101a-4e62-492f-812e-9fe51def9423`.
Upstream `src/api.rs` lines 630 to 661 write `source` as `podcast_person`.
It writes `extraction_path` as `{entity_type}.podcast:person`.

**Field Kind**
Contributor occurrence. ADR 0075 defines it as the subject, the source collection and the position.

**Current Stored Fact**
`src/identity_ingest.rs::persist_contributors`, line 258, writes six typed columns.
It takes the position from the list index and sets the observation time to `None`.
It writes `raw_json` from the data transfer object (DTO).
Packet 001 added the seven claim fields to that DTO, so `raw_json` of a new record holds them.
`src/db.rs::LocalContributorRow`, line 232, has no column for the declared owner,
the normalized role, the assertion source or the extraction path.

**Current Display Result**
`src/local_identity.rs::facts_for_owner`, line 15, maps each row to `ContributorView`.
`src/views.rs::ContributorView`, line 56, holds six fields and no provenance.
The two HeyCitizen credits group into one person.
`src/view_models/entity_detail.rs::people`, line 1167, groups rows by display name.

**Required Stored Fact**
Keep the declared owner, the supplied position and the normalized role as typed facts.
Also keep the assertion source, the extraction path and the observation time.
Keep the two HeyCitizen credits as two occurrences.

**Required Display Result**
Show a contributor section that names the contributor, the role and the source.
The npub stays a contributor identity. It does not become a track identity.

**ADR 0075 Rule**
[Decision 1](../adr/0075-metadata-ownership-and-completeness.md#1-preserve-the-subject-and-the-source).
A contributor occurrence does not establish a global person identity.

### C02

**Source Form**
Constructed example of the channel `podcast:txt` rule.

```xml
<channel>
  <title>Example Feed</title>
  <podcast:txt purpose="npub">npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkwsyjh6w6</podcast:txt>
  <item>
    <guid>item-1</guid>
  </item>
</channel>
```

**Declared Owner**
Feed. Upstream `stophammer-parser/src/engine.rs::extract_entity_ids`, line 802, reads
only direct element children of the node that the caller passes.
The caller passes the channel node for this case.

**Field Kind**
Entity identity with scheme `nostr_npub`.

**Current Stored Fact**
The upstream parser produces a feed-owned identity with scheme `nostr_npub`.
The app's RSS subscribe path stores no identity value.
`src/rss/subscribe.rs::persist_rss_feed_identity`, line 346, calls only
`replace_local_contributors` and `replace_local_identity_links`.
Through the Index route, `src/identity_ingest.rs::persist_source_ids`, line 224,
stores the value with the declared owner from the API.
Through the RSS enrichment route, `src/rss/enrich.rs::enrich_track_from_feed_rss`, line 102,
writes the feed identity with the extraction path `podcast:txt@purpose=nostr`.

**Current Display Result**
`src/views.rs::nostr_npub_from_ids`, line 333, returns the first `nostr_npub` value in row order.
`src/db.rs::local_identity_ids`, line 1568, orders rows by `source COLLATE NOCASE, position, id`.
The alphabetical source label can therefore select the displayed value.

**Required Stored Fact**
Keep the identity with the feed as the declared owner.
Keep the assertion source and the exact extraction path of the source element.

**Required Display Result**
Show the value in a feed section that names the feed as the owner.
Do not show the value in the track header.

**ADR 0075 Rule**
[Decision 3](../adr/0075-metadata-ownership-and-completeness.md#3-use-explicit-field-rules),
the direct channel `podcast:txt` row.
[Decision 4](../adr/0075-metadata-ownership-and-completeness.md#4-keep-fallback-out-of-stored-facts)
requires the owner label.

### C03

**Source Form**
Constructed example of the item `podcast:txt` rule.

```xml
<item>
  <guid>item-1</guid>
  <podcast:txt purpose="npub">npub10elfcs4fr0l0r8af98jlmgdh9c8tcxjvz9qkw038js35mp4dma8qzvjptg</podcast:txt>
</item>
```

**Declared Owner**
Track. `extract_entity_ids`, line 802, reads only direct children of the item node.

**Field Kind**
Entity identity with scheme `nostr_npub`.

**Current Stored Fact**
The upstream parser produces a track-owned identity.
The app's RSS subscribe path stores no identity value for this element.
`src/rss/enrich.rs::enrich_track_from_feed_rss`, line 99, writes a track identity with the
extraction path `podcast:txt@purpose=nostr`.
That path is a constant. It does not record the element that supplied the value.

`src/rss/enrich.rs::nostr_from_extensions`, line 320, reads every element under the
`podcast` extension key, not only `txt`.
The extension map orders element names alphabetically, so a `person` element precedes a `txt` element.
An item with a `podcast:person` npub attribute therefore supplies the track identity value.

**Current Display Result**
`nostr_npub_from_ids`, line 333, returns the first `nostr_npub` value in row order.
A contributor's key can appear as the track's identity through the path above.

**Required Stored Fact**
Keep the identity with the track as the declared owner.
Record the exact extraction path of the `podcast:txt` element.
Do not record a contributor's key as a track identity.

**Required Display Result**
Show the value in the track header, because the track owns it.

**ADR 0075 Rule**
[Decision 3](../adr/0075-metadata-ownership-and-completeness.md#3-use-explicit-field-rules),
the direct item `podcast:txt` row. The review records this difference in
[RSS Extraction Can Assign The Wrong Owner](../reviews/adr-0075-metadata-contract-review.md#rss-extraction-can-assign-the-wrong-owner).

### C04

**Source Form**
Constructed example of two credits with one name and one role.

```xml
<item>
  <guid>item-1</guid>
  <podcast:person role="musician" group="music">HeyCitizen</podcast:person>
  <podcast:person role="host" group="cast">Moe Factz</podcast:person>
  <podcast:person role="musician" group="music">HeyCitizen</podcast:person>
</item>
```

**Declared Owner**
Track. `stophammer-parser/src/engine.rs::extract_persons`, line 779, reads direct
`person` children of the item node and numbers them from zero.
The two HeyCitizen credits take positions 0 and 2.

**Field Kind**
Contributor occurrence.

**Current Stored Fact**
Upstream `src/api.rs`, lines 630 to 661, renumbers the positions in list order.
The app's `persist_contributors`, line 258, also takes the position from the list index.
Both credits remain separate rows. No column records the supplied position.

**Current Display Result**
`src/view_models/entity_detail.rs::people`, line 1167, groups both rows under one name.
`roles`, line 1228, removes a repeated role label.
The display shows one person with one role. It does not show two occurrences.

**Required Stored Fact**
Keep two occurrences with their supplied positions 0 and 2.

**Required Display Result**
Show that the source states this credit two times, or show each occurrence.
Do not represent two source claims as one claim without evidence.

**ADR 0075 Rule**
[Decision 1](../adr/0075-metadata-ownership-and-completeness.md#1-preserve-the-subject-and-the-source).
A contributor occurrence is the subject, the source collection and the position.

### C05

**Source Form**
Constructed example of one key with two names.

```xml
<item>
  <guid>item-1</guid>
  <podcast:person role="musician" npub="npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkwsyjh6w6">HeyCitizen</podcast:person>
  <podcast:person role="producer" npub="npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkwsyjh6w6">Hey Citizen Music</podcast:person>
</item>
```

**Declared Owner**
Track. Each credit is a separate occurrence at positions 0 and 1.

**Field Kind**
Contributor occurrence with a contributor identity.

**Current Stored Fact**
`persist_contributors`, line 258, writes two rows. Each row keeps its own name and npub.
No code merges the two rows by key.

**Current Display Result**
`people`, line 1167, groups by display name, so the display shows two persons.
`src/view_models/entity_detail.rs::identity_actions`, line 1095, gives each row its own
website action and Nostr action.

**Required Stored Fact**
Keep both occurrences and both names. Keep the shared key on each occurrence.

**Required Display Result**
Show both credits. Do not state that the two credits are one person.

**ADR 0075 Rule**
[Decision 1](../adr/0075-metadata-ownership-and-completeness.md#1-preserve-the-subject-and-the-source).
Do not infer identity from a matching public key.

### C06

**Source Form**
Constructed example of one name with two keys.

```xml
<item>
  <guid>item-1</guid>
  <podcast:person role="musician" npub="npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkwsyjh6w6">HeyCitizen</podcast:person>
  <podcast:person role="producer" npub="npub10elfcs4fr0l0r8af98jlmgdh9c8tcxjvz9qkw038js35mp4dma8qzvjptg">HeyCitizen</podcast:person>
</item>
```

**Declared Owner**
Track. Each credit is a separate occurrence at positions 0 and 1.

**Field Kind**
Contributor occurrence with a contributor identity.

**Current Stored Fact**
`persist_contributors`, line 258, writes two rows with the same name and different keys.

**Current Display Result**
`people`, line 1167, groups both rows under one name.
`ContributorPersonVm::row_display` uses the first row for the person's website link.
The view model keeps a separate action list for each row through `identity_actions`, line 1095.
The rendered result for the second key is not established by this corpus.

**Required Stored Fact**
Keep both occurrences and both keys. Do not select one key as the person's key.

**Required Display Result**
Show each key with its role context. Show the unresolved conflict to the operator.

**ADR 0075 Rule**
[Decision 1](../adr/0075-metadata-ownership-and-completeness.md#1-preserve-the-subject-and-the-source).
Do not infer identity from a matching name.

### C07

**Source Form**
Constructed example of a channel `link`.

```xml
<channel>
  <title>Example Feed</title>
  <link>https://example.test/feed-home</link>
</channel>
```

**Declared Owner**
Feed.

**Field Kind**
Website link.

**Current Stored Fact**
`src/rss/subscribe.rs::rss_feed_link_inputs`, line 415, writes `link_type` `website`,
`entity_type` `feed`, position 0 and extraction path `channel/link`.
`persist_rss_feed_identity`, line 346, writes the row with the source label `rss`.
Upstream `extract_links`, line 826, writes `link_type` `website` and extraction path `feed.link`.
The two extraction path values differ. The app writes `channel/link`. The upstream parser writes `feed.link`.

**Current Display Result**
`src/views.rs::website_url_from_links`, line 343, returns the first `website` link in row order.
`src/db.rs::local_identity_links`, line 1538, orders rows by `source COLLATE NOCASE, position, id`.

**Required Stored Fact**
Keep the value as a feed `website` fact with its source and extraction path.

**Required Display Result**
Show the link in a feed section with a feed owner label.

**ADR 0075 Rule**
[Decision 3](../adr/0075-metadata-ownership-and-completeness.md#3-use-explicit-field-rules),
the channel RSS `link` row.

### C08a

**Source Form**
Constructed example of an item `link`.

```xml
<item>
  <guid>item-1</guid>
  <link>https://example.test/episode-1</link>
</item>
```

**Declared Owner**
Track.

**Field Kind**
Page link.

**Current Stored Fact**
`src/rss/subscribe.rs`, line 183, reads `item.link()` into `item_link`.
The upsert at line 233 writes that value to the `tracks.link` column.
`rss_track_link_inputs`, line 435, builds only a transcript link.
The app writes no track `web_page` identity fact.
Upstream `extract_links`, line 826, writes `link_type` `web_page` and extraction path `entity.link`.

**Current Display Result**
The track has no `web_page` identity fact, so no page action is available from that fact.
`website_url_from_links`, line 343, selects only a `website` link.

**Required Stored Fact**
Keep the value as a track `web_page` fact with its source and extraction path.

**Required Display Result**
Give the track an action that opens the page.

**ADR 0075 Rule**
[Decision 3](../adr/0075-metadata-ownership-and-completeness.md#3-use-explicit-field-rules),
the item RSS `link` row. The review records this difference in
[Website Rules Differ](../reviews/adr-0075-metadata-contract-review.md#website-rules-differ).

### C08b

**Source Form**
Constructed example of an item Atom alternate link.

```xml
<item xmlns:atom="http://www.w3.org/2005/Atom">
  <guid>item-1</guid>
  <atom:link rel="alternate" href="https://example.test/episode-1" />
</item>
```

**Declared Owner**
Track.

**Field Kind**
Page link.

**Current Stored Fact**
The `atom` feature of the `rss` crate is not enabled in this build.
`rss-2.0.12/src/item.rs`, lines 650 to 671, therefore stores the element in
`item.extensions()` under the key `atom`.
No app function reads that key. `rss_track_link_inputs`, line 435, builds only a transcript link.
The app writes no fact for this element.
Upstream `extract_links`, line 826, writes `link_type` `web_page` and extraction path
`entity.atom:link[@rel='alternate']`.

**Current Display Result**
The track has no `web_page` identity fact, so no page action is available from that fact.

**Required Stored Fact**
Keep the value as a track `web_page` fact with its source and extraction path.

**Required Display Result**
Give the track an action that opens the page.

**ADR 0075 Rule**
[Decision 3](../adr/0075-metadata-ownership-and-completeness.md#3-use-explicit-field-rules),
the supported Atom alternate link row.

### C09

**Source Form**
Constructed example of a malformed Nostr identifier.

```xml
<item>
  <guid>item-1</guid>
  <podcast:txt purpose="npub">npub1notavalidkey</podcast:txt>
</item>
```

The value keeps the `npub1` prefix. Its length and its checksum are not valid.

**Declared Owner**
Track.

**Field Kind**
Entity identity with scheme `nostr_npub`.

**Current Stored Fact**
Upstream `extract_entity_ids`, line 802, reads the text and applies no syntax check.
The app's `src/rss/enrich.rs::extract_nostr_handle`, line 365, accepts any alphanumeric
run after `npub1` that is longer than the prefix. It applies no checksum check.
This repository holds no Nostr syntax check.
No column records whether the app supports the value.

**Current Display Result**
`nostr_npub_from_ids`, line 333, returns the value. The display shows it as an identity.
The operator cannot see that the value is not valid.

**Required Stored Fact**
Keep the value as evidence. Record whether the app supports the syntax.

**Required Display Result**
Do not present unsupported syntax as an active identity.
Keep the evidence available to the operator.

**ADR 0075 Rule**
[Decision 3](../adr/0075-metadata-ownership-and-completeness.md#3-use-explicit-field-rules),
the unknown links, identifiers or extension syntax row.

### C10

**Source Form**
Constructed example of an alternate namespace prefix.

```xml
<item xmlns:pc="https://podcastindex.org/namespace/1.0">
  <guid>item-1</guid>
  <pc:txt purpose="npub">npub10elfcs4fr0l0r8af98jlmgdh9c8tcxjvz9qkw038js35mp4dma8qzvjptg</pc:txt>
</item>
```

**Declared Owner**
Track. The prefix does not change the namespace, so the element is a Podcast Namespace element.

**Field Kind**
Entity identity with scheme `nostr_npub`.

**Current Stored Fact**
The `rss` crate keys an unknown extension by the literal XML prefix.
`rss-2.0.12/src/extension/util.rs::extension_name`, line 41, splits the element name at the colon
and returns that prefix. `rss-2.0.12/src/item.rs`, line 670, stores the element under that prefix.
The crate resolves only the Atom, iTunes and Dublin Core namespaces to a namespace URI.

The app's `nostr_from_extensions`, line 320, reads the literal key `podcast`.
`contributor_inputs_from_extensions`, line 390, also reads the literal key `podcast`.
Both lookups therefore find no match for the prefix `pc`. The app stores no fact.
Upstream `is_podcast_namespace`, line 535, compares the resolved namespace URI,
so the upstream parser still reads the element.

**Current Display Result**
The track shows no identity from this element through the RSS route.
The Index route can still supply the value, because the upstream parser reads it.

**Required Stored Fact**
Keep the identity with the track as the declared owner, independent of the XML prefix.

**Required Display Result**
Show the same result as case C03.

**ADR 0075 Rule**
[Decision 3](../adr/0075-metadata-ownership-and-completeness.md#3-use-explicit-field-rules),
the direct item `podcast:txt` row. Alternate namespace syntax must not change the owner.

### C11

**Source Form**
Constructed example of a request that omits the collection.

```text
GET /v1/tracks/{track_guid}
```

**Declared Owner**
Not applicable. This case is about the request, not about one value.

**Field Kind**
Collection state, "not requested".

**Current Stored Fact**
`src/application/queries/search.rs::fetch_index_track_detail`, line 744, passes `None`
for the `include` parameter. The response therefore omits the identity collections.
`src/api.rs::Track`, line 157, holds `Option<Vec<T>>` fields with `#[serde(default)]`.
A missing field decodes as `None`.
`src/identity_ingest.rs::persist_source_ids`, line 224, returns early for `None` and keeps the snapshot.
The stored snapshot is unchanged, which matches the required refresh effect.

**Current Display Result**
The display shows the stored facts. It reports no coverage state.

**Required Stored Fact**
Keep the stored snapshot. Record that this request did not ask for the collection.

**Required Display Result**
Show the stored facts. Do not present them as a fresh complete observation.

**ADR 0075 Rule**
[Decision 2](../adr/0075-metadata-ownership-and-completeness.md#2-make-collection-state-explicit),
the "Not requested" row.

### C12

**Source Form**
Constructed example of a null collection.

```json
{
  "track_guid": "track-1",
  "source_contributors": null
}
```

**Declared Owner**
Not applicable. This case is about the collection, not about one value.

**Field Kind**
Collection state, "not returned".

**Current Stored Fact**
`Option<Vec<T>>` decodes an explicit JSON `null` as `None`.
The `Track` and `Feed` types therefore produce the same value for C11 and for C12.
The current types cannot tell C11 apart from C12.
`persist_contributors`, line 258, returns early for `None` and keeps the snapshot.
No code records incomplete coverage.

**Current Display Result**
The display shows the stored facts. It reports no failed or incomplete coverage.

**Required Stored Fact**
Keep the snapshot. Record incomplete coverage for the requested collection.

**Required Display Result**
Show the stored facts and report that the collection was not returned.

**ADR 0075 Rule**
[Decision 2](../adr/0075-metadata-ownership-and-completeness.md#2-make-collection-state-explicit),
the "Not returned" row.

### C13

**Source Form**
Constructed example of an empty collection.

```json
{
  "track_guid": "track-1",
  "source_ids": []
}
```

**Declared Owner**
Not applicable. This case is about the collection, not about one value.

**Field Kind**
Collection state, "returned empty".

**Current Stored Fact**
`persist_source_ids`, line 224, starts its group map with the `musicindex` key and an empty list.
`src/db.rs::replace_local_identity_ids`, line 1435, deletes only the rows of one owner and one source.
The app therefore replaces the `musicindex` group with an empty group.
Rows with another source label, such as `rss`, stay in place.
`src/identity_ingest.rs::source_token`, line 510, maps a missing source to `musicindex`.

**Current Display Result**
The display can still show an older fact from another source label.

**Required Stored Fact**
Replace the complete collection of that provider with an empty snapshot.
The key for replacement is the provider, the declared subject and the collection kind.

**Required Display Result**
Show no fact from that provider for that collection.

**ADR 0075 Rule**
[Decision 2](../adr/0075-metadata-ownership-and-completeness.md#2-make-collection-state-explicit),
the "Returned empty" row.
[Decision 5](../adr/0075-metadata-ownership-and-completeness.md#5-replace-a-complete-provider-snapshot-atomically)
gives the replacement key.

### C14

**Source Form**
Constructed example of a populated collection.

```json
{
  "track_guid": "track-1",
  "source_ids": [
    {
      "entity_type": "track",
      "entity_id": "track-1",
      "position": 0,
      "scheme": "nostr_npub",
      "value": "npub10elfcs4fr0l0r8af98jlmgdh9c8tcxjvz9qkw038js35mp4dma8qzvjptg",
      "source": "podcast_txt",
      "extraction_path": "track.podcast:txt",
      "observed_at": 1779240280
    }
  ]
}
```

**Declared Owner**
Track. The entry declares `entity_type` `track` and `entity_id` `track-1`.

**Field Kind**
Collection state, "returned populated".

**Current Stored Fact**
`persist_source_ids`, line 224, groups the entry by its `source` value.
The group label becomes `podcast_txt`, not `musicindex`.
The app keeps the declared owner, the position, the extraction path and the observation time.
`src/api.rs::SourceEntityId`, line 299, holds each of those fields.

**Current Display Result**
`nostr_npub_from_ids`, line 333, returns the first matching value in row order.
The row order comes from the source label, then the position.

**Required Stored Fact**
Replace that provider's collection with the returned facts.
Keep the declared owner, the source, the extraction path and the observation time.

**Required Display Result**
Show the value under its declared owner with its source.

**ADR 0075 Rule**
[Decision 2](../adr/0075-metadata-ownership-and-completeness.md#2-make-collection-state-explicit),
the "Returned populated" row.

### C15

**Source Form**
Constructed example of a failed request.

```text
GET /v1/tracks/{track_guid}
HTTP/1.1 503 Service Unavailable
```

**Declared Owner**
Not applicable. This case is about the request outcome.

**Field Kind**
Request outcome, "request failed".

**Current Stored Fact**
`src/api.rs::response_text_with_status`, line 755, returns an error for a status that is not success.
`src/application/queries/search.rs::fetch_index_track_result_rows`, line 659, calls `.ok()` on the
detail result and discards the error.
The detail becomes `None`. No code records the failed request.
`src/subscribe_service.rs::enrich_track_context_from_rss`, line 552, also discards its result.

**Current Display Result**
The display shows the result without detail values.
A failed request looks the same as absent data.

**Required Stored Fact**
Keep the stored facts. Record that no successful observation exists for this request.

**Required Display Result**
Show the known facts and report the failed refresh.

**ADR 0075 Rule**
[Decision 2](../adr/0075-metadata-ownership-and-completeness.md#2-make-collection-state-explicit),
the "Request failed" row.

### C16

**Source Form**
Constructed example of a malformed payload with a success status.

```text
HTTP/1.1 200 OK
Content-Type: application/json

{ "track_guid": "track-1",
```

**Declared Owner**
Not applicable. This case is about the request outcome.

**Field Kind**
Request outcome, "request failed".

**Current Stored Fact**
`src/api.rs::response_json`, line 738, returns a decoding error for a body that is not valid JSON.
A caller that uses `.ok()` turns that error into `None`.
A valid JSON object with unexpected keys decodes without an error, because
`Track` and `Feed` apply `#[serde(default)]` to each field.
That second form therefore looks like an empty response, not like a failure.

**Current Display Result**
The display shows the result without detail values.
A malformed payload looks the same as an empty collection.

**Required Stored Fact**
Treat the malformed payload as a failed request. Keep the stored facts.

**Required Display Result**
Show the known facts and report the failed refresh.
Do not report an empty collection.

**ADR 0075 Rule**
[Decision 2](../adr/0075-metadata-ownership-and-completeness.md#2-make-collection-state-explicit).
A malformed payload is a failed request.

### C17

**Source Form**
Constructed example of one GUID under two feeds.

```xml
<channel>
  <title>Feed A</title>
  <item><guid>shared-guid-1</guid></item>
</channel>
```

```xml
<channel>
  <title>Feed B</title>
  <item><guid>shared-guid-1</guid></item>
</channel>
```

**Declared Owner**
Two different tracks. The subject is the feed GUID with the track GUID.

**Field Kind**
Subject key.

**Current Stored Fact**
`src/rss/subscribe.rs`, line 258, upserts a track with the clause `ON CONFLICT(feed_id, item_guid)`.
The storage key therefore includes the feed.
[The storage document](storage-and-metadata.md) records the same key for the `tracks` table.
`persist_rss_track_identity`, line 367, resolves the track with the feed URL and the item GUID.
Local storage keeps the two tracks separate.

**Current Display Result**
`fetch_index_track_detail`, line 744, uses the feed-scoped route only when the search hit
supplies a feed GUID. Without a feed GUID it requests the track GUID alone.
The selected track for the second feed is therefore not established by this corpus.

**Required Stored Fact**
Keep the two tracks separate. Keep each fact with its own subject.

**Required Display Result**
Show the facts of the requested feed's track only.

**ADR 0075 Rule**
[Decision 1](../adr/0075-metadata-ownership-and-completeness.md#1-preserve-the-subject-and-the-source).
The subject is the feed GUID with the track GUID.

### C18a

**Source Form**
Constructed example of a track with its own credits.

```xml
<channel>
  <podcast:person role="host">Feed Host</podcast:person>
  <item>
    <guid>item-1</guid>
    <podcast:person role="musician">Track Musician</podcast:person>
  </item>
</channel>
```

**Declared Owner**
Track. The credit declares `entity_type` `track` and `entity_id` with the track GUID.

**Field Kind**
Contributor occurrence.

**Current Stored Fact**
Upstream `src/db.rs::get_effective_source_contributor_claims_for_track`, line 5556, first reads
the claims of the track entity. The track has claims, so it returns those claims.
Each claim keeps `entity_type` `track` and the track GUID as `entity_id`.
The app's `persist_contributors`, line 258, stores six typed columns and the DTO JSON.

**Current Display Result**
The contributor section shows the track credits with no owner label.

**Required Stored Fact**
Keep the track credits with the track as the declared owner.

**Required Display Result**
Show the track credits. Name the owner and the source for each credit.

**ADR 0075 Rule**
[Decision 4](../adr/0075-metadata-ownership-and-completeness.md#4-keep-fallback-out-of-stored-facts).
Item credits replace channel credits when item credits are present.

### C18b

**Source Form**
Constructed example of the same track after the source removes its own credits.

```xml
<channel>
  <podcast:person role="host">Feed Host</podcast:person>
  <item>
    <guid>item-1</guid>
  </item>
</channel>
```

**Declared Owner**
Feed. The returned credit declares `entity_type` `feed` and `entity_id` with the feed GUID.

**Field Kind**
Contributor occurrence.

**Current Stored Fact**
`get_effective_source_contributor_claims_for_track`, line 5556, finds no track claim.
It then returns the feed claims from `get_source_contributor_claims_for_feed_entity`, line 5570.
Each returned claim keeps `entity_type` `feed` and the feed GUID as `entity_id`.
Before packet 001, the app's DTO dropped those two fields.
After packet 001, the DTO keeps them, and `raw_json` of a new record holds them.
`LocalContributorRow`, line 232, still has no column for the declared owner.

**Current Display Result**
The contributor section shows the feed credits on the track page with no owner label.
The operator cannot see that the feed owns those credits.

**Required Stored Fact**
Keep the feed as the declared owner of each returned credit.
Do not store the feed credit as a track assertion.

**Required Display Result**
Show the credits in a section that names the feed as the owner.

**ADR 0075 Rule**
[Decision 4](../adr/0075-metadata-ownership-and-completeness.md#4-keep-fallback-out-of-stored-facts).
Preserve the original owner of each credit.

### C18c

**Source Form**
Constructed example of the same track after the source removes the feed credits.

```xml
<channel>
  <item>
    <guid>item-1</guid>
  </item>
</channel>
```

**Declared Owner**
Not applicable. No credit exists.

**Field Kind**
Collection state, "returned empty".

**Current Stored Fact**
`get_effective_source_contributor_claims_for_track`, line 5556, returns an empty list.
The API response holds an empty `source_contributors` array.
`persist_contributors`, line 258, replaces the `musicindex` group with an empty group.
Rows with the source label `rss` stay in place.
`src/api.rs::track_with_feed_defaults`, line 190, replaces an absent track list with the feed list.
That helper acts on the API object before storage.

**Current Display Result**
The display can still show an older credit from the RSS source label.

**Required Stored Fact**
Replace that provider's contributor collection with an empty snapshot.
Keep facts from another provider under their own provider key.

**Required Display Result**
Show no contributor from that provider. Report the empty result.

**ADR 0075 Rule**
[Decision 2](../adr/0075-metadata-ownership-and-completeness.md#2-make-collection-state-explicit),
the "Returned empty" row.
[Decision 5](../adr/0075-metadata-ownership-and-completeness.md#5-replace-a-complete-provider-snapshot-atomically)
gives the replacement key.

## Unresolved Results

These results need a rule that ADR 0075 does not select today.

| Case | Unresolved result | Packet that must propose the rule |
|---|---|---|
| C02 | Nostr source selection when two sources supply different validated keys | Packet 003 syntax contract and packet 020 projection. Decision F does not cover Nostr identities |
| C07 | Accepted: fresh direct RSS website before Index. Within-provider and stale-value details remain proposed | Packet 007 and packet 035 comparison rules |
| C05, C06 | The displayed result for a conflict between credits | Packet 003, Nostr syntax and purposes |
| C09 | The supported Nostr syntax, and the display of unsupported syntax | Packet 003, Nostr syntax and purposes |
| C11, C12 | Resolved by request intent. Retain the sent include list with response coverage | Packet 004, implemented later by packets 011 and 017 |

An unresolved result does not permit an invented result.
Each case above still records its current result.

## Correction Regression Scenarios

These constructed scenarios extend the original cases. They define required checks, not current app behavior.
Each implementation packet must convert its applicable scenario into a test at the owning layer.

| Case | Input or transition | Required result and owner |
|---|---|---|
| C19 | Primary `application/pdf` enclosure, then `audio/mpeg` alternate. The selecting operation supports MP3 only | Select the MP3 with its own MIME type and byte count. No action is available when all collection and scalar candidates are unsupported. Packet 020, Decision E |
| C20 | RSS description `<p>Hello <b>world</b></p>` and Index description `Hello world` | Equal readable text creates no discrepancy. Preserve both original strings. Packet 035 defines normalization, packet 036 implements it |
| C21 | Same-owner RSS description `New text` and Index description `Old text`. Repeat the pair, restart, then observe `New text` from both | Fresh RSS supplies display text. One active discrepancy survives repeat and restart. Agreement resolves it and retains its evidence. Packets 011 and 036 |
| C22 | An active description or website discrepancy followed by HTTP 503, an omitted include, or unknown owner coverage | Preserve facts and discrepancy evidence. Do not create or resolve a mismatch from missing evidence. Packets 004, 017, and 036 |
| C23 | A track has no artwork while its feed has an image. Later both owners assert the same URL | Preserve separate owner facts and absent track coverage. URL equality never establishes ownership. Packet 008 upstream request and packet 011 storage |

Validate the positive Nostr examples and intentional negative examples with:

```bash
python3 docs/runbooks/check-adr0075-identity-examples.py
```

The [syntax contract](adr-0075-identity-syntax-contract.md#checked-encoding-vectors) records the decoded public keys and separate profile vector.
The guard checks encoding examples only. The scenarios above still need implementation tests.

## Retained Terms

The shared STE checker reports lexical findings for these technical names.
The [review](../reviews/adr-0075-metadata-contract-review.md#verification) records the
terms that the earlier packets retain. This corpus adds the terms below.

| Retained term | Meaning in this corpus |
|---|---|
| source form | The RSS or JSON syntax that a case supplies |
| declared owner | The subject that the source states for a value |
| field kind | A data classification named by ADR 0075 |
| collection state | Whether a response returned the complete collection |
| occurrence | One credit in one source observation, at one position |
| state | A named collection state or request outcome from ADR 0075 |
| alternate | The Atom `rel="alternate"` link relation |
| valid | Correct against the syntax of the named identifier |
| failed | The recorded outcome of a request with no successful observation |
| order | The sequence of rows or credits that the code produces |
| action | A typed control that the view model exposes |
| prefix | The XML namespace prefix in an element name |
| provider | The service or resource that delivered the facts |

Do not replace a retained term with an unrelated dictionary alternative.

## Checks

The link check and the STE check ran on this document. Their results are in the
[packet report](../tasks/adr-0075-task-002-shared-example-corpus.md).
A person has not reviewed the current results against the cited functions.
That review gate stays open.
