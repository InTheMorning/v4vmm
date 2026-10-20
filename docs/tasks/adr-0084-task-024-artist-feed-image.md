# ADR 0084 Task 024: The Image Of An Artist Feed

Status: Implemented - 2026-10-20. Migration 20: make the backup of the Rollback section first. Cases R24-01 to R24-05 pass. Mechanical checks are Green. V1 to V3 are open, in the final walk of the run.

## Goal

The Library stores the image of each artist feed that an album names. Each Artists row and each singles group of that artist then shows that image as a square, in place of the logo mosaic.

## Authority

- [ADR 0084](../adr/0084-music-column-browser.md) Decision 12, amended on 2026-10-20: an image that an artist feed gives replaces the mosaic and shows as a square.
- [ADR 0016](../adr/README.md): each schema change goes through the migration registry.
- [ADR 0085](../adr/0085-artists-by-confirmation.md): the named feeds of an album and the artist feed.
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) Decision I: MusicIndex is a cache of RSS. The image is the RSS channel image, as MusicIndex reports it.
- The [run record of 2026-10-20](../reviews/orchestrated-run-2026-10-20.md): the orchestrator split this packet from packet 011.

## Recorded Facts - 2026-10-20

- Schema version 19 is current. `MIGRATIONS` in `src/db.rs` is the registry.
- `feed_publisher_relationships` (`src/db/publisher_relationships.rs`) keeps `publisher_feed_title` and no image.
- The MusicIndex album response gives `PublisherResponse.remote_feed_image_url` (`tests/fixtures/musicindex-openapi-0.11.0.json`, decoded in `src/api.rs`). The upsert in `src/db/publisher_relationships.rs` drops it.
- `NamedFeed` (`src/view_models/named_feeds.rs`) has no image field.
- Packet 011 gives each Artists row and each singles group row a `CollectionArtVm` with no provided image.

## Design

- Migration 20 adds `publisher_feed_image_url TEXT NULL` to `feed_publisher_relationships`. It writes no value. The next album hydration of each album fills it.
- The upsert stores `remote_feed_image_url`, trimmed. An empty value stores `NULL`.
- `NamedFeed` gets `image_url: Option<String>`.
- `artist_list_row` gives a feed row `CollectionArtVm::resolve(Some(image), covers, Artist)`. A name row gives no provided image.
- A singles group of a feed-level artist gives the same provided image. A group of a name only gives none.
- No new MusicIndex request. The value arrives with the present album hydration.

## Required Changes

1. Add migration 20 through the registry, with its test.
2. Store the value in the upsert, and read it into `NamedFeed`.
3. Give the provided image to the Artists rows and the singles groups.
4. Update `AGENTS.md` "Stored facts": schema version 20.

## Mechanical Acceptance Criteria

Use the prefix `adr_0084_artist_image_` for unit tests beside the owning code.

| Case | Required proof |
|---|---|
| R24-01 | Migration 20 adds the column to a schema 19 database, and keeps each row |
| R24-02 | The upsert stores a recorded `remote_feed_image_url`, trimmed. An empty value stores `NULL` |
| R24-03 | `NamedFeed` reads the stored value |
| R24-04 | A feed row of the Artists list gives `ProvidedImage` when the value exists, and the mosaic when it does not. A name row never gives `ProvidedImage` |
| R24-05 | A singles group of a feed-level artist gives the same provided image |

## Visual Acceptance Criteria

These are for the operator, in the final walk of the run. No test proves them.

- V1: an artist feed has a channel image. After a feed check or a reload of its album, its Artists row shows that image as a square.
- V2: an artist feed with no channel image shows the mosaic.
- V3: Light and Dark themes, and a narrow width. Each row stays aligned.

## Exclusions

