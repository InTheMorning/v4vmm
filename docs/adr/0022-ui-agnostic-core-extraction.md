# ADR 0022: UI-Agnostic Core Extraction

## Status

Implemented - 2026-05-01. Green criteria met: `subscribe_service`,
`feed_service`, and `metadata_service` own their domains, and `src/metadata.rs`
has no GPUI imports.

## Context

ADR 0015 established that workflow behavior should live in non-UI service
modules. Several services exist and are clean (`db`, `library_service`,
`playlist_service`, `playback`, `playback_owner`, `playback_driver`,
`api`, `musicbrainz`, `audio_tags`, `track_compare`, `rss`,
`track_identity`). They contain no GPUI references and are reachable from
both the CLI and tests.

However, two GPUI files have grown into mixed-concern modules that contain
substantial domain logic alongside their `Render` impls:

| File | Lines | Functions | GPUI references |
|------|-------|-----------|-----------------|
| `src/library.rs` | 5,598 | 142 | 205 |
| `src/search.rs` | 8,020 | 227 | 210 |

Domain logic currently embedded inside these UI files:

- `library::subscribe_library_track` — download, ID3 tag, mark in library.
- `search::subscribe_track_from_search` and `search::subscribe_feed_from_search` —
  near-duplicates of the above with feed-subscription nuance.
- `search::ensure_feed_in_db` — feed insertion side effect of search workflows.
- `search::id3_edits_for_track_context` — pure metadata derivation, imported
  from `search` by `library`.
- `library::subscribe_then_append_to_playlist` (added 2026-04-28) — orchestrates
  download + library membership + playlist append.
- `library::subscribe_search_request` dispatch and `SearchSubscribeOutcome`
  result types.

Additional leakage:

- `metadata.rs` imports `gpui::{Image, SharedString}` and stores `Arc<Image>`
  inside otherwise pure result structs (`TagCompareResult::file_image`,
  `MusicBrainzLookupResult::image`). This forces every consumer to depend on
  GPUI even when they only need the metadata diffs.

Consequences of the current shape:

- Replacing the GPUI frontend would require rewriting the subscribe / download /
  tag pipeline and the playlist-append orchestration, because they live inside
  `LibraryApp` / `SearchApp` modules.
- The same pipeline is implemented twice (library subscribe vs search
  subscribe), drifting independently. Bug fixes have to be applied twice.
- `LibraryApp` and `SearchApp` cannot be unit-tested without constructing a
  GPUI runtime, so domain bugs are caught only through integration paths.
- New consumers (CLI, daemon, alternative TUI, future remote control) cannot
  reach this logic without depending on GPUI.

ADR 0015 leaves "subscription workflows" as a named service boundary but does
not define a concrete extraction. This ADR is that concrete plan.

## Decision

Extract the domain logic currently embedded in `library.rs` and `search.rs`
into UI-agnostic service modules, and remove the residual GPUI types from
`metadata.rs`. After this work, the GPUI files contain only:

- `*App` state structs and event handlers.
- `Render` implementations and view helpers.
- Async glue that schedules service calls on the background executor and maps
  results to `Status` text + `cx.notify()`.

No domain pipeline, no SQL composition, no HTTP calls, no file I/O, and no
ID3 tag computation may live inside a GPUI file.

### Target module layout

```
src/
  metadata_service.rs    (new) — id3_edits_for_track_context, pure derivations
  subscribe_service.rs   (new) — single download+tag+library-membership entry
  feed_service.rs        (new) — ensure_feed_in_db, feed subscription glue
  library_service.rs     (existing) — extended to host subscribe_then_append
  playlist_service.rs    (existing) — unchanged
  metadata.rs            (existing) — GPUI types removed; images become bytes
                                       at this boundary, decoded by UI layer
```

`metadata_service` is extracted first because the current library subscribe path
imports `search::id3_edits_for_track_context`. Moving subscription code before
that pure metadata derivation would either violate this ADR's service-boundary
rules or require a temporary service-to-UI dependency.

`subscribe_service` owns the canonical download pipeline. Both `library.rs`
and `search.rs` call it. `subscribe_track_from_search`,
`subscribe_feed_from_search`, and `subscribe_library_track` collapse into
explicit request types that capture the previous divergence: database identity
strategy, caller-provided ID3 edits vs auto-derived edits, feed subscription
rollback/reconcile behavior, optional MusicIndex hydration, and optional compare
result after download.

