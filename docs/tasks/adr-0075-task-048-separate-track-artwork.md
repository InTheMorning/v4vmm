# ADR 0075 Task 048: Separate Track Artwork

Status: Ready - 2026-09-29. Implementation has not started.
Its visual gate opens when the implementation is complete. Visual checks are paused, so the gate stays open.

## Goal

The app reads the track image and the feed image of a MusicIndex track as two values with their owners.
An Index track shows its own image when it has one, and otherwise the feed image (ADR 0075 Decision C).
The evidence store records each image with its owner.

## Authority

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) Decision C, section 4, and the accepted track-artwork source order of 2026-09-20.
- [MusicIndex API change request](../plans/musicindex-api-change-request.md) change 2, live since 2026-09-23.
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

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
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

The implementer writes this section at completion. It gives numbered steps for V1 and V2.
It states the needed state, what counts as wrong, and the cleanup. The check needs network access to `api.musicindex.org`.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a valid difference.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
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
