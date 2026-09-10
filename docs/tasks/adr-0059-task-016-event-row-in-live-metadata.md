# ADR 0059 Task 016: Event Becomes The First Row Of Live Metadata

Status: Implemented - 2026-09-09. Mechanical acceptance Green. Operator visual
acceptance met: all [broadcast event recovery checks](../runbooks/broadcast-event-recovery-check.md)
passed, confirmed by the operator on 2026-09-09. No acceptance gate remains open.
The [amendment review](../reviews/adr-0059-task-016-amendment-review.md) remains
complete; it does not establish visual acceptance.

## Goal

Move `Event` out of its own card and into `Live Metadata`, as the first of three
rows. Make the section state account for the event. Add `Create`, `Replace`,
and a retryable liveness check so event setup and recovery work inside the app.

## Why The Create Action Belongs Here

An operator lost an event on 2026-09-09 and could not replace it in the app.
`Event` carries `attach` and `detach` and nothing else, because the packet that
specified `create` was superseded before it was built and no packet since has
owned it.

The section that now presents event setup must let an operator do that setup. A
row that reports "no event" and offers no way to make one is the same defect as
the `Missing routes` label that looked pressable and was not.

## Files To Inspect

- `docs/adr/0059-broadcast-control-surface.md`, the 2026-09-09 amendment
- `docs/adr/0063-show-dashboard-layout.md`, `The Grid Holds Three Cards`
- `src/view_models/show.rs`, `EventSectionDisplay`, `PublisherSectionDisplay`,
  `ShowCardKind`, and `show_cards`
- `src/ui/composites/show_detail_panel.rs`
- `src/ui/composites/show_card.rs`
- `src/app/show.rs`, the event and publisher wiring
- `src/broadcast/registry.rs`, for `create_event`, `check_event`,
  `CreatedBroadcastEvent`, and `CheckedBroadcastEvent`
