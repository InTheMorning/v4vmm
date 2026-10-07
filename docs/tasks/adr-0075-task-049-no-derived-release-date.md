# ADR 0075 Task 049: No Derived Release Date

Status: Open - implementation and mechanical checks are complete on 2026-09-29. On 2026-10-07 the operator moved its visual check to the [overhaul plan](../plans/design-and-cleanup-overhaul-plan.md#visual-requirements-moved-from-pending-checks---2026-10-07). The packet that rebuilds the surface carries it.

## Goal

The app shows and writes a release date only when a source proves a release date.
An oldest-item date, a build date and a feed date never become the release date of a feed or a track.

## Authority

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md), the accepted rules of 2026-09-21:
  - Feed release-date evidence: keep publication, build and oldest-item dates separate. Those dates cannot supply a missing feed release date. A `release_date` field name or claim type alone cannot prove release-date meaning.
  - Track publication-date fallback: a feed date is a separate value. It does not create a track-owned date assertion.
- ADR 0075 section 4: a feed fact never becomes a track assertion.
- [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md): the app does not use `lastBuildDate` as a date.
- MusicIndex API change request change 3, live since 2026-09-23.

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

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
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

## Implementation Result

Implemented on 2026-09-29.

### Files Changed

- `src/metadata.rs`, `feed_release_pubdate` and `track_release_pubdate`: each function reads
  a `release_date` claim only when its extraction path is in a proven list. Each list is
  empty today. `feed_release_pubdate` no longer reads `Feed::release_date`,
  `Feed::oldest_item_at`, or the feed's track dates.
- `src/metadata.rs`, `musicindex_release_date`: this function no longer falls back to
  `feed_release_pubdate`. The "RSS item pubdate" row is removed from `track_metadata_rows`
  and `source_value_for_metadata_field`. Its value was always empty, on every track, before
  this packet and after it.
- `src/metadata.rs`: new tests use the prefix `adr_0075_release_date_`.
- `src/discover/tests.rs`: one existing test is renamed. Its old fallback case is corrected.
  See "Changed Existing Tests".
- `src/metadata_service.rs`: two new tests check the `TDRC` edit directly.
- `docs/tasks/adr-0075-task-049-no-derived-release-date.md`: this packet, with its Status
  line, this section, and the Operator Visual Check section.

### Tests Run

```bash
cargo test --lib adr_0075_release_date_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

Each command is Green. `cargo test` reports 1899 passed lib tests and 283 passed
architecture tests, with zero failures. `cargo check --all-targets` reports no warning.

Ten tests carry the `adr_0075_release_date_` prefix or cover this packet directly:

- `metadata::tests::adr_0075_release_date_r49_01_oldest_item_path_gives_no_feed_release_date`
- `metadata::tests::adr_0075_release_date_r49_02_feed_pub_date_path_gives_no_feed_release_date`
- `metadata::tests::adr_0075_release_date_r49_03_feed_scalars_and_track_dates_give_no_release_date`
- `metadata::tests::adr_0075_release_date_r49_04_track_own_date_shows_in_release_date_row`
- `metadata::tests::adr_0075_release_date_r49_05_track_without_own_date_shows_no_release_date_row`
- `metadata::tests::adr_0075_release_date_r49_06_last_build_date_claim_gives_no_date`
- `metadata::tests::adr_0075_release_date_track_ignores_a_copied_feed_claim`
- `discover::tests::adr_0075_release_date_musicindex_release_date_never_uses_a_feed_date`
- `metadata_service::tests::adr_0075_release_date_r49_04_own_date_writes_tdrc`
- `metadata_service::tests::adr_0075_release_date_r49_05_no_own_date_writes_no_tdrc`

### Changed Existing Tests

- `src/discover/tests.rs`: `release_date_prefers_item_then_feed_then_oldest_item_pubdate` is
  renamed to `adr_0075_release_date_musicindex_release_date_never_uses_a_feed_date`. Its third
  case held a track with no own date, in a feed with both a `release_date` and an
  `oldest_item_at`. That case expected the feed's `release_date` as the shown value. It now
  expects no value, per Required Change 2: `musicindex_release_date` never uses a feed date.
- `src/metadata_service.rs`: the "fixed edit tests" the packet names
  (`adr_0075_track_header_r22_06_tag_edits_stay_equal_without_own_identity` and
  `..._with_own_identity`) needed no change. Their track and feed fixtures set no `pub_date`,
  no `release_date`, and no `oldest_item_at`, so they wrote no `TDRC` edit before or after this
  packet.

### Behavior Changed

- A feed's "Release date" is read through `feed_release_pubdate`. Only the compare grid's own
  "Release date" row uses it. That value no longer comes from a claim with the path
  `feed.pub_date` or `oldest_item.pub_date`, from `Feed::release_date`, from
  `Feed::oldest_item_at`, or from a track's own date. With the claims MusicIndex sends today,
  this function gives no feed release date.
- A track's page shows "Release date" and "Release year" rows. Each row, and the `TDRC` edit
  it feeds, comes only from the track's own `pub_date` or from a track-owned `release_date`
  claim. A track with no date of its own shows no value in either row. That track gets no
  `TDRC` edit, even when its feed has a release date or an oldest item date.
- The track page no longer shows an "RSS item pubdate" row. That row's value was always empty,
  before this packet and after it, because the track's own date already decided both
  computations it compared. The row's label used to show, with a blank value, on every track.
  That blank row is now gone.
- The compare grid (`compare_track_rows`, `aligned_compare_rows`) is unchanged. It may still
  read `feed_release_pubdate`, which itself no longer returns an unproven value.

### Deviations From The Task

- Required Change 2 asks that `release_pubdate_from_claims` read a track claim only when its
  path names the item. No MusicIndex path is confirmed to name a track's own claim. This
  packet treats the track side the way Required Change 1 treats the feed side. It adds an
  explicit proven-path list, `TRACK_RELEASE_DATE_PROVEN_PATHS`, empty today. No claim from a
  live feed passes this check yet. A track's own release date today comes from its `pub_date`
  scalar.
- This design also stops a copied feed claim from passing as the track's own assertion.
  `track_with_feed_defaults` can copy a feed claim onto a track with no claims of its own. The
  new test `adr_0075_release_date_track_ignores_a_copied_feed_claim` proves this condition. Add
  a path to either proven-path list only with confirmed evidence. Write that evidence in the
  comment above each constant in `src/metadata.rs`.
- The task did not name the "RSS item pubdate" row for removal. It is removed because its
  value was provably always empty, before this packet and after it. See "Behavior Changed"
  above.
- The compare grid's own "RSS item pubdate" row, inside `compare_track_rows`, is different and
  untouched. That row never reached the compare grid, before this packet or after it.
  `push_compare_row` drops a row when its two compared values are both empty. This packet does
  not change `compare_track_rows`, because it is outside the Required Changes.

### Unresolved Concerns

- No known MusicIndex extraction path proves a `release_date` claim names a track. If
  MusicIndex documents one, add it to `TRACK_RELEASE_DATE_PROVEN_PATHS` in `src/metadata.rs`.
  The same holds for `FEED_RELEASE_DATE_PROVEN_PATHS`, for a feed-owned path.
- `compare_track_rows`'s own "RSS item pubdate" row stays in the code, for the reason stated
  above. A later packet could remove it, and the shared "RSS item pubdate" support in
  `metadata_field_group_key` and `normalized_field_compare_value`.
- Cleanup of the evidence fixture `/tmp/v4vmm-governance.ie6k8TQf` stays unconfirmed. This is
  unrelated to this packet.

## Operator Visual Check

This check reads the app only. It writes no tag, and it needs no fixture. It needs a Linux
desktop session and this checkout.

The "Release date" and "Release year" rows always show their label. Only the value beside
each label changes. A dateless track shows the label with a blank value, not a missing row.

1. Close v4vmm. Build the app:

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --bin v4vmm
   ```
