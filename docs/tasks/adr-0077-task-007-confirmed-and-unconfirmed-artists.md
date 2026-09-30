# ADR 0077 Task 007: Confirmed And Unconfirmed Artists

Status: Ready - 2026-09-29. Implementation has not started.
Its visual gate opens when the implementation is complete. Visual checks are paused, so the gate stays open.

## Goal

A publisher page shows two artist lists from Stophammer ADR 0061. One list holds the artists of the albums that name this publisher. The other holds the artists of the albums that this publisher lists and that do not name it.
The page no longer shows a single "Artists" count that ignores the second group.

## Authority

- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) Decisions 2 and 3, and [ADR 0078](../adr/0078-publisher-page-type-from-stated-role.md): the artist count never selects the page type.
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) Decision I: MusicIndex derives these values. The page names them as derived.
- Stophammer ADR 0061, Accepted on 2026-09-27 and deployed in release 0.2.0.
- The durable set in [AGENTS.md](../../AGENTS.md): renderer portability and token discipline.

## Recorded Facts - 2026-09-29

- The deployed contract `0.2.0` declares four fields on `FeedResponse`: `confirmed_release_artists`, `confirmed_release_artist_count`, `unconfirmed_release_artists` and `unconfirmed_release_artist_count`.
- Stophammer ADR 0061 defines them:
  - A confirmed artist is a distinct `release_artist` of a listed album that also names this publisher.
  - An unconfirmed artist is the same for a listed album that does not name this publisher. An artist of a confirmed album is not also in the unconfirmed list.
  - An album with `release_artist_source` = `placeholder` gives no value. The values keep the order of first listing.
- `distinct_release_artist_count` and `distinct_release_artists` keep their meaning: the albums that name the publisher, listed or not.
- The publisher feed `137aaa9c-75ff-4916-9f23-e02968b2d15e` ("Official DETOX Music") gave `confirmed_release_artist_count` 1 and `unconfirmed_release_artist_count` 0, with and without `include=publisher`.
- On 2026-09-26, 15 publishers had no two-way link. "Master's Scroll" lists 81 albums by 33 artists, and no album names it (Stophammer ADR 0061).
- `PublisherPageVm::header_facts` in `src/view_models/publisher_page.rs` shows one "Artists" fact from `distinct_release_artist_count`, with the label "Derived from album credits".
  A publisher whose listed albums do not name it shows 0 above a full "listed by" group.
- `fetch_index_publisher_page_albums` in `src/application/queries/feed.rs` builds the page facts. The Index page and the Library page both use it.

## Required Changes

### 1. Decode

- Add the four fields to `api::Feed` as optional values.

### 2. Page Facts

- The publisher page facts hold the confirmed list and count, and the unconfirmed list and count.
- Delete the page use of `distinct_release_artist_count` and `distinct_release_artists`. Delete their decode when no reader stays.
- When the response has none of the four fields, the page shows no artist fact. It does not fall back to the distinct count.

### 3. Header Facts

- The view model gives two header facts, in this order:
  1. The confirmed artists, with the proposed label "Artists that name this feed".
  2. The unconfirmed artists, with the proposed label "Artists this feed lists without a link back".
- Each fact shows its count and its names, and the view model names it as derived by MusicIndex.
- A count of 0 shows as 0 when the field is present. An absent field gives no fact.
- The facts never select the page type (ADR 0078).
- The view model owns each label and each text. The screen composes the facts that it already composes. The screen adds no glyph string and no literal.

## Mechanical Acceptance Criteria

Use the prefix `adr_0077_confirmed_artists_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R7-01 | A recorded feed response decodes the four fields. An omitted field decodes as absent |
| R7-02 | Facts with confirmed 1 and unconfirmed 0 give two header facts, with counts 1 and 0 and the confirmed name |
| R7-03 | Facts with confirmed 0 and unconfirmed 33 give two header facts with those counts and the unconfirmed names |
| R7-04 | Facts without the four fields give no artist fact, also when `distinct_release_artist_count` is present |
| R7-05 | The page type is equal for two pages that differ only in their artist lists |
| R7-06 | No page code reads `distinct_release_artist_count` or `distinct_release_artists` |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: the DETOX publisher page shows "Artists that name this feed: 1" with the name, and the unconfirmed fact with 0.
- V2: a publisher whose listed albums do not name it shows 0 confirmed artists and its unconfirmed artists with their names.
- V3: the two labels read clearly. The operator accepts them or gives new labels.
- V4: normal and narrow widths show each fact in its place, in Light and Dark themes, with no clipped text.

## Exclusions

- No change to the album groups, the page type or the roles.
- No new request. The four fields arrive on the present feed request.
- No storage of the fields.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/api.rs`: `Feed`, the publisher decode tests.
- `src/application/queries/feed.rs`: `fetch_index_publisher_page_albums` and its facts.
- `src/application/queries/library.rs`: `fetch_library_publisher_page`.
- `src/view_models/publisher_page.rs`: `PublisherPageFacts`, `PublisherPageVm::header_facts`, `DerivedArtistCount`, `page_type`.
- `src/ui/shells/publisher.rs`: the header fact composition.

## Checks

```bash
cargo test --lib adr_0077_confirmed_artists_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Operator Visual Check

The implementer writes this section at completion. It gives numbered steps for V1 to V4.
It states the needed state, what counts as wrong, and the cleanup.

The check only reads pages. It needs network access to `api.musicindex.org`.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a valid difference.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0077-task-007-confirmed-and-unconfirmed-artists.md`
- ADR 0077 Decisions 2 and 3, and ADR 0078
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": decode the four fields, hold them in the page facts, and show two derived header facts.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Tests use recorded JSON. No test sends a request.
- The view model gives each label and text. The screen only composes. A screen uses the scaled tokens of ADR 0039 and existing shared composites.
- Treat each MusicIndex response as untrusted input.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The album groups, the page type rule and the role display.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R7-01 to R7-06 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V4.

Test commands:
- `cargo test --lib adr_0077_confirmed_artists_`
- `cargo test`
- `cargo test --test architecture_tests`
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- `cargo check --all-targets`
- `cargo build --bin v4vmm`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- A reader other than the publisher page uses `distinct_release_artist_count` or `distinct_release_artists`.
- The header fact composition cannot show two facts without a new shared composite.
- A change needs a file in "Do not touch".
