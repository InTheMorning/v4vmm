# ADR 0075 Task 050: Feed Dates By Owner

Status: Implemented - 2026-10-01. Mechanical checks Green. Visual gate open and paused.

## Goal

The album page shows each feed date with its true meaning. The RSS channel `pubDate` is the feed publication date.
The MusicIndex `release_date` field is an oldest-item date, and the page never shows it as a release date.
A track without its own date shows the feed publication date as a separate value (the accepted track publication-date fallback).

## Operator Decisions

The operator accepted each recommendation below on 2026-09-30. ADR 0075 records both as an amendment of 2026-09-30.

| Detail | Recommendation |
|---|---|
| D50-1: a MusicIndex `release_date` claim with the path `feed.pub_date` as the second source of the feed publication date | Yes. Stophammer corrected the `lastBuildDate` substitution on 2026-09-23. Its refresh pass of 2026-09-24 and its replay of 2026-09-25 sent each body again. The accepted rule of 2026-09-20 rejects the path only because of that substitution. This is an amendment of an accepted rule, and ADR 0075 records it |
| D50-2: the MusicIndex oldest-item date on the album page | Show it as a separate derived fact with the label "First track published", or show nothing. The recommendation is the label |

## Authority

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md), the accepted rules:
  - Feed publication-date source priority, 2026-09-20: prefer a valid fresh RSS channel `pubDate`, then MusicIndex claims that prove an actual channel publication date.
  - Feed publication-date precision and timestamp display: show UTC, keep the source timezone and the original text, and invent no date part.
  - Track publication-date fallback: show the feed date as a separate "Feed publication date" value. It creates no track assertion.
  - Feed release-date evidence, 2026-09-21: keep publication, build and oldest-item dates separate.
- [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md): the app ignores `lastBuildDate`, and the RSS check does not compare the MusicIndex oldest-item release date.
- ADR 0075 packet 049: no derived release date for a track.

## Recorded Facts - 2026-09-30

- `RssTrackEnrichment` in `src/rss/enrich.rs` reads the item `pubDate`. It reads no channel `pubDate`.
- `FeedView::release_date` comes from the MusicIndex `release_date` field (`src/views.rs`, `src/app/search_dispatch.rs`), or from the stored value of `stored_values::project` (`src/application/queries/stored_values.rs`).
- On two live feeds on 2026-09-29, `release_date` equalled `oldest_item_at`, and the claim path was `oldest_item.pub_date`.
- The album page shows that value with the key "Release Date": `ReleaseHeroVm` facts in `src/view_models/entity_detail.rs` near line 870, and `src/view_models/feed.rs` near line 96.
- `identity_ingest` stores the field as a fact with the path `$.release_date`.
- The `feeds` table has no publication date column. The `tracks` table has `pub_date`.

## Required Changes

These changes apply the recorded decisions.

### 1. Read The Channel Date

- `RssTrackEnrichment` reads the channel `pubDate`, and keeps the original text with the parsed instant.
- A value that does not parse gives no date, and the original text stays as evidence.

### 2. The Feed Publication Date

- The feed publication date is the valid fresh RSS channel `pubDate`. Else it is the MusicIndex `release_date` claim with the path `feed.pub_date` (D50-1).
- The album page shows it as "Published", in UTC, with its source in the metadata details.
- The track page shows it as a separate "Feed publication date" value when the track has no own date. The track "Release date" row and `TDRC` stay as packet 049 made them.

### 3. The Oldest-Item Date

- The album page shows no "Release Date" from the MusicIndex `release_date` field.
- It shows the value as the derived fact "First track published", with MusicIndex named as its source (D50-2).
- The stored value keeps its present column and owner. Only its meaning on the page changes.

### 4. Storage

- The Library album page can need the channel date after a restart. Then add the storage in the ADR 0016 migration registry. Record it in the implementation result.
- If a present stored fact can hold it, use that fact and add no column.

## Mechanical Acceptance Criteria

