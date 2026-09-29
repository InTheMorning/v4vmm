# ADR 0080 Task 002: MusicBrainz URL Relations By Type

Status: Ready - 2026-09-29. Implementation has not started.
Its visual gate opens when the implementation is complete. Visual checks are paused, so the gate stays open.

## Goal

A MusicBrainz URL relation goes to the frame of its relation type, as a plain URL. Each other relation type goes into no frame.
The compare grid shows each relation type while the lookup result is open.

## Authority

- [ADR 0080](../adr/0080-tag-frames-follow-their-owner.md) Decisions 6 and 8, and "Files Written Before This ADR".
- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md) Decision I: RSS values and MusicBrainz values stay apart, and no source wins.
- The durable set in [AGENTS.md](../../AGENTS.md): renderer portability and typed action state.

## Recorded Facts - 2026-09-29

- `fetch_release_detail` in `src/musicbrainz.rs` sends one lookup with `inc=artist-credits+labels+recordings+release-groups+media+isrcs+url-rels`.
- `release_url_values` gives each release URL relation as one text value, `<relation type> (<target type>, <direction>): <url>`. `MusicBrainzCandidate::urls` holds them.
- `musicbrainz_value_for_field` in `src/metadata.rs` puts all of them in the "Website" and "RSS feed website" rows.
- A lookup of release `94a78c1e-84fb-455c-ae9b-9ecfee51049f` (Nine Inch Nails, "Ghosts I–IV") on 2026-09-29 gave these relations:
  - The release: `amazon asin` and `discogs`.
  - The release group, with `release-group-level-rels`: `allmusic`, `discogs`, `lyrics`, `official homepage` (`http://ghosts.nin.com/`), `other databases` and `wikidata`.
- Thus a release lookup gives no official homepage. The homepage is a relation of the release group. The same lookup gives it when `inc` also holds `release-group-level-rels`. That adds no request.
- An artist official homepage needs an artist lookup, which is a new request. This packet does not send it.
- A license relation is a relation of the release.
- v4vmm stores no MusicBrainz lookup result. The operator can clear the Library and download again, so this packet converts no earlier file.

## Required Changes

### 1. The Lookup

- Add `release-group-level-rels` to the `inc` value of `fetch_release_detail`. Send no other request.
- Decode the URL relations of the release and of its release group.

### 2. Structured Relations

- Replace `MusicBrainzCandidate::urls` with a list of typed relations. Each relation keeps its relation type, its URL, and its owner: the release or the release group.
- Delete `release_url_values` and its label format.
- Treat each relation URL as untrusted input. Keep a URL only when it parses as a URL.

### 3. Frames By Type

| Relation | Row | Frame |
|---|---|---|
| `official homepage` of the release group | The channel website row, the MusicBrainz column | `WOAR` |
| One `license` of the release | A new "License" row | `WCOP` |
| More than one `license` of the release | The "License" row | `TXXX:LICENSE`, with one value for each URL |
| Each other type | A read-only row for each relation, labeled with its relation type | No frame |

- The item page row (`WOAF`) gets no MusicBrainz value.
- A read-only row has no write action. Its view model gives the label and marks it as not writable.
- A write keeps an RSS value and a MusicBrainz value side by side in `WOAR` (ADR 0080 Decision 6). A second write with the same inputs changes nothing.

### 4. Compare

- The compare shows each MusicBrainz relation with its relation type, while the lookup result is open.
- The round trip of packet 001 holds: a file written from a MusicBrainz selection shows no difference at the next compare with the same lookup result.

## Mechanical Acceptance Criteria

Use the prefix `adr_0080_mb_` for behavioral tests beside the owning code.

