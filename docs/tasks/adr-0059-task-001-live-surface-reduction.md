# ADR 0059 Task 001: Live Surface Reduction

## Goal

Remove the relay publish path from `src/api.rs` and `src/cli.rs`. Keep live item
create, health, and read. Add a guard that blocks the publish path from
returning.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`
- `docs/plans/adr-0059-broadcast-control-surface-phase-plan.md`
- `docs/architecture/broadcast-chain.md`
- `src/api.rs`
- `src/cli.rs`
- `docs/runbooks/workflows.md`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/api.rs`
- `src/cli.rs`
- `docs/runbooks/workflows.md`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/playback.rs`, `src/playback_owner.rs`, `src/playback_driver/**`
- `src/db.rs`
- `src/ui/**`, `src/view_models/**`, `src/app/**`
- `src/http_client.rs`
- `docs/adr/0018-liveitem-metadata-publish-contract.md`
- `docs/adr/0019-live-relay-debug-cli.md`

## Constraints

- This task removes code. Add no new behavior.
- Keep `create_live_item`, `health`, `fetch_live_metadata`, and
  `fetch_live_metadata_optional`.
- Keep `validate_live_metadata_event_id`. The read paths call it.
- `LiveItemCreateResponse` and `LiveMetadataSnapshot` stay.
- Delete a helper only when no caller remains. Check
  `post_json_with_bearer` before you delete it.
- The two superseded ADRs are already updated. Do not edit them again.

## Implementation Steps

1. Remove `Client::publish_live_metadata` and
   `Client::publish_live_metadata_with_token` from `src/api.rs`.
2. Remove `LiveMetadataPublishRequest` and `LiveMetadataPublishResponse`.
3. Remove `validate_live_metadata_request`.
4. Remove `post_json_with_bearer` when no other caller remains.
5. Remove the `src/api.rs` unit tests that cover publish request validation.
   Keep coverage for event identifier encoding by moving that assertion to the
   metadata read path.
6. Remove the `publish` and `publish-now-playing` match arms at the top of
   `src/cli.rs`.
7. Remove `publish_liveitem_metadata` and `publish_liveitem_now_playing`.
8. Remove the now-unused `LiveOptions` fields and their parse arms: token,
   metadata JSON, and dry run. Keep `--json` and `--endpoint`.
9. Remove the `MUSICINDEX_LIVEITEM_TOKEN` read.
10. Update the CLI usage text so it lists only `liveitem health`,
    `liveitem create`, and `liveitem latest`.
11. Update the `Test The Live Relay` section of `docs/runbooks/workflows.md`.
    Delete the paragraph that names the removed commands.
12. Add an architecture test: no file under `src/` contains
    `publish_live_metadata`, and `src/api.rs` posts to no path segment list that
    ends with `metadata`.

## Acceptance Criteria

- `v4vmm liveitem health`, `v4vmm liveitem create --json`, and
  `v4vmm liveitem latest <id> --json` work as before.
- No symbol named `publish_live_metadata` remains in `src/`.
- No CLI command accepts a broadcaster token.
- The runbook names only the three commands that remain.
- The new architecture guard fails when the publish path returns.
- No UI, view-model, database, or playback file changes.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Deviations from task
5. Unresolved concerns

## Escalation Triggers

- A removed helper has a caller outside the live item code.
- Removing the token option breaks an unrelated CLI command.
- An existing test depends on the publish path for a reason the task does not
  cover.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0059-broadcast-control-surface.md`
- `docs/architecture/broadcast-chain.md`
- `src/api.rs`
- `src/cli.rs`
- `docs/runbooks/workflows.md`

Goal:
- Delete the relay publish path and its CLI commands. Keep create, health, and
  read.

Constraints:
- Removal only. No new behavior.
- Keep `validate_live_metadata_event_id`; the read paths call it.
- Delete a helper only when no caller remains.

Do not touch:
- playback, database, UI, view models, `src/http_client.rs`
- the ADR files

Acceptance criteria:
- The three read commands still work.
- No `publish_live_metadata` symbol remains.
- No CLI command accepts a token.
- An architecture guard blocks the publish path from returning.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
