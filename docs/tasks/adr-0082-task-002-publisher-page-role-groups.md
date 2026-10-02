# ADR 0082 Task 002: Publisher Page Role Groups

Status: Ready - 2026-10-02. It runs after [packet 001](adr-0082-task-001-contract-0-7-0-and-link-facts.md). Implementation has not started.
Its visual gate opens when the implementation is complete. Visual checks are paused, so the gate stays open.

## Goal

A publisher page shows no type. Its albums group by how they name the feed, then by the role of their link. Each row shows its role and its source.
The header lists the agreed roles, and a section shows the feeds that share albums with this feed.

## Authority

- [ADR 0082](../adr/0082-publisher-roles-belong-to-each-album-link.md) Decisions 1 to 5. It supersedes [ADR 0078](../adr/0078-publisher-page-type-from-stated-role.md).
- [ADR 0077](../adr/0077-publisher-feed-artist-binding.md) Decisions 1 and 3, and its packets 003, 004 and 007.
- The durable set in [AGENTS.md](../../AGENTS.md): renderer portability, element hierarchy, typed action state and token discipline.

## Recorded Facts - 2026-10-02

- `PublisherPageVm` in `src/view_models/publisher_page.rs` exposes `page_type`, `header_facts` (with `TYPE_LABEL`), `owned_albums`, `listed_by_albums`, `library_albums`, `other_albums`, `other_albums_status`, and the confirmed and unconfirmed artist counts.
- `owned_albums` keeps albums with `music_names_publisher` true. `listed_by_albums` keeps the others.
- `AlbumRoleDisplay` gives `Stated`, `Assumed`, `Conflict` and `Unknown`. Packet 001 handles `Assumed`.
- 13 source comments in `src/` cite ADR 0078. No guard cites it.
- The screen is `src/ui/shells/publisher.rs`. The Index page and the Library page use the same view model, with a different scope.
- Packet 001 decodes and stores `album_names_as`, `role_agreement` and `co_credited_feeds`.

## Required Changes

### 1. No Type

- Delete `page_type`, `PublisherPageType`, `TYPE_LABEL` and the "Type" header fact. Delete each test that tests only the type. Rewrite each comment that cites ADR 0078 to cite ADR 0082, or delete it.

### 2. Groups And Subgroups

- The view model exposes the groups of ADR 0082 Decision 2: "Publisher of" (`album_names_as` is `publisher`) and "Listed, not named" (null), in that order.
- Inside each group, it exposes the role subgroups: one for each agreed role set (`role_agreement` `both`), then "Role stated by one side", "Roles differ, not confirmed" and "Role not stated".
- An album appears one time. An empty group or subgroup is absent.
- On the Library page, keep the present Library scope (`library_albums` and `other_albums`). Apply the groups inside each scope, and record the shape that you chose.
- Replace `owned_albums` and `listed_by_albums` where the groups make them unnecessary. Keep the "In Library" and "Not listed by the publisher" marks.

### 3. Row Role Text

- Each row exposes the role text and the source text of ADR 0082 Decision 3: "agreed by both feeds", "stated by the album only", "stated by the publisher only", the two sets with "not confirmed" for a conflict, and "Role not stated".
- The view model owns each label as a named constant.

### 4. Header Roles

- The header exposes "Roles:" with the different roles of the agreed rows, split from each role set, in sorted order. A page with no agreed row exposes no role fact.
- The confirmed and unconfirmed artist facts of packet 007 stay.

### 5. Shares Albums With

- The view model exposes one entry for each `co_credited_feeds` item, with its title, its roles and its album count.
- Each entry has a typed action that opens the publisher page of its `feed_guid` (`ArtistRef::PublisherFeed`).
- The section is absent when the list is empty.

### 6. Archive ADR 0078

- The orchestrator moves ADR 0078 to `docs/adr/archive/` in the commit of this packet. The implementer does not edit an ADR.

## Mechanical Acceptance Criteria

Use the prefix `adr_0082_role_groups_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R82-2-01 | The view model exposes no page type, and the header exposes no "Type" fact |
| R82-2-02 | Albums with `album_names_as` `publisher` and null go to "Publisher of" and "Listed, not named" |
| R82-2-03 | Rows with `role_agreement` `both` and role sets "artist" and "distributor" give two subgroups. Rows with `one_side`, `conflict` and null give the other three subgroups |
| R82-2-04 | Each album appears one time, and an empty subgroup is absent |
| R82-2-05 | Each `role_agreement` value gives the role text and source text of ADR 0082 Decision 3 |
| R82-2-06 | The header exposes "Roles: artist, distributor" from two agreed rows, and no role fact without an agreed row |
| R82-2-07 | A `co_credited_feeds` entry gives a row with title, roles and count, and an action that opens its publisher page |
| R82-2-08 | No source comment in `src/` cites ADR 0078 |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: the DETOX publisher page shows "Publisher of", with its albums under "Role not stated", and no type.
- V2: a publisher with agreed roles shows each role subgroup and the header roles.
- V3: "Shares albums with" shows when the list has entries, and each entry opens its publisher page.
- V4: normal and narrow widths, Light and Dark themes, and the larger type sizes show each element in its place, with no clipped text.

## Exclusions

- No change to the album page. Packet 003 owns it.
- No new request. The facts come from the present publisher request.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/view_models/publisher_page.rs`, `src/ui/shells/publisher.rs`, `src/app/publisher_dispatch.rs`.
- `src/application/queries/feed.rs` and `src/application/queries/library.rs`: the page facts.
- `tests/architecture_tests.rs`: the ADR 0077 publisher page guards.

## Checks

```bash
cargo test --lib adr_0082_role_groups_
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
- This packet: `docs/tasks/adr-0082-task-002-publisher-page-role-groups.md`
- ADR 0082, and the ADR 0077 packet 004 and 007 documents
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes" 1 to 5: no type, the groups and subgroups, the row role text, the header roles and "Shares albums with".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- The view model gives each label, group, text and action availability. The screen only composes. Use the scaled tokens of ADR 0039 and existing shared composites.
- Tests use recorded JSON. No test sends a request.
- Never run `git checkout`, `git restore`, `git stash`, `git reset` or `git commit`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The album page.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R82-2-01 to R82-2-08 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V4.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- Packet 001 is not complete.
- The Library scope and the groups cannot combine without a new shared composite.
- A change needs a file in "Do not touch".
