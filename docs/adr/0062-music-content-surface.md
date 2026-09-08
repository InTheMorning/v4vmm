# ADR 0062: Music Content Surface

## Status

Accepted - 2026-09-07.

Extends ADR 0060. Reverses the Recent Feeds reachability invariant that ADR
0030 established.

## Context

ADR 0060 task 003 promoted the content filter from a pulldown to visible chips.
An operator then reported that selecting `All`, `Library`, or `Index` changed
nothing.

The filter is not broken. `ContentListPageVm::visible_rows` filters
`cached_rows` correctly, and `matches_filter` is called. The defect is that the
default `Music` view does not render those rows. It renders the source tree:
playlists and artists. The chips are a correct control over a row set that is
not on screen.

The same cause produces the second complaint. Six artist rows stretch across a
1400 pixel window with nothing beside them, because a navigation tree occupies
the region ADR 0060 reserved for content.

ADR 0060 said the content region must hold the dominant share of the window.
Task 003 delivered that geometrically. The region is large and it is filled
with navigation.

Separately, `Recent Feeds` is a toolbar command with its own surface, its own
view model, and a reachability invariant from ADR 0030. It answers one
question: what was released recently. That is a sort order, not a destination.

## Decision

### Music Opens On Recent Music

The default `Music` view renders music, not navigation. With no search and no
selection it shows rows ordered by feed publish date, newest first, paged.

**All music is what scrolling reveals, not a separate query.** Sorting by
recency and paging means the newest rows arrive first and the rest follow. There
is no distinct "everything" mode to load, and no large query at startup.

The source tree and the breadcrumb stay. The tree is navigation and it keeps
its own region. It does not occupy the content region.

### A Row Is An Entity, Not A Fixed Shape

Rows are mixed by entity type:

- A single renders as a track row.
- A release renders as a row that expands to its tracks.
- An artist renders as a row that expands to its releases.

Every row carries two badges:

- an entity badge naming what it is, so a track, release, or artist is
  recognizable without reading the layout
- a library badge stating whether the row is already in the library

Both badges are view-model facts. A renderer does not decide them.

### One Library Control With Three States

The three source chips become one control with three states:

| State | Meaning | Shows |
|---|---|---|
| Highlighted | In library | Library rows only |
| Off | No constraint | Library and index |
| Struck through | Not in library | Index rows only |

The underlying model does not change. `ContentFilter::Library`,
`ContentFilter::All`, and `ContentFilter::Index` already express these three
states, and every consumer keeps working. Only the control and its
accessibility contract change.

The struck-through state must announce itself as `Not in library` to assistive
technology. Strike-through is a visual convention and carries no meaning on its
own.

### Recency Is The Only Sort For Now

Feed publish date is the default order and the only order this ADR ships.

**A sort order over paged remote results cannot be applied locally.** A client
can only order the page it holds, which produces a list that reorders as the
operator scrolls. Every additional sort therefore needs the index to support it,
and this ADR does not assume that support exists.

The sort control is built to hold more than one option, and ships with one.
Adding a sort is an index capability question before it is an interface
question.

The `Recent Feeds` toolbar command and its separate surface are removed. The
ADR 0030 reachability invariant is withdrawn, because the destination it
protected no longer exists.

`src/view_models/recent_feeds.rs` is not deleted. Its index query becomes the
data source behind the default order. The logic is still needed. Only the
destination goes away.

### List And Tiles Are View Modes

`Music` renders as a list or as tiles. Both show the same rows, the same
badges, and respond to the same filter and sort.

The mode is a view preference. It changes no row content and no filter
semantics.

## Invariants

- The `Music` content region renders rows, never a navigation tree.
- The default order is feed publish date, newest first, and it pages.
- No sort order is offered that the index cannot produce.
- The source tree and the breadcrumb remain reachable.
- Every row carries an entity badge and a library badge from a view model.
- The library control has exactly three states, mapping to the existing
  `ContentFilter` values.
- The struck-through state announces `Not in library` and never relies on the
  strike-through alone.
- No toolbar command opens a separate recent-feeds surface.
- The filter and the sort apply identically in list mode and in tile mode.

## Alternatives Considered

### Keep The Tree As The Default View

Rejected. It is the current state and it produced both complaints. It also
leaves the filter inert in the state a curator sees first, which reads as a
broken control.

### Disable The Filter Until A Selection Exists

Rejected. It makes the control honest without making the surface useful, and it
leaves the content region full of navigation.

### Keep The Segmented Control

