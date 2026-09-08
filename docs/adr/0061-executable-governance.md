# ADR 0061: Current-State Governance

## Status

Accepted - 2026-09-07.

Amended 2026-09-07: added the acceptance-criterion rule below. Three ADR 0060
packets reported green while an operator found five visible defects, and one of
those defects failed a criterion the packet itself stated. The criterion was
phrased as a visual property with no mechanical form, so nothing evaluated it
and it passed by omission.

Canonical for all three repositories of the broadcast chain.
`musicindex-live-publisher` and `splitkit` adopt this ADR by reference from
their own `AGENTS.md`.

## Context

An agent session starts cold. It reads `AGENTS.md`, then whatever that file
sends it to, and then acts with confidence on what it found. It has no memory
of previous sessions and no way to tell a live rule from a dead one.

This project now holds 60 ADRs, a 15 kilobyte `AGENTS.md`, two architecture
documents that state binding rules, about 3,700 lines of architecture tests, and
a parked module worth 8 percent of `src/`. The reading path grows with every
decision. The cost of one session grows with it, and the risk of acting on a
dead rule grows faster.

Four failures show what already happened here:

- **`AGENTS.md` described a different project for months.** It instructed
  readers to use `src/verifiers/`, `migrations/`, `tests/common/`, and
  `tracing`. None exist in this repository.
- **ADR 0057 normalized status lines in August 2026.** Seven of 60 ADRs have
  drifted to a different format since.
- **Four ADRs sat at `Proposed`** after their own checklists recorded
  completion.
- **A cross-repository contract had no owner.** This app published a payload
  shape that reached no listener, and no document said whose decision it was.

ADR 0057 already recorded the reason:

> A rule that is routinely broken without consequence trains readers to ignore
> the document that states it.

There is a second requirement that ADR 0057 did not address. The product
direction is still forming and will keep moving. Governance that is expensive
to change gets routed around, and a reading path that only grows makes every
future change more expensive than the last.

## Decision

### The Reading Path Has A Fixed Size

The documents an agent reads before working are a bounded set:

1. `AGENTS.md` - what this project is, where the work stands, the current design
   philosophy, and the working rules.
2. `docs/adr/README.md` - the ADRs that still constrain judgment, one line each.
3. The specific ADRs and task packets its work touches.

Nothing else is required reading. The set does not grow as the project ages.

### `AGENTS.md` States The Present Only

`AGENTS.md` describes what is true now. It carries no history, no superseded
rule, no record of a replaced decision, and no explanation of how the project
reached its current shape.

It answers four questions and stops:

- What is this project?
- Where does the work stand right now?
- What is the current design philosophy?
- What are the working rules for changing it?

`AGENTS.md` carries the durable principles below and the current state. It
never carries a situational rule. Situational rules live in the ADR that made
them and leave with it.

Size stays bounded because situational rules keep dying, not because anyone
counts lines.

### Guards Are Durable Or Situational

Every guard declares which class it belongs to and which ADR owns it.

**A durable guard enforces a principle about how the project is built.** It
survives any design change and no supersession removes it. The durable set is
small and it is listed below.

**A situational guard enforces one design.** It cites exactly one ADR. When
that ADR is superseded, the guard is deleted in the same change.

A situational rule must never be written as a permanent one. `Discover must not
duplicate Library` was correct under ADR 0047 and ADR 0048. `Discover` no
longer exists, so the rule is now noise, and a permanent version of it would
constrain future work for a reason nobody could reconstruct.

This is the mechanism that keeps the reading path and the guard suite from
growing with age. It replaces a size limit, which cannot tell a principle from
an expired detail.

### The Durable Set

These are enforced permanently. They are properties of how this project is
built, not of any particular screen.

**Renderer portability.** View models are free of any renderer type.
Presentation facts, default labels, availability, and command intent live in
view models. Reusable chrome, row layout, and interaction geometry live in
shared primitives and composites. Screens stay thin.

The reason is portability, not taste. Every rule in this group is what allows
the interface to move to or from GPUI without rewriting the product logic. A
violation is a piece of the product welded to one renderer.

**Element hierarchy.** Every element has a defined place in a hierarchy. Title,
subtitle, metadata, state, and actions have predictable placement, weight, and
visibility. Nothing is positioned ad hoc.

