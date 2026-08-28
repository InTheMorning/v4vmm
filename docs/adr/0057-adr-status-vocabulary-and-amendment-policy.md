# ADR 0057: ADR Status Vocabulary and Amendment Policy

## Status

Accepted - 2026-08-28. Supersedes the immutability clause of ADR 0001 and adds
the status vocabulary ADR 0001 left undefined. ADR 0001 remains in force for
everything else: sequential numbering, Nygard structure, and the rule that
reversing a decision requires a new ADR.

## Context

An audit on 2026-08-28 (`docs/reviews/documentation-and-architecture-audit.md`)
found the ADR corpus unreliable at the header line.

Fifty-six ADRs use seven different status phrasings: `Accepted`, `Implemented`,
`Accepted and implemented`, `Implemented for ADR 0047 scope`,
`Accepted - v1 implemented`, `Accepted - ... Implemented.`, and `Proposed`.
ADR 0001 defines only `Accepted` and `Superseded`, and says nothing about
whether `Accepted` means decided or shipped. Both readings are in active use:
ADR 0024 says `Accepted and implemented`, implying `Accepted` alone is not
enough, while ADR 0051 says `Accepted - 2026-05-17. Implemented.`

Four ADRs were left at `Proposed` after their own review checklists recorded
completion. ADR 0038 was contradicted twice: by its checklist
(`Completed on 2026-05-04. Readiness decision: Proceed`) and by the deferred-work
index, which records it as closed on the same date.

Separately, ADR 0001 states that an accepted ADR "is immutable except for status
updates such as `Superseded`", but amendment in place is the actual practice.
ADR 0035 records a scope amendment. ADR 0056 was amended substantially on
2026-08-28, including reversing one of its own rejected alternatives after
implementation disproved it. The rule and the practice have disagreed for
months, which means neither is enforced.

A status line nobody can trust costs more than no status line. A reader who
cannot rely on the header has to verify every ADR against the code, which is the
work the ADR corpus exists to remove.

## Decision

### Status Vocabulary

An ADR's status is exactly one of four values, followed by a date:

- `Proposed - YYYY-MM-DD` - the decision is drafted and not binding. No
  implementation is claimed.
- `Accepted - YYYY-MM-DD` - the decision is binding. Implementation is not
  complete, or completion is not yet verified.
- `Implemented - YYYY-MM-DD` - the decision is binding and fully shipped, with
  verification recorded in a review, checklist, or task packet.
- `Superseded by ADR NNNN - YYYY-MM-DD` - replaced. The superseding ADR carries
  the live decision.

`Accepted and implemented`, `Implemented for X scope`, and
`Accepted - ... Implemented.` are retired. They encode a partial state that now
has its own expression.

### Partial Implementation

Partial states are stated on a second line, never by inventing a fifth status:

```
Accepted - 2026-05-02. Implementation partial: Tasks 001-002 complete,
operator visual smoke outstanding.
```

The status stays `Accepted` until every gate the ADR or its checklist names is
closed. A pending operator visual check is an open gate.

### Amendment Policy

An accepted or implemented ADR may be amended in place when the amendment does
not reverse a decision:

- status and date reconciliation
- correcting a statement that implementation proved wrong
- adding invariants that tighten, not loosen, the existing decision
- recording follow-up work or routing it to the deferred-work index

Every in-place amendment adds a dated sentence to the Status section saying what
changed and why.

Reversing a decision still requires a new ADR, per ADR 0001. Widening scope
counts as reversal when it changes what the ADR forbids.

### Verification Requirement

`Implemented` requires a named artifact: a review document, a review checklist
with no open gates, or a completed task packet series. An ADR may not claim
`Implemented` on the strength of the author's assertion alone.

## Invariants

- Every ADR's status is one of the four values above, with a date.
- No ADR claims `Implemented` while a document it references records an open
  gate.
- No ADR sits at `Proposed` while a review checklist for it records completion.
- Status reconciliation happens in the same commit as the work that changes it,
  per the status hygiene rule in
  `docs/plans/deferred-architecture-work-index.md`.
- In-place amendments are dated and described in the Status section.

## Alternatives Considered

### Leave ADR 0001 Immutability And Forbid Amendments

Rejected. The practice has been amendment for months, across multiple authors,
and the rule did not stop it. A rule that is routinely broken without
consequence trains readers to ignore the document that states it. The ADR 0056
amendment is the clearest case: implementation disproved one of its rejected
alternatives, and issuing a second ADR to say so would have split one decision
across two documents.

### Add A Fifth `Partially Implemented` Status

Rejected. Partial states are open-ended -- which tasks, which gates -- and a
single word cannot carry that. A second line states it precisely, and keeps the
first line machine-checkable.

### Normalize Statuses Without A New ADR

Rejected as the route, though the normalization itself is required. ADR 0001
permits status updates but not changes to its own decision, and defining a
vocabulary changes what ADR 0001 decided. Editing ADR 0001 to define the
vocabulary would have been the first violation of the policy this ADR exists to
fix.

## Consequences

Positive:

- A reader can trust the header line without checking the code.
- `Implemented` becomes a claim backed by a named artifact.
- The amendment practice is legal, bounded, and visible in the Status section.
- Partial states stop being encoded as `Proposed`, which is what made four
  shipped ADRs look unbuilt.

Negative / risks:

- Fifty-six existing headers need a one-time normalization pass, and any ADR
  without an execution artifact cannot be promoted past `Proposed` even when the
  code suggests it shipped.
- `Implemented` is now harder to claim than before, so some ADRs will sit at
  `Accepted` longer than their authors expect.
- The four-value vocabulary does not express "implemented then partially
  regressed". That state should open a new ADR or a troubleshooting document.

## Follow-Up Work

- One-time normalization of all ADR headers to this vocabulary, done in the same
  change as this ADR.
- ADRs with no review, plan, or task artifacts cannot be verified and stay
  `Proposed`. They are listed in the audit and need an owner decision: execute,
  supersede, or archive.

## References

- ADR 0001 - Record architecture decisions
- `docs/reviews/documentation-and-architecture-audit.md`
- `docs/plans/deferred-architecture-work-index.md` - status hygiene rule
