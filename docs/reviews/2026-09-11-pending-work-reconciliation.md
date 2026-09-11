# Pending Work Reconciliation — 2026-09-11

## Status

Complete - 2026-09-11. Independent documentation and guard work is closed;
mechanical verification is Green. No application behavior or human acceptance
changed in this pass.

## Scope And Evidence

Reviewed the current ADR index, delivery order, deferred-work index, ADR 0066
phase plan, pending-human index and their owning reviews/runbooks. Read gate
prose against the recorded operator evidence and the 2026-09-10 governance
review. This is a bounded follow-up to that review, not a claim to have
reverified every historical task or all application behavior.

The operator has deferred playback work. ADR 0068 remains Proposed and the
mpv IPC error remains unresolved. Tasks 001–003 and ADR 0067 retain their
acceptance. Task 004 retains its presentation acceptance and the supplied
preservation results; this pass supplies no new desktop observation.

## Closed Findings

| Finding | Change and proof |
|---|---|
| Seven ADR headers lacked the canonical blank line | Normalized 0001–0006 and 0014 without changing their status values, dates or decisions. The new corpus guard first failed on exactly those seven files, then passed. |
| The ADR index deferred a corpus guard that did not exist | Added situational `adr_0057_status_headers_are_canonical` and its vocabulary/date rejection cases to `tests/architecture_tests.rs`. It reads all current and archived numbered ADRs; the reviewed corpus contains 68. ADR 0057 names its scope. Removed the replaced Known Drift paragraph. |
| Startup work was still advertised as unstarted or coupled to a mandatory player | Reconciled the deferred index, phase-plan assumptions and task 003 handoff with task 004's scoped readers, optional player and recorded partial acceptance. Task 005 and the overall task 004 gate remain open. |
| Completed ADR 0060 frames were still a future configuration-migration dependency | Retired that dependency in deferred item 7. ADR 0066 completion remains the prerequisite for configuration format migration. |
| Operator instructions named Ctrl+K/Space or an unspecified modifier | Corrected Linux search to Ctrl+F and the unavailable-player probe to Ctrl+Alt+P, using ADR 0067 and the live binding registry. Updated the surviving pending criterion; an unbound Space key cannot prove command rejection. |
| The accepted presentation regression still said to keep its recheck open | Reconciled the runbook with the operator's existing presentation pass. Retained the instructions for future regression. No new acceptance was inferred. |
| Strict lint of the edited guard file exposed three existing issues | Simplified two equivalent boolean expressions and removed one unnecessary borrow. No guard assertion or lint severity was relaxed. |

## Work That Remains Open

The [pending-human index](../pending-human-checks.md) is the live gate inventory.
This table records the disposition found during this pass.

| Work | Disposition |
|---|---|
| Task 004 partial path repair | Best next bounded app check: validate the retained warning, three visible entries, unavailable legacy binding and preservation. It requires no audio playback or reachable service. |
| Task 004 Index/player and publisher/producer reports | The remaining report, navigation, search and independent encoder observations can be walked without starting playback. Audio and metadata-publication portions stay deferred. |
| ADR 0043 toolbar search | Small independent Light/Dark, normal/narrow desktop check. Existing screenshots do not establish all four combinations or both submission paths. |
| ADR 0044 playlist reordering | Independent desktop check with at least four downloaded tracks and an unavailable row; the three-track startup fixture alone is insufficient. |
| ADR 0030 scrolling | Needs actually overflowing Music/Settings panes in both themes; no current evidence closes the surviving check. |
| ADR 0037 identity parity and ADR 0054 stored metadata | Need known populated source facts and local/Index comparisons. Empty fixtures cannot close them. |
| Task 004 final fixture cleanup | Still open; preserve the fixture while the unresolved playback evidence and remaining checks need it. |
| ADR 0066 tasks 005–013 | Not started. This review does not silently waive task 004's prerequisite or begin session-drain work. |
| Relay adoption, narrow Show, logs/UTC and other deferred implementations | Keep their existing delivery order and decision requirements; no additional implementation packet started. |

The five inherited check groups remain open. No superseding decision or later
operator evidence was found that would retire another surviving requirement
within the reviewed scope. Playback deferral does not turn a pending check into
a pass.

## Verification

The new guard reproduced the seven known header failures before correction.
Its acceptance/rejection examples cover all four status forms, inline partial
implementation prose, missing/extra spacing, retired vocabulary, malformed
supersession identifiers, invalid calendar dates and a missing sentence period.

Green after the corrections:

- `cargo test --locked --offline --test architecture_tests --quiet`: 235 passed.
- `cargo check --locked --offline --quiet`.
- `cargo clippy --locked --offline --quiet --bin v4vmm --test architecture_tests -- -D warnings`.
- `cargo fmt -- --check` and `git diff --check`.
- Documentation validation: 20 Markdown files, 232 local links/anchors and
  46 shell blocks. Comparison with HEAD confirms the seven normalized ADRs
  differ only by the added blank line.

The initial corpus test and test-target Clippy failures above were resolved
without suppressions. No unit-runtime or audio result is claimed from this
test-only change.

## Operator Visual Check

This documentation and guard change has no new visual criterion. App checks
remain open under their existing owners. The next bounded check is
[partial path repair](../runbooks/startup-recovery-check.md#5-contain-a-partially-applied-path-repair):
use the operator's retained fixture, inspect availability without playing,
close the app and record preservation. Its runbook supplies commands, expected
bindings and cleanup conditions. No playback check or fixture cleanup is
claimed by this review.
