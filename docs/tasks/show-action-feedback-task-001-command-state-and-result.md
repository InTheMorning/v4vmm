# Show Action Feedback Task 001: Command State And Result

Status: Ready - 2026-09-09. Closes items A7, A8, and A9 of
`docs/plans/hig-product-polish-backlog.md`. All three came from operator runs on
2026-09-08 and 2026-09-09.

## Goal

Make a pressed control report its own progress and its own result. A press holds
its state until the command answers, the controls stay where they are, and the
result of `Check all feeds` is readable.

## Why These Three Together

They are one defect in three places: **the surface that starts an action does
not own how that action reports.**

- A8, a watch sample repaints the state the operator just changed.
- A9, the stream action row is removed instead of disabled.
- A7, the feed check result is written where it does not fit.

Fixing one and not the others leaves the same class of surprise on the other two
controls.

## Files To Inspect

- `docs/plans/hig-product-polish-backlog.md`, items A7, A8, and A9
- `src/view_models/show.rs`, `mark_service_working` and `mark_stream_working`
- `src/app/show.rs`, `run_publisher_service_command` and
  `run_stream_encoder_command`
- `src/runtime/broadcast_service_watch.rs`, for the snapshot that arrives late
- `src/library/app_impl.rs`, the feed-update row that holds `feed_status`
- `src/view_models/library.rs`, `feed_check_route_repair_message`
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/view_models/show.rs`
- `src/app/show.rs`
- `src/library/app_impl.rs`
- `src/view_models/library.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/broadcast/**`. The watch keeps its sampling interval and its commands.
- `src/runtime/**`, except for one addition: a stamp taken before a batch of
  reads begins. `at` is taken after they finish and cannot answer freshness.
- The `ShowCardKind` order and the card contract
- The queue display contract
- `feed_check_route_repair_message` wording. This task moves it, and does not
  rewrite it.

## Constraints

### A8, The Requested State Wins Until The Command Answers

- An operator press sets the transition state, and **a watch sample must not
  replace it** while the command is in flight.
- A stop that reports `Active` is a stale sample, not a result. A start that
  reports `Inactive` is the same.
- Hold the in-flight set by service role. Two services change state on their
  own schedules, and one must not clear the other.

**The command result does not end the transition on its own.** Reviewed
2026-09-09: a watch read taken before the command finished can arrive after it,
so releasing on the result still flashes the old state. The transition ends on a
**fresh observation**, which is a watch sample the actor took after the command
returned. The result records that the command finished; the next fresh sample
releases the display.

**Separate who owns the command from what the state reads.** Give each command a
number that rises. A result clears the in-flight record only when the number
still matches. Reviewed 2026-09-09: an agreeing sample could release the
controls while the command was still outstanding, and its later callback then
cleared a newer press by the same operator.

A sample that agrees with the requested direction may end the *display* state,
but it never clears the ownership record. Only the matching result does.

### A9, A Control In Flight Is Disabled, Never Removed

- **Keep the action row mounted.** `mark_stream_working` sets the stream actions
  to `None` today, so both controls vanish and the row collapses.
- A command in flight renders its controls with a typed unavailable state, the
  same way the publisher service row already does.
- The row keeps its height in every state. ADR 0063 requires a card to hold one
  height, and a row that appears and disappears breaks it.

### A7, The Result Renders Where It Fits

- Move the feed check result out of the sidebar header row it shares with the
  button. It reports five counts and the row is sized for a short label.
- Give it its own row under the button, at the sidebar width. The backlog names
  two alternatives if that fails; take them in the order written there.
- **Do not truncate it.** `docs/troubleshooting/column-text-truncation.md`
  records what stacked text does with `truncate()`, and a cut count reads as a
  wrong count.

## Implementation Steps

1. Add an in-flight record to the `Show` page state, keyed by service role and
   by the stream. It holds what the operator asked for and a command number that
   rises with each press.
2. **Do not use `at` as the observation time.**
   `BroadcastServiceWatchSnapshot::new` calls `Instant::now()` after the
   argument list has already run every `show()` read, so `at` is the moment the
   batch finished. A read of the publisher can start before a command returns
   and still land in a snapshot stamped after it. Reviewed 2026-09-09.

   Take the read boundary instead. Record the moment the command returned, and
   accept a snapshot only when the actor **started** its read after that moment.
   The smallest form is a second stamp taken before the reads begin, so a
   snapshot carries both the start and the end of its batch.

   That stamp lives in `src/runtime/broadcast_service_watch.rs`, so this task
   makes one change there and no other. Everything else in `src/runtime/**`
   stays as it is.
3. Change the publisher snapshot projection to keep the transition state while a
   role is in flight, and to release it on a fresh sample that agrees with the
   request.
4. **Release a fresh sample that disagrees, when it reports a settled state.**
   A start that succeeds and then crashes reports `Failed`, which never agrees,
   so a rule that waits for agreement holds `Working` for the rest of the
   session. Reviewed 2026-09-09.

   `Failed`, `NotInstalled`, and `NotReachable` are answers. Show them at once.
   `Starting` and `Stopping` are not, and neither is the state the operator
   asked to leave.
5. **Give the transition an end.** After a small number of fresh disagreeing
   samples, or a bounded wait, release the display and show what the watch
   reports. A transition that never ends is a worse answer than a surprising
   one.
6. Clear the in-flight record on command success and on command failure, and
   only when the command number still matches the record. A later result from an
   older press clears nothing.
7. Change `mark_stream_working` to keep `actions`, with `connect` and
   `disconnect` both unavailable, instead of setting the field to `None`.
8. Release the stream display on the next fresh agreeing sample, and clear its
   ownership record on the matching result. The stream follows the same two-part
   rule as a service, so one control does not behave unlike the others.
9. Move the feed check result to its own row under the `Check all feeds` button.
10. Add view-model tests:
   - a stop in flight ignores a watch sample that still reports `Active`
   - a stop in flight accepts a sample that reports `Inactive`, and ends
   - a failed command ends the in-flight state
   - a result from an older press does not clear a newer one
   - a sample whose read began before the command returned does not release the
     display, even when it agrees with the request
   - a batch that finishes after the command, but began before it, is not fresh
   - a fresh `Failed` releases the display at once, without agreement
   - a transition ends after the bounded wait, whatever the watch reports
   - the publisher in-flight state of one role does not clear the other
   - a stream command in flight still projects an action row, with both
     controls unavailable
11. Add a guard: `mark_stream_working` does not set the stream actions to
    `None`, and the publisher projection does not overwrite a role that is in
    flight.

## Acceptance Criteria

Mechanical, proved by a test:

- A watch sample that reports the pre-command state does not replace a
  transition state.
- Freshness is decided by when the read began, not by when the batch finished.
- A sample that is fresh and agrees with the request ends the transition. A
  sample that agrees but is not fresh does not.
- A fresh settled state that disagrees, such as `Failed`, ends the transition
  at once.
- A transition always ends. No sequence of samples holds `Working` forever.
- A command failure ends the transition and shows the real state.
- A result from an older press clears no record belonging to a newer press.
- One service role in flight does not clear another.
- A stream command in flight projects an action row with both controls
  unavailable, and never an absent row.
- The feed check result renders outside the button row.

Visual, operator only:

- Pressing `Stop` shows the transition and never flashes `Started` on the way.
- A service that fails right after a start reports the failure, and does not sit
  at `Working`.
- Pressing `Connect` or `Disconnect` leaves both controls in place, dimmed.
- The `Check all feeds` result reads in full, including every count.
- No row changes height while a command runs.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test show --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

Do not run the app. Write the operator visual check instead, as AGENTS.md
requires.

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Deviations from task
5. Unresolved concerns
6. Operator visual check

## Escalation Triggers

- The read-start stamp cannot be added to the snapshot without changing what
  other readers of `src/runtime/broadcast_service_watch.rs` expect. Report the
  readers before you change the shape.
- A command gives no completion signal, so the in-flight state would never
  clear. Name the command.
- The sidebar has no room for the result row at the smallest supported width.
  Report the width, and take the next alternative from the backlog item.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/plans/hig-product-polish-backlog.md`, items A7, A8, A9
- `src/view_models/show.rs`, `src/app/show.rs`, `src/library/app_impl.rs`

Goal:
- A pressed control holds its state until the command answers, keeps its
  controls mounted, and reports its result where the result fits.

Constraints:
- A watch sample never replaces a transition state that an operator asked for.
- A command result does not release the display. The next fresh sample does,
  and freshness means the read began after the command returned.
- A transition ends on a fresh settled state, or after a bounded wait. It never
  waits forever for agreement.
- Each press carries a number, so an older result clears no newer record.
- A command in flight disables its controls. It never removes them.
- The feed check result moves out of the button row, and is not truncated.
- Hold in-flight state by service role, so one service does not clear another.

Do not touch:
- `src/broadcast/**`, `src/runtime/**`, the card contract, the queue contract

Acceptance criteria:
- A stale sample cannot end or reverse a transition state.
- A stream command in flight still projects an action row.
- The feed check result renders outside the button row.

Test commands:
- `cargo fmt -- --check`
- `cargo test show --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns
6. operator visual check