- `src/cli.rs`, for the create and check commands that already exist
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/view_models/show.rs`
- `src/ui/composites/show_detail_panel.rs`
- `src/app/show.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/broadcast/registry.rs`. `create_event` and `check_event` already exist;
  this task calls them through application commands.
- `src/broadcast/publisher_targets.rs`. Attach keeps its behaviour.
- `src/runtime/**`
- The `Source` and `Stream` sections

## Constraints

- **`Event` is a row of `Live Metadata`, not a card.** `ShowCardKind` loses its
  `Event` case, and the grid holds three cards.
- The rows read in chain order: `Event`, `Producer`, `Publisher`. Each is a
  precursor of the one under it, so an operator who stops at the first row that
  is not ready has found the thing to fix.
- **The section state combines the event, the attachment, and the services.**
  Absence is not the only unready case. Reviewed 2026-09-09, because criteria
  that tested absence alone would pass while a dead event read `Active`, and
  because the first table omitted the states a new event and a failed target
  query produce.

  The rows are checked in chain order, and the first condition that applies
  decides the section. The types are `EventState`,
  `EventTargetAttachmentState`, and `PublisherServiceStateDisplay`.

  | Condition | Section | Card state | Row offers |
  |---|---|---|---|
  | `EventState::None` | not ready | `Attention` | `Create` |
  | `EventState::Dead` | not ready | `Failed` | `Replace` |
  | `EventState::Unknown` | not ready | `Unknown` | `Check`, or `Retry check` after a check failure; unavailable while checking |
  | attachment `Unknown`, `CommandsUnavailable`, `NotReachable`, or `Failed` | not ready | `Unknown` | nothing, and the row names why it could not ask |
  | attachment `NotAttached` | not ready | `Attention` | `Attach` |
  | a service `Failed` | not ready | `Failed` | the service actions |
  | a service `NotInstalled`, `NotReachable`, or `Unknown` | not ready | `Attention` | the service actions |
  | a service `Inactive` | not ready | `Attention` | `Start` |
  | a service `Starting`, `Stopping`, or `Working` | not ready | `Attention` | nothing while it runs |
  | event `Live`, attached, every service `Active` | ready | `Ok` | nothing |

  Two rules the table encodes, and both matter:

  **A query that failed is not an answer.** An attachment state of `Unknown`,
  `CommandsUnavailable`, `NotReachable`, or `Failed` means this app could not
  ask. The row says that, and offers no `Attach`, because an attach would fail
  the same way. Reading a failed query as `NotAttached` invents a fact.

  **A stopped service is not ready.** `Inactive` publishes nothing. A card that
  reads `Ok` because no service failed is the same comfortable lie as one that
  reads `Ok` with a dead event.

- **A new event is `Unknown` until something checks it.**
  `BroadcastRegistry::create_event` stores `last_status = "unknown"`, so a row
  fresh from `Create` or `Replace` sits at `EventState::Unknown` and offers
  neither `Create` nor `Replace`. That is correct: neither action is safe
  against an event whose liveness nobody has read.

  On successful registration, `Create` and `Replace` show the new identifier
  and token path in the mounted row, then request one liveness check for that
  identifier through `BroadcastRegistry::check_event`. A successful metadata
  read stores `Live`; a `404` stores `Dead`. A failed relay read leaves the
  stored status unchanged. A failure to store the check result is also reported
  as a check failure, never as a successful observation.

- **An unknown event has a retryable check action.** `Check` calls
  `check_event` for the selected unknown event. After a failed check, the same
  action reads `Retry check` and becomes available again. This also covers a
  stored unknown event when Show opens, without needing a preceding creation
  in that session. `Create`, `Replace`, and `Check` cannot overlap; their typed
  availability, progress, failure feedback, and accessibility labels come from
  the view model before rendering.

  Registration success and check failure are separate results. If registration
  succeeded, a failed follow-up check keeps the new event selected and preserves
  its registry entry and token file. The mounted row keeps the identifier and
  token path visible, reports the check failure, and offers `Retry check`.
  Retrying checks that same identifier: it never registers, replaces, forgets,
  or changes publisher configuration. The row refreshes in place on completion.
  `Live` advances to the attachment rules above; `404` enables `Replace` only
  after the registry records `Dead`. Repeated failure keeps `Unknown` and
  re-enables `Retry check`. `Create`, `Replace`, and `Attach` remain unavailable
  while liveness is unknown.

  Added 2026-09-09 after review found that a connection failure immediately
  after registration would strand the operator again. Show refresh only reads
  stored status, and the observation actor does not update the registry. Neither
  supplies the missing retry.

- The card keeps the common height. `Live Metadata` now holds three rows where
  the others hold fewer, and ADR 0063 rejects a card that grows.
- The card summary holds the two lines every card holds. ADR 0063 says the first
  names the earliest row that is not ready, and the second names the state of
  the section. The three rows live in the detail. **Do not add a third line.**
- `Create` registers an event through `BroadcastRegistry::create_event` and
  never prints the token. The registry writes the token file, and the row shows
  the path.
- **Neither `Create` nor `Replace` attaches.** An operator registers, then
  attaches. Two actions with two results are easier to recover from than one
  action with a partial failure, and a `Replace` that also attached would change
  the publisher configuration in the same press that changed the identity.
  Extended to `Replace` on 2026-09-09, after the constraint named `Create` only.
- **A dead event stays selected, so `Create` alone does not recover.** The
  registry keeps a dead entry, and an operator who loses an event still has one
  selected. Add `Replace`, available when the selected event is dead. It
  registers a new event and leaves the dead one for `Forget`.
  Reviewed 2026-09-09, after the first version of this packet made `Create`
  available only with no event and left dead-event recovery in the command line,
  which is where an operator was stranded on the same day.
- **The app never replaces a dead event on its own.** This is an ADR 0059
  invariant. `Create` is an operator action and stays one.
- Keep the feed tag and its copy action. ADR 0059 says a listener finds a live
  event only through that tag.

## Implementation Steps

1. Remove `ShowCardKind::Event`, and fold `EventSectionDisplay` into
   `PublisherSectionDisplay` as its first row. Keep the event display type; only
   its home changes.
2. Change the `Live Metadata` card summary to follow the state table above. The
   first line names the earliest row that is not ready.
3. Change the detail panel to render the three rows in chain order, with the
   event row first.
4. Add a `Create` action to the event row, available when no event is selected,
   a `Replace` action for a selected dead event, and `Check` / `Retry check` for
   a selected unknown event. Make availability and check feedback typed, with
   no overlapping registration or check commands.
5. Wire `Create` and `Replace` to `BroadcastRegistry::create_event`. Follow the
   existing application-command and `Working` pattern. Preserve a successful
   registration independently of its follow-up check result.
6. Project the newly registered event immediately, then dispatch `check_event`
   for its identifier. Wire manual Check and Retry check through the same check
   command. Refresh the mounted row after each result; retain the new identifier
   and token path and expose Retry check after failure.
7. Delete the card-grid entry for `Event`, and any helper only it reached.
8. Add view-model tests:
   - the card is not ready with no event, whatever the services say
   - the rows project in the order `Event`, `Producer`, `Publisher`
   - `Create` is available with no event and unavailable with a live one
   - `Replace` is available with a dead event
   - a live but unattached event reads as not ready, and offers `Attach`
   - registration or checking in flight reports progress and disables
     `Create`, `Replace`, and `Check`
   - a new or previously stored unknown event offers `Check` when idle, with
     `Create`, `Replace`, and `Attach` unavailable
   - a failed check retains `Unknown`, exposes its failure separately from
     registration success, and enables `Retry check`
   - a failed attachment query reads as unknown, not as not attached, and
     offers no `Attach`
   - an `Inactive` service leaves the section not ready
   - the feed tag survives the move
9. Add application-command regression tests named `show_event_*`, so the
   `cargo test show --lib` command below runs them. For both Create and Replace,
   drive registration success, initial check failure, and manual retry success.
   Assert one registration, checks against that same identifier, unchanged
   event identity and token path/file, and an in-place projection of `Live`.
   With a confirmed unattached, reachable target and valid attachment inputs,
   the same event then offers `Attach`. For Replace, the old dead entry remains.
   Neither registration nor checking invokes target add/remove or changes
   publisher configuration. Also cover a failed retry that re-enables retry,
   and a retry returning `404` that offers `Replace` only after `Dead` is stored.
10. Add a situational guard citing ADR 0059: no card kind is named for the event,
    and the detail panel renders the event row before the service rows. Preserve
    the existing queue-separation assertions.

## Acceptance Criteria

Mechanical, proved by a test:

- `ShowCardKind` holds three cases, and the grid projects three cards.
- The `Live Metadata` state follows the table above, so a dead event and an
  unattached event both read as not ready while the services run.
- The rows project in chain order.
- `Create` is available only with no event, and `Replace` only with a dead one.
  Both register through `BroadcastRegistry::create_event`.
- `Replace` leaves the dead entry for `Forget`, and removes nothing on its own.
- Application-command tests prove that Replace selects and projects the new
  event in the mounted Show view, while retaining the old dead entry.
- Application-command tests prove that both successful registration paths
  request an initial check for the newly registered identifier.
- View-model tests prove that an idle unknown event offers `Check`; failure
  exposes `Retry check` and keeps `Create`, `Replace`, and `Attach` unavailable.
- The `show_event_*` regression tests prove the failure/retry sequences in step
  9, preserving the registered identity and token file and projecting each
  result without navigation. Registration runs once; retries only check.
- No create path prints token text.
- Neither `Create`, `Replace`, nor a liveness check mutates publisher targets or
  publisher configuration, as verified by application-command tests.
- The feed tag and its copy action still project.

Visual, operator only:

- `Live Metadata` reads top to bottom as the chain: event, producer, publisher.
- With no event, the card says so, and the create action is the obvious press.
- With a dead event, the card says so, and `Replace` is the obvious press.
- The card holds the same height as `Source` and `Stream`.
- After a create, the row shows the new event without a reload.
- After successful Create or Replace followed by a failed check, the row keeps
  the new identifier and token path visible, names the check failure, and offers
  `Retry check`. After restoring relay access, retry updates that same row.
- During checking, progress is visible and duplicate commands are unavailable;
  another failed check makes retry available again.

## Test Commands

- `cargo fmt -- --check`
- `cargo check --quiet`
- `cargo test show --lib --quiet`
- `cargo test --test architecture_tests --quiet`
- `cargo clippy --quiet -- -D warnings`

Do not run the app. Write the operator visual check instead, as AGENTS.md
requires. Include a reproducible local relay fixture that accepts registration,
fails the first metadata read, and permits a later read or returns `404`. Name
the fixture commands, isolated app configuration, required publisher target
state, expected results, and cleanup. Keep the implementation's visual gate
open in this packet, the delivery order, and `docs/pending-human-checks.md` until
a person walks it.

## Expected Final Report Format

1. Files changed
2. Tests run
3. Behavior changed
4. Deviations from task
5. Unresolved concerns
6. Operator visual check

## Escalation Triggers

- The two summary lines cannot carry the event and the services together.
  Report what you would drop, and do not add a third line.
- A guard names `ShowCardKind::Event`. **Update the assertion, do not delete the
  guard.** Reviewed 2026-09-09: some of those guards also assert that the
  broadcast sections stay separate from the queue, which still applies. Delete a
  guard only when every rule it states is gone with the card kind, and say which
  rules those were.
- `create_event` needs a fact the `Show` surface does not hold.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `docs/adr/0059-broadcast-control-surface.md`, the 2026-09-09 amendment
- `src/view_models/show.rs`, `src/ui/composites/show_detail_panel.rs`
- `src/app/show.rs` for application commands and mounted-view refresh
- `src/broadcast/registry.rs` for `create_event` and `check_event`

Goal:
- `Event` becomes the first row of `Live Metadata`, the grid holds three cards,
  and the event row gains `Create`, `Replace`, and `Check` / `Retry check`.

Constraints:
- Rows read in chain order: event, producer, publisher.
- The section state combines the event, the attachment, and the services. No
  event, a dead event, and an unattached event all read as not ready while the
  services run.
- `Replace` recovers a dead event without leaving the app. `Create` covers the
  no-event case only. After either registers successfully, show the new event
  and token path immediately, then call `check_event` for that identifier.
- An unknown event offers `Check`, or `Retry check` after failure. Keep
  `Create`, `Replace`, and `Attach` unavailable until liveness is known.
  Disable registration and check commands while either kind is running.
- Keep registration success separate from check failure. Preserve the new
  identity, registry entry, and token file; report the failure in the mounted
  row and re-enable retry. Retry checks the same identifier and never registers
  another event. A successful read stores `Live`; `404` stores `Dead` and enables
  `Replace`; a failed read preserves `Unknown` and allows another retry.
- Use typed action availability and check feedback. Show refresh and the
  observation actor do not provide a registry liveness retry.
- A failed attachment query is not `NotAttached`. Say the app could not ask, and
  offer no `Attach`.
- An `Inactive` service leaves the section not ready.
- The card keeps the common height and two summary lines.
- Neither `Create` nor `Replace` prints a token. Neither registration nor
  checking changes publisher targets or publisher configuration.
- A guard naming the removed card kind has its assertion updated. It is deleted
  only when every rule it states died with the card kind.

Do not touch:
- `src/broadcast/registry.rs`, `src/broadcast/publisher_targets.rs`,
  `src/runtime/**`, the `Source` and `Stream` sections

Acceptance criteria:
- Three card kinds, three cards.
- A dead or unattached event reads as not ready while the services run.
- `Create` is available only with no event, `Replace` only with a dead one, and
  neither prints a token.
- View-model tests cover unknown-event Check, failure feedback, Retry check,
  and command availability while work runs.
- Application-command tests named `show_event_*` cover both registration paths:
  initial check failure, retry success, repeated failure, and retry `404`.
  Assert one registration, the same identity and token file throughout, no
  target mutation, and mounted-row updates. A live retry with a confirmed
  unattached and otherwise eligible target enables Attach; `404` enables Replace.

Test commands:
- `cargo fmt -- --check`
- `cargo check --quiet`
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

## Implementation Record - 2026-09-09

- `src/view_models/show.rs`: three card kinds; Event inside Live Metadata;
  Producer before Publisher; the complete readiness table; typed Create,
  Replace, Check / Retry check, and Copy feed tag actions; independent
  registration and check feedback.
- `src/app/show.rs`: registry commands through the existing command runner;
  immediate mounted-row projection followed by a check; preserved registry
  identity and token file after failed checks; refresh protection during and
  after mutations.
- `src/ui/composites/show_detail_panel.rs`, `show_card.rs`, and
  `src/ui/shells/show.rs`: nested event detail, callback slots, shared controls,
  feed-tag copying, and removal of the Event card.
- `tests/architecture_tests.rs`: updated three-card assertions and removed the
  obsolete Absent badge assertion. The queue-separation assertions remain.
  Added `adr_0059_event_row_precedes_services_and_registry_actions_do_not_attach`
  (situational, ADR 0059).

The `show_event_*` tests cover both registration paths followed by initial
failure, retry success, repeated failure, and retry 404. They check one
registration, stable identity/token path/file, preservation of the old dead
entry, independent failure feedback, and projected Attach/Replace availability.
They also cover a failed database status write and a refresh arriving during a
command. The command boundary has no publisher mutation call; the architecture
guard protects that separation.

The old detail had no feed-tag Copy control, so this implementation supplies
it to meet the packet's copy requirement. No registry, target service, runtime,
Source, or Stream behavior changed. The unused Event-card helper and Absent
card state were removed. No phase was added.

Verification:

- `cargo test --quiet`: Green, 1,237 unit tests and 213 architecture guards;
  10 existing documentation tests ignored.
- `cargo test show --lib --quiet`: Green, 65 tests.
- `cargo test --test architecture_tests --quiet`: Green, 213 guards.
- `cargo fmt -- --check`, `cargo check --quiet`,
  `cargo clippy --quiet -- -D warnings`, and `cargo build --quiet`: Green.
- Fixture setup, simulated service/target commands, and relay response
  transitions: Green, without launching the desktop app.
- Whitespace and 121 relative documentation links: Green.

Documentation added: the fixture walkthrough and its Python fixture under
`docs/runbooks/`. Documentation updated: this packet, ADR 0059, the ADR index,
delivery order, pending human checks, broadcast operations, and the docs index.
No files moved, no folders created, and no repository-root docs changed.

Fixture follow-up - 2026-09-09: the operator encountered the generic invalid
directory error while changing relay mode. The fixture now distinguishes empty
arguments from invalid paths and provides `locate` to recover an existing
directory across terminals. Discovery refuses missing or ambiguous fixtures.
The walkthrough's situational ADR 0059 directory regression check is Green;
it also proves an empty argument cannot mutate a fixture in the current
directory. A subsequent screenshot showed the production endpoint and a token
path containing the original placeholder directory name: the running window
had not switched to the recovered fixture. The `verify` command now checks the
fixture endpoint, database, library, host, and command stubs before launch.
The extended directory regression check is Green; the walkthrough requires
checking the displayed Source and endpoint before any event action. These
fixture corrections were included in the accepted operator walkthrough below.

## Operator Verification - 2026-09-09

The operator screenshots show the isolated fixture with three matching cards,
no-event Create, registration success followed by a retryable check failure,
the retained Unknown event after reopening, and Dead with Replace available.
The operator then reported all steps passed for Replace, its initial check
failure, retry to Live on the same new event, and explicit Attach changing the
section to ready. Replacement did not attach automatically.

After the remaining checks were listed, the operator confirmed: "all broadcast
event recovery tests pass". This closes feed-tag Copy, inactive and failed
Producer readiness and recovery, resizing and detail-panel checks, and fixture
registry/token preservation and command separation as well as the event
recovery sequence above. Mechanical and operator acceptance are complete.

The acceptance uses the isolated relay and simulated services; it does not
claim production relay or publisher connectivity. The delivery-order row,
ADR 0059 and its index, and the fixture walkthrough record completion. Task 016
has been removed from pending human checks.

## Operator Visual Check

Passed - 2026-09-09. No further visual check is required for this implementation.
The [isolated event recovery walkthrough](../runbooks/broadcast-event-recovery-check.md)
remains the situational ADR 0059 manual regression check, with numbered setup
commands, expected and wrong results, and cleanup. It requires Python 3.11 or
newer, a desktop session, and free port 17863; publisher and service states are
simulated. The operator ran the desktop app and reported acceptance.
