# ADR 0059 Task 016: Event Becomes The First Row Of Live Metadata

Status: Ready - 2026-09-09. Follows the ADR 0059 amendment of the same day, and
the ADR 0063 amendment that reduces the grid to three cards.

## Goal

Move `Event` out of its own card and into `Live Metadata`, as the first of three
rows. Make the section state account for the event. Add the create action that
the surface has never had.

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
- `src/broadcast/registry.rs`, for `create_event` and `CreatedBroadcastEvent`
- `src/cli.rs`, for the create command that already exists
- `tests/architecture_tests.rs`

## Files Likely To Change

- `src/view_models/show.rs`
- `src/ui/composites/show_detail_panel.rs`
- `src/app/show.rs`
- `tests/architecture_tests.rs`

## Do Not Touch

- `src/broadcast/registry.rs`. `create_event` exists and this task calls it.
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
  | `EventState::Unknown` | not ready | `Unknown` | nothing yet |
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

  `Create` and `Replace` therefore request a liveness check when they finish.
  `BroadcastRegistry::check_event` maps a `404` to `Dead` and any other error to
  a transport failure that changes no stored status, so a check that cannot
  reach the relay leaves the row at `Unknown` rather than claiming a state.

  Without that request the row would sit at `Unknown` until the next
  observation, and an operator who just created an event would see neither a
  confirmation nor an action.

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
   and a `Replace` action, available when the selected event is dead. Both are
   unavailable while either is running.
5. Wire it to `BroadcastRegistry::create_event`, and report the result the way
   the other commands do. Follow the `Working` precedent so the press answers at
   once.
6. Refresh the event row from the registry after a create, so the new identifier
   and token path appear without a reload.
7. Delete the card-grid entry for `Event`, and any helper only it reached.
8. Add view-model tests:
   - the card is not ready with no event, whatever the services say
   - the rows project in the order `Event`, `Producer`, `Publisher`
   - `Create` is available with no event and unavailable with a live one
   - `Replace` is available with a dead event, and recovers without a reload
   - a live but unattached event reads as not ready, and offers `Attach`
   - a create in flight reports it and disables the action
   - neither `Create` nor `Replace` changes the attachment
   - a new event reads as `Unknown` and offers neither action until it is
     checked
   - a failed attachment query reads as unknown, not as not attached, and
     offers no `Attach`
   - an `Inactive` service leaves the section not ready
   - the feed tag survives the move
9. Add a guard: no card kind is named for the event, and the detail panel
   renders the event row before the service rows.

## Acceptance Criteria

Mechanical, proved by a test:

- `ShowCardKind` holds three cases, and the grid projects three cards.
- The `Live Metadata` state follows the table above, so a dead event and an
  unattached event both read as not ready while the services run.
- The rows project in chain order.
- `Create` is available only with no event, and `Replace` only with a dead one.
  Both register through `BroadcastRegistry::create_event`.
- `Replace` leaves the dead entry for `Forget`, and removes nothing on its own.
- An operator recovers from a dead event without leaving the app.
- No create path prints token text.
- Neither `Create` nor `Replace` performs an attach.
- The feed tag and its copy action still project.

Visual, operator only:

- `Live Metadata` reads top to bottom as the chain: event, producer, publisher.
- With no event, the card says so, and the create action is the obvious press.
- With a dead event, the card says so, and `Replace` is the obvious press.
- The card holds the same height as `Source` and `Stream`.
- After a create, the row shows the new event without a reload.

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
- `src/broadcast/registry.rs` for `create_event`

Goal:
- `Event` becomes the first row of `Live Metadata`, the grid holds three cards,
  and the event row gains `Create` and `Replace`.

Constraints:
- Rows read in chain order: event, producer, publisher.
- The section state combines the event, the attachment, and the services. No
  event, a dead event, and an unattached event all read as not ready while the
  services run.
- `Replace` recovers a dead event without leaving the app. `Create` covers the
  no-event case only. A new event is `Unknown` until checked, and offers
  neither, so both request a liveness check when they finish.
- A failed attachment query is not `NotAttached`. Say the app could not ask, and
  offer no `Attach`.
- An `Inactive` service leaves the section not ready.
- The card keeps the common height and two summary lines.
- Neither `Create` nor `Replace` prints a token, and neither attaches.
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
