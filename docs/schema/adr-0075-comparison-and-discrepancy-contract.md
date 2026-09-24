# ADR 0075 Comparison And Discrepancy Contract

> Superseded in part by [ADR 0076](../adr/0076-playlist-rss-check-for-stale-musicindex-records.md) on 2026-09-24.
> The provider priority, freshness, expiry, stale-label and retained-discrepancy parts of this document are not in force.
> The app stores one value for each field, and the ADR 0076 playlist RSS check replaces it.
> The extraction orders, placeholder rules, date and duration rules, URL action rules, fallback sections and the readable-text comparison stay in force.

## Status

Technical review passed - 2026-09-20. URL comparison and action policies have individual acceptance on 2026-09-21.
This document supplies packet 035's concrete comparison and lifecycle rules.
Decisions F and G require RSS priority and retained discrepancy evidence. Implementation and inherited visual gates remain open.

## Scope And Inputs

Compare only feed descriptions, track descriptions, feed websites, and track page links.
The subject key includes the feed GUID and, for a track, its track GUID.
The provider key includes the provider kind and its resource or endpoint.
Never compare a feed fallback with a track assertion.

Each observation retains the original value, extraction path, source assertion, available source times, and actual fetch time.
Preserve the original response before decoding, cleaning, or fallback.
Unknown source times stay unknown. A fetch time does not become a source observation time.

Each field has one coverage state: known value, known absent, or unknown.
An omitted include, summary response, failed request, unsupported contract, or unknown owner supplies unknown coverage.
Only a verified field contract can establish known absence.
A nullable DTO field alone does not establish it.

## Description Comparison, Version 1

Record each value's representation as plain text, HTML, or unknown, using the supplying field's verified contract.
Do not infer HTML from angle brackets or entity-like text.
The inspected Index parser removes tags without inserting word boundaries.
It decodes numeric entities and five named entities: `amp`, `lt`, `gt`, `quot`, and `apos`.
That scalar is plain text under the inspected contract. A live endpoint still needs compatible contract evidence.

1. Keep plain text unchanged before whitespace normalization. Do not decode its entities or parse it as HTML.
2. Parse HTML with a standards-based HTML parser. Do not strip tags with a regular expression.
3. Decode named and numeric character references once during HTML parsing.
4. Ignore comments and the contents of `script`, `style`, and `template` elements.
5. Join inline text without adding spaces between adjacent inline elements.
6. Insert a word boundary for block elements and `br` elements.
7. Reduce consecutive Unicode whitespace to one space. Remove leading and trailing whitespace.
8. Preserve case, punctuation, word order, and other Unicode characters.

The comparison value is derived evidence. Store both original strings unchanged.
An unknown representation, HTML parser limit, or rejected input produces an unknown comparison result.
They do not create or resolve a discrepancy.
Keep the comparison version with its result so later parser changes cannot silently erase history.

| RSS HTML | Index plain text | Result |
|---|---|---|
| `<p>Hello <b>world</b></p>` | `Hello world` | Equal |
| `A&nbsp;B` | `A B` | Equal |
| `<p>A</p><p>B</p>` | `A B` | Equal |
| `inter<b>national</b>` | `international` | Equal |
| `A B` | `AB` | Different |
| `Song` | `song` | Different |
| `&lt;literal&gt;` | `<literal>` | Equal. Plain text must not undergo an HTML parse |
| `A &amp;amp; B` | `A &amp; B` | Equal. Plain text must not undergo entity decoding |
| `<p>A</p><p>B</p>` | `AB` | Different. The inspected Index transform removes the word boundary |
| `A&nbsp;B` | `A&nbsp;B` | Different. The inspected Index transform retains the literal named entity |

The last two rows use actual outputs of the inspected upstream transform.
The comparator must retain those differences. Reparsing Index text would conceal its original output.

## Website And Page Comparison, Version 1

Feed website actions, accepted separately on 2026-09-21: allow only valid HTTP or HTTPS URLs.
Retain other schemes as source evidence without a website action.
Track page actions, accepted separately on 2026-09-21: allow only valid HTTP or HTTPS URLs.
Retain other schemes as source evidence without a page action.

Feed website comparison, accepted separately on 2026-09-21: normalize scheme, host, and default ports.
Preserve path, query, and fragment differences. The rules below implement this acceptance for feed websites.
Track page comparison, accepted separately on 2026-09-21: apply the same scheme, host, and default-port normalization.
Preserve path, query, and fragment differences. The technical rules below apply to both fields.

Remove outer whitespace and parse a URL before comparison.
Use the parser's normalized scheme, host, and default-port representation.
Keep path case, meaningful trailing slashes, query order, query values, and fragments.

