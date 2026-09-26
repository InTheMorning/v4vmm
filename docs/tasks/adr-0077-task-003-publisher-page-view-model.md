# ADR 0077 Task 003: Publisher Page View Model

Status: Implemented - 2026-09-26. Mechanical checks are Green.
This packet changes no screen. It needs no visual acceptance. Packet 004 owns the visual gate.

## Goal

Add the publisher feed as an artist reference. Build one query for the Index publisher page and one for the Library publisher page.
Build one view model that exposes each fact that ADRs 0077 and 0078 require the page to show.

## Authority

- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) Decisions 1 to 3, and its accepted refinements.
- [ADR 0078](../adr/0078-publisher-page-type-from-stated-role.md), the page type.
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) Decision I: a derived value is labeled as derived.
- The durable set in [AGENTS.md](../../AGENTS.md): renderer portability and typed action state.
- Stophammer ADR 0059 owns the `remote_*` summary fields. The [open Stophammer requests](../plans/v4vmm-open-requests.md#deploy-of-2026-09-26-adr-0059) record the contract and its deployment.

## Release Record

Both release conditions are true on 2026-09-26:

1. Stophammer answered the album summary request with ADR 0059. The open requests record the answer.
2. Stophammer deployed ADR 0059 at commit `264706e` on 2026-09-26 at 04:05 UTC.
   v4vmm verified the fields on the live API on the same day.

## Library Page Scope

The operator accepted this scope on 2026-09-24.

A Library publisher page shows two groups:

- Library albums: the local feeds with a stored `music_to_publisher` row for the publisher feed GUID. They come from local data.
- Albums that are not in the Library: the other albums that the publisher relationship lists. They come from one `INDEX_PUBLISHER_PAGE` request.

The Library group shows without the request. When the request fails, the page keeps the Library group and reports the failure for the other group.
An Index publisher page shows each album that the publisher relationship lists, and marks each album that is in the Library.

The Stophammer publisher view lists only the albums that the publisher feed lists.
An Index page therefore receives no album that names the publisher without a listing by it. A Library page shows such albums from stored `music_to_publisher` rows.
The operator decided on 2026-09-25 not to request a reverse list now.

## Required Changes

### Artist Reference

Add `ArtistRef::PublisherFeed(String)` in `src/views.rs`. The value is the publisher feed GUID.
No code builds this value from name text or from `publisher_text`.

### Summary Fields

Add four optional fields to `api::PublisherRelationship`:

| Field | Value |
|---|---|
| `remote_feed_title` | The `title` of the feed that `remote_feed_guid` names |
| `remote_feed_image_url` | The channel image URL of that feed |
| `remote_release_artist` | The `release_artist` of that feed |
| `remote_release_artist_source` | The source of that `release_artist`, as Stophammer sends it |

Each field uses `#[serde(default)]` and `skip_serializing_if = "Option::is_none"`, as the other fields of the type do.
Stophammer sends null when the index holds no feed for the entry.
On a `publisher_to_music` entry, the fields describe the album. On a `music_to_publisher` entry, they describe the publisher feed.

This packet adds no storage column. The Library group uses the stored local feed rows.

### Index Query

Add one request profile, `INDEX_PUBLISHER_PAGE`, to `src/application/request_profiles.rs`:
the path `/v1/feeds/{publisher_feed_guid}` with `include=publisher`.
The query sends that one request through the packet 018 shared owner.

The query reads the album list from the `publisher_to_music` entries.
Each album takes its title, image, artist text and artist source from the `remote_*` fields of its entry.
The query sends no request for an album.

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
| Album title | `remote_feed_title`. Without it: the album feed GUID and the label "No title" |
| Album artist | `remote_release_artist` with `remote_release_artist_source`. Without it: no artist value. The view model invents no placeholder |
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
| R3-13 | `PublisherRelationship` decodes the four `remote_*` fields, and decodes an entry without them as `None` in each |
| R3-14 | An album entry with a null `remote_feed_title` exposes its GUID and "No title". A null `remote_release_artist` exposes no artist value |

## Exclusions

- No screen, no navigation and no breadcrumb. Packet 004 owns them.
- No change to the name search. Packet 004 owns it.
- No paged album fetch.
- No storage column and no migration.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/api.rs`: `PublisherRelationship`, `RoleSource`, `PublisherLinkResolution` and `Feed`.
- `src/views.rs`: `ArtistRef` and `ArtistView`.
- `src/sources.rs`: `MetadataSource::fetch_artist`.
- `src/application/request_profiles.rs` and `src/application/request_reuse.rs`.
- `src/application/queries/feed.rs`: the inspector detail queries.
- `src/application/queries/library.rs`: the album hydration that stores the relationship rows.
- `src/db/publisher_relationships.rs`: the stored rows and `music_to_publisher_remote`.
- `src/view_models/artist.rs` and `src/view_models/artist_detail.rs`.
- `tests/architecture_tests.rs`: the existing ADR 0077 guards.

## Files Likely To Change

| File | Permitted change |
|---|---|
| `src/api.rs` | The four summary fields and their decode tests |
| `src/views.rs` | `ArtistRef::PublisherFeed` |
| `src/sources.rs` | Only the match arms that the new variant requires |
| `src/application/request_profiles.rs` | `INDEX_PUBLISHER_PAGE` and its recorded-literal test |
| `src/application/queries/` | The Index and Library publisher page queries, in the existing query module that fits them |
| `src/db/publisher_relationships.rs` | A read of the rows for one publisher feed GUID |
| `src/view_models/` | The new publisher page view model module and its registration |
| `tests/architecture_tests.rs` | The R3-12 guard |
| This packet | The implementation result |

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

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0077-task-003-publisher-page-view-model.md`
- `docs/adr/0077-publisher-feed-artist-binding.md` and `docs/adr/0078-publisher-page-type-from-stated-role.md`
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".
- These are `ArtistRef::PublisherFeed`, the `remote_*` fields, `INDEX_PUBLISHER_PAGE`, two page queries, and the view model.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`: module `//!` comments that name ADR 0077, `#![warn(clippy::pedantic)]`, typed errors in the application layer, and unit tests beside the code.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Send every MusicIndex request through the packet 018 shared owner in `src/application/request_reuse.rs`.
- Treat each MusicIndex response as untrusted input.
- The view model computes each label, page type, group and mark. No renderer decides one.
- Never build `ArtistRef::PublisherFeed` from name text or `publisher_text`.
- A `role_source` of `default` is never exposed as a stated role.
- Do not commit. Do not run the app: no `cargo run` and no display attempt.

Do not touch:
- `src/ui/` and any screen or shell. Packet 004 owns them.
- The name search and navigation.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R3-01 to R3-14 in "Mechanical Acceptance Criteria" has a passing test with the prefix `adr_0077_publisher_page_`, or a guard for R3-12.
- Each command in "Checks" is Green.

Test commands:
- `cargo test --lib adr_0077_publisher_page`
- `cargo test`
- `cargo test --test architecture_tests`
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- `cargo build --bin v4vmm`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- A rule needs a field that the deployed contract does not send.
- The packet 018 shared owner cannot send this request without a change to its design.
- A test needs a change to a file in "Do not touch".
- The ADR 0078 page type rule and a stored row disagree.

## Implementation Result - 2026-09-26

### Files Changed

- `src/api.rs`: adds four `remote_*` fields to `PublisherRelationship`. Adds decode tests for R3-13 and R3-14.
- `src/views.rs`: adds `ArtistRef::PublisherFeed(String)`.
- `src/sources.rs`: adds one match arm for the new variant in `LocalSource::fetch_artist`, `LocalSource::list_feeds_for_artist`, and `ApiSource::list_feeds_for_artist`. Each new arm returns an error. No arm builds the new variant from name text.
- `src/application/request_profiles.rs`: adds the `INDEX_PUBLISHER_PAGE` profile and its recorded-literal test.
- `src/application/queries/feed.rs`: adds the Index publisher page query and its tests for R3-01 and R3-02b.
- `src/application/queries/library.rs`: adds the Library publisher page query and its tests for R3-02 and R3-02a.
- `src/db/publisher_relationships.rs`: adds `LocalPublisherAlbum` and `local_albums_for_publisher`, with a test for the reader.
- `src/view_models/publisher_page.rs` (new file): adds the publisher page view model, with tests for R3-03 through R3-11 and R3-14.
- `src/view_models/mod.rs`: registers the new module.
- `tests/architecture_tests.rs`: adds the R3-12 guard.
- This packet document: this section, and the updated status line.

### Tests Run

Each named check passed.

- `cargo test --lib adr_0077_publisher_page`: Green. 19 tests passed.
- `cargo test`: Green. 1827 library tests and 281 integration tests passed. 10 doc tests stayed ignored, the same as before this packet.
- `cargo test --test architecture_tests`: Green. 281 tests passed, with the new R3-12 guard.
- `cargo fmt -- --check`: Green.
- `cargo clippy -- -D warnings`: Green.
- `cargo build --bin v4vmm`: Green.

Each acceptance case has a passing test with the prefix `adr_0077_publisher_page_`, or the named guard for R3-12:

- R3-01: `application::queries::feed::adr_0075_request_profile_tests::adr_0077_publisher_page_index_query_sends_one_request_and_no_album_request`.
- R3-02: `application::queries::library::adr_0077_publisher_page_tests::adr_0077_publisher_page_library_query_reads_local_then_one_remote_request`.
- R3-02a: `application::queries::library::adr_0077_publisher_page_tests::adr_0077_publisher_page_library_query_keeps_library_group_on_remote_failure`.
- R3-02b: `application::queries::feed::adr_0075_request_profile_tests::adr_0077_publisher_page_index_query_marks_library_albums`.
- R3-03 through R3-11, and R3-14: `view_models::publisher_page::tests::adr_0077_publisher_page_*`.
- R3-12: `adr_0077_publisher_page_artist_ref_publisher_feed_built_only_from_guid`, in `tests/architecture_tests.rs`.
- R3-13: `api::tests::adr_0077_publisher_page::adr_0077_publisher_page_summary_fields_decode_each_stated_value` and its sibling test for the missing and null values.

### Behavior Changed

No screen changed. No operator flow changed. Each change is additive.

- `PublisherRelationship` decodes four more optional fields. The app decodes an older response without them too. Each new field stays `None`.
- The new `ArtistRef::PublisherFeed` variant exists. No production code path builds it.
- The new profile, the two new queries, and the new view model exist. No screen calls them. Packet 004 wires a screen to them.
- `src/sources.rs` gained one new match arm in three functions, so the code compiles with the new `ArtistRef` variant. Each new arm changes no existing behavior for `ArtistRef::LocalArtistName`.

### Deviations From Task

- The packet describes the Library page as showing two groups: Library albums, and albums that are not in the Library.
- The view model exposes a different grouping: `owned_albums` and `listed_by_albums`, from the "View Model" table, and each album carries an `in_library` flag.
- The Library query builds the Library albums first, from local storage, each marked `in_library = true`. It then adds each remaining network album, marked `in_library = false`.
- Packet 004 can make the two groups the packet names, from the `in_library` flag. This point needs inspection against the intended screen layout.
- The Library query also forwards `distinct_release_artist_count` and `distinct_release_artists` from its one network request, when that request succeeds. The packet's Library Page Scope section does not name this field for the Library query, but the one request carries it too. This keeps the Library and Index pages the same for R3-11.
- Each new `pub(crate)` item has no production caller, because packet 004 owns the screen. Without a suppression, `cargo clippy -- -D warnings` would report an error, because of dead code. Each such item carries `#[allow(dead_code)]` with a short comment naming ADR 0077 Task 003. This follows the existing pattern in `view_models::paged_feed_detail` and `view_models::paged_playlist_detail`.
- The R3-12 guard is a text scan for `ArtistRef::PublisherFeed(` construction sites. It denies the arguments `publisher_text`, `name`, and `title`. It found no violation, because no production code builds the variant.

### Unresolved Concerns

- Packet 004 must wire a screen to `fetch_index_publisher_page`, `fetch_library_publisher_page`, and `PublisherPageVm`. Until then, each `#[allow(dead_code)]` in this packet stays.
- Examine the "two groups" interpretation above before packet 004 builds its screen layout on it.
- No code builds `ArtistRef::PublisherFeed`. Packet 004 is the first site that needs one, from `publisher_feed_guid`, under ADR 0077 Decision 1 and the R3-12 guard.

## Orchestrator Review - 2026-09-26

The orchestrator reviewed the diff against ADRs 0077 and 0078, this packet, and `AGENTS.md`. The page type rule, the role display, the Library merge and the R3-12 guard agree with the packet.
Stophammer ADR 0049 gives a null `role` on a conflict, so the conflict display is correct.

The orchestrator made three corrections:

1. Each `#[allow(dead_code)]` became `#[cfg_attr(not(test), expect(dead_code, reason = ...))]`. `AGENTS.md` requires `expect` with a reason.
   The build then fails when packet 004 calls the item, and packet 004 must remove each expectation.
2. `PublisherPageVm::other_albums_failure` returned the transport error text. `AGENTS.md` requires a display state.
   `other_albums_status` now gives `OtherAlbumsStatus::Unavailable` with an operator report first and the technical detail after it.
3. The view model now gives the two Library page groups: `library_albums` and `other_albums`. A renderer does not compute a group.
   The test `adr_0077_publisher_page_library_groups_come_from_the_view_model` proves this.

These corrections replace deviation 3 and the first two unresolved concerns above.

Checks after the corrections, on 2026-09-26: `cargo fmt -- --check`, `cargo clippy -- -D warnings` and `cargo check --all-targets` with no warning are Green.
1,828 unit tests, 281 guards and 20 packet tests pass. `cargo build --bin v4vmm` is Green.

Follow-up for packet 004: a Library album has no artist text, because the app stores no `remote_release_artist` for a local feed.
Packet 004 can show the stored artist of the local feed. It must not show a placeholder.
