# ADR 0082 Task 002: Publisher Page Role Groups

Status: Implemented - 2026-10-17. Cases R82-2-01 to R82-2-08 pass. Mechanical checks are Green.
Its visual gate is open for V3 in [pending human checks](../pending-human-checks.md). V1, V2 and V4 retired on 2026-10-18, because [ADR 0085](../adr/0085-artists-by-confirmation.md) replaces the groups and the row role text. ADR 0078 moved to the archive with this packet.

## Goal

A publisher page shows no type. Its albums group by how they name the feed, then by the role of their link. Each row shows its role and its source.
The header lists the agreed roles, and a section shows the feeds that share albums with this feed.

## Authority

- [ADR 0082](../adr/0082-publisher-roles-belong-to-each-album-link.md) Decisions 1 to 5. It supersedes [ADR 0078](../adr/archive/0078-publisher-page-type-from-stated-role.md).
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

These are for the operator. No test proves them. V1, V2 and V4 retired on 2026-10-18 by ADR 0085. V3 stays open.

- V1: the DETOX publisher page shows "Publisher of", with its albums under "Role not stated", and no type.
- V2: a publisher with agreed roles shows each role subgroup and the header roles.
- V3: "Shares albums with" shows when the list has entries, and each entry opens its publisher page.
- V4: normal and narrow widths, Light and Dark themes, and the larger type sizes show each element in its place, with no clipped text.

## Exclusions

- No change to the album page. Packet 003 owns it.
- No new request. The facts come from the present publisher request.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../architecture/source-map.md).
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

The check only reads pages. It needs network access to `api.musicindex.org`.
Do not erase `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a correct difference.

### Setup

1. Operate `cargo build --bin v4vmm` in a desktop session.
2. A prior `cargo test` run can leave a GPUI test-support binary at `target/debug/v4vmm`.
   Build again before you start the app.
3. Start `target/debug/v4vmm`.
4. Open Settings. Make sure the MusicIndex endpoint field holds a working endpoint.
5. Open the Music section.

### V1: the DETOX publisher page shows "Publisher of" and "Role not stated", and no type

1. Open the publisher page for the "Official DETOX Music" feed
   (`137aaa9c-75ff-4916-9f23-e02968b2d15e`), from the Index or from the Library.
2. Check that the page shows a group titled "Publisher of".
3. Check that its albums sit below a subgroup titled "Role not stated".
4. Check that the page shows no "Type" row.
5. Check that the page shows no word "Artist" or "Label" as a page type.
6. Wrong: the page shows a "Type" row.
7. Wrong: the "Publisher of" group is missing.
8. Wrong: an album sits below an agreed role subgroup. It must sit below "Role not stated".
9. Cleanup: none.

### V2: a publisher with agreed roles shows each role subgroup and the header roles

1. Find a publisher feed with one agreed role or more.
2. Check the Recorded Facts of this packet and of
   [packet 001](archive/adr-0082-task-001-contract-0-7-0-and-link-facts.md) first.
   The DETOX feed gave only null `role_agreement` values on 2026-10-02.
3. Check [the open Stophammer requests](../plans/v4vmm-open-requests.md) and
   [the publisher feed research notes](../notes/2026-09-23-publisher-feed-artist-research.md)
   for a newer candidate.
4. When those sources name no candidate, search a few Index publisher feeds.
   A feed with a two-way validated link and a stated role on both sides is a correct candidate.
5. Open that publisher's page.
6. Check that the header shows a "Roles:" row.
7. Check that the row names each different agreed role, sorted, for example
   "Roles: artist, distributor".
8. Check that the page shows one subgroup for each agreed role set.
9. Check that an album below an agreed subgroup shows its role with the text
   "agreed by both feeds".
10. When the feed has a one-side row, check its text.
11. The text must read "stated by the album only" or "stated by the publisher only".
    The named side must match the side that stated the role.
12. When the feed has a conflict row, check its text.
13. The text must name both stated sets, and must say the link is "not confirmed".
14. Wrong: the header shows no "Roles:" row, although an agreed album exists.
15. Wrong: two different agreed role sets share one subgroup.
16. Wrong: a one-side row or a conflict row reads the same as an agreed row.
17. Cleanup: none.

### V3: "Shares albums with" shows when the list has entries, and each entry opens its publisher page

1. Open a publisher page whose feed has one co-credited feed or more.
2. Check the sources in V2, step 1, for a candidate.
   A feed with several shared albums, under more than one publisher, is a correct candidate.
3. Check that the page shows a "Shares albums with" section, below the album groups.
4. Check that the section shows one row for each co-credited feed.
5. Check that each row names its title, its roles and its shared album count.
6. Click one row.
7. Check that the app opens that feed's own publisher page.
8. Open a publisher page whose feed has no co-credited feed.
9. Check that the page shows no "Shares albums with" section at all.
10. Wrong: the section shows with an empty list.
11. Wrong: a row shows no title, no roles or no count.
12. Wrong: a click opens the wrong page, or does nothing.
13. Cleanup: use Back, or the breadcrumb, to go to the page you started from.

### V4: normal and narrow widths, Light and Dark themes, and larger type sizes show every element in its place

1. Open the publisher page from V1 or V2.
2. Resize the window to its normal width, then to a narrow width.
3. Pull the window edge until the Library sidebar collapses. Or, resize below the
   narrow-layout width named in the sidebar and toolbar runbooks.
4. Repeat step 2 in Light theme, then in Dark theme (Settings > Appearance).
5. Repeat step 2 at a larger system type size.
6. At each width, theme and type size, check that each element stays in its own place, with
   no clipped text.
7. Check the group captions, the role subgroup captions and the row role text.
8. Check the header "Roles:" row and the "Shares albums with" entries.
9. On the Library page, check the scope heading ("Library Albums" or "Other Albums").
   It must be visibly stronger than the group heading ("Publisher of" or "Listed, not
   named") below it.
10. Check that the group heading is visibly stronger than the role subgroup caption below
    it.
11. Wrong: a caption or a row role text is cut off mid-word, with no visual sign.
12. Wrong: two elements overlap.
13. Wrong: a role or a confirmation condition reads only through color, with no text
    difference.
14. Wrong: the scope heading, the group heading and the subgroup caption look the same
    strength.
15. Cleanup: restore the window to its earlier size and theme.

### Operator correction - 2026-10-17

The operator walked V2 on the HeyCitizen publisher page, Library scope, and found three
defects. Each one is now fixed.

- The empty "Other Albums" heading is gone. The view model gives the screen only the
  non-empty scopes, through a new `library_scopes()` method.
- The scope heading, the group heading and the role subgroup caption now show a visible
  level difference. `SectionHeader` gained a typed `SectionHeaderLevel`, and the Library
  scope headings use its stronger `Container` level.
- The row role text no longer uses the accent tone of a clickable item. A stated role now
  shows in a plain secondary tone.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
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
