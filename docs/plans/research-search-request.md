# Research Request: Shared Search Code And Search Findings

## Status

Received - 2026-10-23. This request comes from the research repository at
`~/build/musicindex-research`. That repository has no Git remote. Its index is
`AGENTS.md`. The report is
`reports/2026-10-23-search-code-sharing-and-stophammer.md` in that repository.

This document binds nothing in v4vmm. v4vmm records each decision in its own
ADR.

The research examined v4vmm at commit `93b2c79`, and the work of ADR 0087
packet 002 that was not in a commit on that day.

Stophammer release 0.14.0 is deployed on 2026-10-24. Section 5 gives the
changes for v4vmm.

## 1. Shared Search Code

The website, v4vmm and musicindex-studio share no search code. Each has its own
wire types, its own request code and its own result logic. The report proposes
three steps:

1. Stophammer gives the facts that each client calculates: the release date and
   the named feeds of a hit (open request 11), the sort rank, the release kind
   and a name key for an artist hit.
2. A small crate in the Stophammer workspace holds the response structs, the
   envelope, the `include` names and the limits, with no HTTP code. v4vmm keeps
   `reqwest`. The crate can own the guard that each decoded field is in the
   contract.
3. The parser of ADR 0087 (`src/view_models/search_query.rs`) moves to a crate,
   for example `musicindex-query`. Its cases R87-1-01 to R87-1-30 go into a JSON
   file. Studio can use the crate. The website can write a JavaScript parser
   that passes the same cases.

ADR 0087 rejects "Stophammer parses the syntax", and one cause is that "the two
clients of Stophammer must then agree on one syntax". One shared parser removes
that cause. Stophammer continues to receive only filters (ADR 0087 D5).

Step 2 changes the stored contract method of ADR 0075 §6. Step 3 changes the
owner of the parser. Each needs a v4vmm ADR.

v4vmm has no `LICENSE` file and no `license` field in `Cargo.toml`. Stophammer
and Studio are AGPL-3.0-only. The operator selects a license before v4vmm uses
or gives shared code.

## 2. Client Findings

These findings need no Stophammer change:

| # | Result | Location |
|---|---|---|
| C1 | One Index search sends three requests in sequence: `type=feed`, `type=track`, then `type=publisher`. v4vmm can send them at the same time. | `src/application/queries/search.rs:440-493` |
| C2 | New releases makes one detail read for each row, four at a time, and a maximum of 120 detail reads for one page. `/v1/feeds/recent` gives `newest_item_at` and accepts `include=named_feeds`. Examine if the list rows give the facts that `row_window_facts` needs. This is an inference. | `src/application/queries/feed.rs:126-194`, ADR 0086 phase plan line 57 |
| C3 | The checks for artist names send one `GET /v1/artist-names?from=…&limit=1` for each candidate name, in sequence and with no limit. Live results (ADR 0087 D4) make this more frequent. | `src/application/queries/search.rs:556-583`, overhaul plan item 17 |
| C4 | The Library query runs with the database lock in `search_results_detail_for_run`, not in the command runner. This is an inference. Live results make it more frequent. | `src/app/search_dispatch.rs:285-291` |

## 3. Stophammer Findings That Concern v4vmm

The research also sent a request to Stophammer:
`docs/plans/research-search-index-request.md` in the Stophammer repository.

- The search index keeps the words that a title had before an edit (S1). A
  search can find a release by a title that it no longer has.
- The Stophammer request supports open requests 10 and 11 with the same items,
  and it cites them.
- Stophammer has no prefix search. Live results find a release only when the
  curator types a full word.
- One request for all types is a proposal. It replaces the three requests of C1.

## 4. Broadcast Audio Sources

On 2026-10-23 the research added Rivendell 4.5.0 by Paravel Systems as a source
in `sources/broadcast-audio/streaming/rivendell`. BUTT has no public Git
repository. The research keeps its release archive 1.47.0, which was the
latest release on 2026-10-23. The research index of broadcast audio gives the
details.

## 5. Stophammer Release 0.14.0

Stophammer deployed release 0.14.0 on 2026-10-24. On that day, `GET
/node/info` gave `git_revision` `499b301`, and `info.version` of
`/openapi.json` gave `0.14.0`. The release delivers Stophammer ADR 0078 and
ADR 0079.

- The search index holds only the text that each entity has now (ADR 0078 §1).
  This corrects the defect of section 3 (S1). A search no
  longer finds a release by a title that it no longer has.
- A retire deletes each index row of the feed. A publisher hit gives the
  quality of its feed (ADR 0078 §3 and §4).
- The node divides `q` into words, and it puts each word in quotation marks
  (ADR 0078 §5). Each `q` with a word gives `200`, also with an apostrophe.
  This delivers item 1 of open request 10. A query with `title:foo` is a
  search for the words `title:foo`.
  A `q` with no word gives `400`, and a `400` message gives no SQLite text.
- A GUID that a retire with no block frees returns only through its last
  source URL, until an operator releases the hold (ADR 0079). The ingest gives
  the reason `retired_source_conflict`, and the admin route
  `DELETE /v1/retired-source-holds/{guid}` is new. v4vmm sends no ingest request.

Read-only checks on the live API on 2026-10-24:
`GET /v1/search?q=haleen'&type=track` gave `200` with 3 rows, and
`q=title:foo` gave `200` with no rows. The OpenAPI document lists
`/v1/retired-source-holds/{guid}`.

These items stay open for v4vmm:

- `tests/fixtures/musicindex-openapi-0.13.0.json` is the stored contract. The
  release changes no response field. It changes the description of `q`, and
  it adds one admin route. ADR 0075 section 6 owns the replacement.
- The test `adr_0084_search_list_http_400_is_a_rejected_query` in
  `src/application/queries/search.rs` records an `fts5` error for `haleen'`.
  Release 0.14.0 gives `200` for that text. The test continues to check the
  `400` path.
- Items 2 to 6 of open request 10, and request 11, have no Stophammer decision.
  ADR 0087 packet 006 continues to wait for them.
- The release has no prefix search and no request for all types. C1 to C4 need
  no Stophammer change.
