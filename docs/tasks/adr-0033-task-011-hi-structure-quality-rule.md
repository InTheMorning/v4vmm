# ADR 0033 Task 011: HI Structure Quality Rule

## Goal

Codify the rule that no UI fix is worth landing unless it improves or
preserves the app's human-interface structure.

## Context

The recent-feed tile regression and playlist popover regressions showed that
local visual patches can make one screenshot look better while leaving the
underlying UI architecture weak. ADR 0033 already defines backend/UI ownership
and HIG-style shared chrome boundaries; this task makes the quality bar
explicit for all future UI work. `AGENTS.md` is an ignored local workspace rule
file in this repository, so ADR 0033 is the committed durable source. When an
`AGENTS.md` file is present, mirror the rule there as local operating guidance.

## Files to Inspect

- `AGENTS.md` (ignored local workspace rule file, when present)
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `tests/architecture_tests.rs`

## Files Likely to Change

- `AGENTS.md` (local mirror only unless repository policy changes)
- `docs/adr/0033-hig-ui-architecture-governance.md`
- `docs/tasks/adr-0033-task-011-hi-structure-quality-rule.md`
- `docs/reviews/adr-0033-task-011-review.md`

## Do Not Touch

- Runtime UI code.
- Backend, database, API, ingest, or metadata behavior.
- Existing architecture-test implementation unless the ADR lists stale test
  names.

## Constraints

- Keep documentation under `docs/` except for canonical top-level
  `AGENTS.md`.
- Treat Apple HIG as a structural guidance source: hierarchy, predictability,
  compact transient popovers, consistent controls, adaptive layout, and clear
  action roles.
- Do not codify vague taste preferences. The rule must point to concrete
  ownership boundaries, view-model contracts, tokens/components, tests, and
  visual proof.

## Implementation Steps

1. Add or update local `AGENTS.md` guidance requiring UI fixes to strengthen
   human-interface structure before they are considered worthwhile.
2. Update ADR 0033 with a `Human-interface structure bar` section.
3. Update ADR 0033's enforcing-test list so it includes the current playlist
   create-mode, value-route fallback, and shared-shell boundary guards.
4. Add invariants that reject one-off symptom patches and require visual
   evidence or an explicit residual-risk note for user-visible presentation
   fixes.
5. Record this task and review under `docs/tasks/` and `docs/reviews/`.

## Acceptance Criteria

- Future agents can find the committed rule in ADR 0033 without reading chat
  history.
- When local `AGENTS.md` guidance is present, it mirrors the ADR 0033 rule.
- ADR 0033 explicitly rejects "looks fixed" UI patches that do not improve
  hierarchy, shared UI ownership, view-model projection, token/component
  discipline, or regression protection.
- ADR 0033 lists the architecture tests that currently enforce the related
  boundaries.
- The change is docs-only and passes the required repository checks.

## Test Commands

```bash
cargo fmt -- --check
cargo check
cargo test --test architecture_tests
git diff --check
```

## Expected Final Report Format

- Files updated and created.
- Verification commands run.
- Commit hash.

## Escalation Triggers

- If enforcing this rule requires new runtime UI architecture tests, create a
  follow-up task instead of mixing implementation into this documentation
  change.
