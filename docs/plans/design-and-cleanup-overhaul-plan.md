# Design And Cleanup Overhaul Plan

## Status

Active - 2026-10-03. This plan is advisory. It states no rule.
Each binding change in it needs its own ADR or ADR amendment before code changes.

## Goal

- The app looks like the musicindex.org website: purposeful color and artwork.
- The Library is a browsable music library that feels like a music player. The RSS and ID3 values stay available for inspection.
- Settings has one clear place for each setting. The verbose error reports and logs stay.
- The documents, the guards and the code are smaller, current and correct.

## Recorded Facts - 2026-10-03

- The repository has 78 current ADRs and 5 archived ADRs, 316 task documents, 61 plans and 648 Markdown files in `docs/`.
- `AGENTS.md` has 612 lines. Most lines are a dated status history of packets. Its own rule says that it describes the present.
- `docs/pending-human-checks.md` has 646 lines.
- `src/` has 172179 lines of Rust in 317 files. The largest files are `src/view_models/library.rs` (7961), `src/view_models/show.rs` (5956), `src/db.rs` (5863) and `src/metadata.rs` (4784).
- `tests/architecture_tests.rs` has 19606 lines.
- The defects of 2026-10-03 had one shape: two sources of truth. Examples are live values against stored values, two copies of the channel link, two description forms, and an error that only stderr showed.
- The website is `../musicindex/search.html`. The operator discarded `index.html` on 2026-10-03. The dark theme is the default:
  - background `#0b0b0d`, sidebar `#141417`, surfaces `#1c1c1f` and `#26262a`, text `#f5f5f7`, muted text `#a1a1a6`
  - one accent `#2d7bff`, and `#0a5bd6` in the light theme
  - one color for each entity type, for example artist, track, feed, label, playlist and live
  - gloss, a grain texture, a shadow under artwork, and a blurred top bar of 52 pixels
  - radii of 6 to 12 pixels, pills of 22 pixels, and a grid of tiles of 150 pixels or more
  - the Figtree font, in `../musicindex/assets/fonts/`, with weights 300 to 800
- Each color, size and font of the app comes from `src/ui/tokens.rs` (durable set: token discipline). A token change can change the look of the full app.
- The cue system and the built-in player are not built. The live status shows one song that does not change.
- `cargo build --release` fails in `gpui-pre-macros 0.3.1`. A release binary does not exist.

## Phases

### Phase 1: Documents

1. ADR triage. Parallel read-only agents classify each ADR. The operator decides each row. One commit archives each batch. [Triage record](../reviews/adr-triage-2026-10-03.md).
   Decided on 2026-10-03: 25 ADRs archived. ADR 0078 archives with ADR 0082 packet 002.
2. Remove completed task documents and completed plans from the reading path. Version control keeps them.
   Done on 2026-10-03: 265 task documents moved to `docs/tasks/archive/`, and 40 plans deleted. 51 open tasks and 22 plans stay.
3. Guard audit. Delete each guard that cites a superseded ADR. A guard of an ADR that archived because each rule is enforced stays.
4. Retire each pending check whose requirement a later decision replaced.
5. Rewrite `AGENTS.md` to the present only: what the project is, where the work stands, the philosophy and the working rules.
   Done on 2026-10-03: `AGENTS.md` went from 612 to 276 lines.

### Phase 2: Design Language

1. A read-only screen inventory: each screen, its view model, its elements and its actions.
   Done on 2026-10-03: [screen inventory](../architecture/screen-inventory/README.md).
2. Static HTML mockups of three screens with the website tokens: the Library grid, an album page with an "Inspect" disclosure for the RSS and ID3 values, and Settings. The operator reviews them in a browser.
3. A design-language ADR records the accepted mockups as the target.
   Done on 2026-10-03: [ADR 0083](../adr/0083-design-language.md), Accepted.

### Design Direction - 2026-10-03

The operator chose this direction on 2026-10-03, from the [mockup canvas](https://claude.ai/artifact/CaY8T2NUdDosRV9uoFy9bk) (private to the operator). [ADR 0083](../adr/0083-design-language.md) records it.

- Version 2 of the mockups is the target: real covers, and each album page tinted by a blurred copy of its cover, as on `search.html`.
- Browsing follows Apple Music: a Music home, an album grid, album pages with one main Play action.
- Value for value is visible: the payment split as a bar, and the show readiness of an album.
- Inspection stays one step away: an "Inspect sources" view with RSS, MusicIndex and file tags side by side.
- From Raycast, the app takes a cohesive status bar and simple icons. The app is not a keyboard-driven power tool: each action works with the mouse, and a "⋯" menu holds the secondary actions.

### Phase 3: Tokens

Three packets implement ADR 0083 Decisions 1 to 4 and 10. The operator walks the visual check of each packet right after it.

1. [Task 001](../tasks/archive/adr-0083-task-001-palette-and-color-roles.md): the palette and the entity colors. Done on 2026-10-03.
2. [Task 002](../tasks/archive/adr-0083-task-002-font-and-artwork-tokens.md): Figtree, the display size, the artwork shadow and the monogram placeholder. Done on 2026-10-03.
3. [Task 003](../tasks/adr-0083-task-003-one-icon-set.md): each icon from the Lucide set. It corrects an emoji icon that Figtree caused.

### Phase 4: Library

Packets for each surface: the album grid with artwork first, the album page with its track list and a play action, and the "Inspect" disclosure. Each packet has its visual check, walked right after it.

### Phase 5: Settings

ADR 0069 and its task 002 continue. The grouped Settings keep each report and log reachable.

### Phase 6: Player And Cue

A separate ADR, after the Library work. The playback defect of one song that does not change is a separate early packet.

### Throughout: Code Correctness

- Single source of truth audit: read-only agents find data written in two places and two paths that compute one output. They also find errors that only stderr shows, and live values where the stored value is the rule. Each finding becomes a failing test and then a fix.
- Split the largest files at their natural seams. No behavior changes in those packets.
- Add `cargo clippy --all-targets -- -D warnings` to the gate. Correct the release build.
- No complete rewrite. Change one surface at a time and keep the gate Green.

## Working Rules For This Plan

- Each implementation session owns one packet.
- The operator walks each visual check right after its packet. A backlog of open checks hides defects.
- The orchestrator does not ask the operator again about a decision that the operator made.