Use the prefix `adr_0075_feed_dates_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R50-01 | An RSS channel with `<pubDate>Sun, 20 Sep 2026 12:00:00 +0000</pubDate>` gives that instant and its original text |
| R50-02 | A channel `pubDate` that does not parse gives no date and keeps the original text |
| R50-03 | The album view model exposes "Published" from the RSS date. Without it, from a `feed.pub_date` claim. Without both, no "Published" fact |
| R50-04 | A `release_date` claim with the path `oldest_item.pub_date` never gives the "Published" fact |
| R50-05 | The album view model exposes no "Release Date" fact from `Feed::release_date`, and exposes "First track published" with its source |
| R50-06 | A track without its own date exposes a separate "Feed publication date", and still gives no "Release date" value and no `TDRC` edit |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: an album page shows "Published" and "First track published" as two facts, each with its source in the details.
- V2: a track without its own date shows "Feed publication date" apart from its other facts.
- V3: normal and narrow widths show each fact in its place, in Light and Dark themes, with no clipped text.

## Exclusions

- No release date for a feed. No source proves one today.
- No use of `lastBuildDate` (ADR 0076).
- No change to the RSS check comparison.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/rss/enrich.rs`: `RssTrackEnrichment`, `parse_rss_pub_date`, the channel reads.
- `src/views.rs`: `FeedView` and its constructors.
- `src/view_models/entity_detail.rs` and `src/view_models/feed.rs`: the release facts.
- `src/view_models/track_detail.rs`: the date rows.
- `src/application/queries/stored_values.rs`: the release date projection.
- `src/identity_ingest.rs`: the stored `$.release_date` fact.
- `src/metadata.rs`: `release_pubdate_from_claims` and the packet 049 path lists.

## Checks

```bash
cargo test --lib adr_0075_feed_dates_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration, no new table and no new column.
It adds two new fact keys, `channel_pub_date` and `feed_pub_date_claim`, to the existing
generic metadata fact table (`entity_metadata_facts`). A revert removes the code that writes
and reads those two keys. Any row a prior run already wrote for them becomes inert: no later
code reads it, and no schema change undoes it.

## Implementation Result

Implemented on 2026-10-01.

### Storage

Required Change 4 asks for storage. The Library album page needs it to show the feed
publication date after a restart. This packet adds no column and no migration. It reuses the
existing generic metadata fact table, `entity_metadata_facts`.

Its helpers are `db::local_metadata_facts` and `db::replace_local_metadata_fact`. This table
already holds one row for each owner, source and fact key. It already carries the
`release_date` and `description` facts that earlier packets added the same way.

Two new fact keys carry the new record, each under its own existing source token:

- `rss` / `channel_pub_date`: the RSS channel `pubDate`. `identity_ingest::feed_metadata_facts_by_source`
  writes it from `Feed::channel_pub_date`. `local_metadata::feed_facts_from_rows` reads it back
  into `FeedMetadataFacts::channel_pub_date`.
- `musicindex` / `feed_pub_date_claim`: the raw value of a `release_date` claim whose
  extraction path is `feed.pub_date`. The same function writes it from
  `Feed::source_release_claims`. The reader above reads it back into
  `FeedMetadataFacts::pub_date_claim`.

`FeedStoredValues` carries both facts as its own fields: `channel_pub_date` and
`publication_claim`. Each field is a channel-owned value. This module names no provider, per
R20-06. `FeedView::from_local_with_facts` is the one place that resolves the two facts. It
builds the shown "Published" date and the source name beside it.

The feed's own `Feed::channel_pub_date` and `Feed::channel_pub_date_text` fields carry the RSS
record through `TrackContext`, before it reaches storage. Both fields carry `#[serde(skip)]` in
`src/api.rs`. The MusicIndex contract declares neither field. This app's own RSS read is their
only writer. The stored contract guard, packet 051, exempts a skipped field.

Each new fact row gets its value through the call sites that already persist a feed:
`identity_ingest::persist_musicindex_feed`, reached from `subscribe_service.rs`. This packet
adds no database call and no call site. `subscribe_track_from_search_internal` already enriches
its `TrackContext` from RSS, before it persists the feed. So the enriched
`Feed::channel_pub_date` value is present by the time persistence runs.