**Apple HIG structure.** Predictable hierarchy and disclosure, clear state,
comfortable density, accessibility, and cautious destructive actions. HIG is
treated as structural guidance and not as visual imitation.

**Token discipline.** Size, spacing, color, typography, icons, and roles come
from named tokens. No raw literal and no glyph string in a renderer.

**Typed action state.** Every action carries typed availability and an
accessibility label before it renders. No renderer decides whether a control is
enabled.

An addition to this set is itself an ADR. The set does not grow casually.

### ADRs Separate Into Current And Historical

`docs/adr/` holds the ADRs that still constrain a judgment call.

`docs/adr/archive/` holds the rest. An archived ADR keeps its number, its file,
and its content. It is available for research and it is out of the reading
path.

**An ADR archives when one of two things is true:**

- it is superseded, or
- every rule it states is enforced by a guard.

The second condition is the important one. Once a rule is mechanically
enforced, the guard is the authority and the ADR is the reasoning behind it. A
reader who needs the reasoning goes and finds it. A reader who needs the rule
gets it from the failing test.

An ADR with an unguarded rule stays current, whatever its status. This is the
safety condition that stops archiving from quietly dropping a live rule.

### A Guard Is The Binding Form Of A Rule

A rule an ADR states is either enforced by a test, or the ADR names the manual
check that replaces it and says why a test cannot.

When a guard starts enforcing a rule, the prose that asked for it is deleted in
the same change. Net prose falls as net enforcement rises.

A rule earns a guard when you can name the time it broke. A rule with no
incident stays prose until it has one.

### An Acceptance Criterion States How It Is Checked

Every criterion in a task packet is exactly one of two kinds, and the packet
says which:

- **Mechanical.** A test proves it. State the property at the layer that owns
  it, not at the layer where a person would see it.
- **Visual.** Only a person can judge it. It belongs in an explicit visual-proof
  list, never among the mechanical criteria.

A criterion that is neither is not a criterion. It passes by omission, because
an implementer evaluates what it can evaluate and reports the rest as met.

Phrase a mechanical criterion at its owning layer. `The section shows six
states` is a render-layer claim that no test can make. `The view model exposes
six states and no raw transport error` is the same requirement at the layer
that owns it, and a test can prove it.

Keep visual criteria separate so they cannot hide among passing ones. A packet
whose visual gate is blocked reports the gate as open, never as met.

### A Guard Message Names Its ADR And The Fix

```text
ADR 0060 §Show Is A Screen Mount: a shell must not import a screen module.
Move this call into src/app/ and pass the result through the view model.
```

The message teaches the boundary at the moment the author meets it. This is
what separates structure from obstruction, and it is why a guard can replace
prose rather than merely accompany it.

### Dead Code And Dead Guards Are Deleted

The rule applies past documents.

- A guard that asserts a superseded rule is deleted with the ADR that
  superseded it.
- Code that no composition root reaches is deleted, not parked. If a pattern in
  it is worth keeping, it is copied into the live surface first.

Git holds the history. The working tree holds the present.

### Changing Direction Is One Change

When a decision changes, the superseding ADR, the index, the guard, the
archived predecessor, and any prose that restated it all move in the same
commit. No artifact may lag.

### Cross-Repository Contracts Have A Named Owner

| Contract | Owning ADR |
|---|---|
| Now-playing drop file | `musicindex-live-publisher` ADR 0002 |
| Show log | `musicindex-live-publisher` ADR 0003 |
| Relay wire format and event lifecycle | `splitkit` README and ADR 0001 |
| Broadcast control surface | `v4vmm` ADR 0059 |
| Workflow surface structure | `v4vmm` ADR 0060 |

Another repository cites the owner. It does not restate the rule.

### The Smaller Repositories Use The Same Shape

`musicindex-live-publisher` and `splitkit` have fewer decisions and less
history. They use the same structure at a smaller size: a present-tense
`AGENTS.md` under its own budget, a current ADR index, an archive, and the
corpus guard.

Their reading path must not grow with age either.

## Invariants

- `AGENTS.md` contains no superseded rule and no historical narrative.
- `AGENTS.md` contains no situational rule.
- Every guard declares its class and its owning ADR.
- A situational guard cites exactly one ADR and is deleted when that ADR is
  superseded.
