# ADR 0058: Outbound HTTP Client Policy

## Status

Accepted - 2026-08-28.

## Context

`docs/reviews/documentation-and-architecture-audit.md` recorded that no client
in `src/` set a timeout and that a silent host would stall a blocking fetch
indefinitely. **The second half of that finding was wrong**, and the correction
is the reason this ADR is narrow.

`reqwest`'s blocking `ClientBuilder::timeout` defaults to
`Some(Duration::from_secs(30))`, not `None`. That default was already in force
at all ten construction sites. How it applies depends on how the caller reads
the body:

- connect and response head are bounded once
- streaming reads through `std::io::copy`, used for enclosure downloads, are
  bounded per `read` call, so a long download is safe and a stalled socket is
  not
- whole-body calls (`bytes`, `text`, `json`), used for artwork and feeds, are
  bounded across the entire body

So the application was not hanging. What it had was ten independent client
constructions inheriting an invisible dependency default, with
`connect_timeout` genuinely unset, so a host that was simply down consumed the
whole operation budget before failing.

That is a smaller problem than the audit claimed, and the same shape as the
defect ADR 0056 addressed: a policy with no owner, where each call site can
drift and nothing reports it.

## Decision

All blocking HTTP clients are constructed in `src/http_client.rs`.

The module owns two numbers:

- `CONNECT_TIMEOUT` (10s) - previously unset. Separates "host is down" from
  "host is slow" so the former fails fast.
- `OPERATION_TIMEOUT` (30s) - matches the current `reqwest` blocking default,
  stated explicitly so a dependency bump cannot move it silently.

It exposes `document()` and `document_builder()` for feed and API fetches, and
`media_builder()` for media transfers. The media builder exists because
`remote_media` adds redirect policy to it, not because the timeouts differ.

The module owns client construction only. Legal URLs, redirect resolution, and
response body validation stay where ADR 0056 put them.

Callers do not construct `reqwest` clients directly.

## Invariants

- No `reqwest` client is constructed outside `src/http_client.rs`.
- Timeout values are named constants in that module, not literals at call sites.
- The operation timeout is stated explicitly even when it equals the dependency
  default, so the value is visible in this codebase.
- Changing a timeout is one edit, not ten.

## Alternatives Considered

### Add Timeouts At Each Call Site

Rejected. Ten sites with duplicated literals is the exact condition that let one
of five media fetches ship without redirect handling under ADR 0056. Duplication
is visible in review; drift between duplicates is not.

### Rely On The Dependency Default

Rejected, though it is defensible. The default is sound today and covers the
streaming case correctly. It is also undocumented in this repository, unowned,
and outside this project's control. A reader auditing timeout behavior would
have to read `reqwest`'s source, which is what this ADR's author had to do.

### Route Document Fetches Through `remote_media`

Rejected, and out of scope. ADR 0056 deliberately excluded feed and API fetches
from the media transport boundary: they fetch documents, want different retry
and error handling, and do not need scheme allowlists or redirect repair.
Sharing client *construction* is not the same as sharing fetch policy, and only
the former is decided here.

### Separate Timeout Budgets For Media And Documents

Rejected as unnecessary. The first draft of this work assumed the blocking API
offered a read-inactivity timeout distinct from a total budget, and modeled two
policies around that. The blocking builder has no `read_timeout`; its single
`timeout` already behaves as per-read inactivity when the body is streamed. The
two-policy design was solving a problem the API does not have.

## Consequences

Positive:

- Timeout policy is one file, two constants, and a guard.
- `connect_timeout` is set for the first time, so unreachable hosts fail in 10
  seconds rather than 30.
- The `reqwest` default is pinned in this repository rather than inherited.

Negative / risks:

- Callers needing a genuinely different budget must extend the module rather
  than configure locally, which is friction by design.
- The 30 second operation timeout means different things for streamed and
  whole-body reads. The module documents this; a future reader changing the
  number has to understand both.
- Retry, backoff, and per-host policy remain unowned. This ADR does not
  introduce them.

## Follow-Up Work

- Consider whether whole-body artwork fetches deserve a smaller budget than
  streamed enclosure downloads, once there is evidence either is a problem.
- Retry and backoff policy, if feed refresh failures justify it, is a separate
  decision.

## References

- ADR 0056 - Remote media fetch validation boundary
- ADR 0015 - Non-UI service boundaries
- `docs/reviews/documentation-and-architecture-audit.md`
