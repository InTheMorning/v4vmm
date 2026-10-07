# ADR 0080 Tag Frames Phase Plan

## Status

Active - 2026-09-29. This plan is advisory. It states no rule.
[ADR 0080](../adr/0080-tag-frames-follow-their-owner.md) owns the rules.

## Packet Register

| Packet | Scope | Owners | Depends on | State |
|---|---|---|---|---|
| [001](../tasks/adr-0080-task-001-rss-frames-and-idempotent-writes.md) | RSS frames by owner, the album description frame, plain URLs, idempotent writes, and the compare and scan that use the writer resolution | ADR 0080 Decisions 1 to 8, for RSS values | ADR 0075 packet 022 | Implemented 2026-09-29. Operator V1 to V3 passed 2026-10-07. V4 moved to the overhaul plan |
| [002](../tasks/adr-0080-task-002-musicbrainz-url-relations-by-type.md) | MusicBrainz URL relations by relation type: release-group official homepage to `WOAR`, license to `WCOP` or `TXXX:LICENSE`, and no frame for each other type | ADR 0080 Decisions 6 and 8, for MusicBrainz values | 001 | Implemented 2026-09-29. Mechanical checks Green. Visual gate open and paused |
| [003](../tasks/archive/adr-0080-task-003-vorbis-date-shares-one-key.md) | The scan compares by the storage key of the format, and a write includes each edit that shares a key. A FLAC date settles after one write | ADR 0080 Decision 6 | 001 | Complete 2026-10-03. Operator V1 passed |
| [004](../tasks/archive/adr-0080-task-004-old-itunes-frames-and-safe-tag-writes.md) | Convert old iTunes v2.2 frames, write tags through a staged copy, and report a download tag-write failure | ADR 0080 Decision 6, ADR 0066 | 003 | Complete 2026-10-03. Operator V1 passed |
| [005](../tasks/archive/adr-0080-task-005-musicindex-image-frame.md) | The artwork URL in `TXXX:MusicIndex Image`: item image, else channel image, `http` or `https` only | ADR 0080 Decision 9 | 004 | Complete 2026-10-04. Operator V80-51 and V80-52 passed |

## Sequence

Packet 001 goes first. It owns the writer, the removal of app values, and the compare. Packet 002 then changes only the MusicBrainz source of URL values.

Until packet 002 is complete, packet 001 keeps each MusicBrainz URL value that the present code writes. Decision 6 does not permit the removal of a MusicBrainz value.

## Recorded Facts - 2026-09-29

- `release_url_values` in `src/musicbrainz.rs` gives each release URL relation as one text value, `<relation type> (<target type>, <direction>): <url>`.
  `musicbrainz_value_for_field` in `src/metadata.rs` puts all of them in the "Website" row, and thus in `WOAR`.
- The lookup requests `url-rels` of a release only. On 2026-09-29 a release lookup gave no official homepage. The release group holds it, and `release-group-level-rels` in the same lookup gives it. Packet 002 records the evidence.
- `changed_frames` in `src/application/queries/tag_update.rs` counts a frame as equal when any file value matches the expected value. A file with extra stale values in the same frame shows no difference.

## Open Defect - 2026-10-03

- An MP4 file has the same defect as packet 003 corrected for Vorbis. The writer stores `TDRC` and `TYER` in two freeform atoms, and the reader gives both values as `TDRC`.
  Thus the scan always reports `TYER` on an MP4 file with a date. The reader label does not name the atom, so a key comparison cannot correct it. No packet owns this defect yet.

## Session Rules

Each implementation session owns one packet. It completes the packet, checks it, and stops.
A packet that changes user-visible behavior leaves its visual gate open in its `Status:` line and in
[pending human checks](../pending-human-checks.md).