The service contract is:

```rust
pub enum SubscribeTrackRequest {
    LibraryTrack {
        track: db::TrackRow,
    },
    SearchTrack {
        track_context: metadata::TrackContext,
        edits: Vec<audio_tags::Id3v24Edit>,
        musicindex_endpoint: String,
        mark_feed_subscribed: bool,
        return_tag_compare: bool,
    },
}

pub struct SubscribeTrackOutcome {
    pub path: PathBuf,
    pub format_warning: Option<String>,
    pub applied_edits: usize,
    pub marked_downloaded: bool,
    pub compare: Option<metadata::TagCompareResult>,
}

pub struct SubscribeFeedRequest {
    pub feed: api::Feed,
    pub musicindex_endpoint: String,
}

pub struct SubscribeFeedOutcome {
    pub downloaded: usize,
    pub applied_edits: usize,
    pub skipped: usize,
}
```

The exact public type names may change during implementation, but the fields
above are mandatory behavioral facts that must not be hidden in UI-only status
strings.

### Boundary rules

- A service module may depend on `db`, `config`, `audio_tags`, `track_compare`,
  `api`, `musicbrainz`, `rss`, and other services.
- A service module must not import `gpui` or `gpui_component`.
- A service module must not import `library`, `search`, `app`, or any `ui_*`.
- A service module must take `&Connection` or `Arc<Mutex<Connection>>`
  parameters; it must not own a connection pool of its own.
- Service functions are blocking. The UI layer is responsible for moving them
  off the foreground executor.

### Image handling

`metadata.rs` carries cover art as raw bytes plus a UI-agnostic MIME hint:

```rust
pub struct ImageBytes {
    pub data: Vec<u8>,
    pub mime_type: String,
}
```

Do not use `gpui::ImageFormat` in `metadata.rs`; that type belongs at the UI
boundary. The UI layer converts `ImageBytes` into `gpui::Image` using a small
helper in `media` or direct `Image::from_bytes` calls at render-preparation
time. `SharedString` uses are replaced with `String` / `Cow<'static, str>`; the
UI converts at the render boundary.

## Consequences

Positive:

- The GPUI frontend becomes replaceable. A future TUI or web frontend can link
  the same service modules without forking domain logic.
- The subscribe pipeline exists once, killing the library/search duplication.
- Service functions become unit-testable without GPUI scaffolding, which
  raises coverage on the highest-risk code (download + tag write).
- `library.rs` and `search.rs` shrink enough to be navigable as view code.

Negative:

- Large mechanical refactor. Touching ~14k lines across two files.
- Async patterns in `library.rs` / `search.rs` must be preserved exactly:
  background executor, weak entity update, status string mapping. A
  regression here breaks UI feedback even when the underlying service works.
- Image-bytes refactor changes a public-ish surface in `metadata.rs`. Any
  callers that relied on `Arc<Image>` directly need updating.
- One ADR-mandated rewrite per workflow (ADR 0015 told us to do this gradually,
  not all at once). This ADR commits to doing it as one definitive sweep,
  which is heavier but eliminates the half-extracted middle state where
  duplicate pipelines coexist.

Neutral:

- No schema changes. No CLI surface changes. No on-disk format changes.
- Playback layer is already clean and is not touched by this ADR.

## Implementation Plan

### Phase 1 — `metadata_service`

1. Create `src/metadata_service.rs`.
2. Move `search::id3_edits_for_track_context` and any pure metadata derivations
   currently exported from `search` or `library`.
3. Update existing call sites (`library.rs`, `search.rs`, future
   `subscribe_service`).

Acceptance: `metadata_service` has no GPUI imports. `library.rs` no longer
imports anything from `search` for non-render purposes.

### Phase 2 — `subscribe_service`

1. Create `src/subscribe_service.rs`.
2. Move `subscribe_library_track` from `library.rs` into
   `subscribe_service::subscribe_track` using the `LibraryTrack` request path.
3. Move `search::prepare_track_for_subscription`,
   `search::compare_downloaded_track_path`, and the shared RSS enrichment needed
   by subscription into service-safe modules.
