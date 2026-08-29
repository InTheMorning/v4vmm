# ADR 0043: Top Toolbar Now Playing Frame and Global Search

## Status

Accepted - 2026-05-08. Implementation partial: follow-up fixes landed.
Operator visual recheck outstanding. See
`docs/reviews/adr-0043-review-checklist.md`.

## Context

The app currently has a top tab bar that also hosts the Now Playing
control group. Library and Discover each own their own search field:
Library search filters local library tree state, while Discover search
queries MusicIndex and owns result filters.

This makes two related problems visible:

- Now Playing does not read as its own framed control space. Track
  identity and transport controls share the same visual plane as app
  navigation.
- Search is split across local and remote contexts even though people
  expect one clear place to find content in an app. Apple HIG guidance
  favors a distinct searchable location, starting broad and allowing
  scope refinement.

The recent UI direction in ADRs 0033, 0038, and 0042 requires that this
be handled structurally, not by adding screen-local wrappers or copied
search rows.

## Decision

Replace the current tab-bar-shaped top strip with an app toolbar that
has three stable zones:

- Leading: a subtle app mark and navigation tabs.
- Center: one global search field with scope control.
- Trailing: a framed Now Playing region containing track state and
  existing transport controls.

The global search field is owned by `TopApp`, not by Library or Search.
Pressing Enter or Search routes to the Search workspace and runs a
query according to `GlobalSearchScope`:

- `All`: grouped local Library results followed by MusicIndex results.
- `Library`: local in-library results only.
- `Index`: MusicIndex results only.

The Discover tab becomes the Search workspace. Its empty state may
continue to show recent feeds/discovery content when no global query is
active.

Now Playing remains app-shell-owned under `src/app/` because it has one
top-level call site. It must not be extracted into `ui/composites`
unless a second real call site appears, per ADR 0042.

## Alternatives Considered

- Keep separate Library and Discover search fields and only restyle
  them. Rejected because it preserves the split mental model and does
  not satisfy the one-searchable-location goal.
- Put global results in an inline toolbar popover. Rejected for v1
  because it introduces more floating chrome and makes grouped result
  rendering harder to verify.
- Move Now Playing to a bottom bar. Rejected because this macOS-style
  app should avoid putting important controls at the bottom edge, where
  windows can be partially obscured.
- Run remote search live while typing. Deferred because existing
  command behavior is Enter/Search driven and live remote search would
  require cancellation/coalescing policy.

## Consequences

Positive:

- Search becomes a predictable top-level command, aligned with HIG
  toolbar search guidance.
- Now Playing gains a clear visual owner and can grow transport affordances
  without crowding navigation.
- Library and Search screens lose duplicate search chrome.
- The architecture gains explicit toolbar view-model contracts and guards.

Negative:

- `TopApp` owns another input entity and must coordinate routing to the
  Search workspace.
- Search results need a small local query path for in-library matches.
- Existing Discover search input code must be retired carefully so
  keyboard focus, recent feeds, and pagination keep working.

## Invariants

- Toolbar display strings, scope labels, placeholders, ids, and
  accessibility labels live in a GPUI-free view model.
- The toolbar and app menu do not expose a product name before naming is
  decided. MusicIndex attribution belongs in a future About/settings surface,
  not persistent top-level chrome.
- Library and Search screens do not create their own visible search
  fields after this ADR lands.
- Local Library search returns only tracks currently in the library.
- MusicIndex type filters apply only to MusicIndex results, not local
  Library results.
- Now Playing uses tokens and icon controls with stable hit targets.
- Now Playing remains in `src/app/` unless a second real call site
  justifies extraction.
- Visual proof is required in light and dark themes before the feature
  is called complete.

## Non-Goals

- No playback queue, scrubber, volume, output picker, or expanded player.
- No Spotlight/system search integration.
- No schema migration.
- No live/debounced remote search in this ADR.
- No palette redesign or broad typography reset.

## Follow-Up Work

- Revisit live local filtering or debounced remote search after the
  toolbar/result architecture is stable.
- Consider richer Now Playing controls in a future playback-specific ADR.
