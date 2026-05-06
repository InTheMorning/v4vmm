# Post-ADR 0026 Task 001 Visual Smoke Review

## Result

Pass with routed follow-up - 2026-05-01.

## Scope

- Copied the user's v4vmm config, thumbnail cache, and SQLite database into
  `/tmp/v4vmm-adr26-*` so the smoke pass did not write to the real library.
- Launched `target/debug/v4vmm` with the copied config/data.
- Compared `The Heycitizen Experience` in Library and Discover at the same
  1436 x 858 viewport.
- Captured screenshots locally:
  - `/tmp/v4vmm-post-adr-0026-library-release.png`
  - `/tmp/v4vmm-post-adr-0026-discover-release.png`

## Findings

| Area | Observation | Classification | Routed To |
|---|---|---|---|
| Header shell | Both surfaces now use the same release-detail shell, artwork size, badge position, title scale, and left-aligned content rhythm. | No follow-up | None |
| Sidebar | Both screenshots keep a left pane visible and the detail surface starts from the same split boundary. Library shows the tree and selected tracks; Discover shows search results and a selected feed result. | Screen-owned behavior | ADR 0024 query/service thinning only if navigation state starts blocking parity |
| Primary actions | Discover shows `Remove Feed` as a quiet outline action. Library shows `Unsubscribe Feed` as a prominent destructive filled action and also exposes MusicBrainz. | Styling/action-state mismatch | ADR 0027 shared entity action state, plus a bounded ADR 0025 control-role sweep |
| Track row actions | Discover uses quiet per-row icon removal, `+ Playlist`, and a trailing checkbox. Library uses large repeated red `Remove` buttons plus `+ Playlist`. | Action-state and styling mismatch | ADR 0027 shared row action descriptors; ADR 0025 for destructive row control treatment |
| Redundant state labels | Library still shows `Downloaded 19`; Discover does not. The label is redundant when Library membership is already expressed by remove actions. | Projection/detail-row state mismatch | ADR 0027 shared action/detail state |
| Metadata density | Discover shows release kind, publisher, release date, language, explicit state, track count, and description. Library shows artist, track count, duration, and downloaded count. | Local data preservation/query gap | Identity/source-fact audit and ADR 0024 query/service thinning |
| Description | Discover renders the feed description; Library does not have it for this local album detail. | Local data preservation/query gap | Identity/source-fact audit |
| Contrast | Current dark profile is readable in both screenshots. The red Library remove buttons are visually dominant relative to repeated row density. | Styling mismatch | ADR 0025 bounded control-role follow-up |
| Contributor identity fixture | The selected release did not expose contributor image, website, or Nostr rows in this smoke path. Search results and detail did show feed identity icons. | Fixture gap | Future contributor-specific visual smoke after a known fixture is available |

## Triage

- ADR 0027 was drafted for shared entity action state. The visual evidence
  shows that layout parity is not enough; Library and Discover still bind
  equivalent membership/removal/playlist states through different row action
  vocabularies.
- Create a bounded ADR 0025 task for destructive row controls after ADR 0027
  names the shared action descriptors. The repeated Library `Remove` buttons
  should become quiet enough for dense rows.
- Keep ADR 0024 query/service thinning as a follow-up, not a blocker for this
  visual pass. The current screenshots prove a data/detail gap, but not yet a
  mandatory application-query boundary change.
- Do not reopen ADR 0026. The shared shell is in place; the remaining gaps are
  state, data preservation, and control treatment.

## Verification

- Manual visual smoke only.
- No runtime code changed.
- Screenshots were captured locally for review and were not committed.
