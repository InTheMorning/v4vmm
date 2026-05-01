# ADR 0023 Task 009: Finish Narrow Command Intents

## Status

Completed 2026-04-30.

## Task Goal

Move the remaining high-noise status formatting and command setup out of
`library.rs` and `search.rs` into narrow GPUI-free intent/result values,
without introducing a broad command bus.

## Files To Inspect

- `src/library.rs`
- `src/search.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- `src/playlist_service.rs`
- `src/subscribe_service.rs`
- `docs/architecture/architecture-diagrams.md`

## Files Likely To Change

- `src/library.rs`
- `src/search.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`
- Focused VM tests
- ADR 0023 docs/task status

## Do Not Touch

- Service function behavior.
- Database schema.
- CommandBus/EventBus/QueryService architecture.
- Metadata or MusicBrainz lookup algorithms.

## Constraints

- Add only small intent/result structs or enums where they remove direct
  status-message construction or repeated service setup from screens.
- Screens still perform actual async/service dispatch in this ADR.
- Do not create framework abstractions named `Manager`, `Service`, or `Bus`
  unless a separate ADR scopes them.
- Preserve existing user-visible status messages unless a test explicitly pins
  a better message.

## Implementation Steps

1. Audit the remaining `self.vm.set_status(format!(...))`,
   `frame.subscription_message = ...`, and similar screen-owned status
   mutations.
2. Pick one or two highest-value workflows in Library and Search; do not try
   to migrate every call site at once.
3. Add intent/result values in the relevant view-model module.
4. Update screens to ask the VM for intent/result formatting while keeping
   service dispatch local.
5. Add focused unit tests for each new pure transition.

## Acceptance Criteria

- [x] The largest remaining duplicated status/command setup paths are mediated by
  view-model intent/result values.
- [x] No GPUI imports are added to `view_models`.
- [x] No broad command bus or event bus is introduced.
- [x] User-visible behavior is preserved except for intentionally improved copy
  documented in the final report.

## Result

- Moved Library `MusicBrainz` track and album lookup status/state transitions
  into `LibraryViewModel` methods. The screen still performs lookup dispatch,
  candidate matching, staging, and persistence.
- Moved Discover inspector subscribe/unsubscribe begin/error message
  formatting into `SearchSubscriptionCommand`.
- Removed the generic Library VM `set_status` backdoor after the remaining
  callers moved to typed transitions.
- Added focused Library/Search VM tests for the new pure transitions.

## Test Commands

- `cargo fmt -- --check`
- `cargo check`
- `cargo test --lib view_models::library`
- `cargo test --lib view_models::search`
- `cargo clippy --lib --tests -- -D warnings`

## Expected Final Summary Format

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Escalation Triggers

- The task requires a broad command dispatcher.
- A workflow cannot be represented without moving service calls into
  `view_models`.
- The change touches more than two workflows.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0023-design-system-and-view-models.md`
- `docs/plans/adr-0023-finalization-plan.md`
- `src/library.rs`
- `src/search.rs`
- `src/view_models/library.rs`
- `src/view_models/search.rs`

Goal:
- Move remaining high-noise status formatting and command setup into narrow
  GPUI-free intent/result values.

Constraints:
- Pick only the highest-value small workflows.
- Screens still dispatch services.
- Preserve existing behavior.
- No broad command bus/event bus/query service.

Do not touch:
- Schema/migrations.
- Service implementation semantics.
- Metadata/MusicBrainz algorithms.

Acceptance criteria:
- New pure intent/result transitions are unit-tested.
- Screens contain less status-formatting glue.
- No `gpui` imports under `view_models`.

Test commands:
- `cargo fmt -- --check`
- `cargo check`
- `cargo test --lib view_models::library`
- `cargo test --lib view_models::search`
- `cargo clippy --lib --tests -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
