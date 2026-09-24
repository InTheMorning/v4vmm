# ADR 0078: Publisher Page Type From Stated Role Only

## Status

Accepted - 2026-09-24. The operator gave this decision on 2026-09-24, during the review of the ADR 0077 proposals.
Implementation has not started.

This ADR supersedes Decision 4 of [ADR 0077](0077-publisher-feed-artist-binding.md) only. The other ADR 0077 decisions stay in force.

Amended 2026-09-24: the operator added the page rule for albums with different roles. The Decision section records it.

## Context

ADR 0077 Decision 4 selects the type of a publisher page from a stated role.
Without a stated role, it selects the type from `distinct_release_artist_count`.
The first ADR 0077 proposal gave a label page for a count of two or more.

Stophammer derives that count from the `itunes:author` text of the albums that name the publisher.
A "feat." credit or a spelling difference can make one artist count as two.
Thus, the count can show one artist as a label.

Two known publisher feeds state a role with `rel`, on 2026-09-24:
Sir Libre Records states `rel="label"`, and Jimmy V states `rel="artist"` and `rel="producer"`.
Other publisher feeds state no role. Stophammer then reports `role = artist` with `role_source = default`.

ADR 0075 Decision I makes RSS the only provenance. A stated `rel` is RSS. The count is a value that MusicIndex derives.

## Decision

A publisher page is a label page only when a feed states a label role.
Each other publisher page is an artist page.

Stophammer reports a role for each album and publisher pair. One owned album with a stated `label` role is sufficient.
An owned album is an album with `music_names_publisher = true`. A pair with `role_source = "conflict"` does not count.
Each album row on the page shows the stated role of its own pair.
This includes a page with no stated role, with `role_source = default`.

`distinct_release_artist_count` never selects the page type.
The page can show the count and `distinct_release_artists` as information. It labels them as derived.

The ADR 0077 proposals for stated role values and for a role conflict stay open. They select which stated values give a label page.

## Alternatives Considered

- A count of two or more selects a label page. Rejected: a "feat." credit or a spelling difference can show one artist as a label.
- A count of three or more selects a label page. Rejected: the count stays a derived value, and a label with two artists shows as an artist page.
- v4vmm normalizes the artist names before it counts. Rejected: v4vmm would then own a text rule on derived data. Stophammer owns that normalization.
- Without a stated role, the page shows no type. Rejected: each page needs one layout. An artist page is the default of the RSS convention that the research note records.

## Consequences

Positive:

- The page type comes from RSS only. No derived text value changes it.
- One artist with a "feat." album never shows as a label.

Negative:

- A label that states no `rel` shows as an artist page. On 2026-09-24, most publisher feeds state no `rel`.
- A label page appears only when its publisher writes a non-standard `rel`.

## Invariants

- No count and no name list selects the page type.
- A page without a stated label role is an artist page.
- Only an owned album can make a page a label page. A "listed by" album cannot.

## Acceptance Criteria

Mechanical:

- The page view model selects the label type only from a stated label role.
- A test shows that a count of two or more with `role_source = default` gives an artist page.
- A test shows that one owned album with a stated `label` role gives a label page.
- A test shows that a "listed by" album or a conflict pair with `label` does not give a label page.
- The view model exposes the count as a derived value, separate from the page type.

Visual, for the operator:

- A publisher with many artists and no stated role shows as an artist page, with the derived count visible.

## Relationship To Other Decisions

- Supersedes ADR 0077 Decision 4. ADR 0077 stays Accepted for its other decisions.
- Removes the ADR 0077 proposal for a label threshold.
- Uses ADR 0075 Decision I for provenance.
