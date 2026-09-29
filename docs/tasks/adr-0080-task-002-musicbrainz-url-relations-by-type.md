# ADR 0080 Task 002: MusicBrainz URL Relations By Type

Status: Implemented - 2026-09-29. Mechanical checks Green. Visual gate open and paused.

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

## Implementation Result - 2026-09-29

### The Lookup (R82-01)

`fetch_release_detail`, in `src/musicbrainz.rs`, now builds its URL through a new `release_detail_url`
function. That function adds `release-group-level-rels` to the `inc` value, in addition to the six values the
lookup already sent. `fetch_release_detail` still sends one request. The new function exists so a test
can read the query string without a network call.

### Structured Relations (R82-02, R82-03)

`MusicBrainzCandidate::urls` (`Vec<String>`) is gone. `MusicBrainzCandidate::url_relations` replaces it,
as a `Vec<MusicBrainzUrlRelation>`. Each relation holds its raw relation type text, its plain URL, and a
new `MusicBrainzRelationOwner` (`Release` or `ReleaseGroup`).

`release_url_values` and its label format (`<type> (<target>, <direction>): <url>`) are deleted.
The new `release_url_relations` function reads the release's own `relations` list. It also reads the
release group's `relations` list, when the lookup decoded one. Both lists pass through the new
`relation_urls` helper. `MbReleaseGroup` gained a `relations` field for this.

`relation_urls` keeps a relation only when its URL parses with `reqwest::Url` (R82-07). A relation with
a bad URL never reaches `url_relations`. No later code can then build a row or an edit from it.

### Frames By Type (R82-04, R82-05, R82-06)

`musicbrainz_value_for_field`, in `src/metadata.rs`, no longer answers `"Website"` at all. The item
page keeps no MusicBrainz value (ADR 0080 Decision 8). It answers `"RSS feed website"` with the release
group's own `"official homepage"` relation. The new `musicbrainz_official_homepage_url` function reads
that relation.

That value flows through the same `WOAR` expansion packet 001 built (`expand_woar_metadata_rows`). The
channel's own website and the MusicBrainz homepage can then sit in the row as two separate `WOAR`
values. Each one reaches the writer as its own edit.

`musicbrainz_remainder_rows` gained two new row builders, run for both `track_metadata_rows` and
`aligned_compare_rows` (both already call it while the lookup panel is open):

- `musicbrainz_license_row` reads the release's own `"license"` relations. One gives a "License" row
  with frame `WCOP`. Two or more give the same row with frame `TXXX:LICENSE`, and a value holding every
  URL, joined the way `join_values` joins any other multi-value field.
- `musicbrainz_relation_type_rows` gives one row for each remaining relation (the homepage and the
  consumed license relations are skipped). Its field is the relation's own type text, and it carries no
  `id3_frame`. `unique_musicbrainz_relation_row_id` keeps two relations of the same type from sharing a
  row id.

Neither builder changed `id3_frame_hint`, `write_mp3_edits`, `write_lofty_edits`, or `tag_field.rs`.
`WCOP` and `TXXX:LICENSE` already had a working Vorbis and MP4 mapping.

Two existing passes already keep a field with no website-specific handling idempotent:
`remove_stale_lofty_keyed_items`, and the `id3` crate's own replace-by-key frame write.
`Barcode` and `Release status` already worked the same way, before this packet.

### Compare And Write Availability (Required Changes 3 and 4)

`TrackMetadataGridVm::musicbrainz_value_is_writable` (`src/view_models/track_metadata_grid.rs`) is
`true` only when the row carries an `id3_frame`. `metadata_musicbrainz_cell`
(`src/ui/shells/discover/track_inspector_metadata_cells.rs`) now calls it before it builds a drag value
for the MusicBrainz column. A read-only relation row then offers no drag.

`auto_populated_pending_id3_edits` already required an `id3_frame` before it stages an edit, with no
change needed there. A read-only row was already excluded from an automatic write. This new check
closes the same gate for the manual drag action.

The Library track-detail screen's MusicBrainz cell (`src/ui/shells/library/track_detail_metadata_cells.rs`)
already renders with no drag at all, for every row, so it needed no change.

### Files Changed

- `src/musicbrainz.rs`
- `src/metadata.rs`
- `src/view_models/track_metadata_grid.rs`
- `src/ui/shells/discover/track_inspector_metadata_cells.rs`
- `src/audio_tags.rs` (one new test only, no production code changed)
- `src/discover/tests.rs`
- This packet document

### Tests

Each command in "Checks" is Green.

- `cargo test --lib adr_0080_mb_`: 10 tests pass — R82-01 through R82-09 (R82-05 has two tests, one
  license and two licenses).
- `cargo test`: 1,890 library tests pass, 283 architecture-guard tests pass, and 10 doc-tests stay
  ignored. All Green.
