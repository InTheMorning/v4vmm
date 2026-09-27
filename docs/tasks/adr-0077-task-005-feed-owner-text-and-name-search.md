# ADR 0077 Task 005: Feed Owner Text And Name Search

Status: Ready - 2026-09-26. Packet 004 is implemented. Implementation has not started.
Its visual gate opens when the implementation is complete. Visual checks are paused, so the gate stays open.

## Goal

Show `publisher_text` as feed owner text that opens no page. Remove the `publisher_text` inspector.
Change the Index artist rows that come from name text into a search. Label the Library name grouping.

The orchestrator divided the earlier packet 004 into packets 004 and 005 on 2026-09-26.

## Authority

- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) Decisions 1, 2 and 6, and its accepted refinements.
- The [publisher relationship request](../plans/stophammer-publisher-relationship-request.md) records that `/v1/publishers` groups feeds by `itunes:owner`. On 2026-09-24, "Wavlake" held 7,352 feeds.
- The durable set in [AGENTS.md](../../AGENTS.md): renderer portability, typed action state and "Delete dead code".

## Recorded Facts - 2026-09-26

The `publisher_text` inspector has these code paths:

- `api::Client::fetch_publisher` and the `"publisher"` arm of the entity detail fetch in `src/api.rs`.
- The `"publisher"` arm of the inspector detail query in `src/application/queries/feed.rs`.
- `EntityDetail::Publisher` in `src/view_models/search/results.rs`, `src/discover/app_impl.rs` and their callers.
- The `"publisher"` skeleton in `src/ui/shells/discover/feed_inspector.rs`.

`artist_rows_from_result_rows` in `src/view_models/search/results.rs` builds artist rows from name text.

## Required Changes

### 1. Feed Owner Text

`publisher_text` shows as "Feed owner" text on the album page. It opens no page.

### 2. Remove The Inspector

List each entry point that opens the `/v1/publishers/{publisher_text}` inspector. Remove each one.
Then delete each code path that no entry point reaches.
This includes `api::Publisher` and `fetch_publisher` when nothing else uses them.

### 3. Name Search

Each Index row that `artist_rows_from_result_rows` builds opens a search result with the title "Tracks matching" and the quoted name.
The row has no artist identity, no role and no page type. Each result row links to its album, and through the album to its publisher page.

### 4. Name Grouping

An album without an owned publisher relationship keeps the Library name grouping. The grouping title shows "Grouped by name".

## Mechanical Acceptance Criteria

Use the prefix `adr_0077_feed_owner_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R5-01 | `publisher_text` is exposed as feed owner text with no action |
| R5-02 | No entry point opens the `/v1/publishers/{publisher_text}` inspector. A guard names ADR 0077 Decision 6 and the fix |
| R5-03 | A name-built Index row exposes a search result title with the quoted name, and no artist identity |
| R5-04 | A Library name grouping exposes the "Grouped by name" label |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: `publisher_text` shows as feed owner text and opens nothing, in Light and Dark themes.
- V2: a name search result shows "Tracks matching" and the name, and opens no artist page.
- V3: the Library name grouping shows "Grouped by name".
- V4: normal and narrow window widths show each element in its defined place, with no clipped text.

## Exclusions

- No change to the publisher page or its navigation. Packet 004 owns them.
- No new request and no stored data.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- Each file in "Recorded Facts".
- `src/view_models/search/feed_detail.rs` and the album page view model.
- `src/library/app_impl.rs`: the Library artist grouping.
- `tests/architecture_tests.rs`.

## Checks

```bash
cargo test --lib adr_0077_feed_owner
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

The implementer writes this section at completion, with numbered steps for V1 to V4, and the cleanup.
The gate stays open in this `Status:` line. The orchestrator adds it to [pending human checks](../pending-human-checks.md).

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0077-task-005-feed-owner-text-and-name-search.md`
- `docs/adr/0077-publisher-feed-artist-binding.md`
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": feed owner text, the inspector removal, the name search, and the name grouping label.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model decides each label and action availability. The screen only composes them.
- Delete code that no entry point reaches. Do not park it.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The publisher page, its navigation and `src/view_models/publisher_page.rs`. Packet 004 owns them.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R5-01 to R5-04 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V4.

Test commands:
- `cargo test --lib adr_0077_feed_owner`
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
- Another feature uses `api::Publisher` or `fetch_publisher`.
- A change needs a file in "Do not touch".
- The name search needs a new request.
