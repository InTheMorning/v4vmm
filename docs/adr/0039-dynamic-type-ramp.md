# ADR 0039: Dynamic Type Ramp

## Status

Proposed - 2026-05-04.

Unscheduled. Text-scale tiers, maximum scale, wrapping, and truncation policy
remain unspecified. No implementation or operator acceptance is claimed.

Reconciled 2026-09-10: the operator retained this proposal without scheduling
it. The text-scaling requirement has not been withdrawn; age is not a reason
to archive it. A defined policy must precede a phase plan and task packets.

## Context

ADR 0038 Task 005 completed accessibility-label contracts for interactive
composites. Text scale behavior remains a separate presentation contract:
the app has scale-aware tokens, but it does not yet define a dynamic-type
ramp, maximum text scale, wrapping policy, or truncation policy for compact
desktop surfaces.

## Decision

Defer the dynamic-type ramp to this child ADR. The ADR will define how text
tokens scale across compact rows, detail pages, sidebars, popovers, and the
now-playing bar without breaking the presentation contracts enforced by
ADR 0038.

## Follow-Up Work

- Define text-scale tiers and max scale behavior.
- Decide where wrapping is allowed and where truncation remains required.
- Add focused architecture and visual-smoke checks for the selected policy.
