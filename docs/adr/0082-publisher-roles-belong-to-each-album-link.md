# ADR 0082: Publisher Roles Belong To Each Album Link

## Status

Proposed - 2026-10-02. The operator approved the design on 2026-10-02. This ADR is not binding until the operator accepts it.

Held on 2026-10-02: the operator waits for Stophammer 0.7.0. Stophammer task 014 of ADR 0049 removes the credit link.
Then `album_names_as` gives only `publisher` or null.
It also adds `role_agreement` (`both`, `one_side`, `conflict` or null), and a `conflict` link stops being a confirmed link. Decisions 2, 3 and 6 must be revised against the 0.7.0 contract before acceptance.

When accepted, it supersedes [ADR 0078](0078-publisher-page-type-from-stated-role.md), and it amends [ADR 0077](0077-publisher-feed-artist-binding.md) Decision 2.

## Context

ADR 0078 gives a publisher page one type. The page is a label page when one owned album states the `label` role, and an artist page otherwise.

The operator stated on 2026-10-02 that one feed can have more than one role at the same time. A publisher feed can be the artist on some albums and the distributor of others. A page with one type cannot show that.

Stophammer 0.6.0 is deployed on 2026-10-02. It gives these facts:

- `role` is the role set of one album link, sorted and joined by `", "`, for example `label, producer` (Stophammer ADR 0049 §6). It is null when no feed states one. `role_source` is then `default`.
- `album_names_as` on each `publisher` row tells how the album names the publisher feed: `publisher`, `credit` or null (Stophammer ADR 0069 §3). An album names one publisher and zero or more credits.
- `co_credited_feeds` on a publisher read lists other publisher feeds, with their `roles` and their `album_count` (Stophammer ADR 0069 §4).
  Each listed feed shares a confirmed album with this feed.
- The node derives no type. The client derives it (Stophammer ADR 0069 §5).

On 2026-10-02 the DETOX publisher feed gave 30 rows with `album_names_as` `publisher`, a null `role`, `role_source` `default`, and an empty `co_credited_feeds`.

## Decision

### 1. A Page Has No Type

A publisher page shows no single type. It shows the roles that its album links state.

### 2. Albums Group By How They Name The Feed

The page groups its albums by `album_names_as`:

| Group | Albums |
|---|---|
| Publisher of | `album_names_as` is `publisher` |
| Credited on | `album_names_as` is `credit` |
| Listed, not named | `album_names_as` is null: the publisher feed lists the album, and the album does not name the feed |

An album appears in one group, one time.

### 3. Each Album Row Shows Its Role Set

Each row shows the role set of its own link, with its source: "stated by album" for `music_rel`, "stated by publisher" for `publisher_rel`.
A null `role` with `role_source` `default` shows "Role not stated". A conflict shows the two stated sets, and neither wins (ADR 0077 Decision 3).

### 4. The Header Lists The Stated Roles

The header shows the publisher feed title and the distinct roles that the album links state, for example "States: artist, distributor".
A page with no stated role shows no role list. The confirmed and unconfirmed artists of ADR 0077 packet 007 stay.

### 5. Shares Albums With

The page shows the entries of `co_credited_feeds`, each with its title, its roles and its album count. Each entry opens the publisher page of its feed.
The section is absent when the list is empty.

### 6. The Album Page Shows Its Publisher And Its Credits

An album page shows the publisher that the album names, and each credit apart from it, each with its role set.
ADR 0077 Decision 2 stays for the publisher. A credit is an album link, not ownership: the album page does not show a credit as its artist.

## Consequences

- A feed with more than one role shows each role with the albums that state it.
- The page derives nothing that the feeds do not state. A count or a name never selects a role.
- ADR 0078 and its label-page rule are superseded.

## Alternatives Considered

- Keep one page type, from a role set that contains `label`. Rejected by the operator: a feed can be an artist and a label at the same time.
- An album in each group of its roles. Rejected: one album then shows two or more times.
- Derive a type from `co_credited_feeds`. Rejected: the node states that it derives no type, and the list rests on links, not on a stated role.

## Verification

Mechanical, phrased at the owning layer:

- The page view model exposes no page type.
- The page view model groups albums by `album_names_as`, one group for each value, with each album one time.
- An album row exposes its role set and its source, or "Role not stated" for a null role with `role_source` `default`.
- The header exposes the distinct stated roles.
- The view model exposes one entry for each `co_credited_feeds` item, with an action that opens its publisher page.
- The album view model exposes its publisher and its credits apart.

Visual, for the operator after the visual pause ends:

- A publisher page shows the three groups, the stated roles in the header, and "Shares albums with" when the list has entries.
- An album page shows its publisher and its credits apart.
