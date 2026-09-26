# ADR 0077 Task 003: Publisher Page View Model

Status: Held - 2026-09-24. This packet waits for the Stophammer answer to the
album summary fields, request 1 of the [open Stophammer requests](../plans/v4vmm-open-requests.md).
Implementation has not started. This packet changes no screen. Packet 004 owns the visual gate.

## Goal

Add the publisher feed as an artist reference. Build one query for the Index publisher page and one for the Library publisher page.
Build one view model that exposes each fact that ADRs 0077 and 0078 require the page to show.

## Authority

- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) Decisions 1 to 3, and its accepted refinements.
- [ADR 0078](../adr/0078-publisher-page-type-from-stated-role.md), the page type.
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) Decision I: a derived value is labeled as derived.
- The durable set in [AGENTS.md](../../AGENTS.md): renderer portability and typed action state.

## Release Conditions

The orchestrator releases this packet when each condition is true:

1. Stophammer answers the album summary request. The orchestrator records the answer in that request document.
2. The deployed API sends the summary fields, or the operator selects a new route for album titles.

The orchestrator then corrects the "Index Query" section to the deployed fields before a session starts.

## Library Page Scope

The operator accepted this scope on 2026-09-24.

A Library publisher page shows two groups:

- Library albums: the local feeds with a stored `music_to_publisher` row for the publisher feed GUID. They come from local data.
- Albums that are not in the Library: the other albums that the publisher relationship lists. They come from one `INDEX_PUBLISHER_PAGE` request.

The Library group shows without the request. When the request fails, the page keeps the Library group and reports the failure for the other group.
An Index publisher page shows each album that the publisher relationship lists, and marks each album that is in the Library.

## Required Changes

### Artist Reference

Add `ArtistRef::PublisherFeed(String)` in `src/views.rs`. The value is the publisher feed GUID.
No code builds this value from name text or from `publisher_text`.

### Index Query

Add one request profile, `INDEX_PUBLISHER_PAGE`, to `src/application/request_profiles.rs`:
the path `/v1/feeds/{publisher_feed_guid}` with `include=publisher`.
The query sends that one request through the packet 018 shared owner.

The query reads the album list from the `publisher_to_music` entries.
With the summary fields, it sends no request for an album.

### Library Query

The query reads `feed_publisher_relationships` rows for the GUID and the local feeds that they name.
The page title comes from the stored `publisher_feed_title`.
The query then sends the one `INDEX_PUBLISHER_PAGE` request for the albums that are not in the Library.

### View Model

Add a publisher page view model under `src/view_models/`. It exposes these values, and no renderer computes them:

| Value | Rule |
|---|---|
| Title | The publisher feed `<title>`. Without a title: the GUID and the label "No title" |
| Page type | `Label` when one owned album has `role = "label"` and `role_source` is `publisher_rel` or `music_rel`. Otherwise `Artist`. ADR 0078 |
| Owned albums | Entries with `music_names_publisher = true` |
| "Listed by" albums | Entries with `music_names_publisher = false`, in a separate group |
| "Not listed" mark | An owned album with `publisher_lists_music = false` or `publisher_link_resolution = "unresolved"` |

The Stophammer source of 2026-09-25 shows that the publisher view lists only the albums that the publisher feed lists.
An Index page therefore receives no album that names the publisher without a listing by it. A Library page shows such albums from stored `music_to_publisher` rows.
The operator decided on 2026-09-25 not to request a reverse list now.
| Album role | The stated role and its source. `role_source = "default"` shows as an assumed role, never as a stated role |
| Role conflict | `role_source = "conflict"` shows `publisher_rel` and `music_rel`, each with its source. No value wins |
| Artist count | `distinct_release_artist_count` and `distinct_release_artists`, labeled as derived. They never select the page type |

Labels, availability and accessibility text for each action come from the view model.

## Mechanical Acceptance Criteria

Use the prefix `adr_0077_publisher_page_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R3-01 | The Index query sends one request with `INDEX_PUBLISHER_PAGE`, and no request for an album |
| R3-02 | The Library query returns the local feeds with a stored row for the GUID without a request. It sends one `INDEX_PUBLISHER_PAGE` request for the other albums, and puts them in a separate group |
| R3-02a | When that request fails, the Library group stays, and the view model reports the failure for the other group |
| R3-02b | The Index page marks each album that is in the Library |
| R3-03 | One owned album with a stated `label` role gives `Label`. ADR 0078 |
| R3-04 | A count of 5 with each role `default` gives `Artist`. ADR 0078 |
| R3-05 | A "listed by" album with a stated `label` role does not give `Label` |
| R3-06 | A conflict pair with `label` on one side does not give `Label`, and exposes both stated values |
| R3-07 | A `default` role is exposed as assumed. A test fails when it is exposed as stated |
| R3-08 | "Listed by" albums are a separate group from owned albums |
| R3-09 | An owned album with `publisher_lists_music = false` or `unresolved` has the "Not listed" mark |
| R3-10 | A publisher feed without a title exposes its GUID and "No title" |
| R3-11 | The artist count is exposed with a derived label, apart from the page type |
| R3-12 | No function builds `ArtistRef::PublisherFeed` from name text or `publisher_text`. A guard names ADR 0077 Decision 1 |

## Exclusions

- No screen, no navigation and no breadcrumb. Packet 004 owns them.
- No change to the name search. Packet 004 owns it.
- No paged album fetch. The orchestrator recommended the Stophammer route on 2026-09-24. The operator can change it.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/views.rs`: `ArtistRef` and `ArtistView`.
- `src/sources.rs`: `MetadataSource::fetch_artist`.
- `src/application/request_profiles.rs` and `src/application/request_reuse.rs`.
- `src/application/queries/feed.rs`: the inspector detail queries.
- `src/view_models/artist.rs` and `src/view_models/artist_detail.rs`.

## Checks

```bash
cargo test --lib adr_0077_publisher_page
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build --bin v4vmm
```

Report each result. Say "Green" for a passing check.

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Operator Visual Check

None. Packet 004 owns the visual gate for the page.
