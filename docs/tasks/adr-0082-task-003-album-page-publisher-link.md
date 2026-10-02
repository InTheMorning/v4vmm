# ADR 0082 Task 003: Album Page Publisher Link

Status: Ready - 2026-10-02. It runs after [packet 001](adr-0082-task-001-contract-0-7-0-and-link-facts.md). Implementation has not started.
Its visual gate opens when the implementation is complete. Visual checks are paused, so the gate stays open.

## Goal

An album page shows the publisher that the album names, with the role, the agreement and the source of that link.

## Authority

- [ADR 0082](../adr/0082-publisher-roles-belong-to-each-album-link.md) Decisions 3 and 6.
- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) Decision 2: the album binds to the publisher that it names. A role never makes the album page show the publisher as its artist.
- The durable set in [AGENTS.md](../../AGENTS.md): typed action state, element hierarchy and token discipline.

## Recorded Facts - 2026-10-02

- `ReleaseHeroVm` in `src/view_models/entity_detail.rs` gives an "open publisher" action (`EntityActionKind::OpenPublisher`) from `FeedView::publisher_feed_guid`. ADR 0077 packet 004 added it.
- `FeedView::publisher_feed_guid` comes from the `music_to_publisher` entry with `music_names_publisher` true, from the response or from `feed_publisher_relationships`.
- The album page shows "Feed owner" text from `publisher_text` (ADR 0077 packet 005). That text is not the publisher link.
- Packet 001 decodes and stores `album_names_as` and `role_agreement`, and accepts a null `role`.
- The publisher page of packet 002 owns the row role text of ADR 0082 Decision 3. This packet uses the same texts.

## Required Changes

1. The album view model exposes the publisher link of the album: the publisher feed title (or its GUID when it has no title), the role text and the source text of ADR 0082 Decision 3, and the "open publisher" action.
2. Share one owner for the role text and the source text with packet 002. When packet 002 is not complete, put the texts in one view-model function, and record it.
3. Index and Library album pages both show the link. The Library page reads the stored relationship. The Index page reads the response.
4. An album that names no publisher shows no publisher link. The view model invents no placeholder.
5. The publisher link never takes the place of the album artist.

## Mechanical Acceptance Criteria

Use the prefix `adr_0082_album_link_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R82-3-01 | An album that names a publisher with `role_agreement` `both` and role "label" exposes the title, "label" and "agreed by both feeds", and an enabled "open publisher" action |
| R82-3-02 | Each `role_agreement` value gives the texts of ADR 0082 Decision 3, from the same owner as the publisher page |
| R82-3-03 | A Library album reads `album_names_as` and `role_agreement` from the stored relationship |
| R82-3-04 | An album that names no publisher exposes no publisher link |
| R82-3-05 | The album artist does not change when the album names a publisher |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: an Index album page and a Library album page show the publisher with its role and source, and the action opens the publisher page.
- V2: an album that names no publisher shows no publisher link.
- V3: normal and narrow widths, Light and Dark themes, and the larger type sizes show each element in its place, with no clipped text.

## Exclusions

- No change to the publisher page. Packet 002 owns it.
- No new request, and no schema change.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/view_models/entity_detail.rs`, `src/views.rs` (`FeedView`), the album page screens under `src/ui/shells/`.
- `src/db/publisher_relationships.rs`: the stored relationship reader.
- `src/view_models/publisher_page.rs`: the role text owner after packet 002.

## Checks

```bash
cargo test --lib adr_0082_album_link_
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

The implementer writes this section at completion. It gives numbered steps for V1 to V3.
It states the needed state, what counts as wrong, and the cleanup. The check only reads pages.
Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a valid difference.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0082-task-003-album-page-publisher-link.md`
- ADR 0082 and ADR 0077 Decision 2
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the publisher link on the album page with its role, agreement and source.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives each label, text and action availability. The screen only composes. Use the scaled tokens of ADR 0039 and existing shared composites.
- Tests use recorded values. No test sends a request.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The publisher page.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R82-3-01 to R82-3-05 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V3.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- Packet 001 is not complete.
- The album page has no place for the publisher link in its present element hierarchy.
- A change needs a file in "Do not touch".
