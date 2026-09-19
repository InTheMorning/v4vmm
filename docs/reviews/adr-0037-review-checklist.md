# ADR 0037 Review Checklist

## Gate Status

Open for task 002 - updated 2026-09-19. Task 001 is complete, including local
and Index feed checks in both themes and confirmed fixture cleanup.
Task 002 track-detail parity retains its separate operator checks.

The operator paused visual checks on 2026-09-19 and prioritised the
[metadata contract refactor](adr-0075-metadata-contract-review.md).
Task 002 and its fixture cleanup remain open.

## Requirement Disposition

| Earlier requirement | Disposition and owner |
|---|---|
| Separate Library and Discover screens and four specifically named screen screenshots | Retired by ADRs 0047/0048/0060. Compare local and Index origins inside Music, in both themes; record entity identity and entry route |
| src/ui_entity.rs, src/ui_track.rs, src/search.rs, and renderer-supplied identity prefixes | Retired by ADR 0038 helper/display migration and ADR 0047 screen retirement. Current helpers are in src/ui/shells/entity.rs and src/ui/shells/track.rs |
| Website/Nostr/RSS payloads, shared identity controls, missing-local-fact regression | Retained on current local and Index paths |
| Track header/action/section parity | Retained for the same source facts; origin differences must not become different shared layouts |
| Library-only advanced panels and Discover-specific navigation controls | Replaced by ADR 0047's download-dependent disclosure and frame navigation; do not demand retired screen-local controls |
| Light/dark proof using populated identity fixtures | Retained; absent source facts do not prove hydration failure or success |

## Mechanical Ownership

Existing guards in tests/architecture_tests.rs:

- release_feed_identity_actions_use_shared_renderer
- track_identity_links_use_shared_renderer

Identity payload tests remain in src/view_models/entity_detail.rs and
src/view_models/track_detail.rs. Local paths are
src/ui/shells/library/feed_detail.rs and track_detail.rs; Index details use
src/ui/shells/search_results_inspector.rs and the same shared helpers.

## Operator Visual Check

