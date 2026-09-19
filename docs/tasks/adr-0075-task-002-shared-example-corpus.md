# ADR 0075 Task 002: Write The Shared Metadata Example Corpus

Status: Ready - 2026-09-19. Work has not started.
The parent ADR 0075 is Accepted from 2026-09-19.
The operator holds the dispatch of every ADR 0075 packet.
This packet produces a document. It changes no code.

## Goal

Write one shared corpus of example cases for RSS and MusicIndex API metadata.
The corpus records the declared owner and the expected result for each case,
in [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md).
Subsequent packets read this corpus as shared ground truth. Each repository
reads it too.
This packet writes the corpus document only.
It changes no parser code, no application code, and no database schema.

## Files To Inspect

- `AGENTS.md`
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md)
- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md), the Packet Register section
- [Review](../reviews/adr-0075-metadata-contract-review.md)
- [Task 001](adr-0075-task-001-contributor-claim-transport.md), especially its Supplied Response section
- `src/rss/enrich.rs`, functions `nostr_from_extensions` and `nostr_from_extension`
- `src/rss/subscribe.rs`, functions `rss_feed_link_inputs`, `rss_track_link_inputs`, and `contributor_inputs_from_extensions`
- `src/api.rs`, the `Track`, `Feed`, and `Contributor` types, and function `track_with_feed_defaults`
- `src/identity_ingest.rs`, functions `persist_source_links`, `persist_source_ids`, and `persist_contributors`
- `src/local_identity.rs`
- `src/views.rs`, functions `website_url_from_links` and `nostr_npub_from_ids`
- `src/db.rs`, functions `local_identity_links`, `replace_local_identity_links`, and `local_contributors`
- `docs/schema/storage-and-metadata.md`, the `tracks` table entry, for the `(feed_id, item_guid)` storage key

Inspect these upstream files as read-only copies, at commit `a220f44`, in
`/home/citizen/build/stophammer`. Do not write in that checkout.

- `stophammer-parser/src/engine.rs`, functions `extract_links`, `extract_persons`, and `extract_entity_ids`
- `stophammer-parser/src/types.rs`, types `IngestPerson`, `IngestEntityId`, and `IngestLink`
- `src/query.rs`, type `SourceContributorClaimResponse`
- `src/db.rs`, functions `get_effective_source_contributor_claims_for_track` and `get_source_contributor_claims_for_feed_entity`
- `src/ingest.rs`

Case C10 below depends in part on the code that handles extensions inside
the `rss` crate. This repository's `Cargo.lock` pins that crate at version
2.0.12. The `rss` crate is a dependency, not part of the two repositories
named above. Check its behavior before you write it as fact. You can find a
local copy at
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rss-2.0.12/src/extension/`.

## Deliverable

Write `docs/schema/adr-0075-metadata-example-corpus.md`.

The deliverable has:

- A short scope note. See Constraints for its required statements.
- A summary index: one table row for each case identifier, with a one-line
  scenario label and the ADR 0075 rule it proves.
- One subsection for each case identifier. Required Content gives the case
  identifiers in sequence. Follow that sequence.

Give each case subsection the case identifier as its heading, for example
`### C01`.
Give each case subsection these six labeled fields: **Source Form**,
**Declared Owner**, **Field Kind**, **Expected Stored Fact**, **Expected
Display Result**, and **ADR 0075 Rule**.
Write the source form as a fenced code block with an RSS or JSON fragment.
Write the expected stored fact. Give its provenance: the source, the
extraction path, and the position, where the cited source gives them.

## Do Not Touch

- Application code, tests, or configuration in this repository.
- The SQLite schema or a production database.
- `AGENTS.md`, ADR 0075, the phase plan, the review, or task 001.
- The Stophammer checkout at `/home/citizen/build/stophammer`.
- Any live network request. Construct each example from the cited source
  files and the supplied fixture.
- Visual acceptance gates that are already in place. This packet requests no visual check.

## Constraints

Write each sentence in ASD-STE100 Simplified Technical English.

Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
Write compliant prose directly. Do not make a draft that requires conversion to STE.

Use the shared checker in Checks. Correct only confirmed errors in the affected prose.
A Green result covers the configured checks. It does not prove full dictionary compliance.

Write, in the deliverable's scope note, that the corpus records each
observed source form and the expected result in ADR 0075.
Write that the corpus does not claim that production data is correct.