- No Index artist list. No code reads `/v1/artist-names`.
- No new request.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/db.rs` (`MIGRATIONS`), `src/db/publisher_relationships.rs`, `src/api.rs`, `tests/fixtures/musicindex-openapi-0.11.0.json`.
- `src/view_models/named_feeds.rs`, `src/view_models/library.rs` (`artist_list_row`), `src/view_models/singles_group.rs`, `src/view_models/collection_art.rs` (packet 011).
- The archived packet 011 and the migration tests in `src/db.rs`.

## Checks

```bash
cargo test --lib adr_0084_artist_image_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Migration 20 changes the database. Before the first start of the new build, close the app. Then copy the Library database file as the backup.
To roll back, close the app, revert the commit, and restore the backup. An older build refuses a database at schema 20.

## Implementation Result - 2026-10-20

### Files Changed

- `src/db/publisher_feed_image.rs` (new): migration 20, `publisher_feed_image`. It adds `publisher_feed_image_url TEXT NULL` to `feed_publisher_relationships` and gives its read contract.
- `src/db.rs`: `CURRENT_VERSION` is 20. `MIGRATIONS` has migration 20. `schema_contract` gives the migration 20 columns from version 20.
  `migrate_schema_internal` verifies the retained records after migration 20, the same as after migration 19. The two version lists of the tests include 20.
- `src/db/upgrades.rs`: `create_fixture` accepts version 20.
- `src/db/maintenance/upgrade.rs`: the failure text names migrations 12 to 20. The tests expect target 20 and the three boundaries of migration 20.
- `src/view_models/startup/database.rs`: one test expects version 20.
- `src/db/publisher_relationships.rs`: the upsert stores `remote_feed_image_url`, trimmed, in `publisher_feed_image_url`. An empty value stores null. `named_feeds` reads the column into `NamedFeed::image_url`.
  The test of migration 19 now expects migration 20 after it. Tests R24-01, R24-02 and R24-03 are in this file.
- `src/view_models/named_feeds.rs`: `NamedFeed` has `image_url: Option<String>`. The new function `named_feed_image` gives the first image of one feed GUID in a list of named feeds.
- `src/view_models/library.rs`: `artist_row_art` takes the provided image. `artist_list_row` gives the feed image to a feed row and `None` to a name row. Test R24-04 is in this file.
- `src/view_models/singles_group.rs`: `SinglesGroup::collection_art` gives the feed image to a group of an artist feed and `None` to a group of a name. Test R24-05 is in this file.
- `src/views.rs`: the Index named feeds read `image_url` from the response entry, trimmed.
- `src/application/queries/library.rs`, `src/view_models/artist_identity.rs`, `entity_detail.rs`, `release_page.rs` and `track_detail.rs`: the test values get the new field. The hydration test now sends a `remote_feed_image_url` and expects the trimmed stored value.
- `AGENTS.md`: schema version 20 is current.

### Behavior Changed

- Migration 20 adds one column with null in each row. It changes no other value.
- Each album hydration from `MusicIndex` stores the channel image of each named feed. A later response with no image stores null.
- An Artists row of an artist feed shows the feed image as a square when the stored value exists. It shows the mosaic or the entity color when no value exists.
- A singles group of an artist feed shows the same image on its row and on its page. A name row and a group of a name do not change.
- If the image does not load, the row shows the mosaic of packet 011 (`without_provided`).

### Checks - 2026-10-20

| Check | Result |
|---|---|
| `cargo test --lib adr_0084_artist_image_` | Green, 5 tests |
| `cargo test` | Green, 2058 unit tests and 339 guards. Ten documentation examples stay ignored |
| `cargo test --test architecture_tests` | Green, 339 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

No application launch and no production-data change occurred. Each test uses an in-memory database.

### Deviations From The Task

1. The stop condition did not occur. The album hydration in `src/application/queries/library.rs` gives each complete `PublisherRelationship` entry to the upsert. Each entry holds `remote_feed_image_url`.
2. The Index named feeds in `src/views.rs` also read the image, so `NamedFeed::image_url` has one meaning on the two paths. No Index list uses the value.
3. Two albums can give different images for one feed. The row then shows the first image in the album sequence of the row.
4. The hydration test of `src/application/queries/library.rs` uses the test URL `https://example.test/publisher-artist.jpg`. The recorded album response of that test has no image value.

### Unresolved Concerns

