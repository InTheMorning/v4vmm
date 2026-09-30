# ADR 0075 Task 050: Feed Dates By Owner

Status: Waits for the operator decisions of 2026-09-30. Implementation has not started.
Its visual gate opens when the implementation is complete. Visual checks are paused, so the gate stays open.

## Goal

The album page shows each feed date with its true meaning. The RSS channel `pubDate` is the feed publication date.
The MusicIndex `release_date` field is an oldest-item date, and the page never shows it as a release date.
A track without its own date shows the feed publication date as a separate value (the accepted track publication-date fallback).

## Operator Decisions

The operator decides these details before dispatch. The recommendation is in the second column.

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

These changes assume the recommended decisions. Change them to match the recorded decisions.

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

Revert the working tree. If the packet adds a migration, the implementer records its rollback steps here.

## Operator Visual Check

The implementer writes this section at completion. It gives numbered steps for V1 to V3.
It states the needed state, what counts as wrong, and the cleanup. The check only reads pages.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a valid difference.

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
