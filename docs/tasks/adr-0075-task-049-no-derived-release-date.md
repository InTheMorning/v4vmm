# ADR 0075 Task 049: No Derived Release Date

Status: Ready - 2026-09-29. Implementation has not started.
Its visual gate opens when the implementation is complete. Visual checks are paused, so the gate stays open.

## Goal

The app shows and writes a release date only when a source proves a release date.
An oldest-item date, a build date and a feed date never become the release date of a feed or a track.

## Authority

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md), the accepted rules of 2026-09-21:
  - Feed release-date evidence: keep publication, build and oldest-item dates separate. Those dates cannot supply a missing feed release date. A `release_date` field name or claim type alone cannot prove release-date meaning.
  - Track publication-date fallback: a feed date is a separate value. It does not create a track-owned date assertion.
- ADR 0075 section 4: a feed fact never becomes a track assertion.
- [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md): the app does not use `lastBuildDate` as a date.
- [MusicIndex API change request](../plans/musicindex-api-change-request.md) change 3, live since 2026-09-23.

## Recorded Facts - 2026-09-29

- Stophammer gives a `release_date` claim with the path `feed.pub_date` or `oldest_item.pub_date`, and a separate `last_build_date` claim.
- Two live feeds on 2026-09-29, "Monster" and "Way to Go", gave a `release_date` claim with the path `oldest_item.pub_date`.
  Their `release_date` field equals `oldest_item_at`.
- `feed_release_pubdate` in `src/metadata.rs` gives the first of these values:
  1. the first `release_date` claim, with no check of its path
  2. `Feed::release_date`
  3. `Feed::oldest_item_at`
  4. the earliest `pub_date` of the feed tracks
- Each of the four is an oldest-item date or a date with no proof of release meaning.
- `musicindex_release_date` gives the track date, else `feed_release_pubdate`. The "Release date" row reads it. The tag writer writes that row to `TDRC`.
  Thus a track without its own date gets the oldest item date of its feed. The screen and the file show it as the release date.
- `release_pubdate_from_claims` reads any `release_date` claim, with no check of its path.

## Required Changes

### 1. The Feed Release Date

- `feed_release_pubdate` gives no value from a claim with the path `oldest_item.pub_date` or `feed.pub_date`, from `Feed::release_date`, from `Feed::oldest_item_at`, or from the track dates.
- With the present MusicIndex claims, the app thus shows no MusicIndex feed release date. That is correct under the accepted rule.
- A claim path that proves a release date does not exist today. Do not invent one. Accept no path by default.

### 2. The Track Date

- `musicindex_release_date` gives only the track's own date. It never uses a feed date.
- The "Release date" row and its `TDRC` edit use the track's own date only. A track without its own date gets no `TDRC` edit from a feed.
- `release_pubdate_from_claims` reads a track claim only when its path names the item.

### 3. Remove The Unused Readers

- Delete each reader that has no caller after these changes. Keep the raw values in the evidence store.

## Mechanical Acceptance Criteria

Use the prefix `adr_0075_release_date_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R49-01 | A feed with a `release_date` claim with the path `oldest_item.pub_date` gives no feed release date |
| R49-02 | A feed with a `release_date` claim with the path `feed.pub_date` gives no feed release date |
| R49-03 | A feed with only `release_date`, `oldest_item_at` or track dates gives no feed release date |
| R49-04 | A track with its own date gives that date in the "Release date" row and in the `TDRC` edit |
| R49-05 | A track without its own date, in a feed with an oldest item date, gives no "Release date" value and no `TDRC` edit |
| R49-06 | A `last_build_date` claim gives no date in any row |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: a track page of a track without its own date shows no release date.
- V2: a track with its own date shows that date.
- V3: normal and narrow widths show each row in its place, in Light and Dark themes.

## Exclusions

- No new "Feed publication date" row. The app reads no channel `pubDate` from RSS today. That needs its own packet.
- No "first item date" row.
- No change to the MusicBrainz release date.
- No schema change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/metadata.rs`: `feed_release_pubdate`, `track_release_pubdate`, `musicindex_release_date`, `release_pubdate_from_claims`, `track_metadata_rows`, `source_value_for_metadata_field` and the "Release date", "Release year" and "RSS item pubdate" rows.
- `src/metadata_service.rs`: `id3_edits_for_track_context` and the fixed edit tests.
- `src/discover/tests.rs`: the `musicindex_release_date` tests.
- `src/api.rs`: `SourceReleaseClaim`.

## Checks

```bash
cargo test --lib adr_0075_release_date_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Operator Visual Check

The implementer writes this section at completion. It gives numbered steps for V1 to V3.
It states the needed state, what counts as wrong, and the cleanup. A step that writes tags uses the isolated fixture of ADR 0080 packet 001.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a valid difference.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0075-task-049-no-derived-release-date.md`
- ADR 0075, the date rules and section 4
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": no derived feed release date, the track's own date only, and removal of unused readers.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Tests use recorded values. No test sends a request.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The RSS parser.
- The tag writer, except for the `TDRC` value that this packet changes.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R49-01 to R49-06 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V3.

Test commands:
- `cargo test --lib adr_0075_release_date_`
- `cargo test`
- `cargo test --test architecture_tests`
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- `cargo check --all-targets`
- `cargo build --bin v4vmm`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- A track claim path does not tell if the claim names the item.
- A screen other than the track page and the compare grid reads `feed_release_pubdate`.
- A change needs a file in "Do not touch".
