# ADR 0077 Task 007: Confirmed And Unconfirmed Artists

Status: Implemented - 2026-09-30. Mechanical checks Green. Visual gate open and paused.

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

## Implementation Result

### Files

Changed files:

- `src/api.rs`: `Feed` now decodes `confirmed_release_artist_count`, `confirmed_release_artists`,
  `unconfirmed_release_artist_count`, and `unconfirmed_release_artists`. It no longer decodes
  `distinct_release_artist_count` or `distinct_release_artists`. The two earlier ADR 0077 decode
  tests (R2-01, R2-02) drop their assertions on those two removed fields. A new
  `adr_0077_confirmed_artists` test module adds the R7-01 decode tests.
- `src/application/queries/feed.rs`: `publisher_page_facts_from_feed` copies the four new `Feed`
  fields into `PublisherPageFacts`.
- `src/application/queries/library.rs`: `fetch_library_publisher_page` reads the four new fields
  from its one remote request. Its test fixture and two test assertions use the new field names.
- `src/view_models/publisher_page.rs`: `PublisherPageFacts` holds the four new fields.
  `PublisherPageVm` gained the constants `CONFIRMED_ARTISTS_LABEL` and `UNCONFIRMED_ARTISTS_LABEL`.
  It gained the methods `confirmed_artist_count` and `unconfirmed_artist_count`, which replace
  `derived_artist_count`. `header_facts` adds the confirmed fact, then the unconfirmed fact, each
  only when its count is present.
- The same file's tests: three earlier tests use the new field and method names. Four new tests
  prove R7-02 through R7-05.
- `docs/tasks/adr-0077-task-007-confirmed-and-unconfirmed-artists.md`: this packet document.

No file needed creation. `src/ui/shells/publisher.rs` needed no change: it already loops over
`header_facts()` and builds one `DetailHeaderDataRow` for each fact.

### Tests

- `cargo test --lib adr_0077_confirmed_artists_`: 6 passed. Two prove R7-01 in `src/api.rs`. Four
  prove R7-02, R7-03, R7-04 and R7-05 in `src/view_models/publisher_page.rs`.
- `cargo test`: 1,925 library tests and 283 architecture tests passed. 0 failed.
- `cargo test --test architecture_tests`: 283 passed.
- `cargo fmt -- --check`: Green.
- `cargo clippy -- -D warnings`: Green, no warning.
- `cargo check --all-targets`: Green, no warning.
- `cargo build --bin v4vmm`: Green.

### Behavior

The publisher page no longer shows one "Artists" fact from `distinct_release_artist_count`. It
shows two header facts, in this order. The first fact is labeled "Artists that name this feed." It
states the confirmed count and the confirmed artist names. The second fact is labeled "Artists
this feed lists without a link back." It states the unconfirmed count and the unconfirmed artist
names.

A count of 0 still shows its row. An absent field adds no row for that fact. The app stores and
reads no fallback value. The old field is gone from both `Feed` and `PublisherPageFacts`.

The Index query and the Library query each carry the four new fields on the one existing request.
This packet sends no new request. The album groups, the page type rule, and the role display keep
their present behavior.

### Deviations

- Required Change 2 states: "Delete their decode when no reader stays." After the page's use of
  the two old fields was removed, no code under `src/` read them. `distinct_release_artist_count`
  and `distinct_release_artists` are deleted from `api::Feed`, not only from the page facts. The
  two ADR 0077 decode tests that had asserted those two fields are trimmed to match. Serde ignores
  an unrecognized JSON key by default. The recorded JSON bodies in those tests stay as the
  historical record.
- The R7-01 decode test sits in a new `adr_0077_confirmed_artists` module in `src/api.rs`, beside
  the existing `adr_0077_publisher_page` module. It follows the same recorded-response pattern as
  that module.

### Concerns

No "Stop and report" condition came up during this task.

- No reader other than the publisher page used `distinct_release_artist_count` or
  `distinct_release_artists`.
- `DetailHeaderDataRow` already accepts any number of rows. The header fact composition needed no
  new shared composite to show two facts.
- No change needed a file named in "Do not touch."

## Operator Visual Check

The check only reads pages. It needs network access to `api.musicindex.org`.

**Setup**

1. Build and open the desktop binary:

   ```bash
   cargo build --bin v4vmm && target/debug/v4vmm
   ```

   Run this command first. A prior `cargo test` run can leave a GPUI test-support binary at
   `target/debug/v4vmm`. This step is the only step that starts the app.
2. Open Settings. Confirm the MusicIndex endpoint field holds a working endpoint.

**V1 - the DETOX publisher page shows both counts**

3. Open the publisher page for the "Official DETOX Music" feed
   (`137aaa9c-75ff-4916-9f23-e02968b2d15e`), from the Index or from the Library.
4. Read the header facts.
   - Correct: a row labeled "Artists that name this feed" states 1, with the name "Official DETOX
     Music."
   - Correct: a row labeled "Artists this feed lists without a link back" states 0.
   - Wrong: the page shows one plain "Artists" row.
   - Wrong: either label is missing, or a row shows the wrong count for its label.

**V2 - a publisher with unconfirmed artists and no confirmed link**

5. Open the publisher page for a publisher feed whose listed albums do not name it back. The
   packet's Recorded Facts name "Master's Scroll" as one example, on 2026-09-26.
6. Read the header facts.
   - Correct: "Artists that name this feed" states 0.
   - Correct: "Artists this feed lists without a link back" states a count above 0, with its artist
     names.
   - Wrong: the page shows no fact for one or both rows.
   - Wrong: an artist name appears under the wrong label.

**V3 - the labels read clearly**

7. Read both header labels on the pages from step 3 and step 5.
   - Accept the labels "Artists that name this feed" and "Artists this feed lists without a link
     back." Or, give new wording for either label.

**V4 - normal and narrow widths, Light and Dark themes**

8. Repeat step 3 and step 4 at the normal window width. Then repeat them at a narrow width. Pull
   the window edge until the Library sidebar collapses. Or resize the window below the
   narrow-layout width named in the sidebar and toolbar runbooks.
9. Repeat step 8 in Light theme. Then repeat it in Dark theme (Settings > Appearance).
   - Wrong: either header row clips its text at either width or in either theme.
   - Color alone is not a valid difference between a correct result and a wrong one.

**Cleanup**

10. Close the app window. This packet writes no file and stores no new row. No step above changes
    stored state. Do not delete `/tmp/v4vmm-governance.ie6k8TQf`.

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

## Orchestrator Review - 2026-09-30

The orchestrator reviewed the diff and ran each check. Each check is Green: 1,925 unit tests, 283 guards, and no warning.

- The Index page and the Library page read the four fields from the present request. No request is added.
- `distinct_release_artist_count` and `distinct_release_artists` have no reader, and their decode is deleted.
- The screen needed no change. It shows each fact that `header_facts` gives.
- The orchestrator rewrote two test comments that described earlier code. They now state the present code.
- The operator decides the two labels at V3.