Rejected. The current implementation is a segmented control, and
`adr_0047_phase_d_filter_chip_strip_renders_through_frame_shell` requires
`SegmentedControl::new(selected).filter_style()`. This ADR therefore replaces a
working control rather than adding one, and that guard changes with it.

The reason is that a segmented control states the wrong structure.

**Library membership is one axis, not three options.** A segmented control
reading `All`, `In library`, `Not in library` presents three peers. They are
not peers. `In library` and `Not in library` are the two real positions, and
`All` is the absence of a constraint. A control that renders the null case as a
sibling of the two real cases misdescribes what the operator is choosing.

**Filter axes multiply and segmented controls do not.** Library membership is
the first filter this surface needs and it will not be the last. Downloaded
state, artwork presence, and payment-route presence are all the same shape:
a property to require, ignore, or exclude. Four segmented controls in a row is
not a filter bar. Four tri-state toggles is.

This is the deciding argument. The choice is not between two ways to render one
filter. It is between a control that composes as filters accumulate and one
that does not.

**Apple HIG does not cover this case.** HIG names patterns for choosing among
options and for boolean settings. `components/toggles.md` defines the mixed
state as indeterminacy, "show mixed state when subordinate checkboxes have
different states", and calls it "rarely useful" on radio buttons. Segmented
controls answer "select only one segment at a time". A three-position filter
axis is neither of those things.

**The design fills a gap in the guidance rather than breaking a rule.** HIG
remains structural guidance for this project, and this is a considered
departure recorded as one, not an oversight.

Requirements that follow from the choice:

- The control carries an explicit text label for its current state, so the
  state is never carried by the visual treatment alone.
- The struck-through state announces `Not in library` to assistive technology.
- The three states are reachable by keyboard in a predictable order.

Revisit if operators misread the struck-through state in use.

### Keep Recent Feeds As A Destination

Rejected. It answers a sort question with a whole surface, a view model, a
toolbar command, and an architecture guard. Every one of those is cost carried
to express an ordering.

### Track Rows Only

Rejected. A release that cannot expand forces a curator to reason about a
release through its tracks, and the artist tree already proves that releases
and artists are how this library is browsed.

## Consequences

Positive:

- `Music` opens on music. The name and the surface agree.
- The filter acts on what the curator is looking at, in the first state they
  see.
- The content region holds content rather than navigation.
- One surface answers browse, filter, sort, and recency. There is no second
  destination to keep reachable.
- Badges make library membership and entity type readable without opening a
  detail.

Negative and risks:

- The tri-state control is less immediately self-describing than the segmented
  control it replaces. An operator learns the cycle once. The text label carries
  the state in the meantime.
- Replacing a guarded control means the ADR 0047 filter-chip guards change with
  this ADR rather than surviving it.
- The default view is a paged remote query rather than a local tree read. Paging
  and windowing from ADR 0041 apply, and this ADR does not restate them.
- One sort order is a thin sort control. It exists to make the shape right, and
  it looks like unfinished work until the index supports a second order.
- Mixed row shapes mean one row contract carries track, release, and artist
  cases. That contract must not become a union of three unrelated shapes.
- Removing the `Recent Feeds` command retires an ADR 0030 invariant and its
  guards. A curator who used that button must learn the sort.
- Tile mode is a second renderer for the same rows and can drift from the list.

## Follow-Up Work

- Delete the `Recent Feeds` reachability guards, including the one asserting
  `return_to_recent_feeds` in the parked discover module.
- Update `adr_0047_phase_d_filter_chip_strip_renders_through_frame_shell` and
  `adr_0047_task_010_content_list_filter_chips_are_frame_local`. Both require
  the segmented chip strip that this ADR replaces.
- Reuse rather than rebuild. `RecentFeedsViewMode` already provides tiles and
  list. `RecentFeedsPageVm` already provides cursor paging and load-more.
  `ArtistResultDisplay`, `FeedResultDisplay`, and `TrackResultDisplay` already
  exist, separated today by `SearchResultsTab`. The mixed row merges those three
  tabs into one list, and the entity badge replaces the tab.
- Amend ADR 0030 to record that its reachability invariant is withdrawn.
- Task packets for the row contract, the library control, the sort, and the
  tile mode.
- Decide whether tile mode persists per section or globally.
- Establish which sort orders the MusicIndex API can produce, before any second
  sort is designed.

## References

- ADR 0030 - Discovery and library UI correctness fixes
- ADR 0041 - Windowed paged view models
- ADR 0047 - Library and search unification
- ADR 0060 - Workflow surface structure and vocabulary
- ADR 0061 - Current-state governance
- `docs/plans/curator-workflow-ui-design-brief.md`
