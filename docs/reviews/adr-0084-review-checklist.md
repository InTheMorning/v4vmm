# ADR 0084 Review Checklist

This checklist is advisory. It states no rule. [ADR 0084](../adr/0084-music-column-browser.md) owns the rules.

## Reviewed Artifact

- ADR: `docs/adr/0084-music-column-browser.md`
- Plan: `docs/plans/adr-0084-music-column-browser-phase-plan.md`
- Task: `docs/tasks/archive/adr-0084-task-001-one-navigation-stack.md`
- Task: `docs/tasks/archive/adr-0084-task-002-one-release-route.md`
- Task: `docs/tasks/archive/adr-0084-task-003-one-track-route.md`
- Task: `docs/tasks/archive/adr-0084-task-013-index-release-page-parity.md`
- Task: `docs/tasks/archive/adr-0084-task-014-music-column-scrollbars.md`
- Task: `docs/tasks/archive/adr-0084-task-007-release-kind.md`
- Task: `docs/tasks/archive/adr-0084-task-015-singles-groups.md`

## Required Checks

- Does each navigation entry for an item key on its identity, not on its origin?
- Is a release or a track one entry, so the location path holds it one time?
- Does the origin select only the data source of the page?
- Does the render path read a stored source and do no database lookup?
- Does a download change the data source of the open page in place, with no navigation change?
- Does each Index fetch keep its stale-result check?
- Does the packet keep the Library screens free of navigation state (task 001 guard)?
- Is each mechanical criterion a test at the owning layer, with the packet prefix?
- Is each visual criterion in the packet, in pending human checks and in the delivery order, with the gate open?

## Architectural Drift Checks

- No GPUI type in `src/view_models/`.
- No second resolver for one kind of item.
- No `label` field in an identity key or a navigation entry.
- No new `cx.spawn` in a screen.
- No schema change and no new MusicIndex request, unless the packet names it.
- No guard text for a replaced rule stays. Each changed guard names ADR 0084.
- No change to a file in the "Do not touch" list of the packet.
- No merge of the page renderers before a packet names it.

## Merge Recommendation Template

1. Pass or fail:
2. Required fixes:
3. Optional improvements:
4. Can the packet merge:
5. Does the next packet need a change:
