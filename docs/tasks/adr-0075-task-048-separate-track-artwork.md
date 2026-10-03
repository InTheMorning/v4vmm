# ADR 0075 Task 048: Separate Track Artwork

Status: Implemented - 2026-09-29. Mechanical checks Green. Visual gate open and paused.

## Goal

The app reads the track image and the feed image of a MusicIndex track as two values with their owners.
An Index track shows its own image when it has one, and otherwise the feed image (ADR 0075 Decision C).
The evidence store records each image with its owner.

## Authority

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) Decision C, section 4, and the accepted track-artwork source order of 2026-09-20.
- MusicIndex API change request change 2, live since 2026-09-23.
- Stophammer ADR 0042: a response that carries a track gives `track_image_url` and `feed_image_url` beside the resolved `image_url`.

## Recorded Facts - 2026-09-29

- The deployed contract `0.2.0` declares `image_url`, `track_image_url` and `feed_image_url` on `TrackResponse`. `track_image_url` is null when the track has no image of its own.
- `image_url` on a track is the resolved value: the track image, else the feed image.
- `api::Track` decodes only `image_url`. The app reads it as a track value.
  - `TrackView::from_api` and `index_track_artwork_url` in `src/app/search_dispatch.rs` show it as the track image.
  - `api_track_from_view` in `src/app/search_dispatch.rs` sets `image_url` to the track image, else the feed image.
- `src/provider_observation/musicindex.rs` lists the observed fields. It has `image_url`, and no `track_image_url` or `feed_image_url`. It still lists `artist_credit`, which the contract removed on 2026-04-08.
- The Library track image comes from RSS (`src/rss/subscribe.rs`). This packet does not change it.
- `index_feed_artwork_url` shows the first track image when the feed has no image. That puts a track value on a feed. This packet records it and does not change it.

## Required Changes

### 1. Decode

- Add `track_image_url` and `feed_image_url` to `api::Track` as optional values.
- Keep `image_url` decoded for a response that has no owner fields. Such a response has unknown ownership (the accepted legacy-artwork rule).

### 2. Owners On The Index Route

- An Index track view holds the track image from `track_image_url` and the feed image from `feed_image_url`, as two values.
- The track page and the track row show the track image, else the feed image (Decision C). The view model makes this choice. The screen does not.
- When a response has only `image_url`, the view keeps it as an image of unknown owner. It never marks it as a track image.
- `api_track_from_view` sets no feed image as a track image.

### 3. Evidence

- Add `track_image_url` and `feed_image_url` to the observed MusicIndex fields.
- Delete `artist_credit` from the observed field list. The contract does not declare it.

## Mechanical Acceptance Criteria

Use the prefix `adr_0075_track_artwork_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R48-01 | A recorded track with both owner fields decodes both, and `image_url` |
| R48-02 | A track with a `track_image_url` exposes it as the track image. Its view model shows it |
| R48-03 | A track with a null `track_image_url` and a `feed_image_url` exposes no track image. Its view model shows the feed image |
| R48-04 | A track with only `image_url` exposes an image of unknown owner, and no track image |
| R48-05 | `api_track_from_view` gives no track image from a feed image |
| R48-06 | An observation of a track response records `track_image_url` and `feed_image_url`, and no `artist_credit` |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: an Index track with its own image shows that image. An Index track without one shows the album image.
- V2: normal and narrow widths show the artwork in its place, in Light and Dark themes.

## Exclusions

- No change to the Library artwork or to the RSS artwork.
- No change to the feed artwork fallback to a track image. The phase plan records it as a finding.
- No change to the tag artwork frame.
- No schema change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/api.rs`: `Track` and its decode tests.
- `src/views.rs`: `TrackView::from_api` and the artwork reference types.
- `src/app/search_dispatch.rs`: `index_track_artwork_url`, `index_track_row_artwork_url`, `api_track_from_view`.
- `src/application/queries/search.rs`: the track row artwork.
- `src/provider_observation/musicindex.rs`: the observed field list and its tests.
- `src/application/queries/stored_values.rs`: the packet 020 projection, for the legacy-artwork rule.

## Checks

```bash
cargo test --lib adr_0075_track_artwork_
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

This check needs a desktop session and network access to `api.musicindex.org`.

**Setup**

1. Assemble the desktop binary and open it:

   ```bash
   cargo build --bin v4vmm && target/debug/v4vmm
   ```

   Do this step first. A prior use of `cargo test` can keep a GPUI test-support binary at
   `target/debug/v4vmm`. Each next step happens in the open app.
2. Open Settings. Make sure the MusicIndex endpoint field holds a value, and the service
   answers.

**V1 - an Index track shows its own image, else the album image**

3. Open Music. Use toolbar search to find an Index feed with more than one track.
4. Open the feed's page. Note its album image.
5. Open the track list below the album image. Find a track row. Its thumbnail must be
   different from the album image. Open that track's page.
   - This result is incorrect: the track page hero image does not match the row thumbnail.
6. Go back to the track list. Find a track row. Its thumbnail must match the album image.
   Open that track's page.
   - This result is incorrect: the track page hero image is blank, broken, or a placeholder.
   - This result is incorrect: the track page hero image differs from the album image.
7. Repeat steps 3 to 6 in Dark theme (Settings > Appearance).

**V2 - the artwork keeps its place at normal and narrow widths**

8. At about 1400 pixels wide, open an Index track page and an Index feed page. Note the hero
   image position, size, and frame on each page.
9. Narrow the window to about 560 pixels. Open the same two pages again.
   - This result is incorrect: the hero image overlaps another element, is clipped, or changes
     size or frame from step 8.
10. Repeat steps 8 and 9 in Dark theme.

Color alone is not a valid difference. This check only reads pages, and it requests images
over the network. It makes no local or database change, so it needs no cleanup step.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. This check does not use it.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0075-task-048-separate-track-artwork.md`
- ADR 0075 Decision C and section 4
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": decode the owner fields, keep the owners apart on the Index route, and record them as evidence.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Tests use recorded JSON. No test sends a request.
- The view model chooses the image to show. The screen only composes.
- Treat each MusicIndex response as untrusted input.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The Library route artwork, the RSS parser and the tag writer.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R48-01 to R48-06 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 and V2.

Test commands:
- `cargo test --lib adr_0075_track_artwork_`
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
- A caller outside the Index route reads `Track::image_url` as a track-owned image for a stored value.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-09-29

### 1. Files Changed

- `src/api.rs`: `Track` decodes two new optional fields, `track_image_url` and
  `feed_image_url`, with the existing `image_url`. One new test, R48-01, decodes a
  recorded response with both fields set.
- `src/views.rs`: `TrackView` holds the same two fields.
  - `TrackView::from_api` decodes them from `api::Track`.
  - A new method, `TrackView::display_artwork_url`, picks the track's own image. Then it
    picks the feed's image. Then it picks the legacy `image_url` of unknown ownership
    (Decision C).
  - A new private helper, `trimmed`, gives this method the same blank-value filter that
    `nonempty_owned` gives the rest of the file.
  - `TrackView::from_local_with_facts` (the Library route) sets both new fields to `None`.
    It keeps its existing `image_url` projection unchanged.
  - Three new tests, R48-02 to R48-04, cover the track-image, feed-image, and
    unknown-ownership shapes.
- `src/app/search_dispatch.rs`: `index_track_artwork_url` calls
  `TrackView::display_artwork_url`. Before this change, it read `image_url` on its own. The
  track page hero image and the track row thumbnail both call this function, so both now
  use the view model's choice. `api_track_from_view` sets `track_image_url` and
  `feed_image_url` from the view's own fields, with no feed fallback for
  `track_image_url`. One new test, R48-05, checks this.
