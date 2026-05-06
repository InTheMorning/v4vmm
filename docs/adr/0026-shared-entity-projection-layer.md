# ADR 0026: Shared Entity Projection Layer

## Status

Implemented - 2026-05-01.

## Context

ADR 0023 established the token, primitive, composite, and GPUI-free
view-model foundation. ADR 0024 introduced the application boundary for
commands, queries, and events. ADR 0025 tightened the visual-system boundary
so theme, icon, and control-style changes live under `src/ui/` instead of in
screen files.

Those decisions reduced duplication, but Library and Discover still project
the same entities through different screen-local paths. A feed/album shown in
Discover and the same feed/album shown from Library can have different action
layout, button severity, metadata density, row controls, and identity
affordances. The user-visible result is that the same content can feel like
two different products.

The MusicIndex API has also grown richer identity data. Artists, items, and
individual contributors can now carry Nostr identities, webpages, and images
through fields such as `source_links`, `source_ids`, and contributor-level
identity fields. The app currently preserves some of these source facts for
feeds and tracks, but contributor identity is under-modeled and the UI
flattens identity affordances inconsistently. ADR 0026 must therefore solve
two related problems:

1. Library and Discover need one shared entity projection contract.
2. Identity facts must be preserved and projected consistently without
   discarding provenance.

The ideal architecture in `docs/architecture/architecture-diagrams.md`
describes GPUI views as thin presentation adapters over view-models, shared
design-system components, and application/domain services. This ADR advances
that target by adding a canonical entity projection layer between loaded
source facts and GPUI rendering.

## Decision

Introduce a shared, GPUI-free entity projection layer for artists,
feeds/releases, tracks/items, and contributors. The layer normalizes loaded
API/local data into source-preserving view facts, then projects those facts
into display-ready entity-detail models consumed by shared UI composites.

The intended flow is:

```text
application queries / existing screen loads
  -> source-specific API / DB rows
  -> source-normalized entity facts
  -> shared entity projections
  -> slot-based design-system shells
  -> thin screen adapters bind actions
```

The projection layer does not fetch network data, read SQLite, call services,
mutate state, or own GPUI handlers. It receives already-loaded facts and
returns plain Rust display data.

### Module shape

Add or extend these modules:

```text
src/views.rs
  IdentityLinkFact
  IdentityIdFact
  ArtworkRef
  ArtistView
  FeedView
  TrackView
  ContributorView
  EntityIdentityLinks

src/view_models/entity_detail.rs
  EntityHeaderVm
  ReleaseDetailVm
  TrackListVm
  SharedTrackRowVm
  ContributorListVm
  IdentityLinksVm
  EntityActionVm
  EntityActionTarget
  EntitySurfaceContext

src/ui_entity.rs
  release detail shell

src/ui/composites/identity_action.rs
  IdentityActionKind
  identity_action_button
```

`src/views.rs` remains the source-normalized fact layer. It may contain API
and local-row conversion constructors, but the resulting public view facts must
stay GPUI-free, screen-free, and independent of the concrete MusicIndex API
struct layout. API rows should be copied into local source-fact structs such as
`IdentityLinkFact` and `IdentityIdFact` instead of exposing
`api::SourceEntityLink` or `api::SourceEntityId` directly from the shared view
contract. This keeps a future query service or non-MusicIndex source from
having to mimic the HTTP client's Rust structs.

`src/view_models/entity_detail.rs` owns the pure shared projections. It
formats titles, subtitles, metadata rows, summaries, identity action labels,
track-row labels, and contributor groups. It emits semantic action
descriptors, not GPUI buttons.

`src/ui_entity.rs` is a thin GPUI shell over the shared projections. It
composes ADR 0023 and ADR 0025 design-system pieces such as `DetailHeader`,
`DetailGrid`, `TrackRow`, `TagBadge`, `Thumbnail`, `ReleaseDetailSurface`,
`Icon`, and `ControlStyle`, but it must not own workflow dispatch. Shared
identity affordance styling lives in `src/ui/composites/identity_action.rs`.
To avoid a generic renderer that imports `SearchApp` or `LibraryApp`, action
controls are provided through explicit slots or binder structs supplied by the
screen adapter. `src/ui_entity.rs` may render the common surface and place the
slots; the screen adapter binds click handlers to existing commands, popovers,
and state transitions.

