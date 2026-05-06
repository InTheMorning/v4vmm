# ADR 0036 Review Checklist

## Reviewed Artifacts

- `docs/adr/0036-feed-visual-and-provenance-surface-consistency.md`
- `docs/plans/adr-0036-feed-visual-and-provenance-consistency-phase-plan.md`
- `docs/tasks/adr-0036-task-001-feed-surface-typed-slots.md`
- `docs/tasks/adr-0036-task-002-visual-system-enforcement.md`
- `docs/tasks/adr-0036-task-003-advanced-provenance-panel-consistency.md`

## Gate Status

Status: Task 001, Task 002, and Task 003 complete.

User-provided screenshots on 2026-05-02 cover Library feed detail with an open
playlist popover, Discover feed detail with an open playlist popover, Library
track detail, Discover track detail, and Library advanced compare/provenance
panels.

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
| Task 001 feed surface typed slots | Complete | Typed release surface slots, VM-consumption guard, full checks | Full checks green; screenshots received 2026-05-02 |
| Task 002 visual system enforcement | Complete | Token/primitive ownership of repeated visual decisions | Playlist popover rows use leading alignment and shared padding; release detail spacing uses scale-aware tokens; full checks green; screenshots received 2026-05-02 |
| Task 003 advanced provenance panels | Complete | Shared advanced panel grammar and guards | Library advanced compare cells now use `TrackMetadataGrid` child composites; architecture guard added; full checks green; screenshot received 2026-05-02 |

## Visual Smoke

- Library feed detail: passed with user screenshot on 2026-05-02.
- Discover feed detail: passed with user screenshot on 2026-05-02.
- Library normal track detail: passed with user screenshot on 2026-05-02.
- Discover normal track detail: passed with user screenshot on 2026-05-02.
- Playlist popovers: passed with user screenshots on 2026-05-02.
- Advanced metadata panels: passed with user screenshot on 2026-05-02.

## Automated Checks

- `cargo fmt -- --check`: green
- `cargo check`: green
- `cargo test`: green
- `cargo clippy -- -D warnings`: green
- `git diff --check`: green

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
