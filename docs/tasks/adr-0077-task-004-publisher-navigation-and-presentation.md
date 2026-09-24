# ADR 0077 Task 004: Publisher Navigation And Presentation

Status: Held - 2026-09-24. This packet starts after packet 003 is complete.
Implementation has not started. Its visual gate opens when the implementation is complete.

## Goal

Open the publisher page from an album and from a track. Show the page in the Music section.
Show `publisher_text` as the feed owner. Change the Index artist rows that come from name text into a search.
Label the Library name grouping.

## Authority

- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) Decisions 1, 2 and 6, and its accepted refinements.
- [ADR 0078](../adr/0078-publisher-page-type-from-stated-role.md), the page type.
- [ADR 0060](../adr/0060-workflow-surface-structure.md): the Music section holds Index results and the Library.
- The durable set in [AGENTS.md](../../AGENTS.md): element hierarchy, token discipline, typed action state and the UI change acceptance gate.

## Required Changes

### Navigation

Add a navigation entry for a publisher page, keyed on the publisher feed GUID, in `src/view_models/workspace/nav.rs`.
Add its breadcrumb in `src/view_models/workspace/breadcrumb.rs`.

- An album with a stored or received relationship where `music_names_publisher = true` opens the publisher page.
- A track opens the publisher page of its album feed. No value is stored on the track.
- An album without such a relationship keeps the Library name grouping. Its title shows "Grouped by name".

### Feed Owner Text

`publisher_text` shows as "Feed owner" text on the album page. It opens no page.
List each entry point that opens the `/v1/publishers/{publisher_text}` inspector. Remove each one.
Delete the inspector code that no entry point reaches after that removal.

### Name Search

The Index search builds artist rows from feed and track name text (`artist_rows_from_result_rows` in `src/view_models/search/results.rs`).
Each such row opens a search result with the title "Tracks matching" and the quoted name. It has no artist identity, no role and no page type.
Each result row links to its album, and through the album to its publisher page.

### Page Presentation

The screen composes the packet 003 view model. It decides no page type, no role label and no action availability.
New repeated elements use a shared primitive or composite. Size, spacing, color, type and icons come from named tokens.

## Mechanical Acceptance Criteria

Use the prefix `adr_0077_publisher_navigation_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R4-01 | An album view model with an owned relationship exposes an enabled "open publisher" action with the GUID and an accessibility label |
| R4-02 | An album view model without a relationship exposes no publisher action, and its artist grouping exposes the "Grouped by name" label |
| R4-03 | A track view model exposes the publisher action of its album feed. No track row stores a publisher value |
| R4-04 | `publisher_text` is exposed as feed owner text with no action |
| R4-05 | No entry point opens the `/v1/publishers/{publisher_text}` inspector. A guard names ADR 0077 Decision 6 |
| R4-06 | A name-built Index row exposes a search result title with the name, and no artist identity |
| R4-07 | The publisher page breadcrumb uses the view model title, and a GUID-based key |
| R4-08 | The screen module reads the page type and role labels from the view model only. A guard names the durable renderer portability rule |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: an album page opens its publisher page. The page shows its title, its type and its albums, in Light and Dark themes.
- V2: a label page, if one exists in the data, shows its albums. A page with a derived artist count shows the count as derived.
- V3: an assumed role looks different from a stated role. A "listed by" album and a "Not listed by the publisher" album look different from an owned album.
- V4: `publisher_text` shows as feed owner text and opens nothing.
- V5: a name search result shows "Tracks matching" and the name, and opens no artist page.
- V6: the Library name grouping shows "Grouped by name".
- V7: normal and narrow window widths show each element in its defined place, with no clipped text.

## Exclusions

- No new request and no new stored data. Packets 002 and 003 own them.
- No change to the page type rule or to the view model values. Packet 003 owns them.
- No playback, Show or Settings change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/view_models/workspace/nav.rs` and `src/view_models/workspace/breadcrumb.rs`.
- `src/library/app_impl.rs`: the artist navigation handler near line 1611.
- `src/app.rs`: the navigation entries near lines 783 and 1168.
- `src/view_models/search/results.rs` and `src/view_models/search/feed_detail.rs`.
- `src/discover/app_impl.rs`: the inspector detail handling.
- `src/ui/shells/artist.rs` and `src/view_models/artist_detail.rs`.
- [Column text truncation](../troubleshooting/column-text-truncation.md) before styling stacked text.

## Checks

```bash
cargo test --lib adr_0077_publisher_navigation
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build --bin v4vmm
```

Report each result. Say "Green" for a passing check. Run `cargo build --bin v4vmm` last, before the operator check.

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Operator Visual Check

The implementer writes this section at completion, with numbered steps for V1 to V7.
It names the fixture or the real Library state that each step needs, and the cleanup.
The gate stays open in this `Status:` line and in [pending human checks](../pending-human-checks.md) until the operator walks it.