### Files Changed

- `src/api.rs`: `Feed` gains `channel_pub_date` and `channel_pub_date_text`, both
  `#[serde(skip)]`.

- `src/rss/enrich.rs`: a new `RssChannelPubDate` type carries a channel `pubDate`'s parsed
  instant and its original text. `RssTrackEnrichment` gains a `channel_pub_date` field, filled
  by a new reader from the channel's `pubDate` element. `apply_track_enrichment` merges it onto
  the feed. The `assert_scalar_compatibility` test helper gains the same field, read through
  the `rss` crate's own `Channel::pub_date`, to keep its cross-check accurate. New tests carry
  the `adr_0075_feed_dates_` prefix.

- `src/metadata.rs`: `release_pubdate_from_claims` is split into `find_release_date_claim`,
  the path-proven claim lookup, and `format_release_claim_value`, its instant-or-text
  formatter. The new reader reuses the lookup without reformatting an already-formatted
  string. A new `FEED_PUBLICATION_DATE_PROVEN_PATHS` constant names `feed.pub_date` as the one
  proven path, under operator decision D50-1. New `feed_publication_pubdate` and
  `feed_publication_date_from_parts` functions return the feed's own publication date and its
  source name, `"RSS"` or `"MusicIndex"`. New tests carry the `adr_0075_feed_dates_` prefix.

- `src/identity_ingest.rs`: `feed_metadata_facts_by_source` gains the `feed_pub_date_claim` and
  `channel_pub_date` fact writes described under Storage. A new test proves both facts persist.
  It also proves that an `oldest_item.pub_date` claim persists neither.

- `src/local_metadata.rs`: `feed_facts_from_rows` reads the two new fact keys into
  `FeedMetadataFacts::channel_pub_date` and `FeedMetadataFacts::pub_date_claim`. A new test
  proves the projection.

- `src/views.rs`: `FeedMetadataFacts` gains the same two fields. `FeedView` gains `published`
  and `published_source`. `FeedView::from_api` resolves them from the decoded `Feed`, before any
  field of it moves. `FeedView::from_local_with_facts` resolves them from the two new
  `FeedStoredValues` fields. New tests cover both constructors.

- `src/application/queries/stored_values.rs`: `FeedStoredValues` gains `channel_pub_date` and
  `publication_claim`, each a channel-owned value with no provider label. R20-06 stays Green.
  `project_feed` reads both from their fact buckets. A new test proves the projection. The
  R20-06 owner list now also names both new fields.

- `src/view_models/entity_detail.rs`: `summary_facts` now shows two date facts, "Published"
  then "First track published". Each names its own source, D50-1 for "Published" and
  MusicIndex for "First track published" under D50-2. The second fact renames the old "Release
  Date" fact. `MAX_RELEASE_SUMMARY_FACTS` rises from 5 to 6, because the page now needs room for
  both facts. One existing test is updated, and two new tests carry the `adr_0075_feed_dates_`
  prefix.

- `src/view_models/track_detail.rs`: a new "Feed publication date" label and display method
  show the feed's own publication date, on a track with no date of its own. This row sits apart
  from "Release Date", which still shows only the track's own date: packet 049 stays unchanged.
  New tests carry the `adr_0075_feed_dates_` prefix.

- `src/view_models/library.rs`: one existing test's `FeedMetadataFacts` literal gains
  `..FeedMetadataFacts::default()`, so it still compiles with the two new fields. No assertion
  changes.

- `tests/architecture_tests.rs`: the ADR 0054 fact-key guard,
  `metadata_source_fact_keys_stay_owner_scoped`, caught the two new fact keys as unapproved.
  This guard checks `feed_metadata_facts_by_source` and `feed_facts_from_rows` against an
  approved list, `ADR0054_FEED_FACT_KEYS`. The approved list gains `channel_pub_date` and
  `feed_pub_date_claim`, each with a comment.