- `cargo test --test architecture_tests`: 283 tests pass.
- `cargo fmt -- --check`: Green.
- `cargo clippy -- -D warnings`: Green.
- `cargo check --all-targets`: Green, with no warning.
- `cargo build --bin v4vmm`: Green.

### Behavior Changed

- A MusicBrainz lookup's channel website row now shows the release group's own official homepage, as a
  plain URL. It no longer shows every URL relation joined with a label. The item's own "Website" row
  shows no MusicBrainz value.
- A release with one license relation now offers a "License" row and a `WCOP` edit. Two or more license
  relations offer the same row with a `TXXX:LICENSE` edit holding every URL.
- Each other MusicBrainz URL relation type (for example `discogs`, `wikidata`, `other databases`) now
  shows as its own row, labeled with that type, in the "Other metadata" group. The row offers no write:
  it stages no automatic edit, and its MusicBrainz value cannot be dragged into a frame.

### Deviations From The Task

The round-trip test for R82-09 first lived in `src/discover/tests.rs`, beside the other row-building
tests this packet touches.

The architecture guard `adr_0076_route_readiness_route_frame_writes_read_the_stored_route` failed
against it. That guard reads every file under `src/`. It strips text only from a literal
`#[cfg(test)]` / `mod tests {` line pair to the end of that same file. `src/discover/tests.rs` is its
own file, with no such pair inside it. The guard then read the whole file as production code. It
flagged the direct `write_id3v24_edits` call as a route-frame write with no stored-route read.

The test itself writes no route frame. It is a false positive against a test-only file the guard does
not recognize.

The test now lives in `src/metadata.rs`'s own `#[cfg(test)] mod tests { ... }` block instead. The
guard's existing stripping rule already excludes that block. Every other real-file write test in
`src/audio_tags.rs` uses the same protection. No guard file changed.

### Unresolved Concerns

- The Discover screen's own `apply_pending_id3_edits` (`src/discover/app_impl.rs`) has no button
  wired to it. Only the Library track-detail screen applies staged MusicBrainz edits today. This
  predates this packet and is unrelated to it. The Operator Visual Check below uses the Library screen
  for this reason.
- Cleanup of the evidence fixture `/tmp/v4vmm-governance.ie6k8TQf` stays unconfirmed. This is
  unrelated to this packet, as the standing note in `AGENTS.md` records.

## Operator Visual Check

This check needs a Linux desktop session, this checkout, network access to `musicbrainz.org`, and the
`sqlite3`, `python3` and `mid3v2` commands (`kid3-cli` also works for the read step).

**This check runs the app on an isolated fixture, copied from the real database.** The fixture's own
music folder starts empty and gains only the one test file this check copies into it. No step writes to
or copies into the real music folder.

1. Close v4vmm. Build the app:

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --bin v4vmm
   ```

2. Open the real app, read-only, in Library. Open track pages until you find one with a local file
   (its page shows a file path and a working "MusicBrainz" button, not a greyed-out one). Write down its
   track id. Close the app.

   Stop, and report the result, if no track has a local file.
3. Set the track id from step 2:

   ```bash
   T=<track id from step 2>
   ```
4. Create the fixture root, and copy the real database into it. The fixture's own music folder starts
   empty:

   ```bash
   gate_dir=$(mktemp -d /tmp/v4vmm-adr-0080-task-002.XXXXXXXX)
   python3 -c 'import json, os, pathlib, sqlite3, sys, tomllib
