# ADR 0073: Show Card Overflow Scrolling

## Status

Implemented - 2026-09-16.

The operator requested this correction during ADR 0066 task 007 acceptance,
after accepting the narrow Library and bounded recovery-notice corrections.
Implementation and the focused visual gate belong to
[task 007](../tasks/adr-0066-task-007-optional-tool-correction-and-retry.md).
The shared-viewport correction and mechanical checks are complete. The operator
accepted the focused Show visual check: all cards remain reachable with logs
closed and open, the transport stays fixed, and the requested theme/scale checks
pass. Fixture preservation is accepted: configuration bytes are unchanged, the
library and music are intact, and no probes or candidates remain. The operator
confirmed successful fixture cleanup and directory absence. This correction's
visual, preservation and cleanup gates are closed; task 007's earlier V1
fixture confirmations remain separate.

This decision supersedes ADR 0063's remaining prohibition on scrolling the
card grid and ADR 0070's restriction of that scrolling to an open log.
ADR 0070 still owns log-height priority. The completed shared-log packet stays
closed; compact cards, full-width log docking and transport visibility remain
separate scheduled work.

## Context

At narrow widths the three Show cards form a column. With logs closed, the
shared Show log composite mounts the card grid without a scrolling viewport.
A short window, recovery notices or a wrapped command failure can reduce the
available height enough to clip the lower cards behind the transport. Opening
a log happens to restore scrolling because only that branch mounts it.

The operator's screenshot shows this with three configuration issues, no active
show and no open log. Card access must depend on available space, not on opening
an unrelated report.

## Decision

The shared Show log composite always mounts the card grid inside the same
bounded vertical scroll container. With logs closed, the viewport fills the
main region above the transport. With logs open, ADR 0070's split and height
budget allocate that viewport. The scrollbar appears when content overflows.

Card order, sizes, typed selection/actions and view-model column count stay
unchanged. The shared page-content gutter keeps cards clear of the scrollbar.
The sidebar and transport remain outside this scroll container. The correction
introduces no configuration field, service command or playback behavior.

## Verification

- A situational ADR 0073 renderer test scrolls overflowing content to its last
  card with logs closed and open, at different available heights. It checks
  viewport bounds and the independent transport allocation.
- Existing ADR 0070 view-model and architecture checks retain log priority,
  measured geometry and sidebar independence.
- The [operator procedure](../runbooks/startup-recovery-check.md#show-card-overflow-follow-up--adr-0073)
  checks all three cards with logs closed, open and closed again, recovery
  issues present, short/narrow windows, both themes and larger scale. Visual
  acceptance, fixture preservation and confirmed cleanup are recorded in task 007.

## Alternatives And Consequences

Requiring a log to be open leaves ordinary Show navigation incomplete. Shrinking
or redesigning cards belongs to the separately scheduled compact-card work and
does not guarantee access at every available height.

Some statuses now require scrolling when the cards do not fit. All cards remain
reachable, and normal windows with enough height need no scrolling.
