# ADR 0075 Task 035: Comparison And Discrepancy Rules

Status: Document complete. Technical review passed. Individual URL comparison and action policies accepted on 2026-09-21. Implementation remains open.

The review corrected comparison-version identity and added actual upstream HTML-loss cases.
Decision H and the field refinements accept absence, source order, conflicts, and stale labels for all four comparison fields.
Packet 018 still owns numeric freshness and request scheduling.

Feed website actions allow only valid HTTP or HTTPS URLs, accepted separately on 2026-09-21.
Retain other schemes as source evidence without a website action.
Track page actions also allow only valid HTTP or HTTPS URLs, accepted separately on 2026-09-21.
Retain other schemes as source evidence without a page action.

Feed website comparison normalizes scheme, host, and default ports while preserving path, query, and fragment differences.
The operator accepted this rule separately on 2026-09-21.
Track page comparison applies the same normalization, preserving path, query, and fragment differences, with separate acceptance on 2026-09-21.

## Goal And Deliverable

Specify comparison and discrepancy retention for accepted Decisions F and G.
The [contract](../schema/adr-0075-comparison-and-discrepancy-contract.md) defines the proposed details.
It covers representations, readable text, URLs, field coverage, discrepancy identity, transitions, and retained evidence.
Packet 018 owns numeric cache expiry after packet 016 measures current requests.

## Scope

Compare only descriptions, feed websites, and track page links for matching declared owners.
Keep raw responses, original values, resources, source times, and actual fetch times separate.
Specify how repeated mismatches, changed values, agreement, failures, and endpoint changes affect the same discrepancy.
Do not implement an update hook, storage, requests, or presentation.

## Review Criteria

- Mechanical: local links resolve and changed prose passes the configured structural checks.
- Technical: comparison distinguishes HTML from already decoded plain text.
- Technical: unknown coverage and failed observations cannot create or resolve discrepancies.
- Technical: storage can retain conflict evidence after snapshot replacement and restart.
- Policy: source occurrence selection, website and page URL comparison, and URL actions have individual operator acceptance.

The accepted discrepancy requirement does not need another acceptance decision.
Unaccepted selection details remain proposals.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- ADR 0075, Decisions F and G, and packets 005 and 007.
- The field inventory and collection completeness rules.

Goal:
- Complete the comparison contract for later storage and projection packets.

Constraints:
- Preserve accepted rules. Mark additional product policy as proposed.

Do not touch:
- Application code, upstream code, shared status documents, and operator data.

Acceptance criteria:
- The review criteria above have explicit results and remaining gates.

Test commands:
- Run the repository link checker and shared STE checker on this packet and its deliverable.

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Operator Visual Check

This document changes no presentation. Existing visual gates remain paused.
