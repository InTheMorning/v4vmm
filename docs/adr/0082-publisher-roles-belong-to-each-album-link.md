# ADR 0082: Publisher Roles Belong To Each Album Link

## Status

Accepted - 2026-10-02. The operator approved the design and accepted the revised text on 2026-10-02.

Revised on 2026-10-02 against Stophammer 0.7.0, after the operator held it for that release. The credit link of Stophammer 0.6.0 is gone from this design.

It supersedes [ADR 0078](0078-publisher-page-type-from-stated-role.md), and it amends [ADR 0077](0077-publisher-feed-artist-binding.md) Decision 2.
ADR 0078 moves to the archive in the commit of [packet 002](../tasks/adr-0082-task-002-publisher-page-role-groups.md), which deletes its page type code.

## Context

ADR 0078 gives a publisher page one type. The page is a label page when one owned album states the `label` role, and an artist page otherwise.

The operator stated on 2026-10-02 that one feed can have more than one role at the same time. A publisher feed can be the artist on some albums and the distributor of others. A page with one type cannot show that.

Stophammer 0.7.0 is deployed on 2026-10-02, at commit `5ba3d1f`. It follows the link rules of podcast-namespace PR #793 (Stophammer ADR 0049 §6 and §6a, and ADR 0069 §1a):

- An album has one publisher link: the item inside `<podcast:publisher>`, or the first bare publisher item of an album with no wrapper.
- `album_names_as` on each `publisher` row gives `publisher` or null. Null means that the publisher feed lists the album, and the album does not name the feed.
- `role` is the role set of one link, sorted and joined by `", "`, for example `label, producer`. It is null when no side states one, and null on a conflict.
- `role_agreement` gives `both`, `one_side`, `conflict`, or null when no side states a role. A `conflict` row is not a confirmed link, and no Stophammer count uses it.
- `role_source` gives `publisher_rel`, `music_rel`, `default` or `conflict`.
- `co_credited_feeds` on a publisher read lists other publisher feeds that share a confirmed album, with their `roles` and their `album_count`.
- The node derives no type. The client derives it (Stophammer ADR 0069 §5).

On 2026-10-02 the DETOX publisher feed gave 30 rows with `album_names_as` `publisher`, a null `role` and a null `role_agreement`.
Each row gave `role_source` `default`, and `co_credited_feeds` was empty.

The 0.7.0 contract text of `album_names_as` still names `credit`. The Stophammer release record states `publisher` or null.

## Decision

### 1. A Page Has No Type

A publisher page shows no single type. It shows the roles that its album links state.

### 2. Albums Group By How They Name The Feed, Then By Role

The page groups its albums first by `album_names_as`:

| Group | Albums |
|---|---|
| Publisher of | `album_names_as` is `publisher` |
| Listed, not named | `album_names_as` is null |

Inside each group, the albums group by the role of their link:

| Subgroup | Rows |
|---|---|
| The agreed role set, for example "artist" or "label, producer" | `role_agreement` is `both`. One subgroup for each different role set |
| Role stated by one side | `role_agreement` is `one_side` |
| Roles differ, not confirmed | `role_agreement` is `conflict` |
| Role not stated | `role_agreement` is null |

An album appears one time. A subgroup with no album is absent.

### 3. Each Album Row Shows Its Role And Its Source

- An agreed row shows its role set and "agreed by both feeds".
- A one-side row shows the stated set and its side: "stated by the album only" for `music_rel`, or "stated by the publisher only" for `publisher_rel`.
- A conflict row shows the two stated sets. Neither wins (ADR 0077 Decision 3), and the row says that the link is not confirmed.
- A row with no stated role shows "Role not stated".

### 4. The Header Lists The Agreed Roles

The header shows the publisher feed title and the different roles of the agreed rows, for example "Roles: artist, distributor".
A page with no agreed row shows no role list. The confirmed and unconfirmed artists of ADR 0077 packet 007 stay.

### 5. Shares Albums With

The page shows the entries of `co_credited_feeds`, each with its title, its roles and its album count. Each entry opens the publisher page of its feed.
The section is absent when the list is empty.

### 6. The Album Page Shows Its Publisher Link

An album page shows the publisher that the album names, with the role, the agreement and the source of that link, as Decision 3 gives.
ADR 0077 Decision 2 stays: the album binds to the publisher that it names. A role never makes the album page show the publisher as its artist.

## Consequences

- A feed with more than one role shows each role with the albums whose link states it.
- The page derives nothing that the feeds do not state. A count or a name never selects a role.
- A conflict link stays visible, and the page does not treat it as confirmed.
- ADR 0078 and its label-page rule are superseded.

## Alternatives Considered

- Keep one page type, from a role set that contains `label`. Rejected by the operator: a feed can be an artist and a label at the same time.
- Show `role` only when `role_agreement` is `both`, as the Stophammer request record advises. Rejected: a one-side role is an RSS statement, and Provenance First shows it with its side.
- An album in each subgroup of its roles. Rejected: one album then shows two or more times.
- Derive a type from `co_credited_feeds`. Rejected: the node states that it derives no type.

## Verification

Mechanical, phrased at the owning layer:

- The page view model exposes no page type.
- The page view model groups albums by `album_names_as`, then by the role subgroups of Decision 2, with each album one time.
- An album row exposes the role text and the source text of Decision 3 for each `role_agreement` value.
- The header exposes the different roles of the agreed rows.
- The view model exposes one entry for each `co_credited_feeds` item, with an action that opens its publisher page.
- The album view model exposes its publisher link with the role, the agreement and the source.

Visual, for the operator after the visual pause ends:

- A publisher page shows the groups and subgroups, the agreed roles in the header, and "Shares albums with" when the list has entries.
- An album page shows its publisher with the role and its source.
