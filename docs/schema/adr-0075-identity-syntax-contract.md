# ADR 0075 Identity Syntax And Purpose Contract

## Scope

This document is the identity syntax and purpose contract for ADR 0075 packet 003.
It states which Nostr identity syntax forms this app supports.
It states where each supported form may appear.
A form may appear at the feed, at the track, or at the contributor occurrence.
This document changes no parser code and no application code.

A statement labeled "Proposed" needs operator acceptance before a code packet may implement it.
A statement labeled "Current behavior" describes code that exists today, with its cited file and function.
Do not read a current-behavior statement as an accepted rule.

## Evidence

| Source | Revision | Use |
|---|---|---|
| This repository, working tree | Commit `d3c6ee4` | Current app behavior |
| `/home/citizen/build/stophammer` | Commit `a220f44` | Current upstream parser behavior |
| `rss` crate | Version 2.0.12, pinned by `Cargo.lock` | Current XML extension behavior |
| [NIP-19](https://github.com/nostr-protocol/nips/blob/master/19.md) | Current specification, retrieved 2026-09-19 | Bech32 encoding specification and checked vectors |
| [The `podcast:txt` tag](https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/txt.md) | GitHub commit `588a45e`, dated 2026-03-09 | `podcast:txt` tag specification |
| [The `podcast:person` tag](https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/person.md) | GitHub commit `588a45e`, dated 2026-03-09 | `podcast:person` tag specification |

This document records no request to a live MusicIndex service.
It records no request to a live publisher feed.
No agent launched the app to write this document.

## NIP-19 Encodings

[NIP-19](https://github.com/nostr-protocol/nips/blob/master/19.md) defines `npub` as a bech32 encoding of a bare 32-byte public key.
An `npub` value holds no other field.
NIP-19 defines `nprofile` as a bech32 encoding of a type-length-value (TLV) structure.
An `nprofile` value contains the profile public key.
An `nprofile` value can also contain one or more relay hints.
NIP-19 states that the relay field type "may be included multiple times."

[ADR 0075, Decision 3](../adr/0075-metadata-ownership-and-completeness.md#3-use-explicit-field-rules)
states: "The parser must preserve that distinction."
The app must not treat `npub` and `nprofile` as one interchangeable form.

Current behavior does not keep this distinction in one path.
`src/rss/enrich.rs::enrich_track_from_feed_rss`, lines 99 and 102, calls
`append_track_source_id` and `append_feed_source_id` with the fixed scheme `nostr_npub`.
The app assigns this scheme to any match from the `npub1` prefix or the `nprofile1` prefix.
The stored record does not keep which prefix matched.
This is a current gap against the stated rule. It is not a proposal.

## Supported podcast:txt Purpose Values

Upstream `stophammer-parser/src/engine.rs::extract_entity_ids`, line 802, reads only
direct child elements of the passed node.
It reads only elements named `txt` in the Podcast Namespace.
For each such element, it reads the `purpose` attribute.
It trims the value and converts it to lowercase.

It accepts the value `npub`. It maps that value to the scheme `nostr_npub`.
Line 814 drops every other `purpose` value from the returned list.

Checked example, traced against the cited function:

```xml
<item xmlns:podcast="https://podcastindex.org/namespace/1.0">
  <guid>item-1</guid>
  <podcast:txt purpose="npub">npub10elfcs4fr0l0r8af98jlmgdh9c8tcxjvz9qkw038js35mp4dma8qzvjptg</podcast:txt>
</item>
```

Resulting `stophammer-parser/src/types.rs::IngestEntityId`, line 149:

```json
{
  "position": 0,
  "scheme": "nostr_npub",
  "value": "npub10elfcs4fr0l0r8af98jlmgdh9c8tcxjvz9qkw038js35mp4dma8qzvjptg"
}
```

The raw element still exists after the drop.
`extract_podcast_namespace_from_node`, line 1010, scans all descendants in the Podcast Namespace.
It keeps the raw tag, path, attributes and text of every such element.
A dropped `purpose` value therefore stays in this raw snapshot.
It stays absent from the typed `entity_ids` list.

The [`podcast:txt` specification](https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/txt.md)
lets each service pick its own `purpose` values.
It gives a list of values in common use. That list does not include `npub` or `nostr`.
This document does not claim that the specification requires either word.

## Compatibility Decision For purpose="nostr"

Decision: **not supported**. The operator decided this on 2026-09-19.
The app supports the purpose value `npub` only.
The app treats `purpose="nostr"` as unsupported syntax and keeps the value as evidence.
[ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) records this as Decision D.

The operator rejected the compatibility proposal below. The reasons are kept as a record:

- `src/rss/enrich.rs`, lines 99 and 102, already writes the fixed extraction path
  `podcast:txt@purpose=nostr` for every RSS Nostr match it stores today.
  This app-chosen label uses the concept name, not the encoding prefix `npub`.
- The [`podcast:txt` specification](https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/txt.md)
  leaves `purpose` values open to each service and states no fixed list.
  Adding `nostr` as a synonym does not break the specification.
- Upstream's `extract_entity_ids` accepts only the literal word `npub` today.
  A synonym rule guards against silent loss if a feed uses the other spelling.

No cited source shows a real feed that publishes `purpose="nostr"`.
The reasons above support the proposal. They do not prove that spelling is in real use.

Consequence for a feed that already publishes `purpose="nostr"`:
Upstream's `extract_entity_ids` currently drops that row, because it checks only the word `npub`.
The app's current RSS scan does not check the `purpose` attribute at all.
This scan can currently store a `purpose="nostr"` value by accident.

The Current Behavior section below names the function that causes this accidental match.
Packet 009 must narrow that scan to a direct `podcast:txt` element with the purpose `npub`.
Packet 009 must treat a `purpose="nostr"` value as unsupported syntax.

The fixed extraction path `podcast:txt@purpose=nostr` in `src/rss/enrich.rs`, lines 99 and 102,
names no supported purpose value. Packet 009 must replace that label.

## The podcast:person npub Attribute

The `npub` attribute belongs to one `podcast:person` element.
Upstream's `extract_persons`, line 779, reads the `npub` attribute from a direct `person` child.
`stophammer-parser/src/types.rs::IngestPerson`, line 129, keeps the `npub` field beside
`name`, `role`, `group_name`, `href` and `img` on that one occurrence.

Rule: a person's key belongs to that one contributor occurrence.
It never belongs to the track or the feed that holds it.

The [`podcast:person` specification](https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/person.md)
lists four attributes: `role`, `group`, `img` and `href`.
It does not list `npub` as a documented attribute.
The `npub` attribute is a convention that this app and the upstream parser both read.
It is not a word from the cited specification page.

`src/rss/subscribe.rs::contributor_inputs_from_extensions`, line 390, reads the app's own
`npub` value from a direct `person` child, at line 408.
`clean_attr`, line 462, copies that raw attribute text. It applies no prefix check.

The review section
[RSS Extraction Can Assign The Wrong Owner](../reviews/adr-0075-metadata-contract-review.md#rss-extraction-can-assign-the-wrong-owner)
records an observed break of the ownership rule.
`src/rss/enrich.rs::nostr_from_extension` can read a `podcast:person` npub attribute.
It can assign that value to the enclosing track or feed as a `podcast:txt` fact.
That assignment moves the key away from its one contributor occurrence.

## Valid Positions For podcast:txt

Rule: the channel gives a feed identity through its direct child `podcast:txt` element.
The item gives a track identity through its direct child `podcast:txt` element.
A deeper descendant value does not establish a track or feed identity under this contract.

This rule is grounded in upstream's `extract_entity_ids`, line 802.
That function calls `node.children()`. It does not run a descendant search.
It reads only the direct children of the passed channel or item node.

Current behavior in this app does not yet match this rule for the RSS scan path.
`src/rss/enrich.rs::nostr_from_extensions`, line 320, reads every element name under the
`podcast` key.
`nostr_from_extension`, line 327, then recurses into every child of every such element.
This scan therefore reads values from deep descendants, not only direct children.
See Current Behavior Compared With The Upstream Parser below for the full comparison.

## Prefix Validity Without Content Validity

[ADR 0075, Decision 3](../adr/0075-metadata-ownership-and-completeness.md#3-use-explicit-field-rules)
states: "A recognized prefix alone does not validate a value."

Example: the value `npub1notavalidkey` keeps the `npub1` prefix.
Its length and its checksum are not valid.
`src/rss/enrich.rs::extract_nostr_handle`, line 365, accepts this value.
It checks only the prefix. It applies no length check and no checksum check.
A search of `src/` for the word `bech32` returns no result in this repository.

The app keeps this value as evidence.
ADR 0075, Decision 3, also states: "Do not treat them as validated identifiers for identity actions."

Current behavior does not yet enforce that rule everywhere.
`src/view_models/entity_detail.rs::identity_actions`, line 1095, and `nostr_npub`, line 1141,
offer a Nostr action for any non-empty stored value.
Neither function checks the prefix, the length or the checksum of that value.
This is a current gap against the stated rule. It is not a proposal.

## Unsupported Syntax

Under this contract, the app does not support these forms as an identity.

- An `nsec`, `note`, `nevent`, `naddr` or `nrelay` value.
  NIP-19 assigns the `nsec` prefix to a private key.
  `extract_nostr_handle`, line 365, checks only for the `npub1` and `nprofile1` prefixes.
  It does not match any of these five other prefixes.
  The app writes no record of such a value through the Nostr extraction path.
- A `purpose` value other than `npub`. Decision D rejects `nostr` compatibility.
  Upstream's `extract_entity_ids`, line 814, drops such a value from `entity_ids`.
  The raw element stays in upstream's general Podcast Namespace snapshot.
  See Supported podcast:txt Purpose Values above.
  This repository's own RSS code keeps no equivalent general raw record of such a value.
- A value shaped like `npub`, found outside a direct `podcast:txt` or `podcast:person` element.
  Upstream's direct-children rule excludes it from `entity_ids`.
  The Current Behavior section below describes how the app's RSS scan can still match this
  shape today.
  That match is an observed gap. It is not a supported form under this contract.

For each of these forms, the app must not offer the raw value as an identity action.
The app may keep the raw value as evidence, where a cited function already retains it.

## Current Behavior Compared With The Upstream Parser

`src/rss/enrich.rs::nostr_from_extension`, line 327, differs from upstream's
`extract_entity_ids` and `extract_persons` in scope and in trigger condition.

Scope: the app scans all extensions under the `podcast` key, including their children.
This scan includes elements other than `txt`, such as `person`, `transcript` and `value`.
Upstream's `extract_entity_ids` reads only direct `txt` children.
Upstream's `extract_persons` reads only direct `person` children.

Trigger: the app matches the `npub1` or `nprofile1` prefix in any text or attribute value.
It does not require a `purpose` attribute at all.
Upstream requires a `purpose` attribute equal to the word `npub`, after trim and lowercase.

Fixed label: `enrich_track_from_feed_rss`, lines 99 and 102, marks every match with the
fixed extraction path `podcast:txt@purpose=nostr`.
It writes this label apart from the element or the attribute that supplied the value.
A match from a `podcast:person` npub attribute therefore receives a `podcast:txt` label it
did not earn.

Correction required in packet 009:

- A track or feed Nostr identity must come from a direct `podcast:txt` element with a
  supported `purpose`.
- A `podcast:person` attribute cannot become a `podcast:txt` fact for the enclosing track
  or feed.

Second difference, for the record:
`src/rss/enrich.rs` looks up extensions by the literal key `"podcast"`, for example through
`exts.get("podcast")`.
The `rss` crate, version 2.0.12, keys an unknown extension by its declared XML prefix.
`rss-2.0.12/src/extension/util.rs::extension_name`, line 41, splits the element name at the
colon. It returns that prefix as the key.
`rss-2.0.12/src/item.rs`, line 670, and `rss-2.0.12/src/channel.rs`, line 1312, both store
the element under that literal prefix.
The crate resolves only the Atom, iTunes and Dublin Core namespaces to their namespace URI.

If a feed declares an alternate prefix, such as
`xmlns:pc="https://podcastindex.org/namespace/1.0"`, the app's lookup `exts.get("podcast")`
finds no match.
Upstream's `is_podcast_namespace`, line 535, compares the resolved namespace URI, not the
declared prefix.
An alternate prefix therefore does not affect upstream extraction.
This claim is checked against the `rss` crate source cited above.

## Checked Encoding Vectors

These public examples come from [NIP-19](https://github.com/nostr-protocol/nips/blob/master/19.md), retrieved on 2026-09-19.
Their decoded values are checked locally against the expected public keys below.

| Encoding | Expected public key |
|---|---|
| `npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkwsyjh6w6` | `3bf0c63fcb93463407af97a5e5ee64fa883d107ef9e558472c4eb9aaaefa459d` |
| `npub10elfcs4fr0l0r8af98jlmgdh9c8tcxjvz9qkw038js35mp4dma8qzvjptg` | `7e7e9c42a91bfef19fa929e5fda1b72e0ebc1a4c1141673e2794234d86addf4e` |
| `nprofile1qqsrhuxx8l9ex335q7he0f09aej04zpazpl0ne2cgukyawd24mayt8gpp4mhxue69uhhytnc9e3k7mgpz4mhxue69uhkg6nzv9ejuumpv34kytnrdaksjlyr9p` | `3bf0c63fcb93463407af97a5e5ee64fa883d107ef9e558472c4eb9aaaefa459d` |

The profile vector also contains relay hints `wss://r.x.com` and `wss://djbas.sadkb.com`.
A valid profile encoding remains distinct from a bare public-key encoding.
These vectors add no supported purpose value and authorize no scheme conversion.
The corpus's successful cases use the two valid public keys. C09 retains its invalid value.

Run the situational ADR 0075 fixture guard from the repository root:

```bash
python3 docs/runbooks/check-adr0075-identity-examples.py
```

The guard checks checksum, decoded length, padding, profile TLV structure, and the expected public keys.
It rejects malformed positive fixtures. It allows the named invalid fixture only in the negative sections.
This check does not implement application identity validation or prove parser ownership behavior.

## Retained Terms

| Retained term | Meaning |
|---|---|
| purpose | The `podcast:txt` attribute that names the free-form use of the element |
| scheme | The typed category label this app and the parser assign to an extracted identity value |
| prefix | The leading bech32 characters, such as `npub1`, that mark an entity type under NIP-19 |
| entity occurrence | One credit or one identity value in one source observation |
| snapshot | A stored copy of all Podcast Namespace elements from one parsed document |
| evidence | Source material that supports a recorded claim |

Do not replace these terms with unrelated dictionary alternatives.

## References

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md)
- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md)
- [Review](../reviews/adr-0075-metadata-contract-review.md)
- [Task 003 packet](../tasks/adr-0075-task-003-nostr-syntax-and-purposes.md)
- [Example corpus, cases C02, C03, C09 and C10](adr-0075-metadata-example-corpus.md)
- [NIP-19](https://github.com/nostr-protocol/nips/blob/master/19.md)
- [The `podcast:txt` tag](https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/txt.md)
- [The `podcast:person` tag](https://github.com/Podcastindex-org/podcast-namespace/blob/main/docs/tags/person.md)
