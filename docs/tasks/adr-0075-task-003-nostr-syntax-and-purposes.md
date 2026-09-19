# ADR 0075 Task 003: Write The Identity Syntax And Purpose Contract

Status: Ready - 2026-09-19. Work has not started.
The parent ADR 0075 is Accepted from 2026-09-19.
The operator holds the dispatch of every ADR 0075 packet.
This packet produces a document. It changes no code.

## Goal

Write one contract that states which Nostr identity syntax this app
supports.
Write, for each supported form, where it may appear: at the feed, at the
track, or at the contributor occurrence.
This packet writes the contract document only. It changes no parser code
and no application code.

## Files To Inspect

- `AGENTS.md`
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md), Decision 3
- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md), the Packet Register section
- [Review](../reviews/adr-0075-metadata-contract-review.md), the section named "RSS Extraction Can Assign The Wrong Owner"
- `src/rss/enrich.rs`, functions `nostr_from_extensions`, `nostr_from_extension`, `extract_nostr_handle`, and `enrich_track_from_feed_rss`

Inspect these upstream files as read-only copies, at commit `a220f44`, in
`/home/citizen/build/stophammer`. Do not write in that checkout.

- `stophammer-parser/src/engine.rs`, functions `extract_entity_ids`, `extract_persons`, and `is_podcast_namespace`
- `stophammer-parser/src/types.rs`, types `IngestPerson` and `IngestEntityId`

Read these external pages:

- [NIP-19](https://github.com/nostr-protocol/nips/blob/master/19.md)
- [The `podcast:txt` tag](https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/txt.md)
- [The `podcast:person` tag](https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/person.md)

`src/rss/enrich.rs` looks up extensions with the literal key `"podcast"`,
through calls such as `exts.get("podcast")`. The app's `rss` crate
dependency, version 2.0.12, may key its extension map by the declared XML
prefix, and not by the resolved namespace URI. Check this against the `rss`
crate source before you rely on it. A local copy can be at
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rss-2.0.12/src/extension/`.

## Deliverable

Write `docs/schema/adr-0075-identity-syntax-contract.md`.

Give the deliverable these required sections, in this order, with these
exact headings:

- `## NIP-19 Encodings`
- `## Supported podcast:txt Purpose Values`
- `## Compatibility Decision For purpose="nostr"`
- `## The podcast:person npub Attribute`
- `## Valid Positions For podcast:txt`
- `## Prefix Validity Without Content Validity`
- `## Unsupported Syntax`
- `## Current Behavior Compared With The Upstream Parser`

## Do Not Touch

- Application code, tests, or configuration in this repository.
- `AGENTS.md`, ADR 0075, the phase plan, or the review.
- The Stophammer checkout at `/home/citizen/build/stophammer`.
- Any live network request. Use the cited source files and the cited pages.
- Visual acceptance gates that are already in place. This packet requests
  no visual check.

## Constraints

Write each sentence in ASD-STE100 Simplified Technical English.

Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
Write compliant prose directly. Do not make a draft that requires conversion to STE.

Use the shared checker in Checks. Correct only confirmed errors in the affected prose.
A Green result covers the configured checks. It does not prove full dictionary compliance.

Write that the `podcast:txt` specification lets each service pick its own
`purpose` values.
Do not claim that the specification requires the word `npub`, or the word
`nostr`.

Ground each claim about the app or the parser in a cited file and function.
Ground each claim about NIP-19, `podcast:txt`, or `podcast:person` in the
cited page.

Decide the `purpose="nostr"` compatibility question inside the deliverable,
not inside this packet.
Base that decision on the evidence in Files To Inspect. Record the reason in
the deliverable.
Do not add support for a purpose value that no cited source or example uses
today, unless the deliverable records a reason.

## Required Content

### NIP-19 Encodings

Write the `npub` encoding: a bech32 form of a 32-byte public key, with no
other field.
Write the `nprofile` encoding: a bech32 form of a TLV structure that carries
the public key, and can carry one or more relay hints.
Write that the app must keep this distinction. Do not treat `npub` and
`nprofile` as one interchangeable form.

### Supported podcast:txt Purpose Values

Write that upstream's `extract_entity_ids` accepts only a `purpose`
attribute equal to `npub`, after it trims and lowers the case, mapped to the
scheme `nostr_npub`.
Give one checked example: a direct channel or item `<podcast:txt
purpose="npub">` element with a valid `npub1` value, and the `IngestEntityId`
it produces.
Write that, at the cited commit, the parser drops every other `purpose`
value from `entity_ids` today. The raw element stays present in the
parser's general Podcast Namespace snapshot.

### Compatibility Decision For purpose="nostr"

Decide if the app treats `purpose="nostr"` as the same as `purpose="npub"`,
or as unsupported syntax.
Record the reason with a citation: to the cited source files, to the
specification, or to an example the operator supplied.
Write the consequence for a feed that already publishes `purpose="nostr"`,
if the cited sources show one.

### The podcast:person npub Attribute

Write that the `npub` attribute belongs to one `podcast:person` element,
read by upstream's `extract_persons`.
Write the rule: a person's key belongs to that contributor occurrence. It
never belongs to the track or the feed that holds it.
Cite the review's section named "RSS Extraction Can Assign The Wrong Owner"
as the observed break of this rule.

### Valid Positions For podcast:txt

Write the rule: the channel gives a feed identity through its direct child.
The item gives a track identity through its direct child.
Write that a value in a deeper descendant element is not a track identity
or a feed identity, under this contract.
Ground this rule in upstream's `extract_entity_ids`, which reads only
direct children of the passed node.

### Prefix Validity Without Content Validity

Write the rule: a recognized prefix alone does not validate a value.
Give one example of a value with a valid prefix and invalid content. Write
that the app keeps it as evidence.
Write that the app must not offer an unvalidated value as an identity
action.

### Unsupported Syntax

List each syntax form the app does not support as an identity, with the
record the app writes for it. Cover a minimum of these:

- an `nsec`, `note`, `nevent`, `naddr`, or `nrelay` value
- a `purpose` value other than `npub`, and other than `nostr` if the
  Compatibility Decision section adds it
- a value shaped like `npub`, in a place other than a direct `podcast:txt`
  or `podcast:person` element

### Current Behavior Compared With The Upstream Parser

Compare `src/rss/enrich.rs::nostr_from_extension` with upstream's
`extract_entity_ids` and `extract_persons`.
Write that the app's function scans each extension under the `podcast` key,
not only `txt` elements, and goes into child elements below that.
Write that the app's function matches by the shape of the value, the
`npub1` or `nprofile1` prefix, and not by a `purpose` attribute.
Write that `enrich_track_from_feed_rss` marks each match with the fixed
extraction path `podcast:txt@purpose=nostr`. It does this apart from the
element or the attribute the value came from.
Write the exact difference that packet 009 must correct: the app must read
a Nostr identity only from a direct `podcast:txt` element with a supported
`purpose`. The app must not assign a `podcast:person` attribute's value to
the track or the feed that holds it, as a `podcast:txt` fact.

Write a second difference for the record. `src/rss/enrich.rs` looks up
extensions by the literal key `"podcast"`. If the `rss` crate keys its
extension map by the declared XML prefix, and not by the resolved namespace
URI, then an alternate prefix breaks this lookup. Write that upstream's
`is_podcast_namespace` matches by URI, so an alternate prefix does not
affect upstream extraction. Check this claim against the `rss` crate source
cited above before you write it as fact.

## Acceptance Criteria

### Mechanical Criteria

- The file `docs/schema/adr-0075-identity-syntax-contract.md` exists.
- The deliverable contains each required heading in Deliverable, in the
  listed order.
- The `## Supported podcast:txt Purpose Values` section contains a minimum
  of one fenced code block with a checked `purpose="npub"` example.
- The `## Compatibility Decision For purpose="nostr"` section states
  "supported" or "not supported", plus a reason.
- The `## Current Behavior Compared With The Upstream Parser` section names
  both `src/rss/enrich.rs::nostr_from_extension` and the upstream function
  it compares.
- Each relative link in the deliverable resolves to a file or an anchor
  that is present.
- The shared STE command in Checks reports no unresolved error for the deliverable.

### Review Criteria

A reviewer checks that the NIP-19 description agrees with the cited page.
A reviewer checks that the purpose-value table agrees with the real
behavior of the cited upstream function.
A reviewer checks that the compatibility decision states a clear reason,
and not only a preference.
A review check that did not run reports this gate as open, not as met.

## Checks

Do these checks against the deliverable after you write it.
No application test applies. This packet changes no code.

```bash
# From the repository root. Confirms every relative markdown link resolves.
grep -oE '\]\([^)]+\)' docs/schema/adr-0075-identity-syntax-contract.md \
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
  --check --no-heuristics docs/schema/adr-0075-identity-syntax-contract.md
```

## Escalation Triggers

Stop and report if any of these happen:

- A cited function is no longer present, or its behavior no longer matches
  this packet, at commit `a220f44`.
- The cited pages state a rule that conflicts with this packet's Required
  Content.
- You find no source or example to support a compatibility decision for
  `purpose="nostr"`.
- An accurate comparison might need a code change or a live network
  request.

Revise this packet, or ask the operator, before you invent a fact to fill a
gap.

## Expected Report

Report the deliverable's file path and its section list.
Report the compatibility decision for `purpose="nostr"` and its stated
reason.
Report the checks you did and their results.
Name each fact you could not check in the cited source or page.
Name each deviation from this packet's Required Content.

## Prompt for lower-context model

You write one bounded document from a larger plan. Do not write code. Do not
redesign ADR 0075.

Read:

- This packet and each file in its Files To Inspect section.

Goal:

- Write `docs/schema/adr-0075-identity-syntax-contract.md` with the
  sections listed in Deliverable.

Constraints:

- Apply this packet's Constraints and Required Content sections.
- Decide the `purpose="nostr"` compatibility question inside the
  deliverable. Record the reason.
- Write each sentence in ASD-STE100 Simplified Technical English.

Do not touch:

- Each boundary named in this packet's Do Not Touch section.

Acceptance criteria:

- Pass each mechanical criterion above.
- Leave each review criterion for a person. Report it as open.

Checks:

- Do the commands in this packet's Checks section.

At the end, report:

1. deliverable file path and section list
2. the purpose="nostr" decision and its reason
3. checks done and their results
4. facts you could not check
5. deviations from this packet
