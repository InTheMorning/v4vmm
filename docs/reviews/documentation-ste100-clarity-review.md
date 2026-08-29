# Documentation STE100 Clarity Review

## Reviewed Artifact

Mechanical scan scope:

- `README.md`
- `AGENTS.md`
- `docs/**/*.md`

Manual review focus:

- `AGENTS.md`
- ADR 0056, ADR 0057, and ADR 0058
- ADR 0043 and ADR 0044 status lines
- ADR 0031, ADR 0032, and ADRs 0046-0053 from the current docs index
- ADR 0046 and ADR 0047 phase plans
- ADR 0047 Task 016
- ADR 0056 task packets
- ADR 0056 implementation review
- documentation and architecture audit
- deferred-work index
- active UI guidance docs
- current runbook and schema overview docs
- `docs/README.md`
- `README.md`

The corpus contains 444 Markdown files. The scan used STE-flavored mode for ADRs,
plans, reviews, and explanatory prose. It used Strict mode for agent instructions,
task constraints, acceptance criteria, and status rules.

This review does not claim certified ASD-STE100 compliance. The installed skill
does not include the official ASD dictionary, so this review checks structure,
consistency, and plain-word direction only.

## Result

Targeted fixes applied. The current unstaged Markdown set removes the clarity
defects that could change implementation behavior.

The full corpus is not certified STE-clean. The remaining scan counts identify
future review targets, not known behavior-changing defects.

## Applied Fixes

### ADR 0056 Amendment Status

Problem: ADR 0056 used repeal-style wording in its amendment status. ADR 0057
reserves that meaning for a governance change that requires a new ADR.

Applied rewrite:

- ADR 0056 now says the amendment replaces a deferred implementation note with
  the shared fetch module.
- ADR 0056 now has status `Implemented - 2026-08-28`.
- The status says Tasks 001-004 are complete.

Impact: ADR 0056 no longer conflicts with the ADR 0057 amendment policy.

### Task 002 Image Rules

Problem: Task 002 still preserved the old APIC rule. It said one image rule
applied everywhere and that APIC behavior was unchanged.

Applied rewrite:

- APIC artifact writes accept only image types recognized from bytes.
- Display paths sniff bytes first.
- If display-path bytes are not recognized, the display path may use a declared
  `image/*` type.
- Display paths reject every other response.

Impact: the task packet now matches ADR 0056 and the implementation review.

### ADR 0058 Status

Problem: ADR 0058 had status `Accepted - 2026-08-28`, but the deferred-work
index described implemented and guarded behavior.

Applied rewrite:

- ADR 0058 now has status `Implemented - 2026-08-28`.
- `docs/reviews/adr-0058-implementation-review.md` records the implementation
  review.
- The deferred-work index points to that review.

Impact: the ADR, review, and deferred-work index now use one status.

### AGENTS.md Strict Instructions

Problem: `AGENTS.md` used dense mandatory rules that packed multiple actions into
one sentence. It also used unclear default-value terms in several UI ownership
rules.

Applied rewrite:

- The default ownership rule is split into separate owner instructions.
- The quick-fix rule is split into separate mandatory instructions.
- The Recent Feeds rule now separates the invariant from the two paths.
- The subagent scope rule now separates frame and search responsibilities.
- The UI ownership rules now use "default labels" and "default strings."

Impact: the strict agent instructions now have fewer multi-action sentences.

### Current ADR And Task Prose

Problem: current ADR and task text used semicolons and dash clauses in places an
agent must parse as instructions.

Applied rewrite:

- ADR 0056, ADR 0058, the deferred-work index, and ADR 0056 Tasks 001-004 now
  split the highest-risk compound sentences.
- ADR 0057 and the documentation audit now describe the ADR 0056 amendment with
  the same "deferred implementation note" wording.
- ADR 0043 and ADR 0044 partial-status text now names each open gate in a
  separate sentence.
- ADR 0046 and ADR 0047 phase plans now separate status, scope, acceptance
  conditions, and mitigation steps.
- ADR 0047 Task 016 now uses one command per sentence in the lower-context
  prompt.
- The ADR 0056 implementation review now uses the same terms as the amended ADR
  and task packets.
- `docs/README.md` now links to the new STE review and ADR 0058 implementation
  review.

### Current Index And Overview Text

Problem: `README.md` and `docs/README.md` had long scan lines and dense index
entries.

Applied rewrite:

- `README.md` now splits long overview and metadata sentences.
- `docs/README.md` now groups entries and wraps descriptions.

### Current Docs Index Slice

Problem: the current docs index still linked to ADRs, plans, runbooks, and
schema notes with semicolon-joined requirements.

Applied rewrite:

- ADR 0031, ADR 0032, and ADRs 0046-0053 now split the current-docs semicolon
  clauses outside ADR 0057's deliberate governance-term definition.
- `docs/runbooks/workflows.md` now separates playback state changes from their
  effects.
- `docs/schema/storage-and-metadata.md` now states the enclosure selection order
  with one verb form.
- `docs/plans/adr-0023-design-system-migration.md`,
  `docs/plans/adr-0024-library-index-data-parity-follow-up-plan.md`,
  `docs/plans/discovery-library-ui-fixes.md`, and
  `docs/plans/inspector-source-ownership-phase-plan.md` now split the flagged
  implementation constraints.

## Optional Improvements

- Add a documentation style guide for agent-facing files. Use Strict mode for
  task packets, ADR invariants, acceptance criteria, and AGENTS rules.
- Add a lightweight Markdown clarity scan for semicolons in strict sections.
  Semicolons are not always wrong in prose, but they are high-risk in task
  instructions.
- Archive or annotate older task packets whose instructions were superseded by
  later review fixes. Task 002 is the current example.
- Consider a separate pass for completed task-packet inventories where legacy UI
  default-value terms are part of historical text or test names. Do not rename
  test identifiers only to satisfy prose style.

## Corpus Notes

Mechanical scan results:

- 446 Markdown files scanned, including `README.md`, `AGENTS.md`, and
  `docs/**/*.md`.
- 64 sentence-like blocks exceed 30 words outside fenced code blocks.
- 1,327 lines contain semicolons outside fenced code blocks.
- 1,171 lines contain en dashes or em dashes outside fenced code blocks.
- 321 lines contain common soft terms from the STE scan list outside fenced
  code blocks.

These counts are not defects by themselves. They identify where to spend manual
review effort. The applied fixes above are the cases where wording can change
implementation behavior.

## Merge Recommendation

Merge these targeted documentation edits. Do not treat the corpus as certified
STE-clean. Use this review as the baseline for future STE passes.
