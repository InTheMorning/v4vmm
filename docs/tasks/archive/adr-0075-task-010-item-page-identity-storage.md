# ADR 0075 Task 010: Store The Item Page Identity

Status: Complete - 2026-09-20. Technical review passed. Integrated checks and the normal binary build are Green.

Three focused storage tests and all six RSS subscription tests are Green.
The format check and `cargo check` are Green. No visual gate applies to this packet.

## Goal And Accepted Rule

Preserve a direct RSS item `link` as a track-owned `web_page` identity fact during subscription.
[ADR 0075](../../adr/0075-metadata-ownership-and-completeness.md), Decision 3, already requires this mapping.
This packet does not select between providers or choose an active page action.

## Files And Contract

Inspect `src/rss/subscribe.rs`, `src/db.rs::LocalIdentityLinkInput`, and the existing RSS identity tests.
Change `src/rss/subscribe.rs` and its unit tests only.
The item page retains `entity_type = track`, the item GUID, and extraction path `entity.link`.
Keep its supplied text in raw evidence before trimming the usable URL.
Retain unknown source observation time as `None`.

Keep the existing transcript fact and its own extraction path.
Use separate positions for page and transcript facts. Repeated imports must not duplicate the stored collection.
A missing page must not substitute the channel website or remove a supplied transcript fact.

The existing `rss_track_link_inputs` helper currently accepts only transcript data.
Extend that helper and both current call sites to include the item page.
Keep the plain `tracks.link` column and existing subscription behavior.
No database migration, network request, UI change, or upstream edit belongs here.

## Acceptance And Checks

- A test imports an item page and reads its owner, kind, raw evidence, and path from identity storage.
- A test preserves page and transcript facts together with separate positions.
- Tests cover missing and whitespace-only pages without feed website inheritance.
- A repeat import replaces RSS facts without duplicates or changes to MusicIndex facts.

```bash
cargo test --locked --offline --lib adr_0075_item_page
cargo test --locked --offline --lib rss::subscribe
cargo fmt -- --check
cargo check --locked --offline
```

The orchestrator runs integrated checks. Rollback removes only this additive item-page mapping and its tests.
Stop if the change requires schema, provider-selection, or presentation policy.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md`, ADR 0075, this packet, and the named RSS and storage files.
- The shared Rust and ASD-STE100 skills.

Goal:
- Preserve the direct item page as a track-owned identity fact.

Constraints:
- Keep page and transcript evidence separate. Preserve other working changes.
- Do not commit or run the app.

Do not touch:
- API DTOs, other RSS files, database schema, UI, or shared status documents.

Acceptance criteria:
- The storage-level tests above pass through existing database helpers.

Test commands:
- Run the commands in Acceptance And Checks.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Operator Visual Check

This packet changes stored evidence. It adds no presentation and closes no existing visual gate.
