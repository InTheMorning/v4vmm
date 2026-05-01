# ADR 0027 Task 005 Final Visual Smoke Review

## Result

Pass - 2026-05-01.

## Scope

- Copied the user's v4vmm config, thumbnail cache, and SQLite database into
  `/tmp/v4vmm-adr27-1777665559` so the smoke pass did not write to the real
  library.
- Launched `target/debug/v4vmm` with the copied config/data.
- Compared `The Heycitizen Experience` in Library and Discover at the same
  1676 x 1008 viewport.
- Captured screenshots locally:
  - `/tmp/v4vmm-adr27-library-release-final2.png`
  - `/tmp/v4vmm-adr27-discover-release-final2.png`

## Findings

| Area | Observation | Result |
|---|---|---|
| Release shell | Artwork, badge, title, subtitle, action row, detail grid, description/card treatment, and track summary now follow the same shared shell rhythm. | Pass |
| Feed actions | Both surfaces use `Remove Feed` and `Add feed to playlist`. Library also exposes album MusicBrainz because it has the local album lookup handler. | Pass with accepted Library-specific action |
| Track membership actions | Both surfaces now render text `Remove` row actions through quiet destructive control treatment. | Pass |
| Playlist row actions | Both surfaces use `+ Playlist` row actions with the same compact control family. | Pass |
| Redundant state | Library no longer renders `Downloaded 19`; membership is implied by `Remove Feed` and row `Remove` actions. | Pass |
| Contrast | Destructive row actions use `DangerLabel`, which is covered by existing contrast tests. They read as destructive without returning to large filled red buttons. | Pass |
| Remaining data difference | Discover shows release-date/language/explicit/description facts from MusicIndex. Library local album detail still has a smaller local fact set. | Deferred to identity/source-fact persistence or ADR 0024 query work |
| Screen-owned selection | Library sidebar still shows tree selection state while Discover shows search result selection state. | Accepted screen-owned navigation behavior |

## Architectural Drift

None found. The final fixes stayed within pure view-model projection and ADR
0025 control styles. GPUI handlers, services, database access, and command
dispatch remain screen/application owned.

## Verification

Passed:

```bash
cargo fmt -- --check
cargo check
cargo test album_detail_vm_omits_downloaded_count_when_membership_actions_cover_state
cargo test track_row_action_vm_labels_match_download_state
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
cargo test
```

## Merge Recommendation

Mergeable.