2. Open your real app:

   ```bash
   target/debug/v4vmm
   ```
3. Open Library or Index tracks until you find one whose page shows a value beside "Release
   date". Write down its title.

   Stop, and report the result, if no open track shows a value there.
4. Open Library or Index tracks until you find one with no value beside "Release date". Confirm
   "Release year" also shows no value. Write down its title.

   Stop, and report the result, if every open track shows a "Release date" value.

**V1 and V2 — the row states the track's own date, or states none**

5. Open the track from step 3.
   - Correct: "Release date" shows a value, and "Release year" shows the matching year.
   - Wrong: either label shows with no value.
6. Open the track from step 4.
   - Correct: "Release date" and "Release year" each show their label, with a blank value.
   - Wrong: either label shows a date or a year.

**V3 — normal and narrow widths, Light and Dark**

7. With the track from step 4 open, resize the window to its normal width, then to its
   narrowest width. Open Settings → General (`Ctrl+Comma`). Select Light, then Dark. Return to
   the track page after each change.
   - Correct: "Release date" and "Release year" keep their usual place among the surrounding
     rows, at each width and in each theme. Neither row's blank value leaves a visible gap.
   - Wrong: a row shifts position, overlaps another row, or leaves a visible gap.
8. Repeat step 7 with the track from step 3 open.
   - Correct: the "Release date" and "Release year" rows show their label and value in full,
     at each width and in each theme.
   - Wrong: a label or value is cut off, in a way a wider window would not fix by wrapping.
9. Look for a row labeled "RSS item pubdate" on either track's page.
   - Correct: no such row shows.
   - Wrong: the row shows, with or without a value.
10. Close the app.

**Cleanup**

None. This check reads your real library. It writes no file, and it creates no fixture.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
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

## Orchestrator Review - 2026-09-29

The orchestrator reviewed the diff and ran each check. Each check is Green: 1,899 unit tests, 283 guards, and no warning.

- The two empty lists of proven claim paths are correct. They also stop a feed claim that `track_with_feed_defaults` copies to a track.
- The removed "RSS item pubdate" row always gave no value before this packet. Its removal changes no output.
- The orchestrator corrected one code comment. Stophammer corrected the `lastBuildDate` substitution on 2026-09-23. The comment now gives the rule of ADR 0075 as the reason.
- The operator check only reads pages. It writes no tag, so it needs no fixture.