Follow [Identity And Detail Parity](../runbooks/inherited-ui-checks.md#identity-and-detail-parity--adr-0037-tasks-001-and-002).

| Task | Light | Dark | Required fixture |
|---|---|---|---|
| 001: local/Index feed identity and hydration | Local and Index controls, Nostr, Website and RSS passed. Corrected launch and Firefox accepted. | Local and Index controls, Nostr, Website and RSS passed. | Same feed with known Website/Nostr/RSS facts. Fixture removal confirmed on 2026-09-19. |
| 002: local/Index track detail parity | Open | Open | Same track, known Website/Nostr facts and a downloaded local copy |

Capture each entry route, not just one shared shell. Check link/copy targets
and contextual disclosure without requiring different source claims to match.

## Evidence

### Task 001 Fixture Reference — 2026-09-19

The operator confirmed the private database and audio copy at
`/tmp/v4vmm-governance.JN95oN81`. A read-only query of the copied database
reported the following feed with tracks in the local library:

- Title: `The Heycitizen Experience`.
- Local feed ID: `2`.
- Feed GUID: `a2d2e313-9cbd-5169-b89c-ab07b33ecc33`.
- Stored RSS URL: `https://files.heycitizen.xyz/Songs/Albums/The-Heycitizen-Experience/the_heycitizen_experience.xml`.

| Fact | Stored source | Value |
|---|---|---|
| Website | `rss` | `https://v4vmusic.com/?publisher=cmne6j9cn9h2bod0i6hjev8uh` |
| Website | `rss_link` | `https://v4vmusic.com/?publisher=cmne6j9cn9h2bod0i6hjev8uh` |
| Nostr | `podcast_txt` | `npub12um9zqae9uaydfszralpn0e0r90d559gd4qsrzar0j2yvut7t2zqwff5ck` |
| Nostr | `rss` | `npub12um9zqae9uaydfszralpn0e0r90d559gd4qsrzar0j2yvut7t2zqwff5ck` |

These are the stored facts before the fixture app opens. The operator's
query output establishes fixture contents. It does not establish visible
controls, action targets or current Index facts. The operator batches below
record visual acceptance. The operator confirmed fixture cleanup after those
checks. Task 002 retains its separate gate.

### Task 001 Operator Batches — 2026-09-19

The operator reported `pass` for the Light local-feed batch at Medium scale
and normal width. The instructed entry route was toolbar search for
`heycitizen`, then Feeds, then the `The Heycitizen Experience` result marked
`Library`.

The operator accepted visible, readable Website, Nostr and RSS controls.
The Nostr action copied the identifier matching the stored fixture reference
when pasted into an unsaved document.

That batch did not test the local Website or RSS action.
This record uses the operator's report. The agent did not run the app.

During the next Website/RSS batch, the operator reported that the action
opened Chromium instead of the desktop's default browser. The report did not
identify which action was used or confirm either URL target.

The shared feed controls call `open::that` in `src/ui/shells/entity.rs`.
The pinned `open` 5.3.3 Linux implementation tries `xdg-open` first.
The fixture launch overrides `XDG_CONFIG_HOME` and `XDG_DATA_HOME`.
The operator then reported these read-only query results:

| Query | Handler |
|---|---|
| Desktop default browser | `firefox.desktop` |
| Desktop HTTPS handler | `firefox.desktop` |
| Fixture HTTPS handler | `chromium-snapshot-bin.desktop` |

These results confirm that the fixture environment changes handler selection.
The earlier launch omitted desktop lookup paths. The corrected runbook launch
adds the normal config and data directories to the XDG lookup paths.
The app keeps its private configuration, database, music and cache directories.
The command changes no desktop defaults.

A synthetic `xdg-mime` lookup check passed for desktop, isolated and corrected
environments. The agent launched no browser. The check removed its temporary
files. This mechanical result does not establish operator acceptance.

The operator then reported `pass` for the corrected launch and Light local
Website check. The launch required `firefox.desktop` before opening the app.
The operator accepted Firefox opening the Website URL in the fixture reference.
The instructed entry route remained toolbar search for `heycitizen`, then the
`The Heycitizen Experience` feed result marked `Library`.
The operator reported no redirect detail.

The fixture browser correction is accepted. The runbook's
[browser selection check](../runbooks/inherited-ui-checks.md#browser-selection-before-identity-checks)
retains the handler comparison and actual-browser check for regression.
The operator then reported `pass` for the Light local RSS check on the same
Library-origin feed. The operator accepted Firefox opening the stored RSS URL
in the fixture reference. The operator reported no downloaded file path.

The operator then reported `pass` for the Light Index-feed controls and
Nostr copy. The instructed entry route was toolbar search for `heycitizen`,
then Feeds, then the `The Heycitizen Experience` result marked `Index`.
The check used normal width and Medium scale. The operator accepted visible,
readable Website, Nostr and RSS controls. The copied Nostr identifier matched
the fixture reference when pasted into the unsaved scratch document.

The operator then reported `pass` for the Light Index Website and RSS actions.
Both actions opened Firefox with the URLs in the fixture reference.
The operator reported no different target, redirect or downloaded file path.

The operator then reported `pass` for the Dark local-feed controls and Nostr
copy. The operator selected Dark in Settings, then searched `heycitizen` in
Music and opened the `The Heycitizen Experience` feed result marked `Library`.
The check used normal width and Medium scale. Website, Nostr and RSS remained
visible and readable. The copied Nostr identifier matched the fixture reference
when pasted into the scratch document.

The operator then reported `pass` for the Dark local Website and RSS actions
on the same Library-origin feed. Both actions opened Firefox with the URLs
in the fixture reference. The operator reported no different target, redirect
or downloaded file path.

The operator then reported `pass` for the Dark Index-feed controls and Nostr
copy. The instructed entry route was toolbar search for `heycitizen`, then
Feeds, then the `The Heycitizen Experience` result marked `Index`.
The check used normal width and Medium scale. Website, Nostr and RSS were
visible and readable. The copied Nostr identifier matched the fixture reference
when pasted into the scratch document.

The operator then reported `pass` for the Dark Index Website and RSS actions.
Both actions opened Firefox with the URLs in the fixture reference.
The operator reported no different target, redirect or downloaded file path.

Task 001's local-feed and Index-feed visual checks are accepted in both themes
on 2026-09-19. These results are operator reports.
The agent did not run the app. No screenshots were supplied during this pass.
Task 002 retains its separate gate.

### Task 001 Cleanup — 2026-09-19

The operator replied `removed` after the guarded fixture removal instructions.
Those instructions named `/tmp/v4vmm-governance.JN95oN81` and checked its marker
before removal. Fixture cleanup is confirmed by the operator's report.
The agent did not remove or independently inspect the desktop fixture.
No separate source-preservation inspection is recorded for this acceptance pass.
Task 001 is complete. Task 002 and ADR 0054 metadata checks remain open.

### Task 002 Fixture Preparation — 2026-09-19

The operator confirmed `Green: fixture copy ready at /tmp/v4vmm-governance.ie6k8TQf`.
The desktop command copied the database through a read-only source connection
and copied the audio into the fixture. It created a private configuration
with Light theme, Medium scale and the existing MusicIndex endpoint.
This report establishes fixture preparation. It does not establish a suitable
track, populated track identity facts or visual acceptance. Track selection,
both entry routes in both themes and fixture cleanup remain open.

The first read-only candidate query returned `[]`. It required library
membership, a feed GUID, a nonempty copied audio file and populated Website
and Nostr facts owned by the track. No track met all those conditions.
That result alone did not identify the missing condition or establish a UI
failure. The operator then reported these read-only diagnostic results:

| Library requirement | Track count |
|---|---|
| Library membership | 79 |
| Feed GUID | 79 |
| Local file record | 79 |
| Nonempty audio file inside the fixture | 79 |
| Nonempty Website fact owned by the track | 0 |
| Nonempty Nostr fact owned by the track | 9 |

Missing stored track Website facts prevent the initial fixture selection.
This result alone is not evidence of a UI defect.

The first downloaded candidate is local track `2`, `MoeFactz`, from feed `2`,
`The Heycitizen Experience`. Its track GUID is
`d489101a-4e62-492f-812e-9fe51def9423`. Its feed GUID is
`a2d2e313-9cbd-5169-b89c-ab07b33ecc33`. It has a stored Nostr fact but no
stored track Website fact. This candidate does not meet the populated fixture
requirement. Keep track and feed facts separate when inspecting the Index response.

The operator's Index lookup returned `source_links: []` and `source_ids: []`
for that scoped MoeFactz track. The parent feed returned a Website fact from
`rss_link` at `feed.link` and a Nostr fact from `podcast_txt` at
`feed.podcast:txt`. Both feed values match the earlier parent-feed reference.
These are feed-owned facts. They do not establish populated track controls.
The stored local Nostr fact and current Index track response differ.
No visual failure or acceptance follows from that source difference.

The operator then reported 79 successful scoped Index lookups with no errors.
The check counted zero tracks with Website facts, zero with Nostr facts and
zero candidates. This result covers the copied library, not the full Index.
It does not establish that all metadata or all link types are absent.

Read-only source inspection found these differences on 2026-09-19:

- The Stophammer parser maps an item `link` to `web_page`. It maps a feed
  `link` to `website`. The app's `website_url_from_links` accepts only `website`
  in `src/views.rs`. A `website`-only candidate check can miss track-page links.
- The Stophammer parser reads entity Nostr IDs from direct `podcast:txt`
  children with `purpose="npub"`. It preserves a `podcast:person` npub on the
  contributor claim. The API exposes these claims through `source_contributors`.
- The app's `nostr_from_extension` in `src/rss/enrich.rs` also scans extension
  attributes and nested children. It can classify a contributor npub as a
  track Nostr fact. The nine stored keys need a source check before attribution.

The inspected upstream owners are `stophammer-parser/src/engine.rs`
(`extract_entity_ids`, `extract_links`, contributor extraction), `src/api.rs`
(track claim construction) and `src/query.rs` (separate API includes).
This inspection establishes checkout behavior. The deployed revision and the
current RSS item contents remain unverified. It does not establish the cause
of the empty MoeFactz lists or prove an ingestion failure.

### Task 002 Contributor Source Check — 2026-09-19

The operator repeated the scoped MoeFactz request with
`include=source_links,source_ids,source_contributors`. The response contained
empty track identity lists and three contributor claims:

| Contributor | Role | Group | Identity evidence |
|---|---|---|---|
| HeyCitizen | musician | music | Nostr key and image |
| HeyCitizen | audio engineer | audio-production | The same Nostr key and image |
| Moe Factz | host | cast | `href: https://www.moefactz.com/` |

Both HeyCitizen claims contain
`npub12um9zqae9uaydfszralpn0e0r90d559gd4qsrzar0j2yvut7t2zqwff5ck`.
All three claims identify this track, use source `podcast_person`, and record
extraction path `track.podcast:person`. Their recorded `observed_at` value is
`1779240280`. The operator supplied the response. The agent did not fetch it.

MusicIndex supplies these contributor identities. An `entity_type: track`
claim attaches the contributor credit to the track. It does not assign the
contributor's Nostr key or website to the track itself. The earlier candidate
check omitted contributor claims and cannot establish a general metadata gap.
This response does not establish how the nine local track keys were stored.

Read-only inspection found these current app paths:

- `src/views.rs` preserves contributor `href`, image and npub fields in
  `ContributorView`. The API type accepts these fields.
- `src/application/queries/search.rs` fetches Index track search details with
  no include list. Those requests do not request `source_contributors`.
- `src/app/search_dispatch.rs::index_track_detail_slots` supplies the hero
  image but no contributor section. The current Index track shell adds track
  identity controls. It does not add contributor controls.
- `src/feed_service.rs::fetch_library_track_detail` requests contributor
  claims. The Library metadata grid uses `musicindex_contributors_id3_value`
  in `src/metadata.rs`, which formats names and roles. That summary does not
  expose contributor website or Nostr actions.

These findings identify app request and presentation gaps for the supplied
contributor facts. They are source inspection results, not operator visual
acceptance. No application code changed. Do not promote contributor facts into
track header facts to satisfy the fixture requirement. No synthetic fixture
was applied. Track selection, visual checks and fixture cleanup remain open.

### Earlier Evidence

On 2026-05-02, screenshots of Way to Go and The Heycitizen Experience showed
Index identity controls missing from the local route. The follow-up hydration
fix was recorded, but its visual recheck was not. The 2026-05-03 MoeFactz
attempt had no stored track identity facts, so it did not prove task 002.
These observations document the original gap, not a current reproduced failure.

The original task reviews retain dated implementation evidence. Their retired
screen instructions are replaced by this checklist and the current runbook.

## Merge Recommendation

Keep ADR 0037 Accepted. Task 001 is complete. Close task 002 after its populated
track passes through both origins in both themes and the operator confirms cleanup.