| Case | Required proof |
|---|---|
| R82-01 | The release lookup URL holds `release-group-level-rels`, and the lookup sends one request |
| R82-02 | A decoded release with a release-group `official homepage` gives one relation with that type, URL and owner |
| R82-03 | The candidate holds no label text in any URL. No code builds the form `<type> (<target>, <direction>): <url>` |
| R82-04 | A selected official homepage gives one `WOAR` edit with the plain URL, and no `WOAF` edit |
| R82-05 | One license gives one `WCOP` edit. Two licenses give `TXXX:LICENSE` with both URLs, and no `WCOP` edit |
| R82-06 | A `discogs` relation gives a read-only row with its type, and no edit |
| R82-07 | A relation with a URL that does not parse gives no row and no edit |
| R82-08 | An MP3 temporary file written with the RSS channel website and a MusicBrainz homepage holds both in `WOAR`. A second write gives equal frames |
| R82-09 | Round trip: the compare of a file written from a MusicBrainz selection, against the same lookup result, reports no difference |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: a MusicBrainz lookup of a track shows the release-group homepage in the channel website row, and a "License" row when the release states a license.
- V2: each other relation shows as a read-only row with its relation type, and it offers no write.
- V3: after a write of the homepage and the license on a test copy, an external tag reader shows plain URLs in `WOAR` and `WCOP`.
- V4: normal and narrow widths show each row in its place, in Light and Dark themes, with no clipped text.

## Exclusions

- No artist lookup and no new request.
- No storage of a MusicBrainz result.
- No conversion of a file written before ADR 0080.
- No change to the RSS frames of packet 001, to the route frame or to the confirmation flow.

## Files To Inspect

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- `src/musicbrainz.rs`: `fetch_release_detail`, `MbRelease`, `MbReleaseGroup`, `MbRelation`, `MusicBrainzCandidate`, `merge_release_detail`, `release_url_values` and their tests.
- `src/metadata.rs`: `musicbrainz_value_for_field`, `musicbrainz_key_for_field`, `aligned_compare_rows`, `expand_woar_metadata_rows`, `woar_metadata_urls`, `id3_frame_hint`.
- `src/audio_tags.rs`: the write of `WCOP` and `TXXX` frames, and the packet 001 removal of app values.
- `src/tag_field.rs`: the `WCOP` mapping for Vorbis and MP4.
- The compare grid view model and its screen under `src/ui/shells/discover/`.

## Checks

```bash
cargo test --lib adr_0080_mb_
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check --all-targets
cargo build --bin v4vmm
```

## Rollback

Revert the working tree. This packet adds no migration and no stored data.

## Operator Visual Check

The implementer writes this section at completion. It gives numbered steps for V1 to V4.
It states the needed state, what counts as wrong, and the cleanup.

Use the isolated fixture of the packet 001 check. It has a database copy and a music folder with only test copies. No step writes to the real Library.
The check needs network access to `musicbrainz.org`. Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. Color alone is not a valid difference.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `.github/copilot-instructions.md`
- This packet: `docs/tasks/adr-0080-task-002-musicbrainz-url-relations-by-type.md`
- ADR 0080, all decisions, and the packet 001 document for the writer rules
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes": the lookup, the structured relations, the frames by type, and the compare.

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Tests use recorded JSON and temporary files. No test sends a request to `musicbrainz.org`.
- The view model gives each row label and each write availability. The screen only composes. A screen uses the scaled tokens of ADR 0039 and existing shared composites.
- Treat each MusicBrainz response as untrusted input.
- Do not commit. Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The RSS frames and the idempotent removal of packet 001, except to add the MusicBrainz values.
- The route frame and `with_stored_route_frame`.
- The database schema and the migration registry.
- Any ADR, and each document other than this packet.
- The Stophammer checkout at `../stophammer`.

Acceptance criteria:
- Each case R82-01 to R82-09 has a passing test.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Operator visual check" section for V1 to V4.

Test commands:
- `cargo test --lib adr_0080_mb_`
- `cargo test`
- `cargo test --test architecture_tests`
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- `cargo check --all-targets`
- `cargo build --bin v4vmm`

At the end, report:
1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

Stop and report the problem, and do not guess, when:
- The compare grid has no read-only row form, and one needs a new shared composite.
- `TXXX:LICENSE` cannot hold more than one value in the present writer.
- A change needs a file in "Do not touch".
