# ADR 0075 Task 022: Track Header Identities

Status: Open - implementation and mechanical checks are complete on 2026-09-28. On 2026-10-07 the operator moved its visual check to the [overhaul plan](../plans/design-and-cleanup-overhaul-plan.md#visual-requirements-moved-from-pending-checks---2026-10-07). The packet that rebuilds the surface carries it.

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

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
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

**Setup**

1. Assemble the desktop binary and open it:

   ```bash
   cargo build --bin v4vmm && target/debug/v4vmm
   ```

   Do this step first. A prior use of `cargo test` can keep a GPUI test-support binary at
   `target/debug/v4vmm`. Each next step happens in the open app.
2. Open Settings. Make sure the MusicIndex endpoint field holds a value, and the service
   answers.

**V1 - the header hides a copied feed identity, and a feed section names the feed**

3. Open Music. Use toolbar search to find an Index track. Open its track page.
4. Open the metadata panel for this track. Find the "Website" row and the "Nostr handle" row.
5. Pick a track where each row is blank. This track has no website and no Nostr key of its own.
6. Look at the track header, above the metadata panel.
   - This result is incorrect: the header shows a website link or a Nostr identity action.
7. Look below the header for a labeled section that names a feed as its owner.
   - This result is incorrect: no such section shows, though the feed has a website or a Nostr
     key.
   - This result is incorrect: the section names no feed title as its label.
   - This result is incorrect: an identity action in this section does not name the feed in its
     own text.
8. Repeat steps 3 to 7 in Dark theme (Settings > Appearance).

**V2 - the track's own identity stays apart from the feed's**

9. Open the metadata panel of an Index track. Find a track where the "Website" row has a value,
   the "Nostr handle" row has a value, or each row has a value.
10. Look at the track header. It shows each value found in step 9, as a website link, a Nostr
    identity action, or the two together.
11. Look at the feed identity section below the header.
    - This result is incorrect: the feed section repeats the track's own website or Nostr key.
    - This result is incorrect: the feed section is missing, though the feed has its own website
      or Nostr key that differs from the track's.

**V3 - a missing track description shows no feed description**

12. Open the metadata panel of a track. Find the "Description" row. Note if it holds a
    value.
13. Look at the track page's description panel.
    - This result is incorrect: the metadata panel's "Description" row is blank, and the page
      shows a description text.
    - This result is correct: the "Description" row is blank, and the page shows no description
      panel.

**V4 - layout at the usual and the narrow width**

14. At the usual window width, look at the header, the feed identity section, and the
    description panel. Each one shows in its own position, with no clipped text.
15. Narrow the window until the Library sidebar collapses, or resize below the narrow-layout
    width named in the toolbar runbook. Repeat step 14.
16. Repeat steps 14 and 15 in Dark theme.

**Cleanup**

17. Close the app window, or press `Ctrl+C` in the terminal. This check reads existing Index
    data. It writes no configuration and no stored file. No fixture and no generated file needs
    removal.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
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

## Implementation Result - 2026-09-28

### 1. Files Changed

`src/api.rs`: `track_with_feed_defaults` no longer copies a feed's `source_links`,
`source_ids` or `description` into a track. It copies each other value as before. A new
test, `adr_0075_track_header_r22_01_stops_identity_and_description_copies`, proves the stop
and the seven surviving copies. An older test is renamed to
`track_with_feed_defaults_inherits_missing_feed_level_metadata` and no longer checks the
three stopped fields.

`src/view_models/track_detail.rs`: `TrackDetailVm` gets a feed field, the
`with_feed_identity` method, the `feed_identity_section` method and the
`feed_identity_action_prefix` method. A new `FeedIdentitySectionVm` type carries the owner
label and the actions. `TrackDetailPageVm` exposes the same two new methods. Six new tests
carry the `adr_0075_track_header_` prefix, for R22-02 to R22-05.

`src/view_models/entity_detail.rs`: `EntityActionVm` gets a private `identity_a11y_label`
field and the `with_identity_a11y_label` method. `identity_display` uses this text when the
caller supplies it, and keeps `IdentityActionDisplayKind::default_a11y_label` when not. This
lets the feed identity section's screen-reader text name the feed, apart from the track's
own header actions, which keep the shared default text.

`src/ui/composites/track_detail_surface.rs`: the labeled box of `render_text_section` moves
into a new private helper, `render_labeled_box`. `render_text_section` calls it with no
visual change. A new `render_feed_identity_panel` function calls the same helper for the
feed identity panel, with the scaled tokens ADR 0039 requires. `src/ui/composites/mod.rs`
exposes `render_feed_identity_panel` at crate scope only, since its `String` parameters keep
it apart from the composite module's public display-contract surface (ADR 0038).

`src/ui/shells/track.rs`: adds `render_track_feed_identity_section`, which reads
`TrackDetailPageVm` data and calls `render_feed_identity_panel`. This function decides no
label, no owner name and no box style of its own. Each fact comes from the view model or the
shared composite.

`src/ui/shells/discover/track_inspector.rs`: passes `track_context.feed` into
`with_feed_identity`. It passes its own `cx` into `render_track_feed_identity_section`, and
it lists the feed identity section first in the page's list of section elements.

`src/ui/shells/library/track_detail.rs`: `render_library_track_detail_core` gets a `feed`
parameter. It passes the parameter into `with_feed_identity`, and it passes its own `cx`
into `render_track_feed_identity_section`. It adds the feed identity section to the page's
section elements.

`src/feed_service.rs`: corrects one existing test,
`library_track_context_inherits_feed_level_musicindex_metadata`. It checks that the track
keeps no copied feed identity and no copied feed description. It checks that `context.feed`
keeps the feed's own values.

`src/metadata_service.rs`: adds a test module with the two R22-06 tests and their fixed
edit lists.

### 2. Tests Run

Each command ran at the repository root, after the fix named in "Deviations From Task"
below.

- `cargo test --lib adr_0075_track_header`: 9 passed. None showed an error.
- `cargo test`: 1857 lib tests, 283 architecture tests, and 10 doc tests. All lib and
  architecture tests passed. The doc tests are ignored by design. No test showed an error.
- `cargo test --test architecture_tests`: 283 passed. None showed an error.
- `cargo fmt -- --check`: Green.
- `cargo clippy -- -D warnings`: Green.
- `cargo check --all-targets`: Green. No warning appeared.
- `cargo build --bin v4vmm`: Green.

### 3. Behavior Changed

A track page's header shows only the track's own website and its own Nostr key. A track
with none of its own shows no identity action in its header. This result corrects the
Decision B defect named in "Recorded Facts". The Index route no longer shows a feed's
website or Nostr key as though the track asserted it.

A new labeled section below the header names the feed as the owner of the feed's own website
and Nostr key. Each identity action in this section names the feed in its own label and its
own screen-reader text.

A track page's description panel shows only the track's own description. A track with no
description of its own shows no description panel. This result holds when its feed has a
description of its own.

The tag output does not change. A tag row that read a copied feed value reads the feed in
the track's context directly. R22-06 holds two fixed edit lists as proof of this equal
result. One list is for a track with none of its own values. The other list is for a track
with its own values.

The feed identity panel's box, text dimension and space between its actions come from the
same scaled tokens as the description panel. The panel changes its dimension with the
operator's dynamic type adjustment (ADR 0039). The first version kept one fixed dimension.

### 4. Deviations From Task

The packet requires three changes: the stop of the three copies, the equal tag output, and
the feed identity section apart from the track's header. Each one is complete. The packet
names `with_feed_identity` as an example builder method name. The code uses this same name.

The orchestrator's inspection of the first version found one defect. That version built the
feed identity box in the screen, `src/ui/shells/track.rs`, with fixed tokens apart from the
scaled tokens ADR 0039 needs. The shared labeled box existed in
`src/ui/composites/track_detail_surface.rs`, in `render_text_section`, with the scaled
tokens.

The fix moves `render_text_section`'s box into a new helper, `render_labeled_box`, in the
same composite file. `render_text_section` calls it with no visual change. A new function,
`render_feed_identity_panel`, calls the same helper for the feed identity panel. The screen
composes this panel from the shared composite. It builds no box, and it picks no token, of
its own.

`render_feed_identity_panel` takes `String` parameters. A test named
`composite_signatures_take_display_contracts_not_loose_strings` (ADR 0038) marks a `pub fn`
with this shape as a defect, apart from a named allowed condition. The fix keeps this
function at crate scope, `pub(crate)`, rather than adding a named allowed condition.
`src/ui/composites/settings.rs` holds an existing `pub(crate)` function with the same shape,
`settings_cached_row`.

### 5. Unresolved Concerns

The orchestrator checked the Library refresh path on 2026-09-28. It keeps no copied feed
identity after this packet.

Two identity rows in the metadata panel, "Nostr handle" and "RSS feed nostr handle", map to
one ID3 frame, `TXXX:RSS Nostr Handle`. When a track has its own Nostr key apart from its
feed's, only one of the two values reaches the written tag. This condition exists in
`track_metadata_rows` apart from this packet's change. The packet's Exclusions name the
proposed tag mapping ADR as its owner. Proposed ADR 0080 covers this class of defect.

No open Stop-and-report condition applies. Each caller of `track_with_feed_defaults` was
checked. No caller needed a copied identity or description apart from a tag row.

## Orchestrator Review - 2026-09-28

The orchestrator reviewed the diff and ran each check. Each check is Green: 1,857 unit tests, 283 guards, and no warning.

- R22-06: the orchestrator ran the two tests on commit `d04f23d`, before this packet. Both passed with the same fixed edits. Thus the tag output is equal.
- The first version built the feed identity box in a screen with fixed tokens. The orchestrator sent it back. The box now uses the shared labeled box in `src/ui/composites/track_detail_surface.rs`, with the scaled tokens of ADR 0039.
- The owner label is the feed title. The visual check decides if the title alone names the feed clearly.

The R22-06 fixed edits record two defects of the present tag output. ADR 0080 owns both:

- A track with its own Nostr key gets the feed key in `TXXX:RSS Nostr Handle`. The track key is lost.
- `WOAR` holds link label text before the URL, for example `download for free (url, forward): https://example.test/feed`.
