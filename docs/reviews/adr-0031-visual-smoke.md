# ADR 0031 Visual Smoke Review

## Result

Pass with residual fixture gaps - 2026-05-02.

## Scope

- Launched `target/debug/v4vmm` against an isolated copy of the user's config,
  thumbnail cache, and SQLite database under `/tmp/v4vmm-adr31-smoke`.
- Used the current ADR 0031 implementation after Tasks 001-003.
- Reviewed user-attached screenshots from the smoke session:
  - Image #1: Library release detail, baseline action/fact/track viewport.
  - Image #2: Library release detail with the track playlist picker open.
  - Image #3: Library release detail after triggering MusicBrainz lookup.
  - Image #4: Discover release detail for the same HeyCitizen release.
  - Image #5: Discover one-track release detail.
  - Image #6: Discover recent-feeds grid.
- Captured local startup/navigation evidence before the user-driven smoke pass:
  - `/tmp/v4vmm-adr31-initial.png`
  - `/tmp/v4vmm-adr31-library-expanded.png`
  - `/tmp/v4vmm-adr31-library-heycitizen-click.png`

## Findings

| Area | Observation | Result |
|---|---|---|
| Library first viewport | Image #1 shows a clear title, creator, feed badge, restrained primary actions, compact facts, demoted RSS identity, and the start of the track section in the first viewport. | Pass |
| Library playlist behavior | Image #2 shows the row playlist picker opening from the shared release-track row path. The album-level playlist control also remains in the release action row. | Pass |
| Library MusicBrainz behavior | Image #3 shows `MusicBrainz: album lookup for 19 tracks...`, the album MusicBrainz button disabled, and per-track `MB: pending` state after triggering lookup. | Pass |
| Discover same-release detail | Image #4 shows the same release rendered through the shared skeleton with Website, Nostr, and RSS as identity actions, compact summary facts, demoted raw identity rows, and shared track rows. | Pass |
| Discover one-track fixture | Image #5 shows a one-track release with compact facts and the track section visible immediately. Download and playlist actions remain screen-owned trailing actions. | Pass |
| Description placement | Image #4 shows the description in one panel below summary/actions. It does not appear in the hero or facts. Library Image #1 has no duplicated description. | Pass |
| Raw identity placement | Image #4 keeps full Website, Nostr, Feed URL, and GUID values below summary/actions. Image #1 demotes RSS away from primary actions. | Pass |
| Track skeleton parity | Images #1-#5 show the shared dense row structure: number, artwork/thumb, title, duration, and trailing surface actions. | Pass |
| Cleanup | Removed the stale release-specific `EntityHeaderVm`, `ReleaseDetailVm::header`, `ReleaseDetailVm::detail_rows`, and `header_data_rows` path that could still project description/raw identity rows into a pre-contract header shape. | Pass |

## Residual Fixture Gaps

- The available smoke set did not include a zero-track release.
- The available smoke set did not include a 100+ track release.
- The Website/Nostr fixture in Image #4 includes a description panel, but the
  screenshots do not prove a multi-paragraph body.
- Library track metadata compare and playback were verified by code-path review
  from the shared row slot, not by a visible screenshot state change in this
  smoke set.

## Architecture Review

- `ReleaseDetailBehaviorSlots` remains narrow: screen modules can inject
  resolved images, actions, overlays, track rows, and after-section panels, but
  cannot replace hero, facts, description, or identity-panel placement.
- `render_release_detail_shell` consumes `ReleaseDetailPageVm`; it does not
  classify raw `FeedView` fields.
- Screen-owned command dispatch remains in `src/library.rs`, `src/search.rs`,
  `src/ui_feed.rs`, and `src/ui_track.rs`.
- No database, service, playback, playlist, download, subscription, or
  MusicBrainz service semantics changed.
- No schema migration or source-fact inference was introduced.

## Verification

Passed:

```bash
cargo fmt -- --check
cargo check
cargo test view_models::entity_detail
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
git diff --check
```

## Merge Recommendation

Mergeable. The remaining gaps are fixture availability gaps, not ADR 0031
architecture defects.