`src/ui_entity.rs` may import GPUI and design-system modules. It may not import
MusicIndex clients, SQLite services, RSS services, MusicBrainz services,
download services, `search`, or `library`.

### Identity model

Add one shared identity container:

```rust
pub struct IdentityLinkFact {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub position: Option<i64>,
    pub link_type: Option<String>,
    pub url: Option<String>,
    pub source: Option<String>,
    pub extraction_path: Option<String>,
    pub observed_at: Option<i64>,
}

pub struct IdentityIdFact {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub position: Option<i64>,
    pub scheme: Option<String>,
    pub value: Option<String>,
    pub source: Option<String>,
    pub extraction_path: Option<String>,
    pub observed_at: Option<i64>,
}

pub struct EntityIdentityLinks {
    pub nostr_npub: Option<String>,
    pub website_url: Option<String>,
    pub image_url: Option<String>,
    pub source_links: Vec<IdentityLinkFact>,
    pub source_ids: Vec<IdentityIdFact>,
}

pub enum ArtworkRef {
    Url(String),
    CacheKey(String),
    LocalPath(String),
    EmbeddedBytesKey(String),
}
```

`ArtistView`, `FeedView`, and `TrackView` must each expose an
`EntityIdentityLinks` value. `ContributorView` must represent contributor
identity explicitly:

```rust
pub struct ContributorView {
    pub name: Option<String>,
    pub role: Option<String>,
    pub group_name: Option<String>,
    pub href: Option<String>,
    pub image_url: Option<String>,
    pub nostr_npub: Option<String>,
}
```

Feed and track contributor collections must use `ContributorView`, not
`api::Contributor`, once they cross into the shared view fact layer.

API structs must be extended to deserialize contributor identity fields from
MusicIndex responses. Expected fields include at least `href`, `img`, and
`npub`; the implementation should preserve raw source facts through
`source_links` and `source_ids` where available. The source-normalized view
layer owns the conversion from concrete API structs to `IdentityLinkFact` and
`IdentityIdFact`.

Identity extraction rules must be conservative:

- `source_ids` with `scheme = "nostr_npub"` may populate `nostr_npub`.
- `source_links` with `link_type = "website"` may populate `website_url`.
- entity image fields may populate `image_url`.
- contributor `npub`, `href`, and `img` populate contributor identity fields.
- raw `source_links` and `source_ids` remain available even when a convenience
  field is populated.

The projection layer must not infer identity from title, filename, publisher
text, artist name, or fuzzy matching.

### Shared release/detail projection

Feeds from Discover and albums from Library represent the same presentation
shape: release-like content with header identity, metadata, description,
contributors, tracks, and actions. Represent that shared shape as
`ReleaseDetailVm`.

`ReleaseDetailVm` owns:

- header title, subtitle, entity kind, and artwork source as plain data
- identity affordances such as RSS, Nostr, website, and external link
- scalar detail rows
- description text and collapsed/expanded labels
- contributor list projection
- track-list summary, such as `19 total · 1 h 28 min`
- shared track-row projections
- semantic action descriptors

The projection layer must not carry `gpui::Image`, `SharedString`,
`AnyElement`, `Window`, `App`, or loaded image handles. Artwork should be
represented as `ArtworkRef` or an equivalent plain-data type that the
screen/image-cache adapter resolves later.

Library and Discover may differ in available actions, but not in layout
contract. Examples:

- Discover track row: download/remove toggle, add-to-playlist, play.
- Library track row: remove, add-to-playlist, play, optional compare and
  MusicBrainz status affordances.
- Library must not render "downloaded" text for tracks already in Library;
  the remove action implies the state.
- Repeated destructive track actions must use a quiet destructive treatment
  through `ControlStyle`, not large filled destructive buttons.

### Action descriptors

Projection VMs return action descriptors instead of GPUI elements:

```rust
pub enum EntityActionKind {
    Download,
    Remove,
    AddToPlaylist,
    Play,
    CompareMetadata,
    OpenMusicBrainz,
    OpenWebsite,
    CopyNostr,
}

pub enum EntityActionTarget {
    Artist(ArtistRef),
    Feed(FeedRef),
    Track(TrackRef),
    Contributor(String),
}

pub enum EntityActionTone {
    Primary,
    Secondary,
    Quiet,
    DestructiveQuiet,
}

pub struct EntityActionVm {
    pub kind: EntityActionKind,
    pub target: EntityActionTarget,
    pub label: String,
    pub enabled: bool,
    pub tone: EntityActionTone,
}
```