Do not remove tracking parameters or perform network requests to infer URL equality.
Invalid URLs produce an unknown comparison result and remain available as raw evidence.

## Selection And Freshness Proposal

For a matching owner and field, use a usable fresh RSS value before a usable fresh Index value.
For RSS descriptions, use the direct owner's description. Do not inherit a description into a track assertion.
For Index descriptions, use an owner-matching description claim by supplied position, then the scalar value.
Conflicting claims with missing or tied positions remain unresolved alternatives.

Accepted separately for feed and track descriptions on 2026-09-20: prefer the sourced description over the plain Index description field.
Use declared source order for multiple sourced descriptions. Report conflicting ties as unresolved.

For RSS websites and pages, use direct `link`, then a supported Atom alternate link for an item page.
For Index links, use the owner and kind, then the supplied position.

Accepted on 2026-09-20: fresh verified RSS absence hides retained Index descriptions, feed websites, and track page links.
Keep the earlier values and discrepancy evidence. Do not select the retained Index value as a fallback in this case.
The operator separately confirmed the feed-description case. ADR 0075 Decision H records that acceptance.

For all four fields, fresh verified Index absence hides the stale RSS value and retains its evidence.
The operator accepted this rule separately for each field on 2026-09-20.
Track pages prefer the direct item page link before a supported Atom alternate link, accepted separately on the same day.

Packet 018 must set the numeric freshness limit after measuring the request baseline.
Use monotonic time for an active cache and recorded UTC fetch time for durable evidence.
A future wall-clock timestamp cannot establish freshness after restart.
Explicit refresh bypasses expiry and starts a newer request generation.
A response from an older generation cannot replace a newer accepted observation.

A failed refresh preserves facts and separately reports the failure.
If both providers are stale, retain the last selected field state with its stale label.
The operator accepted this rule separately for feed descriptions, track descriptions, feed websites, and track page links on 2026-09-20.
An earlier selected absence stays absent. Expiry must not resurrect a value that a verified observation removed.
If no source supplies a usable value, expose no value or action.

Request work must measure the five cases in the phase plan before changing scheduling.
This interface adds no configuration key and changes no audio-tag policy.

## Discrepancy Identity And Lifecycle

Use the subject, field, and ordered RSS/Index provider pair with both requested resources as the discrepancy key.
Do not put values or fetch times in that key.
Keep the RSS resource separate from the Index endpoint and requested resource.
Record the comparison version on every compared pair and state transition.

A comparison-version change alone does not create, resolve, or duplicate a discrepancy.
A new successful comparable observation pair can change the existing record under the new version.
Preserve prior versions and their evidence when recording that transition.

| Observation pair | Effect |
|---|---|
| Two comparable values differ | Create or reactivate the discrepancy. Preserve both original values and provenance |
| The same mismatch repeats | Update the latest observation times on the same discrepancy |
| A different mismatch replaces the previous pair | Retain the previous evidence and record the new pair |
| Comparable observations later agree | Mark resolved and retain the conflict evidence plus the resolving pair |
| Both fields are verifiably absent | Resolve the discrepancy with explicit absence evidence |
| Only one field is verifiably absent | Record the absent/value discrepancy |
| Either field has unknown coverage or failed comparison | Preserve existing discrepancy state without creating or resolving a mismatch |

Record transitions when content or state changes. Repeated equal content does not require duplicate history rows.
Store sufficient evidence in each transition to survive replacement of provider snapshots.
Restart, cache expiry, and endpoint changes must not erase existing discrepancy records.
An endpoint change creates a new provider pair. It cannot resolve a discrepancy for the old pair.

Only current, successful, comparable observations can change active/resolved state.
A discrepancy establishes disagreement. It does not establish which provider is stale.
Expose typed discrepancy data for future use. This contract creates no update hook or outbound request.

## Implementation Checks

The comparison tests must cover the examples above and malformed input.
Storage tests must cover repeat, changed pair, resolution, reactivation, restart, and snapshot replacement.
Coverage tests must distinguish explicit absence from omission, failure, summary coverage, and an unsupported contract.
Tests must reject older response generations and keep providers isolated after an endpoint change.
Version-upgrade tests must retain one discrepancy identity and preserve prior comparison evidence.

## Related Work

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md), Decisions F and G.
- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md), packets 011, 018, 020, 035, and 036.
- [Field inventory](adr-0075-metadata-field-inventory.md).

## Operator Visual Check

This contract requests no app launch. Future presentation changes retain their own visual acceptance gate.
