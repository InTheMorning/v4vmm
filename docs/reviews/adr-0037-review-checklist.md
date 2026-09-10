# ADR 0037 Review Checklist

## Gate Status

Open - reconciled 2026-09-10. Task 001 feed-identity hydration and task 002
track-detail parity retain operator checks. Earlier mechanical evidence does
not close the visual gaps.

## Requirement Disposition

| Earlier requirement | Disposition and owner |
|---|---|
| Separate Library and Discover screens and four specifically named screen screenshots | Retired by ADRs 0047/0048/0060. Compare local and Index origins inside Music, in both themes; record entity identity and entry route |
| src/ui_entity.rs, src/ui_track.rs, src/search.rs, and renderer-supplied identity prefixes | Retired by ADR 0038 helper/display migration and ADR 0047 screen retirement. Current helpers are in src/ui/shells/entity.rs and src/ui/shells/track.rs |
| Website/Nostr/RSS payloads, shared identity controls, missing-local-fact regression | Retained on current local and Index paths |
| Track header/action/section parity | Retained for the same source facts; origin differences must not become different shared layouts |
| Library-only advanced panels and Discover-specific navigation controls | Replaced by ADR 0047's download-dependent disclosure and frame navigation; do not demand retired screen-local controls |
| Light/dark proof using populated identity fixtures | Retained; absent source facts do not prove hydration failure or success |

## Mechanical Ownership

Existing guards in tests/architecture_tests.rs:

- release_feed_identity_actions_use_shared_renderer
- track_identity_links_use_shared_renderer

Identity payload tests remain in src/view_models/entity_detail.rs and
src/view_models/track_detail.rs. Local paths are
src/ui/shells/library/feed_detail.rs and track_detail.rs; Index details use
src/ui/shells/search_results_inspector.rs and the same shared helpers.

## Operator Visual Check

Follow [Identity And Detail Parity](../runbooks/inherited-ui-checks.md#identity-and-detail-parity--adr-0037-tasks-001-and-002).

| Task | Light | Dark | Required fixture |
|---|---|---|---|
| 001: local/Index feed identity and hydration | Open | Open | Same feed, known Website/Nostr/RSS facts |
| 002: local/Index track detail parity | Open | Open | Same track, known Website/Nostr facts and a downloaded local copy |

Capture each entry route, not just one shared shell. Check link/copy targets
and contextual disclosure without requiring different source claims to match.

## Evidence

On 2026-05-02, screenshots of Way to Go and The Heycitizen Experience showed
Index identity controls missing from the local route. The follow-up hydration
fix was recorded, but its visual recheck was not. The 2026-05-03 MoeFactz
attempt had no stored track identity facts, so it did not prove task 002.
These observations document the original gap, not a current reproduced failure.

The original task reviews retain dated implementation evidence. Their retired
screen instructions are replaced by this checklist and the current runbook.

## Merge Recommendation

Keep ADR 0037 Accepted. Close tasks 001 and 002 separately when their populated
fixtures pass in both themes; reconcile the ADR, delivery, and pending checks.