4. Fold `subscribe_track_from_search` into the `SearchTrack` request path. The
   `mark_feed_subscribed = false` path with feed-subscription reconciliation is
   preserved exactly. The compare-after-download path is controlled by
   `return_tag_compare`.
5. Fold `subscribe_feed_from_search` into `subscribe_service::subscribe_feed`.
   It hydrates tracks from MusicIndex when a track GUID is available, applies
   feed defaults and RSS enrichment, calls `subscribe_track` per item with
   feed-level subscription semantics, and preserves the zero-success rollback.
6. Add tests using a temp `Connection`, fixture mp3s, and one approved HTTP test
   seam. Prefer a local in-process HTTP server for real `reqwest` clients. If
   that becomes too heavy, add a narrow download/API trait in this ADR's service
   layer and implement it for `reqwest`.

Verify: download path, existing-file path, tag-write failure non-fatal,
search-track reconcile behavior, feed rollback on zero successes, and feed
summary counts.

Acceptance: `library.rs` and `search.rs` no longer contain `subscribe_*`
helpers. Both call `subscribe_service::*`. All existing tests pass.

### Phase 3 — `feed_service`

1. Create `src/feed_service.rs`.
2. Move `search::ensure_feed_in_db` to `feed_service::ensure_feed_in_db`,
   accepting `&mut Connection`, `&Config`, `feed_guid`, `feed_url`,
   `musicindex_endpoint`.
3. Update `search.rs` to call the service. Keep the GPUI-side wrapper that
   acquires the mutex and surfaces errors to `status`.

Acceptance: `ensure_feed_in_db` is reachable from CLI and tests. No GPUI
imports added to `feed_service`.

### Phase 4 — `library_service` extension

1. Move `subscribe_then_append_to_playlist` and `AppendToPlaylistOutcome` from
   `library.rs` into `library_service.rs`.
2. The function calls `subscribe_service::subscribe_track` and
   `playlist_service::append_track`.
3. Remove the now-unused intermediate `SubscribedTrackOutcome` type from
   `library.rs` if no UI code consumes it.

Acceptance: GPUI files reference only `library_service::subscribe_then_append_to_playlist`,
not the underlying subscribe pipeline.

### Phase 5 — `metadata.rs` GPUI removal

1. Replace `Option<Arc<gpui::Image>>` fields in metadata result structs with
   `Option<ImageBytes>` where `ImageBytes { data: Vec<u8>, mime_type: String }`.
2. Replace `SharedString` returns with `String`.
3. Update UI consumers: `library.rs`, `search.rs`, `ui_*.rs` decode bytes via
   a UI/helper conversion at render-preparation time and convert strings via
   `SharedString::from`.

Acceptance: `grep "gpui" src/metadata.rs` returns zero matches.

### Phase 6 — File splits (optional, deferred)

After phases 1–5, `library.rs` and `search.rs` are still large but contain
only view code. Splitting them into `*_state.rs` + `*_view.rs` is a separate
ADR if it happens at all. Not required by this ADR.

### Verification gates

Each phase must pass before the next begins:

- `cargo build` clean.
- `cargo test --lib` clean.
- `cargo clippy -- -D warnings` clean.
- `cargo fmt -- --check` clean.
- `grep "use gpui" src/<service-module>.rs` returns zero matches.
- `grep -E "use crate::(library|search|app|ui_)" src/<service-module>.rs`
  returns zero matches.

### Out of scope

- Replacing the GPUI frontend. This ADR makes replacement possible; choosing
  a replacement frontend is a separate decision.
- Splitting `library.rs` / `search.rs` into multiple files.
- Restructuring playback. ADRs 0014, 0020, 0021 already cover that surface
  and it is already UI-agnostic.
- Async runtime changes. Services remain blocking; UI keeps its background
  executor pattern.

## Green Criteria

This ADR is complete when:

- `subscribe_service`, `feed_service`, and `metadata_service` exist and own
  their respective domains.
- `library.rs` and `search.rs` import only service modules and view helpers
  for non-render concerns.
- The subscribe pipeline exists once.
- `metadata.rs` has no GPUI imports.
- All previously passing tests still pass, plus new tests for
  `subscribe_service` covering the three former code paths.
- ADR 0015 can be marked as fulfilled for the subscription workflow.
