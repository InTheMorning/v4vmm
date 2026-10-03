# ADR 0076 Task 009: A Download Writes The Stored Values

Status: Ready - 2026-10-03. It runs after [ADR 0066 task 014](adr-0066-task-014-download-failures-and-dismissal.md). Implementation has not started.
Its operator check opens when the implementation is complete.

## Goal

A fresh download writes the tags of the stored values of its track. The next "Update n file(s)" scan shows no difference for that file.

## Authority

- [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) Decision 10, amended 2026-10-03: a download writes the stored values, after the RSS update of its feed, with the projection of the scan.
- ADR 0076 Decisions 8 and 9.
- [ADR 0080](../adr/0080-tag-frames-follow-their-owner.md): the channel website (RSS channel `<link>`) goes to `WOAR`.

## Recorded Facts - 2026-10-03

- The operator downloaded the HeyCitizen feed again. The "Update 19 files" popup then listed "MoeFactz", "Milves in Teslas", "The Platform" and other files, each with one line:
  `WOAR: https://v4vmusic.com/?publisher=cmne6j9cn9h2bod0i6hjev8uh (file: no value)`.
- The live RSS channel `<link>` holds that URL. MusicIndex records it as `rss_link`, `feed.link`. The stored value is correct.
- `subscribe_feed_retaining` in `src/subscribe_service.rs` builds `TrackContext::new(track, Some(feed.clone()))` from the request feed, then `id3_edits_for_track_context`. It does this before the RSS upsert of each track.
- The request feed has no `source_links`. `api_feed_from_view` in `src/app/search_dispatch.rs` and `api_feed_from_album` in `src/library/app_impl.rs` fill it with `..Default`. Thus the download writes no `WOAR`.
- `subscribe_track_from_search_internal` builds a `refreshed_context`, but writes the earlier `edits`.
- `Materialization::run` in `src/subscribe_service/materialization.rs` writes `self.edits`. Only the route frame comes from the stored route (`with_stored_route_frame`, Decision 9).
- The scan builds its expected edits with `track_row_to_track_context_with_local_identity` and `id3_edits_for_track_context` in `src/application/queries/tag_update.rs`.
- The channel link has two stored copies: `feeds.link` and the feed `rss` row in `entity_identity_links`. `src/rss/subscribe.rs` writes both. The RSS check in `src/rss/check_apply.rs` updates only `feeds.link`. The `WOAR` projection reads only the identity link.

## Required Changes

1. `Materialization::run` builds the edits that it writes after `validate_subject`, from the stored row: `track_row_to_track_context_with_local_identity` and `id3_edits_for_track_context`. The route frame keeps the stored route of Decision 9.
2. The live request context only finds and fetches the enclosure. Delete each early edit construction, edit field and route patch step that the stored edits make unnecessary.
3. The retained-conversion retry path writes the same stored edits. Keep the ADR 0066 task 014 behavior.
4. When the RSS check applies a channel link, it also replaces the feed `rss` website row in `entity_identity_links`. Both copies then hold the same value.

## Mechanical Acceptance Criteria

Use the prefix `adr_0076_download_stored_values_` for tests beside the owning code.

| Case | Required proof |
|---|---|
| R76-9-01 | A download from a request feed with no `source_links`, for a feed whose RSS has a channel `<link>`, writes `WOAR` with that link |
| R76-9-02 | After the download of R76-9-01, the scan gives no changed frame for that file |
| R76-9-03 | The edits that the download writes equal the edits that the scan expects for that track |
| R76-9-04 | A retained conversion retry writes the same stored edits |
| R76-9-05 | An RSS check that changes the channel link updates `feeds.link` and the feed `rss` website identity row to the same value |

## Visual Acceptance Criteria

None. The operator check reads the button count after a download.

## Exclusions

- No change to the tag scan, the popup, or the frames that the app writes.
- No change to the failure classes of ADR 0066 task 014.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/subscribe_service.rs`, `src/subscribe_service/materialization.rs`.
- `src/feed_service.rs`: `track_row_to_track_context_with_local_identity`, `hydrate_feed_identity`.
- `src/metadata_service.rs`: `id3_edits_for_track_context`.
- `src/application/queries/tag_update.rs`: the scan projection.
- `src/rss/subscribe.rs` and `src/rss/check_apply.rs`: the two writers of the channel link.
- `src/db/payment_routes.rs`: the stored route of Decision 9.

## Checks

```bash
cargo test --lib adr_0076_download_stored_values_
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

The implementer writes this section at completion. It downloads one album into an isolated fixture with its own configuration, database and music folder, and reads the "Update n files" count.
The check never writes the real Library. Do not delete `/tmp/v4vmm-governance.ie6k8TQf`.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0076-task-009-download-writes-stored-values.md`
- ADR 0076 Decisions 8, 9 and 10
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": a download writes the stored values, and the RSS check keeps both copies of the channel link equal.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Tests use a local `TcpListener` and a parsed RSS fixture. No test sends a request to a remote host.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The tag scan, the popup and `src/ui/`.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R76-9-01 to R76-9-05 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The stored row does not exist yet at the point where the download must build its edits.
- A stored projection omits a value that the live download writes today and that the operator can lose. Name each such frame.
- A change needs a file in "Do not touch".