Ground each case in a cited file, function, and line range, or in the
supplied fixture in task 001.
Do not invent a parser behavior, an extraction path string, or a default
role or group.
When a fact differs between this app and the upstream parser, record each
value and name its source.
When you can not check a fact, write it as an unverified fact. Do not guess
a missing value.

Use relative links from `docs/tasks/` for each internal reference, for
example `../adr/0075-metadata-ownership-and-completeness.md`.
Each relative link must resolve to a file or an anchor that is present.
The deliverable path is not present when you read this packet. Do not write
it as a clickable link inside this packet.

## Required Content

The corpus covers the 21 cases below, in 9 scenario families from ADR 0075
and the review.
Use the case identifiers as given here. Do not renumber or merge a case.

### Case Family 1: The Supplied MoeFactz Response

- **C01** — The three contributor credits from task 001's Supplied Response.

Two credits carry HeyCitizen's npub. The host credit carries
`https://www.moefactz.com/` as `href`, with no `npub`. Link to task 001's
fixture for the source form. Do not write the JSON again. Rule: ADR 0075
Decision 1. A contributor occurrence is not a global identity.

### Case Family 2: Single-Owner Identities

- **C02** — An identity that belongs only to a feed. Use a channel-level
  `<podcast:txt purpose="npub">` value.
- **C03** — An identity that belongs only to a track. Use an item-level
  `<podcast:txt purpose="npub">` value.

Ground the extraction rule for both cases in
`stophammer-parser/src/engine.rs::extract_entity_ids`. That function reads
only direct children of the passed node: the channel node for C02, the item
node for C03.

### Case Family 3: Duplicate And Conflicting Claims

- **C04** — Duplicate roles for one contributor name. Two `<podcast:person>`
  elements share one name and one role, at two different positions.
- **C05** — Names that conflict for one key. One npub value appears under
  two different display names, in two separate credits.
- **C06** — Keys that conflict for one name. One display name appears with
  two different npub values, in two separate credits.

Ground C04 in ADR 0075's definition of a contributor occurrence: the subject,
plus the source collection, plus the position. Cases C05 and C06 each prove
ADR 0075 Decision 1. Do not infer identity from a repeated name, URL, public
key, or payment address.

### Case Family 4: Website And Page Links

- **C07** — A channel RSS `<link>`.
- **C08a** — An item RSS `<link>`.
- **C08b** — An item `<atom:link rel="alternate">`.

Ground the owner and the field kind for each case in the field rule table of
ADR 0075: the row for channel RSS `link` for C07, the row for item RSS
`link` for C08a, and the row for a supported Atom alternate link for C08b.

Record the current app behavior for C08a and C08b.
Today, `src/rss/subscribe.rs::rss_track_link_inputs` does not store either
form as a track identity fact. Cite the review's section named "Website
Rules Differ".

### Case Family 5: Malformed And Alternate Syntax

- **C09** — A malformed identifier that must stay as evidence. Use a value
  with the `npub1` prefix and an invalid length or checksum.
- **C10** — An alternate XML namespace prefix for the same tag. Declare the
  Podcast Namespace under a prefix other than `podcast`, for example `pc`.
  Repeat a `podcast:txt purpose="npub"` value under that prefix.

Record the current app behavior for C10.
`src/rss/enrich.rs::nostr_from_extensions` and
`src/rss/subscribe.rs::contributor_inputs_from_extensions` each look up
extensions with the literal key `"podcast"`. The `rss` crate's extension map
may key entries by the declared XML prefix, not by the resolved namespace
URI. If so, an alternate prefix causes these lookups to find no match. Write
this as a claim for the deliverable to check against the cited `rss` crate
source. Do not write it as an established fact before that check.

### Case Family 6: Collection State

- **C11** — A collection that is not requested. The API request omits the
  collection from its `include` parameter.
- **C12** — A collection that is null. The response includes the field with
  a JSON `null` value.
- **C13** — An empty collection. The response includes the field as an
  empty JSON array.
- **C14** — A populated collection. The response includes the field with
  one or more entries.

Write, for these four cases, if the current `Option<Vec<T>>` fields in
`src/api.rs` can tell C11 apart from C12. Ground this in the field's Serde
behavior, in the cited struct definitions.

### Case Family 7: Request Failure

- **C15** — A failed request. The HTTP call returns an error status or a
  transport error.
