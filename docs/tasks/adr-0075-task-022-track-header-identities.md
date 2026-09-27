# ADR 0075 Task 022: Track Header Identities

Status: Ready - 2026-09-26. ADR 0077 packet 005 is implemented. Implementation has not started.
Its visual gate opens when the implementation is complete. Visual checks are paused, so the gate stays open.

## Goal

A track page shows only the track's own identities in its header.
The feed identities show in a separate section that names the feed as the owner. ADR 0075 Decision B.
A track without its own description shows no description.

This packet changes no tag frame. A proposed tag mapping ADR owns the frames.

## Authority

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) Decision B, section 4, and the accepted track-description placement of 2026-09-20.
- [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md): one projection in `src/application/queries/stored_values.rs` owns the stored values.
- The durable set in [AGENTS.md](../../AGENTS.md): element hierarchy, typed action state, renderer portability and token discipline.

## Recorded Facts - 2026-09-26

- `api::track_with_feed_defaults` in `src/api.rs` copies these feed values into a track that has none: `source_links`, `source_ids`, `source_contributors`, `description`, `image_url`, `publisher_text`, `release_artist`, `source_release_claims`, `payment_routes`, `feed_guid` and `feed_title`.
- A track page builds its identity actions from `TrackView::identity`. On the Index route, that value comes from the copied `source_links` and `source_ids`.
  Thus the header shows the feed website and the feed Nostr key as the track's own.
- The Library route fills `source_links` and `source_ids` from the track-owned local facts (`hydrate_track_identity` in `src/feed_service.rs`). It does not copy.
- Subscribe stores the track from a value that it takes before the copy. Stored facts do not contain a copied feed value.
- The tag rows map "Website" and "RSS feed website" to `WOAR`, and "Nostr handle" and "RSS feed nostr handle" to `TXXX:RSS Nostr Handle` (`id3_frame_hint` in `src/metadata.rs`).
  The "Description" tag row reads the track description, and no feed description row exists.

## Required Changes

### 1. Stop Three Copies

In `api::track_with_feed_defaults`, delete the copies of `source_links`, `source_ids` and `description`.
Keep each other copy. Decision C permits the artwork fallback. The other copies are outside this packet.

### 2. Keep The Tag Output

The tag edits for a track must not change in this packet.

- Where a tag row read a copied feed value, it reads the feed value from the `TrackContext` feed explicitly. `source_value_for_metadata_field` in `src/metadata.rs` already does this for other fields.
- The "Description" row reads the track description, then the feed description.
- The "Website" and "Nostr handle" rows read only track-owned values. The "RSS feed website" and "RSS feed nostr handle" rows continue to supply the feed values.

### 3. Track Header

- The track header shows only the identities of the track: its own website or page link, and its own Nostr key.
- `TrackDetailVm` gets the feed identities through a builder, for example `with_feed_identity`. It exposes them as a separate section with the feed title as the owner label.
- Each identity action in that section names the feed as its owner in its label and its accessibility text, before the operator activates it.
- A track without its own identities shows no identity action in its header. The view model invents no placeholder.

### 4. Description

A track without its own description shows no description on its page. The page shows no feed description in its place.

## Mechanical Acceptance Criteria

Use the prefix `adr_0075_track_header_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R22-01 | `track_with_feed_defaults` does not copy `source_links`, `source_ids` or `description`. It still copies each other value |
| R22-02 | A track has no own identities, and its feed has a website and a Nostr key. The header of its view model exposes no identity action |
| R22-03 | The same view model exposes a feed identity section. The feed title is its owner, and each action label names the feed |
| R22-04 | A track with its own website and Nostr key exposes them in the header. The feed section exposes the feed values apart from them |
| R22-05 | A track without its own description exposes no description |
| R22-06 | `metadata_service::id3_edits_for_track_context` gives equal tag edits before and after this packet. Test a track with no own values and a track with its own values. The test holds the expected edits as fixed values |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: an Index track page of a track without its own identities shows no website or Nostr action in its header. The feed section shows the feed website and Nostr key, with the feed named as owner, in Light and Dark themes.
- V2: a track with its own identities shows them in its header, apart from the feed section.
- V3: a track without its own description shows no description.
- V4: normal and narrow window widths show each element in its defined place, with no clipped text.

## Exclusions

- No change to a tag frame, to the tag compare or to "Update n file(s)". The proposed tag mapping ADR owns them.
- No change to the contributor copy or to the credit list. ADR 0076 packet 006 owns the credit list.
- No change to the Library route identity source.
- No schema change and no new request.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/api.rs`: `track_with_feed_defaults` and its tests.
- `src/metadata.rs`: `source_value_for_metadata_field`, `track_metadata_rows`, `track_website`, `track_nostr`, `feed_website` and `id3_frame_hint`.
- `src/metadata_service.rs`: `id3_edits_for_track_context`.
- `src/views.rs`: `TrackView` and `EntityIdentityLinks`.
- `src/view_models/track_detail.rs`: `TrackDetailVm` and its identity actions.
- `src/feed_service.rs`, `src/subscribe_service.rs` and `src/application/commands/payment_routes.rs`: the callers of `track_with_feed_defaults`.
- The track page screens under `src/ui/shells/`: `track.rs`, `library/track_detail.rs` and `discover/track_inspector.rs`.

## Checks

```bash
cargo test --lib adr_0075_track_header
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

The implementer writes this section at completion, with numbered steps for V1 to V4, the needed state, what counts as wrong, and the cleanup.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a valid difference.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0075-task-022-track-header-identities.md`
- ADR 0075 Decision B and section 4
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": stop three copies, keep the tag output, and show track and feed identities apart on the track page.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives each label, owner name, section and action availability. The screen only composes them.
- No raw display literal and no glyph string in a renderer.
- The tag edits stay equal. Prove it with R22-06 before you change a tag row.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- Tag frames, the tag compare, `id3_frame_hint` values and the "Update n file(s)" flow.
- The contributor copy and the credit list.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R22-01 to R22-06 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V4.

Test commands:
- `cargo test --lib adr_0075_track_header`
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
- A caller of `track_with_feed_defaults` needs a copied identity or description for a purpose other than a tag row.
- The tag edits change and an explicit feed read cannot keep them equal.
- A track page has no feed in its context, so it cannot show the feed section.
- A change needs a file in "Do not touch".
