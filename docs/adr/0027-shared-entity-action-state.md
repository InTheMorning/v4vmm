# ADR 0027: Shared Entity Action State

## Status

Proposed - 2026-05-01.

## Context

ADR 0026 introduced shared entity projections and a shared release-detail shell.
The post-ADR 0026 visual smoke compared `The Heycitizen Experience` in Library
and Discover at the same viewport and confirmed that the shell now aligns.

The remaining mismatch is not primarily layout. Equivalent entity actions are
still modeled and rendered through screen-local state:

- Discover renders a quiet `Remove Feed` action while Library renders a
  prominent destructive `Unsubscribe Feed` action for the same release.
- Discover track rows render quiet icon removal, playlist, and selection
  affordances while Library track rows render large repeated `Remove` buttons.
- Library still renders detail state such as `Downloaded 19` even when Library
  membership is already implied by the row removal affordance.
- MusicBrainz, playlist popovers, in-flight download/remove state, and
  membership state are still composed from screen-local view-models.

ADR 0026 already has `EntityActionVm`, `EntityActionKind`,
`EntityActionTarget`, and `EntityActionTone`, but those descriptors are not yet
fed by a shared action-state input. Screens can therefore agree on layout while
still disagreeing on state, tone, labels, and row density.

The ideal architecture in `docs/architecture/architecture-diagrams.md` keeps
GPUI thin over pure view models and application services. This ADR advances
that target by making action state a GPUI-free input to the shared projection
layer while keeping command dispatch in screen/application adapters.

## Decision

Introduce shared, GPUI-free action-state inputs for release and track detail
projections. These inputs describe already-known state; they do not fetch data,
read SQLite, call services, open popovers, or execute commands.

The intended flow is:

```text
application query / existing screen state
  -> EntityActionState inputs
  -> shared entity projection action descriptors
  -> shared shell/control roles
  -> screen adapter binds click handlers and popover state
```

Add narrow pure-data structs under `src/view_models/entity_detail.rs`, or a
small sibling module if the file becomes too large:

```rust
pub struct ReleaseActionState {
    pub membership: ReleaseMembershipState,
    pub musicbrainz: MusicBrainzActionState,
    pub playlist: PlaylistActionState,
}

pub enum ReleaseMembershipState {
    RemoteOnly,
    InLibrary,
    PartiallyInLibrary,
    Updating,
}

pub struct TrackActionState {
    pub membership: TrackMembershipState,
    pub playlist: PlaylistActionState,
    pub musicbrainz: MusicBrainzActionState,
}

pub enum TrackMembershipState {
    RemoteOnly,
    Downloading,
    InLibrary,
    Removing,
}
```

Exact names may change during implementation, but the public shape must remain
plain data with no GPUI, service, DB, or screen imports.

`ReleaseDetailVm` and shared track-row projections use these inputs to emit
`EntityActionVm` descriptors with consistent:

- action kind
- target
- label
- enabled/busy state
- tone
- implied detail-state suppression

Screen adapters remain responsible for:

- converting current screen/application state into the pure action-state input
- binding click handlers
- routing commands through ADR 0024 boundaries
- owning popover open/closed state
- resolving GPUI images and controls

## Invariants

- `src/views.rs` and `src/view_models/entity_detail.rs` remain GPUI-free.
- Shared projections do not import `library`, `search`, services, SQLite, or
  MusicIndex clients.
- Action-state structs are input state only; they do not own dispatch.
- Repeated destructive row actions use quiet destructive treatment through the
  ADR 0025 control system.
- Library detail projections do not show redundant downloaded state when
  membership is already represented by remove actions.
- Screens may expose extra actions, such as MusicBrainz, only through shared
  action descriptors or explicit shell slots with matching tone rules.

## Consequences

- Library and Discover can keep different command handlers while sharing the
  same visible action vocabulary.
- Visual fixes for row buttons and destructive treatments become mostly
  design-system work after the projection state is shared.
- Tests can validate action labels, tones, and redundant-state suppression
  without launching GPUI.
- This creates a clearer handoff to ADR 0024: application queries can later
  produce the same state inputs without changing the shared shell.

## Alternatives Considered

### Keep Screen-Local Action View-Models

Rejected. This is the current state and it allowed equivalent content to keep
different row action vocabulary after ADR 0026 shell parity landed.

### Move Command Dispatch Into Shared UI

Rejected. It would violate ADR 0024 and ADR 0026 by pulling GPUI handlers,
screen state, and service calls into the shared projection/UI layer.

### Fix Only Button Styling

Rejected as incomplete. Styling can make the Library rows less loud, but it
does not solve divergent membership labels, busy states, redundant detail rows,
or screen-local action semantics.

## Non-Goals

- Do not move service calls, database reads, or command execution into shared
  projections.
- Do not redesign Library or Discover navigation.
- Do not define the local identity/source-fact persistence schema; that belongs
  in a separate schema ADR if accepted.
- Do not implement non-URL artwork resolution.

## Follow-Up Work

- Use `docs/plans/adr-0027-shared-entity-action-state-phase-plan.md` for the
  migration sequence.
- Start with `docs/tasks/adr-0027-task-001-track-row-action-state.md`, which
  covers track-row membership actions for Library and Discover.
- Add projection tests for action-state inputs, labels, tones, and redundant
  downloaded-row suppression.
- Add architecture tests preventing GPUI/screen/service imports in any new
  action-state module.
- After ADR 0027 lands, create a bounded ADR 0025 task for destructive row
  control treatment if the existing `ControlStyle` roles are insufficient.