- **C16** — A malformed payload. The HTTP call returns a 200 status with a
  body that does not match the expected JSON shape.

Ground the required rule in the collection state table of ADR 0075. Treat a
malformed payload as a failed request. Do not treat it as an empty
collection.

### Case Family 8: Feed-Scoped Track Identity

- **C17** — Identical track GUIDs under two different feeds. Two feeds each
  publish an item with the same `<guid>` value.

Ground the storage key in the `tracks` table's `(feed_id, item_guid)`
composite key. That key is visible in `src/rss/subscribe.rs`'s `ON
CONFLICT(feed_id, item_guid)` clause, and recorded in
`docs/schema/storage-and-metadata.md`.

### Case Family 9: Contributor Credit Transitions

- **C18a** — A track with its own contributor credits.
- **C18b** — The same track after the source removes its own credits. The
  feed still has credits.
- **C18c** — The same track after the source also removes the feed's
  credits.

Ground these three states in
`get_effective_source_contributor_claims_for_track` at the cited upstream
commit. Write the declared `entity_type` and `entity_id` that each state
returns.

## Acceptance Criteria

### Mechanical Criteria

- The file `docs/schema/adr-0075-metadata-example-corpus.md` exists.
- The deliverable contains a summary index table with 21 rows, one for each
  case identifier C01 through C18c.
- The deliverable contains one subsection for each case identifier, using
  the exact headings `### C01` through `### C18c`.
- Each case subsection contains all six required labels: **Source Form**,
  **Declared Owner**, **Field Kind**, **Expected Stored Fact**, **Expected
  Display Result**, and **ADR 0075 Rule**.
- Each relative link in the deliverable resolves to a file or an anchor
  that is present.
- The shared STE command in Checks reports no unresolved error for the deliverable.

### Review Criteria

A reviewer checks that the expected stored fact and the expected display
result for each case agree with the real behavior of the cited source
function.
A reviewer checks that the deliverable does not claim that production data
is correct.
A review check that did not run reports this gate as open, not as met.

## Checks

Do these checks against the deliverable after you write it.
No application test applies. This packet changes no code.

```bash
# From the repository root. Confirms every relative markdown link resolves.
grep -oE '\]\([^)]+\)' docs/schema/adr-0075-metadata-example-corpus.md \
  | sed -E 's/^\]\((.*)\)$/\1/' \
  | grep -v '^https\?://' \
  | while read -r link; do
      target="${link%%#*}"
      [ -z "$target" ] && continue
      (cd docs/schema && test -e "$target") || echo "BROKEN: $link"
    done
```

```bash
python3 "$HOME/.agents/skills/asd-ste100/scripts/ste_lint.py" \
  --check --no-heuristics docs/schema/adr-0075-metadata-example-corpus.md
```

## Escalation Triggers

Stop and report if any of these happen:

- A required case has no support in a cited file, function, or the supplied
  fixture.
- A cited function is no longer present, or its behavior no longer matches
  this packet, at commit `a220f44`.
- An accurate case would need a code change or a live network request.
- Two cited sources disagree about a fact, and this packet does not say
  which source governs.

Revise this packet, or ask the operator, before you invent a fact to fill a
gap.

## Expected Report

Report the deliverable's file path, its case count, and its case
identifiers.
Report the checks you did and their results.
Name each fact you could not check in the cited source, and the case it
affects.
Name each deviation from this packet's Required Content.
State if the summary index is complete. State if each case subsection is
complete.

## Prompt for lower-context model

You write one bounded document from a larger plan. Do not write code. Do not
redesign ADR 0075.

Read:

- This packet and each file in its Files To Inspect section.

Goal:

- Write `docs/schema/adr-0075-metadata-example-corpus.md` with the 21 cases
  listed in Required Content.

Constraints:

- Apply this packet's Constraints and Required Content sections.
- Ground each case in a cited source. Do not invent a value.
- Write each sentence in ASD-STE100 Simplified Technical English.

Do not touch:

- Each boundary named in this packet's Do Not Touch section.

Acceptance criteria:

- Pass each mechanical criterion above.
- Leave each review criterion for a person. Report it as open.

Checks:

- Do the commands in this packet's Checks section.

At the end, report:

1. deliverable file path and case count
2. checks done and their results
3. facts you could not check
4. deviations from this packet
5. unresolved concerns