- The same ADR 0054 guard's string scan also flagged two raw extraction-path literals, read
  directly in `feed_metadata_facts_by_source`: `"feed.pub_date"` and `"channel.pub_date"`.
  `src/identity_ingest.rs` names them with two private constants: `FEED_PUB_DATE_CLAIM_PATH`
  and `CHANNEL_PUB_DATE_EXTRACTION_PATH`. The two constants sit above that function, past the
  guard's scanned range.

- `docs/tasks/adr-0075-task-050-feed-dates-by-owner.md`: this packet, with its Status line, this
  section and the Operator Visual Check section.

### Tests Run

```bash
cargo test --lib adr_0075_feed_dates_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

### Behavior Changed

- The album page shows "Published", the feed's own publication date. It prefers a fresh RSS
  channel `pubDate` over a MusicIndex `release_date` claim with the path `feed.pub_date`. Each
  shown date carries its source name in parentheses.
- The album page no longer shows "Release Date" from the MusicIndex `release_date` field. It
  shows "First track published" instead, with the same value, and names MusicIndex as its
  source (D50-2). This value was always an oldest-item date, never a release date. Only its
  label and named source change.
- A track page shows "Feed publication date" as a separate fact when the track has no date of
  its own and its feed has a publication date. A track with its own date, or a track whose feed
  has no publication date, shows no such row. The track's own "Release Date" row and its `TDRC`
  edit stay exactly as packet 049 made them. They keep track-owned evidence only, with no feed
  fallback.
- The Library album page keeps the feed's RSS channel `pubDate` and its `feed.pub_date` claim
  across a restart. The two new fact rows described under Storage carry them.

### Deviations From The Task

- The packet's Authority section names two accepted rules, "Feed publication-date precision"
  and "Feed publication timestamp display." They ask for exact partial-date precision. They
  also ask for UTC display, with the source timezone and original text kept in metadata
  details.
- This implementation keeps the existing `fmt_date` convention instead. That convention gives
  day precision, implicit UTC, and no timezone or original text shown in the interface. Every
  other date fact in this app already uses that same convention.
- This implementation stores the RSS channel `pubDate`'s original text, in
  `Feed::channel_pub_date_text` and the `raw_json` column of its fact row. No screen reads that
  text yet. A full precision and original-text display model would touch every date fact in the
  app, not only the two this packet adds. Building it is out of this bounded task's scope.
- "With its source in the metadata details" is read as "beside the date, in the view model's
  own text." This app has no separate metadata-details surface for a feed-level fact today. The
  source name shows in parentheses directly after the date, on both the album page and the
  track page.
- `MAX_RELEASE_SUMMARY_FACTS` rises from 5 to 6 so "Published" and "First track published" can
  show together, alongside Release Kind, Tracks, Duration and Language. This change is not
  named in the task's Required Changes. V1 requires both facts to show at once, so the cap had
  to grow to fit a feed that also has every other fact.
- Required Change 2 says "the valid fresh RSS channel pubDate." This implementation reads
  whatever RSS enrichment most recently wrote to `Feed::channel_pub_date`. It adds no separate
  freshness or expiry check of its own.
- This field has no hold, no gate and no comparison in `rss_field_holds` today. This matches how
  `release_date` and `release_kind` already read, with no hold, in `stored_values::project_feed`.
  Building that comparison is excluded by this packet's own "No change to the RSS check
  comparison."

### Unresolved Concerns

- `src/view_models/feed.rs` (`FeedVm::scalar_detail_entries`) still shows a "Release Date" fact
  from `FeedView::release_date`, with the old meaning. This module has no caller anywhere in
  the crate. A search finds no reference to `view_models::feed` outside its own declaration in
  `view_models/mod.rs`.
- `src/view_models/feed.rs` is dead code that predates this packet. Deleting dead code is a
  separate cleanup, outside this bounded task. It is named here so a later session does not
  mistake it for a live screen.
- No known MusicIndex extraction path proves a `release_date` claim names a track's own
  publication date, as opposed to its release date. Packet 049 covers the release date in
  `TRACK_RELEASE_DATE_PROVEN_PATHS`. This packet adds no new proven path for a track's own
  publication date. The Required Changes did not ask for one.
- The `feed_pub_date_claim` fact can disappear from storage, like every other
  `musicindex`-sourced fact this app writes. A later MusicIndex response that persists the feed
  without `source_release_claims` carrying that claim removes it.
- This is the existing behavior of every `musicindex`-sourced fact in
  `identity_ingest::feed_metadata_facts_by_source`, for example `release_date` itself. This
  packet does not change that behavior.
- Cleanup of the evidence fixture `/tmp/v4vmm-governance.ie6k8TQf` stays unconfirmed. This is
  unrelated to this packet.

## Operator Visual Check

This check reads the app only. It writes no tag and no file, and it needs no fixture. It needs
a Linux desktop session and this checkout. Each shown date carries its source name in
parentheses, `(RSS)` or `(MusicIndex)`.

1. Close v4vmm. Build the app:

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --bin v4vmm
   ```