1. V1 to V3 below need the operator. No test proves a rendered image, the square shape or the alignment.
2. A row shows the image only after the next album hydration of one of its albums. A hydration that reuses a stored response writes no row.
3. The new visual checks V1 to V3 are not in `docs/pending-human-checks.md` or in the phase plan. The task instructions named only this packet as a document to edit.
4. The source map does not list `src/db/publisher_feed_image.rs`. It also does not list the migration 19 file.

## Operator Visual Check

This check needs a Linux desktop session and this checkout. It also needs these items:

- A Library album that names an artist feed with a channel image, for example a Wavlake album.
- A Library album that names an artist feed with no channel image, if one is available.
- A network connection to `MusicIndex`.

The first start of this build changes the Library database to schema version 20. Make the backup before step 2.

1. Close the app. Copy the Library database file as the backup: `cp ~/.local/share/v4vmm/v4vmm.sqlite ~/v4vmm-before-schema-20.sqlite`.
   If `config.toml` sets a different `db_path`, copy that file.
2. Build the app: `cargo build --bin v4vmm`. Then start `target/debug/v4vmm` from a desktop session.
3. Open Music, then Artists, in the sidebar.

### V1: An artist feed with a channel image

4. Open the album that names the artist feed with a channel image. Wait until the album page shows its data. As an alternative, run a feed check of the album.
5. Open Artists again. Find the row of that artist feed.
6. The row must show the channel image of the feed as a square, in place of the logo mosaic.
7. If the artist has two or more singles in the Library, open its singles group. The group row and the group page must show the same image.

**Wrong if:** the row still shows the mosaic after step 4, or the image is not square. Also wrong: a row of an artist name, marked "by name", shows a feed image.

### V2: An artist feed with no channel image

8. Open the album that names the artist feed with no channel image. Then open Artists again.
9. The row of that feed must show the logo mosaic of its album images, or the entity color with no album image.

**Wrong if:** the row shows an empty frame, or an image of a different feed.

### V3: Themes and a narrow width

10. Change between the Light and the Dark theme in Settings. Look at the Artists list in each theme.
11. Make the window narrow, then make it wide again.
12. In each condition, compare the image rows and the mosaic rows.

**Wrong if:** an image row is taller or wider than a mosaic row, or a label moves out of its column.

**Cleanup:** the app keeps schema version 20, and this check needs no other cleanup. To roll back, close the app and revert the commit. Then copy `~/v4vmm-before-schema-20.sqlite` back to the Library database path. Do not delete `/tmp/v4vmm-governance.ie6k8TQf`.

## Orchestrator Review - 2026-10-20

Pass. The code needs no fix.

- `cargo fmt -- --check` and `cargo clippy -- -D warnings` are Green. `cargo test` gives 2058 library tests and 339 guards, all passing.
- The hydration reuses a response only inside a time window of the running session. After a restart, the first view of an album page sends a new request and stores the image. For V1, open one album page of the artist first.
- Open finding: the Artists list gets each image only after a view of one album of that artist. No packet owns a fill for each artist. The phase plan records it.
- The orchestrator added the migration file to the source map.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-024-artist-feed-image.md`
- ADR 0084 Decision 12 with its amendment of 2026-10-20, ADR 0016 and ADR 0085
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the migration, the stored value and the provided image.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Treat the MusicIndex value as untrusted input.
- Tests use recorded values and an in-memory database. No test sends a request.
- Never run `git checkout`, `git restore`, `git stash`, `git reset`, `git add` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The logo composition of packet 011, except its input.
- Any ADR, and each document other than this packet and the schema line of `AGENTS.md`.

Acceptance criteria:
- Each case R24-01 to R24-05 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - 2026-10-20" section and an "Operator Visual Check" section for V1 to V3, with the backup step first.

Test commands:
- The commands in "Checks", with `cargo fmt -- --check` after the last edit and `cargo build --bin v4vmm` last.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The album hydration does not pass `remote_feed_image_url` to the upsert.
- The migration registry needs a change to its own rules.
- A change needs a file in "Do not touch".
