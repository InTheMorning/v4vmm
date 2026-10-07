# ADR 0077 Task 004: Publisher Page And Navigation

Status: Open - implementation and mechanical checks are complete on 2026-09-26. On 2026-10-07 the operator moved its visual check to the [overhaul plan](../plans/design-and-cleanup-overhaul-plan.md#visual-requirements-moved-from-pending-checks---2026-10-07). The packet that rebuilds the surface carries it.

## Goal

Open the publisher page from an album and from a track, and show the page in the Music section.
The screen composes the packet 003 view model.

Packet 005 owns the feed owner text, the removal of the `publisher_text` inspector, the name search, and the "Grouped by name" label.
The orchestrator divided the earlier packet 004 into packets 004 and 005 on 2026-09-26.

## Authority

- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) Decisions 1 and 2, and its accepted refinements.
- [ADR 0078](../adr/0078-publisher-page-type-from-stated-role.md), the page type.
- [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md): one projection in `src/application/queries/stored_values.rs` owns the stored values.
- [ADR 0060](../adr/0060-workflow-surface-structure.md): the Music section holds Index results and the Library.
- The durable set in [AGENTS.md](../../AGENTS.md): element hierarchy, token discipline, typed action state, renderer portability and the UI change acceptance gate.

## Packet 003 Result That This Packet Uses

