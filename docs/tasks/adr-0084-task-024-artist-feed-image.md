# ADR 0084 Task 024: The Image Of An Artist Feed

Status: Ready - 2026-10-20.

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