- No situational rule is written in a permanent form.
- Every file in `docs/adr/` has a status line in the ADR 0057 vocabulary, in one
  canonical format.
- A superseded ADR names a superseding ADR that exists.
- No ADR in `docs/adr/` states a rule that is neither guarded nor accompanied by
  a named manual check.
- No ADR is archived while it states an unguarded rule.
- `docs/adr/README.md` matches the current ADR directory exactly.
- No guard asserts a rule from a superseded or archived ADR.
- Every guard failure message names its owning ADR.
- Every acceptance criterion is mechanical or visual, and the packet says which.
- No mechanical criterion is phrased at the render layer.
- No document outside `docs/adr/` states a binding rule without citing its
  owner.

## Alternatives Considered

### Keep One Growing ADR Directory

Rejected. It is the current state. Sixty ADRs give a reader no way to tell
which of them still constrain a decision, so either every session reads all of
them or every session guesses. Both get more expensive every month.

### Delete Superseded ADRs Outright

Rejected. The reasoning behind a replaced decision is the most valuable thing
in the corpus when a similar question returns. The payload-shape failure was
diagnosed by reading a superseded ADR. Archiving keeps that and removes the
context cost.

### Keep Prose Governance And Rely On Review

Rejected. It is the current state and it produced every failure in the context
above. Review catches what a reviewer remembers to check. Four ADRs sat wrong
across months of reviews.

### Bound `AGENTS.md` With A Line Budget

Rejected. It was the first draft of this ADR. A budget cannot tell a durable
principle from an expired detail, so it forces the removal of whatever is
easiest to cut rather than whatever is no longer true. Classifying rules by
lifetime bounds the file for the right reason.

### Generate `AGENTS.md` From The ADRs

Rejected, though it is attractive. A generated file cannot carry the current
design philosophy or where the work stands, which are the two things a cold
session most needs. The line budget gives the freshness pressure without the
machinery.

### Guard Everything

Rejected. A guard for a rule that has never broken costs maintenance and buys
nothing, and a wrong guard blocks work.

## Consequences

Positive:

- A cold session reads a bounded set and knows where the project stands.
- The reading path stays the same size as the project ages.
- A dead rule cannot be mistaken for a live one, because dead rules leave the
  path.
- The guard suite shrinks when a design is replaced, instead of accumulating
  constraints from designs nobody runs.
- Direction changes in one commit, so the repository keeps up with intent.
- A failure message teaches the boundary instead of only reporting a violation.

Negative and risks:

- `docs/architecture/ui-backend-boundary.md` and
  `docs/architecture/ui-regression-ratchet.md` state binding rules with no
  owning ADR. Each rule must be promoted to an ADR or marked advisory.
- `AGENTS.md` must be rewritten to the present tense and cut to its budget.
- Two repositories need a guard harness they do not have.
- A wrong guard blocks work, which costs more than misleading prose.
- The archive condition needs judgment. An ADR whose rules are "mostly" guarded
  stays current, and mostly is a decision a person makes.
- Classifying a guard needs judgment too. A rule that looks durable can turn out
  to be situational, and the cost of that error is a permanent constraint whose
  reason nobody can reconstruct. When the class is unclear, situational is the
  safer choice, because a situational guard can be promoted later and a
  permanent one is rarely questioned.
- The existing architecture tests are unclassified. Sorting roughly 3,700 lines
  into durable and situational is real work, and it is the only way to know
  which of them are already expired.

## Follow-Up Work

- Rewrite `AGENTS.md` to the present tense, carrying the durable set and the
  current state only.
- Classify every existing architecture test as durable or situational, and
  delete the situational guards whose ADRs are already superseded.
- Write `docs/adr/README.md` for the current corpus.
- Write the corpus guard and normalize the seven drifted status lines.
- Move superseded and fully guarded ADRs to `docs/adr/archive/`.
- Audit the two architecture documents for rules with no owner.
- Mine and delete the parked discover module.
- Give the same shape to the two smaller repositories.

## References

- ADR 0001 - Record architecture decisions
- ADR 0057 - ADR status vocabulary and amendment policy
- ADR 0060 - Workflow surface structure and vocabulary
- `docs/reviews/documentation-and-architecture-audit.md`
- `docs/notes/2026-05-discover-module-parked.md`