- `ArtistRef::PublisherFeed(String)` in `src/views.rs`. The value is the publisher feed GUID.
- `fetch_index_publisher_page` in `src/application/queries/feed.rs` and `fetch_library_publisher_page` in `src/application/queries/library.rs`.
- `PublisherPageVm` in `src/view_models/publisher_page.rs`: `title`, `page_type`, `owned_albums`, `listed_by_albums`, `library_albums`, `other_albums`, `other_albums_status`, and `derived_artist_count`.
- Each packet 003 item carries `#[cfg_attr(not(test), expect(dead_code, reason = ...))]`. When this packet calls an item, the expectation becomes unfulfilled and the build fails. Remove each expectation that the build names.
- The [packet 003 review](archive/adr-0077-task-003-publisher-page-view-model.md#orchestrator-review---2026-09-26) records a follow-up: a Library album has no artist text.

## Required Changes

### 1. Navigation

- Add a navigation entry for a publisher page, keyed on the publisher feed GUID, in `src/view_models/workspace/nav.rs`.
- Add its breadcrumb in `src/view_models/workspace/breadcrumb.rs`. The breadcrumb text is the view model title.
- The existing `ArtistDetail(String)` entry keys on name text. Do not reuse it for a publisher page.

### 2. Entry Points

- An album with a stored or received relationship where `music_names_publisher = true` exposes an "open publisher" action with the publisher feed GUID.
- A track exposes the "open publisher" action of its album feed. No value is stored on the track.
- An album without such a relationship exposes no publisher action. Packet 005 owns its "Grouped by name" label.
- The action carries typed availability and an accessibility label, from the view model.

### 3. Library Album Values

In `fetch_library_publisher_page`, each Library album takes its title, image and artist from `stored_values::feed_values` for that local feed.
The artist is `album_artist` with its stored owner as the source text. Add the local `feed_id` to `LocalPublisherAlbum` for this read.
A Library album with no stored artist shows no artist. The view model invents no placeholder.

### 4. Page Screen

- Add the publisher page screen under `src/ui/shells/`. It composes `PublisherPageVm`.
- The screen decides no page type, no role label, no group and no action availability.
- On a Library page, the screen shows `library_albums` and `other_albums` as two groups. When `other_albums_status` is `Unavailable`, the screen shows its report first and its detail after it.
- On an Index page, the screen shows `owned_albums` and `listed_by_albums`, and marks each album with `in_library`.
- New repeated elements use a shared primitive or composite. Size, spacing, color, type and icons come from named tokens.
- Read [column text truncation](../troubleshooting/column-text-truncation.md) before you style stacked text.
- The page loads through the ADR 0040 runtime. A screen does not call `cx.spawn`.
- The mounted page updates in place when its data changes.

## Mechanical Acceptance Criteria

Use the prefix `adr_0077_publisher_navigation_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R4-01 | An album view model with an owned relationship exposes an enabled "open publisher" action with the GUID and an accessibility label |
| R4-02 | An album view model without a relationship exposes no publisher action |
| R4-03 | A track view model exposes the publisher action of its album feed. No track row stores a publisher value |
| R4-04 | The publisher navigation entry keys on the GUID. Its breadcrumb uses the view model title |
| R4-05 | A Library album gets its title, image and artist from `stored_values::feed_values`. A Library album without a stored artist exposes no artist |
| R4-06 | The screen module reads the page type, role labels, groups and action availability from the view model only. A guard names the durable renderer portability rule |
| R4-07 | No packet 003 `dead_code` expectation remains on an item that this packet calls, and `cargo check --all-targets` gives no warning |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: an album page opens its publisher page. The page shows its title, its type and its albums, in Light and Dark themes.
- V2: a track page opens the publisher page of its album.
- V3: a Library publisher page shows the Library albums and the other albums as two groups. With MusicIndex unreachable, the Library group stays and the report shows.
- V4: an assumed role looks different from a stated role. A "listed by" album and a "Not listed" album look different from an owned album. A derived artist count shows as derived.
- V5: normal and narrow window widths show each element in its defined place, with no clipped text.

## Exclusions

- No feed owner text, no inspector removal, no name search and no "Grouped by name" label. Packet 005 owns them.
- No new request and no stored data.
- No change to the page type rule or to the view model rules of packet 003.
- No playback, Show or Settings change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/view_models/publisher_page.rs`, `src/application/queries/feed.rs`, `src/application/queries/library.rs` and `src/db/publisher_relationships.rs`.
- `src/application/queries/stored_values.rs`: `feed_values` and `FeedStoredValues`.
- `src/view_models/workspace/nav.rs` and `src/view_models/workspace/breadcrumb.rs`.
- `src/app.rs` and `src/library/app_impl.rs`: the navigation handlers for artist and album pages.
- `src/view_models/artist_detail.rs` and `src/ui/shells/artist.rs`: the existing artist page pattern.
- `src/runtime/`: the ADR 0040 actors. `src/runtime/playback_polling.rs` is the reference.
- `tests/architecture_tests.rs`: the renderer portability guards.

## Checks

```bash
cargo test --lib adr_0077_publisher_navigation
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

Report each result. Say "Green" for a passing check. Run `cargo build --bin v4vmm` last.

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Implementation Result - 2026-09-26

### Files Changed

Navigation and breadcrumb:

- `src/view_models/workspace/nav.rs`: new `FrameNavigationEntry::PublisherDetail(String)`, keyed on the publisher feed GUID.
- `src/view_models/workspace/breadcrumb.rs`: a breadcrumb ID for the new entry.
- `src/app/breadcrumb.rs`: the breadcrumb text for a publisher page is the view model title. It also calls `restore_publisher_page_for_nav`, so a breadcrumb move to a different publisher feed GUID fetches that page again.
- `src/app.rs`: a window title for the new entry, the content-router arm, the `publisher_page` field on `TopApp`, and the new `publisher_page_routes` map. That map keeps the Library-or-Index context each publisher feed GUID used last. The back-navigation handler also calls `restore_publisher_page_for_nav`.
- `src/app/publisher_dispatch.rs` (new file, rewritten after the orchestrator's review): `PublisherPageState` carries its publisher feed GUID in each step. `TopApp::open_publisher_page` pushes the navigation entry and starts the fetch. `TopApp::restore_publisher_page_for_nav` reads that GUID and the recorded context, and starts a new fetch only when the mounted page does not match. The fetch dispatch drops a result if its GUID no longer matches the mounted page. The render dispatch selects a `PublisherPageLoadDisplay`, and the screen composes it.
- `src/library.rs`: new `LibraryAppEvent::OpenPublisherPage`.
- `src/library/app_impl.rs`: `LibraryApp::open_publisher_page` (emits the event), `mounted_track_publisher_feed_guid`, and the updated `AlbumNode` construction sites.

Entry points (view models):

- `src/view_models/entity_detail.rs`: new `EntityActionKind::OpenPublisher` and `ReleaseDetailVm::publisher_action`.
- `src/view_models/track_detail.rs`: `TrackDetailVm` gains `with_publisher_feed_guid` and `publisher_action`.
- `src/views.rs`: `FeedView` gains `publisher_feed_guid`, read from the received relationship in `FeedView::from_api`.
- `src/sources.rs`: `LocalSource::local_feed_view` reads the stored relationship for `FeedView::from_local_with_facts`.

Entry points (screens):

- `src/ui/shells/library/feed_detail.rs`: the "Open publisher" button on a Library album.
- `src/ui/shells/library/track_detail.rs` and `src/ui/shells/library/detail.rs`: the "Open publisher" button on a Library track, and the plumbing that carries the album feed's publisher feed GUID down from the already-loaded Library tree.
- `src/app/search_dispatch.rs`: the "Open publisher" button on an Index album.

Library album values:

- `src/db/publisher_relationships.rs`: `LocalPublisherAlbum` gains `feed_id`. A new reader, `owned_publisher_feed_guid`, gives the stored owned relationship of one feed.
- `src/application/queries/library.rs`: `fetch_library_publisher_page` reads each Library album's title, image and artist from `stored_values::feed_values`. A new command, `FetchLibraryPublisherPage`, wraps that function. `build_tree` reads each album's stored owned relationship.
- `src/view_models/library.rs`: `AlbumNode` gains `publisher_feed_guid`.

Page screen and dead-code cleanup:

- `src/ui/shells/publisher.rs` (new file, extended after the orchestrator's review): the publisher page screen. It composes `PublisherPageVm` only. It reads its header data rows and title from `header_facts`/`title_text`, and each album artist line from `AlbumArtistDisplay::display_text`. It renders its loading, failed and empty states through the new `render_publisher_page_status`. It shows each album's resolved artwork from a caller-supplied thumbnail map.
- `src/view_models/publisher_page.rs`: `PublisherPageContext` and small, additive display methods (`PublisherPageType::label`, `AlbumRoleDisplay::text`/`is_stated`/`conflict_text`, `PublisherPageAlbumVm::NOT_LISTED_LABEL`/`IN_LIBRARY_LABEL`). The page type rule and the role rules from packet 003 are unchanged. Added after the orchestrator's review: `PublisherPageHeaderFact` and `PublisherPageVm::header_facts`/`title_text`, `AlbumArtistDisplay::display_text`, `DerivedArtistCount::display_text`, `PublisherPageLoadDisplay` (the loading, failed and empty text), and `PublisherPageVm::album_image_urls`.
- `src/application/queries/feed.rs`: new `FetchIndexPublisherPage` command.
- `src/ui/shells/mod.rs`: registers the new `publisher` screen module.
- Removed every packet 003 `#[expect(dead_code, ...)]` that named packet 004, in `src/view_models/publisher_page.rs`, `src/db/publisher_relationships.rs`, `src/application/queries/feed.rs`, `src/application/queries/library.rs`, and `src/application/request_profiles.rs`.

Tests: `src/db/publisher_relationships.rs`, `src/view_models/entity_detail.rs`, `src/view_models/track_detail.rs`, `src/view_models/workspace/tests.rs`, `src/app/publisher_dispatch.rs` (stale-result and navigation-restore tests, added after the orchestrator's review), `src/view_models/publisher_page.rs` (load-display and artwork-URL tests, also added after that review), `src/application/queries/library.rs`, and `tests/architecture_tests.rs` (the renderer-portability guard, extended after that review to also catch a glyph escape and a quoted `Label::new` text).

### Tests

All checks are Green, after the fixes below:

```text
cargo test --lib adr_0077_publisher_navigation   Green (22 passed)
cargo test                                       Green (1854 lib, 282 architecture, 10 doc tests ignored)
cargo test --test architecture_tests             Green (282 passed)
cargo fmt -- --check                             Green
cargo clippy -- -D warnings                      Green
cargo check --all-targets                        Green, no warning
cargo build --bin v4vmm                          Green
```

### Behavior Changed

- An album with a stored or received owned publisher relationship shows an "Open publisher" button, in the Library album page and the Index album page. A track shows the same button, reading its album feed's relationship, in the Library track page.
- Opening a publisher page pushes a `PublisherDetail` navigation entry, fetches the page through the ADR 0040 runtime (`FetchLibraryPublisherPage` or `FetchIndexPublisherPage`), and renders a new page: title, page type, the derived artist count when present, and two album groups.
- A Library publisher page's albums show their title, image and artist from `stored_values::feed_values`. A Library album can now show an artist, where it showed none before.
- No stored data and no new request beyond the fetches above.
- Added after the orchestrator's review: a publisher page shows each album's own artwork, resolved through the same thumbnail path other album rows use.
- Added after that review: an older fetch result for a publisher feed GUID the app already left no longer replaces the mounted page. A breadcrumb or a back move to a `PublisherDetail` entry for a different GUID fetches that page again, with the context (Library or Index) it used before.

### Deviations From Task

- The Index track page shows no "Open publisher" button. Its fetched `Track` response carries no publisher relationship field, and reading one would need a new request to the track's feed. The Exclusions section forbids a new request, so this entry point stays deferred. `TrackDetailVm::publisher_action` is ready for a caller that supplies the value once a packet adds that read.
- An album row on the publisher page is not a link to that album's own page. The required changes ask only for the page's groups and marks. Row navigation stays a follow-up.
- `PublisherPageContext` (Library or Index, plus small label helpers) was added to `src/view_models/publisher_page.rs` rather than the screen module, to keep every label in the view model. This is additive: it does not change the page type rule or the role rules from packet 003.

### Orchestrator Review Fixes - 2026-09-26

The orchestrator reviewed this packet and returned it with five required fixes. Each fix
changes code already in this packet. No fix adds new scope.

1. **Status text.** `PublisherPageLoadDisplay` in `src/view_models/publisher_page.rs` now
   holds the loading, failed and empty text, and its fixed report for a failed load.
   `render_publisher_page_content` picks a display value. `render_publisher_page_status` in
   `src/ui/shells/publisher.rs` lays it out. A failed load shows its report first, and the
   technical detail after it, apart from the report.
2. **Screen labels.** `render_header` reads `header_facts`/`title_text`. An album row reads
   `AlbumArtistDisplay::display_text`. The screen writes no label and no glyph of its own. The
   R4-06 guard in `tests/architecture_tests.rs` now also stops a `\u{` glyph escape and a
   string literal passed to `Label::new`.
3. **Stale result.** `PublisherPageState` carries its publisher feed GUID in each step.
   `publisher_page_result_is_current` checks that GUID before a fetch result applies. The app
   drops a result for a GUID it already left.
4. **Restore with a different GUID.** `TopApp.publisher_page_routes` keeps the context (Library
   or Index) each publisher feed GUID used last. A breadcrumb or a back move can restore a
   `PublisherDetail` entry. `restore_publisher_page_for_nav` reads the recorded context, and
   starts a new fetch only when that entry's GUID differs from the one already mounted.
5. **Album artwork.** `PublisherPageVm::album_image_urls` gives every album artwork URL on the
   page. `render_publisher_page_content` resolves each one through
   `TopApp::index_remote_detail_hero_image`, the same thumbnail path other album rows use, and
   hands the resolved map to the screen. This needed no new request and no new runtime actor.

### Unresolved Concerns

- The visual gate (V1-V5) is open and paused, per the operator's standing instruction. The steps below are ready for the next visual batch.

## Operator Visual Check

Each step names its commands, the Library state it needs, what counts as wrong, and its cleanup.
A Library album whose album hydration stored a `music_to_publisher` row is necessary for V1 to V3.

### Setup: a Library album with a stored publisher relationship

1. Run `cargo build --bin v4vmm` in a desktop session.
2. Start the app and open the Music section.
3. Subscribe an album from a publisher-linked feed, for example a Wavlake artist release, through the Index route: search, open the album, and download it.
4. Open that album once in the Library. Opening it hydrates its identity facts, which stores the `music_to_publisher` row this check needs.
5. Cleanup for this setup: remove the album from the Library after the checks below (Library album row, remove action). Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. It is a separate evidence fixture.

### V1: an album page opens its publisher page

1. In the Library, open the album from the setup above.
2. Check that its detail page shows an "Open publisher" button.
3. Click the button.
4. Check that the content pane shows a new page. The page must show a title, a page type ("Artist" or "Label"), and one album group at least.
5. Switch the app to Dark from Settings, then back to Light. Check that the page reads correctly in both themes.
6. Wrong: no "Open publisher" button appears. Or, the button does nothing. Or, the page shows no title or no page type.
7. Cleanup: none beyond the setup cleanup.

### V2: a track page opens the publisher page of its album

1. In the Library, open a track from the album in the setup above.
2. Check that its detail page shows an "Open publisher" button.
3. Click the button.
4. Check that the app opens the same publisher page as V1, with the same title.
5. Wrong: no button appears on the track page. Or, the button opens a different publisher page than the album's own.
6. Cleanup: none beyond the setup cleanup.

### V3: a Library publisher page shows two groups, and keeps its Library group when MusicIndex is unreachable

1. From V1, on the open publisher page, check for two groups. One group is for the Library album. One group is for other albums from the same publisher.
2. Stop network access to the configured MusicIndex endpoint. For example, point Settings to an unreachable host, or disconnect the network.
3. Reopen the publisher page from the album (V1, steps 1 to 3).
4. Check that the Library album group still shows the Library album.
5. Check that a report line appears above the groups. The report must name that the other albums did not load. Its technical detail must follow the report, not come before it.
6. Restore network access, or the correct MusicIndex endpoint, in Settings.
7. Wrong: the Library group disappears when the network is unreachable. Or, no report appears. Or, the report shows only a technical error, with no plain sentence first.
8. Cleanup: confirm the MusicIndex endpoint in Settings is restored to its working value.

### V4: role and listing marks look different from each other

1. Open a publisher page with more than one album, or a publisher feed known to state a role. See `docs/notes/2026-09-23-publisher-feed-artist-research.md` for candidate feeds.
2. Check that a stated role and a role that the app assumed show different text. Color alone is not enough.
3. Check that an album marked "Not listed by the publisher" looks visually different from an album with no such mark.
4. If the page shows a derived artist count, check that its text names the count as derived, not as a stated fact.
5. Wrong: an assumed role and a stated role show the same text. Or, the "Not listed" mark is missing, or looks the same as an unmarked album. Or, the derived count reads as a plain fact.
6. Cleanup: none.

### V5: normal and narrow window widths show no clipped text

1. With the publisher page open, resize the window to its normal width, then to a narrow width.
2. Check that the title, the album titles, artist names and role text stay in their place.
3. Check that no text is cut off mid-word, and no element overlaps another.
4. Wrong: text overlaps another element. Or, a word is cut in the middle, with no visual indication.
5. Cleanup: restore the window to its previous size.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0077-task-004-publisher-navigation-and-presentation.md`
- `docs/adr/0077-publisher-feed-artist-binding.md` and `docs/adr/0078-publisher-page-type-from-stated-role.md`
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the navigation entry, the entry points, the Library album values, and the page screen.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model decides each label, group, page type and action availability. The screen only composes them.
- Use named tokens and shared primitives or composites. Put no raw literal and no glyph string in a renderer.
- Never build `ArtistRef::PublisherFeed` from name text or `publisher_text`.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- `publisher_text` display, the `/v1/publishers/{publisher_text}` inspector, the name search and the name grouping. Packet 005 owns them.
- The page type rule and the role rules in `src/view_models/publisher_page.rs`.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R4-01 to R4-07 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V5.

Test commands:
- `cargo test --lib adr_0077_publisher_navigation`
- `cargo test`
- `cargo test --test architecture_tests`
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- `cargo check --all-targets`
- `cargo build --bin v4vmm`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The navigation model cannot hold a GUID key without a change to its design.
- A screen needs a value that the view model does not expose.
- A change needs a file in "Do not touch".
- The page cannot load through the ADR 0040 runtime without a new actor design.