Screen adapters map descriptors to existing commands, event handlers, and
popover state. `EntityActionTone` is a pure projection concept; `src/ui_entity`
maps it to ADR 0025 control roles. This keeps shared projections UI-agnostic
while still letting Library and Discover provide different behavior for the
same semantic action.

The projection input must include a small, explicit context value rather than
letting screens fork projection logic:

```rust
pub enum EntitySurfaceContext {
    Discover,
    Library,
}
```

If a later phase needs more state, such as MusicBrainz status or in-flight
download state, add narrow GPUI-free input structs to
`view_models::entity_detail`; do not read screen state directly from the
projection.

### Relationship to ADR 0024

This ADR does not introduce a new fetching abstraction. Earlier planning notes
considered a `MetadataSource` trait consumed by shared UI views. ADR 0026
rejects that direction because fetching belongs behind application queries,
existing screen load paths, or future ADR 0024 query-service work. Shared
projections must receive loaded facts and stay pure.

## Invariants

1. `src/views.rs` and `src/view_models/entity_detail.rs` must not import
   `gpui`, `gpui_component`, `library`, `search`, or service modules.
2. `src/view_models/entity_detail.rs` must not import `ui`, `ui_entity`, or
   API client types.
3. `src/views.rs` may contain conversion constructors from API and DB rows,
   but its exported shared fact types must not expose concrete API row structs
   as public fields.
4. Shared projection constructors and accessors are pure and unit-testable
   without a GPUI runtime.
5. Identity convenience fields never replace raw source facts. Raw
   `source_links`, `source_ids`, and contributor identity fields remain
   available for provenance and future UI.
6. Screens may choose context and handle actions, but they must not fork the
   shared layout contract for the same entity type.
7. New reusable buttons, icons, badges, and colors used by this work must flow
   through ADR 0025 boundaries.
8. No service call, database query, network fetch, download, MusicBrainz
   lookup, RSS parse, or tag write may be added to the shared projection layer.
9. `src/ui_entity.rs` must not import `search`, `library`, or service modules;
   screen adapters bind handlers through slots or binder structs.

## Non-Goals

- Moving all Library and Discover service dispatch to ADR 0024 application
  queries in one pass.
- Introducing a generic `MetadataSource` trait for UI rendering.
- Implementing a theme redesign, visual rebrand, or custom accent editor.
- Adding artist identity reconciliation or fuzzy identity matching.
- Changing database schema unless a task proves local identity facts cannot be
  preserved without it.
- Replacing all remaining screen-level GPUI code in one migration.

## Alternatives Considered

### Continue screen-local projection

Rejected. This preserves current drift: Library and Discover can keep rendering
the same feed or track with different metadata ordering, row action shape, and
identity affordances.

### Shared renderer without shared projections

Rejected. A shared GPUI function over raw `api::Track` and `db::TrackRow`
would still embed source-specific branching and action policy in the renderer.
The correct seam is a source-normalized projection.

### `MetadataSource` trait consumed by UI

Rejected for this ADR. Fetching through a UI-owned source trait would conflict
with ADR 0024's direction and make shared rendering responsible for workflow
coordination. Application queries can provide loaded data later without
changing the projection contract.

### Big-bang Library and Discover rewrite

Rejected. The migration has enough surface area that it must proceed through
vertical slices: API identity, view facts, projection VMs, Discover adoption,
Library adoption, then contributor identity UI.

## Consequences

- There will be more intermediate Rust types, but they make display semantics
  testable and keep GPUI screens thinner.
- Library and Discover still have separate action handlers. That is acceptable
  as long as action descriptors and layout projection are shared.
- The richer identity model creates a stable place to expose Nostr, webpage,
  and contributor image affordances without spreading link extraction across
  screens.
- Future ADR 0024 query-service work can feed the same projection layer
  without changing GPUI rendering.

## Follow-Up Work

Implementation is tracked in
`docs/plans/adr-0026-shared-entity-projection-phase-plan.md`, starting with
`docs/tasks/adr-0026-task-001-identity-facts.md`.