root = pathlib.Path(sys.argv[1]).resolve()
source = pathlib.Path(os.environ.get("XDG_CONFIG_HOME", str(pathlib.Path.home()/".config")))/"v4vmm/config.toml"
cfg = tomllib.loads(source.read_text())
target = root/"config/v4vmm"
target.mkdir(parents=True)
(root/"music").mkdir()
src = sqlite3.connect(pathlib.Path(cfg["db_path"]).resolve().as_uri()+"?mode=ro", uri=True, timeout=5)
dst = sqlite3.connect(root/"app.sqlite", timeout=5)
src.backup(dst)
dst.close()
src.close()
settings = {"music_dir": str(root/"music"), "db_path": str(root/"app.sqlite")}
settings.update({"musicindex_endpoint": cfg["musicindex_endpoint"]} if "musicindex_endpoint" in cfg else {})
(target/"config.toml").write_text("\n".join(k+" = "+json.dumps(v) for k,v in settings.items())+"\n")
print("Fixture database copy ready:", root)' "$gate_dir"
   ```

   Expect the printed path to equal `$gate_dir`. Stop, and report the result, if the script fails.
5. Read the real file path of track `$T`, from the fixture's own database copy. Copy it into the
   fixture's music folder only, at its stored relative path:

   ```bash
   real_music=$(sed -n 's/^music_dir = "\(.*\)"$/\1/p' ~/.config/v4vmm/config.toml)
   p=$(sqlite3 "$gate_dir/app.sqlite" "SELECT path FROM local_files WHERE track_id = $T;")
   case "$p" in /*) real_f="$p" ;; *) real_f="$real_music/$p" ;; esac
   mkdir -p "$(dirname "$gate_dir/music/$p")"
   cp -p "$real_f" "$gate_dir/music/$p"
   f="$gate_dir/music/$p"
   ```

   Stop, and report the result, if the real file is missing. No later step reads or writes a file under
   `$real_music`. Every later step reads and writes only `$gate_dir/music`.
6. Launch the fixture from this terminal:

   ```bash
   (
     export XDG_CONFIG_HOME="$gate_dir/config"
     export XDG_DATA_HOME="$gate_dir/data"
     export XDG_CACHE_HOME="$gate_dir/cache"
     target/debug/v4vmm
   )
   ```

**V1 — the release-group homepage shows in the channel website row, and a License row shows when the release states one**

7. Open track `$T`'s page in Library. Click "MusicBrainz". Wait for the candidate list.
8. Look at the top candidate's rows.

   - Correct: the "RSS feed website" row's MusicBrainz column shows a URL, with no text before it,
     when the release group states an official homepage.
   - Correct: a "License" row shows, with a plain URL in its MusicBrainz column, when the release
     states a license.
   - Wrong: the item's own "Website" row shows a MusicBrainz value.
   - Wrong: a value carries text before the URL, such as "official homepage (url, forward):".

   Not every release states a homepage or a license. If the top candidate shows neither, open the
   candidate list (when there is more than one) or repeat steps 2 and 3 for another local track.
   Stop, and report the result, if no track and no candidate shows either one after a reasonable
   search. The Recorded Facts section of this packet found both on the Nine Inch Nails release
   `94a78c1e-84fb-455c-ae9b-9ecfee51049f` ("Ghosts I–IV") on 2026-09-29. A track by that artist, with
   a matching title, is the most reliable way to reproduce this step.

**V2 — each other relation type shows as a read-only row**

9. On the same candidate, look for a row whose label is a MusicBrainz relation type other than
   "official homepage" or "license", for example "discogs" or "wikidata".

   - Correct: the row shows the relation type as its label and a plain URL as its value.
   - Correct: the row offers no drag from its MusicBrainz value (no move cursor, no drag preview).
   - Wrong: the row's value carries text before the URL.
   - Wrong: the row's ID3 column shows a frame id.

**V3 — a write holds plain URLs in `WOAR` and `WCOP`**

10. Below the panel, find the "Apply tags (N)" button. Click it. Wait for it to finish.

    This click writes only the fixture's own copy of the file at `$f`. It never reads or writes a file
    under `$real_music`.
11. Close the app. Read the file's tags:

    ```bash
    mid3v2 -l "$f"
    ```

    - Correct: `WOAR` holds the channel website and the homepage URL, when the candidate stated one,
      each with no text before it.
    - Correct: `WCOP` holds the license URL, when the candidate stated exactly one. The `TXXX:LICENSE`
      frame holds every license URL, when the candidate stated more than one, and `WCOP` is then absent.
    - Wrong: a `WOAR` or `WCOP` value carries text before the URL.
    - Wrong: a relation type other than the homepage or the license appears in any frame.

**V4 — normal and narrow widths, Light and Dark**

12. Launch the fixture again, the same way as step 6. Open Settings → General (`Ctrl+Comma`). Select
    Light. Open track `$T`'s page and its MusicBrainz panel again.

    - Wrong: a row's text is cut off, in a way that a wider window would not fix by wrapping.
13. Narrow the window. Repeat the same look.

    - Wrong: a row's text is cut off at the narrow width.
14. Select Dark (Settings → General). Repeat the same look, at both widths.

    - Wrong: a row shows in only one of the two themes.
    - Wrong: the two themes show a row with the same text but a difference only in its color. Color
      alone is not a valid difference.
15. Close the app.

**Cleanup**

16. Remove the fixture root:

    ```bash
    rm -rf "$gate_dir"
    ```

    Do not delete `/tmp/v4vmm-governance.ie6k8TQf`. It is a separate, unrelated fixture.

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

## Orchestrator Review - 2026-09-29

The orchestrator reviewed the diff and ran each check. Each check is Green: 1,890 unit tests, 283 guards, and no warning.
The operator check uses the isolated fixture of packet 001, and its cleanup removes only the fixture root.

The orchestrator records three findings. None blocks this packet:

- `TXXX:LICENSE` holds more than one license as one joined text value. Picard writes one ID3v2.4 value for each URL. The joined value is the present v4vmm form for each multi-value `TXXX` frame.
- The guard `adr_0076_route_readiness_route_frame_writes_read_the_stored_route` reads `src/discover/tests.rs` as production code, because that file has no `mod tests` block. The implementer moved one test to avoid a false result. The guard needs a correction in its own change.
- `apply_pending_id3_edits` in `src/discover/app_impl.rs` has no live caller. It belongs to the parked discover code of the ADR 0077 phase plan finding.