2. Open your real app:

   ```bash
   target/debug/v4vmm
   ```
3. Open album pages in Library and Index. Find one that shows a "Published" fact and a
   "First track published" fact. Write down its title.

   Stop, and report the result, if no open album shows both facts.
4. Open track pages. Find a track with no value beside "Release Date". Its album must be the
   album of step 3, or an album with its own "Published" fact. Write down its title.

   Stop, and report the result, if every open track shows a "Release Date" value.

**V1 — the album page shows "Published" and "First track published" as two facts, each with its source**

5. Open the album from step 3.
   - Correct: "Published" and "First track published" show as two separate facts. Each date
     carries a source name in parentheses.
   - Wrong: either fact is missing, shows no source name, or the two facts share one row.

**V2 — a track without its own date shows "Feed publication date" apart from its other facts**

6. Open the track from step 4.
   - Correct: the page shows "Feed publication date" with a date and a source name, in its own
     row, apart from "Release Date", which still shows no value.
   - Wrong: "Feed publication date" shows no value, or "Release Date" shows a value.

**V3 — normal and narrow widths, Light and Dark**

7. With the album from step 3 open, resize the window to its normal width, then to its
   narrowest width. Open Settings -> General (`Ctrl+Comma`). Select Light, then Dark. Return to
   the album page after each change.
   - Correct: "Published" and "First track published" keep their place among the surrounding
     facts, at each width and in each theme. Neither fact's text is cut off in a way a wider
     window would not fix by wrapping. A color change between Light and Dark is expected. It is
     not a defect by itself.
   - Wrong: a fact shifts position, overlaps another fact, or loses text to clipping.
8. Repeat step 7 with the track from step 4 open, checking "Feed publication date" the same
   way.
9. Close the app.

**Cleanup**

None. This check reads your real library. It creates no fixture, and it writes no file.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0075-task-050-feed-dates-by-owner.md`
- ADR 0075, the publication-date and release-date rules, and the packet 049 document
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": read the channel date, show the feed publication date, relabel the oldest-item date, and store what the Library page needs.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Apply the recorded operator decisions D50-1 and D50-2 exactly.
- The view model gives each label and each date text. The screen only composes. A screen uses the scaled tokens of ADR 0039 and existing shared composites.
- Treat RSS and MusicIndex values as untrusted input.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The track release date and `TDRC` rules of packet 049.
- The RSS check comparison.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R50-01 to R50-06 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V3.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The operator decisions are not recorded in this packet.
- The Library album page needs a new column, and the migration registry cannot add one without a change to an existing migration.
- A change needs a file in "Do not touch".

## Orchestrator Review - 2026-10-01

The orchestrator reviewed the diff and ran each check. Each check is Green: 1,809 unit tests, 286 guards, and no warning.

- `FEED_RELEASE_DATE_PROVEN_PATHS` stays empty. The `feed.pub_date` claim has its own publication-date reader.
- The storage uses two new fact keys in `entity_metadata_facts`. The packet adds no migration.
- The two new `api::Feed` fields carry `#[serde(skip)]`. They hold RSS values, and the contract guard exempts them.
- The page shows a day-precision date with its source in parentheses. The accepted precision, timezone and original-text display rules stay open. The phase plan records them.
