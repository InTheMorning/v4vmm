# ADR 0001: Record Architecture Decisions

## Status
Accepted

## Context
We need a durable way to track significant architectural choices made during the development of `v4vmm`. Without a record, the rationale behind decisions disappears quickly and later changes become harder to evaluate on their merits.

## Decision
We will use Architecture Decision Records (ADRs) following the Michael Nygard style. ADRs live in `docs/adr/`, use sequential numbering, and capture context, decision, and consequences. Once accepted, an ADR is immutable except for status updates such as `Superseded`.

## Consequences
- Architectural reasoning is preserved in version control alongside the code.
- New contributors can understand why the project is shaped the way it is.
- Reversing or replacing a prior decision requires a new ADR, which keeps architectural drift explicit.