- `src/provider_observation/musicindex.rs`: the observed `SCALARS` list gains
  `track_image_url` and `feed_image_url`, and it drops `artist_credit`, a field the deployed
  contract no longer sends. One new test, R48-06, checks this.
- `docs/tasks/adr-0075-task-048-separate-track-artwork.md`: this packet. The Status line, this
  section, and the Operator Visual Check section.

### 2. Tests Run

Each command ran at the repository root.

- `cargo test --lib adr_0075_track_artwork_`: 6 passed. No test failed.
- `cargo test`: 1905 lib tests, 283 architecture tests, and 10 doc tests. All lib and
  architecture tests passed. The doc tests are ignored by design. No test failed.
- `cargo test --test architecture_tests`: 283 passed. No test failed.
- `cargo fmt -- --check`: Green.
- `cargo clippy -- -D warnings`: Green.
- `cargo check --all-targets`: Green. No warning appeared.
- `cargo build --bin v4vmm`: Green.

### 3. Behavior Changed

An Index track response that states `track_image_url` and `feed_image_url` now keeps both
facts apart. The track page hero image and the track row thumbnail show the track's own
image first, then the feed's image, matching Decision C. A response that states a null
`track_image_url` with a present `feed_image_url` now shows the feed image correctly. A
response that states neither field still shows its `image_url` value, unchanged from
before. The view now records that value as an image of unknown ownership, not as a
track-owned claim.

`api_track_from_view` runs when the operator downloads a track or a feed from the Index
route. It now carries the track's own claimed image forward. It never builds a track-owned
image claim from the feed's image.

Each existing test that set only `image_url` on a `TrackView` or an `api::Track` literal
keeps its prior result. The new fields default to `None`, and the fallback chain still ends
on `image_url`. No existing test needed a change.

The evidence store (`ProviderObservation`) now records `track_image_url` and
`feed_image_url` as separate coverage facts, each with its own declared-ownership
evidence. Every other scalar field already receives this same generic treatment. The
evidence store no longer records `artist_credit`.

The Library route, the RSS parser, and the tag writer are unchanged. A local `TrackView`
keeps `track_image_url` and `feed_image_url` at `None`. Its `image_url` field alone still
carries the one artwork value the Library route already picks (ADR 0076 Decision 1).

### 4. Deviations From Task

None. Each required change in this packet is implemented as stated, and no excluded file
changed.

### 5. Unresolved Concerns

- `TrackView.artwork` (the `ArtworkRef` field) still derives only from the legacy
  `image_url`, not from `display_artwork_url`. No current render path reads this field for a
  track, so this causes no visible difference today. A future reader of this field should use
  `display_artwork_url` instead, or this packet's separation is incomplete for it.
- `index_track_row_artwork_url` keeps its own fallback to the passed-in `FeedView.image_url`
  after `TrackView::display_artwork_url` runs. This preserves behavior from before this
  packet, and it needed no change. It means a row can still show a feed image from a
  different, separately fetched `FeedView`. That image can differ from the `feed_image_url`
  the track's own response states. The packet's exclusions do not ask for a change here.
- I checked every writer that persists a MusicIndex `api::Track` or `api::Feed` to the local
  database (`identity_ingest.rs`, `feed_service.rs`). None of them reads `image_url`,
  `track_image_url`, or `feed_image_url` for a stored value today, so the listed "Stop and
  report" condition did not occur. Artwork storage for the Index route remains future work,
  consistent with this packet's "No schema change" exclusion.

## Orchestrator Review - 2026-09-29

The orchestrator reviewed the diff and ran each check. Each check is Green: 1,905 unit tests, 283 guards, and no warning.

- `TrackView::display_artwork_url` makes the Decision C choice in the view layer. The track page and the track row use it.
- A response with only `image_url` still shows its image. The view never marks that image as a track image.
- `TrackView.artwork` has no reader. The only `.artwork` reader in `src/view_models/entity_detail.rs` reads a `FeedView`. The phase plan records this as a finding.
- The check only reads pages. It needs no fixture.
