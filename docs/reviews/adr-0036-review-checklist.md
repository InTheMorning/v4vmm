# ADR 0036 Review Checklist

## Reviewed Artifacts

- `docs/adr/0036-feed-visual-and-provenance-surface-consistency.md`
- `docs/plans/adr-0036-feed-visual-and-provenance-consistency-phase-plan.md`
- `docs/tasks/adr-0036-task-001-feed-surface-typed-slots.md`
- `docs/tasks/adr-0036-task-002-visual-system-enforcement.md`
- `docs/tasks/adr-0036-task-003-advanced-provenance-panel-consistency.md`

## Gate Status

Status: Task 001 code complete; visual smoke pending. Tasks 002 and 003 are
blocked until Task 001 is visually checked.

## Structural Review Questions

- Do Library and Discover feed detail still route through
  `render_release_detail_shell`?
- Does the shell consume `ReleaseDetailPageVm` from `ReleaseDetailVm`?
- Are release surface behavior slots typed instead of free-form shared
  `AnyElement` APIs?
- Are command handlers and image resolution still screen-owned?
- Did the task avoid visual pixel tuning outside shared tokens/primitives?
- Did the task add or strengthen architecture tests?
- Did visual smoke use user-provided screenshots?

## Task Results

| Task | Status | Required Evidence | Notes |
|---|---|---|---|
| Task 001 feed surface typed slots | Code complete; visual pending | Typed release surface slots, VM-consumption guard, full checks | `cargo fmt -- --check`, `cargo check`, `cargo test`, `cargo clippy -- -D warnings`, and `git diff --check` green |
| Task 002 visual system enforcement | Blocked | Token/primitive ownership of repeated visual decisions | Wait for Task 001 |
| Task 003 advanced provenance panels | Blocked | Shared advanced panel grammar and guards | Wait for Task 002 |

## Visual Smoke

- Library feed detail: pending.
- Discover feed detail: pending.
- Library normal track detail: covered by ADR 0035, retest after visual pass.
- Discover normal track detail: covered by ADR 0035, retest after visual pass.
- Playlist popovers: retest after visual pass.
- Advanced metadata panels: pending Task 003.

## Merge Recommendation Template

```text
Status: Pass / Fail

Required fixes:
- ...

Optional improvements:
- ...

Architectural drift:
- ...

Missing tests or visual proof:
- ...

Feature-readiness impact:
- Proceed / Proceed with constraints / Do not proceed
```
