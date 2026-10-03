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
- The website at `../musicindex/index.html` uses these tokens:
  - accent `#ff5a00`, warm accent `#ff7a2f`, purple `#7b5cff`, lime `#c7ff00`, blue `#2d7bff`
  - ink `#0d0d0d`, charcoal `#1b1c20`, deep gray `#3f4148`, middle gray `#73737d`, warm gray `#d9d9df`, off-white `#f7f7f8`
  - radii 12, 20, 32 and 48 pixels
  - the Figtree font, in `../musicindex/assets/fonts/`
- Each color, size and font of the app comes from `src/ui/tokens.rs` (durable set: token discipline). A token change can change the look of the full app.
- The cue system and the built-in player are not built. The live status shows one song that does not change.
- `cargo build --release` fails in `gpui-pre-macros 0.3.1`. A release binary does not exist.

## Phases

### Phase 1: Documents

1. ADR triage. Parallel read-only agents classify each ADR. The operator decides each row. One commit archives each batch. [Triage record](../reviews/adr-triage-2026-10-03.md).
   Decided on 2026-10-03: 21 ADRs archived. Four archive after a citation packet, and ADR 0078 archives with ADR 0082 packet 002.
2. Remove completed task documents and completed plans from the reading path. Version control keeps them.
3. Guard audit. Delete each guard that cites a superseded ADR. A guard of an ADR that archived because each rule is enforced stays.
4. Retire each pending check whose requirement a later decision replaced.
5. Rewrite `AGENTS.md` to the present only: what the project is, where the work stands, the philosophy and the working rules.

### Phase 2: Design Language

1. A read-only screen inventory: each screen, its view model, its elements and its actions.
2. Static HTML mockups of three screens with the website tokens: the Library grid, an album page with an "Inspect" disclosure for the RSS and ID3 values, and Settings. The operator reviews them in a browser.
3. A design-language ADR records the accepted mockups as the target.

### Phase 3: Tokens

One packet maps the website palette, radii and font onto `src/ui/tokens.rs` and the theme profiles. The operator walks the visual check right after the packet.

### Phase 4: Library

Packets for each surface: the album grid with artwork first, the album page with its track list and a play action, and the "Inspect" disclosure. Each packet has its visual check, walked right after it.

### Phase 5: Settings

ADR 0069 and its task 002 continue. The grouped Settings keep each report and log reachable.

### Phase 6: Player And Cue

A separate ADR, after the Library work. The playback defect of one song that does not change is a separate early packet.

### Throughout: Code Correctness

- Single source of truth audit: read-only agents find data written in two places, two paths that compute one output, errors that only stderr shows, and live values where the stored value is the rule. Each finding becomes a failing test and then a fix.
- Split the largest files at their natural seams. No behavior changes in those packets.
- Add `cargo clippy --all-targets -- -D warnings` to the gate. Correct the release build.
- No complete rewrite. Change one surface at a time and keep the gate Green.

## Working Rules For This Plan

- Each implementation session owns one packet.
- The operator walks each visual check right after its packet. A backlog of open checks hides defects.
- The orchestrator does not ask the operator again about a decision that the operator made.
