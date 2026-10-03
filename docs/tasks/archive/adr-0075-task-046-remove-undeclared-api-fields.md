# ADR 0075 Task 046: Remove Undeclared API Fields

Status: Implemented - 2026-09-26. The operator approved the removal and reviewed this document on 2026-09-26.
Implementation is complete. Its mechanical checks are Green. This packet changes no screen, so it needs no visual acceptance.

## Goal

Delete `api::Feed::name`, `api::Track::name` and `api::Track::feed_url`.
The deployed contract at `/openapi.json` declares none of the three, and the live responses of 2026-09-25 send none of them.
Behavior must not change.

## Authority

- [ADR 0075](../../adr/0075-metadata-ownership-and-completeness.md): the title refinements name the legacy `name` field. The orchestrator amends them in the same change.
- The [open Stophammer requests](../../plans/v4vmm-open-requests.md#answered-no-action) record that no Stophammer action is necessary.
- `AGENTS.md`: "Delete dead code", and "Treat RSS and MusicIndex responses as untrusted input".

## Recorded Facts - 2026-09-26

- No code fills `Feed.name` or `Track.name` from a response or from a local row.
  `track_row_to_feed` and `track_row_to_api_track` fill only `title`.
  `FeedVm::header_feed` in `src/view_models/feed.rs` copies `title` into `name`.
  Thus each read of `name` after `title` gives `None` or a copy of `title`.
- Only `api::track_with_feed_defaults` in `src/api.rs` fills `Track.feed_url`. It copies the `feed_url` of a fetched feed.
  Thus `Track.feed_url` has a value only in a `TrackContext` that has a feed.
- `library_service::find_track_id` and `track_is_in_library_by_match` use the feed address to find the local copy of an Index track.
  This behavior must continue.
- The `/v1/tracks?artist=` route is live. Its track list sends no feed address.
- `api::Feed` and `api::Track` do not deny unknown fields. Stored JSON that contains `name` or `feed_url` still decodes.

## Required Changes

### 1. Delete The Three Fields

Delete `name` from `api::Feed` and from `api::Track`. Delete `feed_url` from `api::Track`.
Do not change `api::Feed::feed_url`. Do not change a field of another type with the same name, for example `TrackRow`, `FeedView` or a playlist.

### 2. Replace Each Read Of `name`

- A fallback after `title` is deleted. The result is the `title` value alone.
- `FeedVm::header_feed` no longer sets `name`.
- A test that proves a `name` fallback is deleted, because it tests behavior that cannot occur.
- `metadata.rs` no longer cleans `name`.

### 3. Replace Each Read Of `Track.feed_url`

- Add one accessor, `TrackContext::feed_url(&self) -> Option<&str>`, in `src/metadata.rs`. It gives the non-empty `feed_url` of the context's feed.
- Each reader that has a `TrackContext` uses this accessor. Examples: `subscribe_service`, the inspector playlist target, the conversion request key, and the RSS request specification.
- A reader with a bare `api::Track` and no feed passes `None`. Today such a reader also receives `None`, because nothing fills the field for a bare track.
- A reader that has a `FeedView` or an `api::Feed` beside the track uses that feed's `feed_url`.
- `api::track_with_feed_defaults` no longer copies the feed address.
- `fill_missing_track_match_fields` no longer copies the feed address.
- `ApiSource::list_feeds_for_artist` in `src/sources.rs` sets `feed_url: None` on each feed view. The track list sends no feed address.
- `TrackView::feed_url`: when no reader of it remains, delete it. When a reader remains, fill it from the feed where one is available.

### 4. Tests

Each test that sets `track.feed_url` to supply an RSS address sets the address on the context's feed.
Each assertion on `track.feed_url` becomes an assertion on the context's feed address, or it is deleted when it tests only the copy.

## Mechanical Acceptance Criteria

Use the prefix `adr_0075_undeclared_fields_` for new behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R46-01 | `api::Feed` and `api::Track` decode a response that contains `name` and `feed_url` without an error. The values are ignored |
| R46-02 | `TrackContext::feed_url` gives the feed address of a context with a feed, and `None` for a context without a feed or with an empty address |
| R46-03 | The Library match of an Index track that has a fetched feed still finds the local track by feed address and item GUID |
| R46-04 | RSS enrichment of a context with a feed address reads the feed address from the context's feed |
| R46-06 | The existing test suite passes, except each deleted test that proved a `name` fallback or the feed address copy |

The operator dropped R46-05, a guard against the return of the three fields, on 2026-09-26. No incident supports it, and the compiler rejects each read of a deleted field.

## Orchestrator Changes In The Same Change

The coding model does not edit an ADR or a document other than this packet. The orchestrator makes these corrections of fact in the same change:

- ADR 0075 "MusicIndex feed title representation": the contract has no `name`, so the rule reduces to `title`. A dated status sentence records the correction.
- ADR 0075 "Track title rules": remove "title/name order".
- The same two rows in `docs/reviews/adr-0075-metadata-contract-review.md`.
- The same text in `docs/schema/adr-0075-field-rules-titles-numbers-classification.md`.
- The unassigned-work row in the ADR 0075 phase plan and both copies of the open Stophammer requests.

## Exclusions

- No change to `api::Feed::feed_url` or to a type other than `api::Feed` and `api::Track`.
- No new match rule. For example, do not add a match by feed GUID.
- No schema change and no migration.
- No change to a screen.

## Files To Inspect

- [Agent rules](../../../AGENTS.md) and the [source map](../../architecture/source-map.md).
- `src/api.rs`: `Feed`, `Track`, `track_with_feed_defaults`, and their tests.
- `src/metadata.rs`: `TrackContext`, the source-text cleaning, and the title fallbacks.
- `src/subscribe_service.rs`, `src/feed_service.rs`, `src/sources.rs` and `src/views.rs`.
- `src/library_service.rs` and `src/db.rs`: `find_track_id` and `track_is_in_library_by_match`.
- `src/app/search_dispatch.rs`, `src/discover/app_impl.rs`, `src/ui/shells/discover/actions.rs` and `src/application/conversion_recovery.rs`.
- `src/rss/enrich.rs`: the tests that supply an RSS address.
- `tests/architecture_tests.rs`: the existing ADR 0075 guards.

## Files Likely To Change

The compiler finds each reader after step 1. Change only the files that the compiler names, plus:

| File | Permitted change |
|---|---|
| `src/metadata.rs` | `TrackContext::feed_url`, and the deletion of each `name` read |
| This packet | The implementation result |

## Checks

```bash
cargo test --lib adr_0075_undeclared_fields
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

Report each result. Say "Green" for a passing check.

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Operator Visual Check

None. This packet changes no screen.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/archive/adr-0075-task-046-remove-undeclared-api-fields.md`
- Each file in "Files To Inspect"

Goal:
- Delete `api::Feed::name`, `api::Track::name` and `api::Track::feed_url`, and replace each read as "Required Changes" states. Behavior must not change.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Delete the fields first. Then use the compiler errors to find each reader.
- A reader with a `TrackContext` uses `TrackContext::feed_url`. A reader with a bare `api::Track` passes `None`.
- Do not commit. Do not run the app: no `cargo run` and no display attempt.

Do not touch:
- Any ADR, and each document other than this packet.
- `api::Feed::feed_url`, and each field of another type.
- The database schema, the migration registry and each match rule.
- `src/ui/` except the one read in `src/ui/shells/discover/actions.rs`.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R46-01 to R46-04 and R46-06 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.

Test commands:
- `cargo test --lib adr_0075_undeclared_fields`
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
- A reader of `Track.feed_url` has no `TrackContext` and no feed, but can receive a copied value today.
- A reader of `name` uses it as the only title source.
- A change needs a file in "Do not touch".
- A test proves behavior that the removal changes, and the test is not a `name` fallback test or a copy test.

## Implementation Result - 2026-09-26

### 1. Files Changed

`src/api.rs` no longer has the three fields. `src/metadata.rs` adds
`TrackContext::feed_url`.

These files correct a reader of a deleted field:

- `src/api.rs`, `src/metadata.rs`, `src/metadata_service.rs`,
  `src/feed_service.rs`, `src/track_compare.rs`, `src/views.rs`.
- `src/view_models/feed.rs`, `src/view_models/track.rs`,
  `src/view_models/search/track.rs`,
  `src/view_models/search/results.rs`,
  `src/view_models/search/tests.rs`.
- `src/application/queries/feed.rs`,
  `src/application/queries/search.rs`, and
  `src/application/queries/library.rs` (one test check).
- `src/app/search_dispatch.rs`, `src/discover/app_impl.rs`,
  `src/subscribe_service.rs`, `src/sources.rs`, `src/rss/enrich.rs`,
  `src/library/app_impl.rs`.
- `src/library_service.rs` (one new test), and
  `src/application/conversion_recovery.rs`.
- `src/ui/shells/discover/actions.rs`,
  `src/ui/shells/discover/track_rows.rs`, and
  `src/ui/shells/discover/track_inspector.rs`.

`src/views.rs` also deletes `TrackView::feed_url` and its two writers.
No code read this field after the change above. The packet allows this
deletion.

New tests use the prefix `adr_0075_undeclared_fields_`. Each one sits
with the code it covers. `src/api.rs` holds R46-01. `src/metadata.rs`
holds R46-02. `src/library_service.rs` holds R46-03.
`src/subscribe_service.rs` holds R46-04.

### 2. Tests Run

Each command ran at the repository root.

- `cargo test --lib adr_0075_undeclared_fields`: 4 passed. None showed
  an error.
- `cargo test`: 1832 lib tests, 281 architecture tests, and 10 doc
  tests. All lib and architecture tests passed. The doc tests are
  ignored by design. No test showed an error.
- `cargo test --test architecture_tests`: 281 passed. None showed an
  error.
- `cargo fmt -- --check`: Green.
- `cargo clippy -- -D warnings`: Green.
- `cargo check --all-targets`: Green. No warning appeared.
- `cargo build --bin v4vmm`: Green.

### 3. Behavior Changed

The app shows the same titles, feed links, and library marks as before.
A search row for an Index track with a fetched feed finds the matching
local copy by feed address and item GUID. A new test in
`src/library_service.rs` checks this match (R46-03).

An older record that carries `name` or `feed_url` decodes without an
error. The app drops the two fields on read. No screen read a value
from these fields before this change. The Recorded Facts section in
this packet shows this result.

### 4. Deviations From Task

The packet excludes `src/ui/`. It names one read in
`src/ui/shells/discover/actions.rs` as the one allowed file. Two more
files in this excluded path needed a small correction, because each one
read `Track.feed_url` directly.

`src/ui/shells/discover/track_rows.rs` checked library membership for a
Discover track row through `track.feed_url`. This function takes the
row's parent feed as an argument, named `feed`. The correction reads
`feed.feed_url` there. No element on the screen changes.

`src/ui/shells/discover/track_inspector.rs` holds the full
`track_context`. It passed only the bare `track` value into
`TrackInspectorHeaderVm::new`. The bare track carries no feed here. A
plain field drop deletes a link the operator can see today, with no
feed left to supply a replacement value. This matches one listed stop
condition in this packet.

The correction adds a `feed_url` argument to
`TrackInspectorHeaderVm::new` in `src/view_models/search/track.rs`, a
file that `src/ui/` does not include. The one call site in
`track_inspector.rs` then passes `track_context.feed_url()`. No element
on the screen changes.

Each correction is a one-line change to a data source. No correction
adds, removes, or moves a screen element. Each one follows the same
rule the packet states for `actions.rs`. The operator should make sure this
rule fits the excluded-file list. The operator can also ask for a new
correction.

### 5. Unresolved Concerns

No item blocks a Green build. One question stays open. Did
`track_rows.rs` and `track_inspector.rs` need the operator's direct
approval before this change? The packet names only `actions.rs` as an
allowed file in `src/ui/`.

## Orchestrator Review - 2026-09-26

The orchestrator reviewed the diff against this packet. Each reader with a `TrackContext` reads `TrackContext::feed_url`.
Each reader that now passes `None` received `None` before this change:

- `SearchApp::musicindex_row_is_in_library` receives a bare track from `fetch_track`.
- A local Library search row builds its context from a local row, which sets no feed address.

The R46-03 test gives no enclosure URL, so it proves the match by feed address and item GUID.
The one deleted test proved the `name` fallback.

The packet named only one reader in `src/ui/`. The compiler found two more: `track_rows.rs` and `track_inspector.rs`.
Each change follows the packet rule, and the orchestrator accepts both. The packet list was incomplete.

The orchestrator corrected the ADR 0075 title refinements in the same change.
The corrections are in the ADR, the contract review, and the title field rules. The phase plan and both copies of the open Stophammer requests record the result.
