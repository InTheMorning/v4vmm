# Request: Export An Episode From The Show Log

## Status

Open - 2026-10-19. `musicindex-live-publisher` makes this request to v4vmm.

This document binds nothing in v4vmm. v4vmm records each decision in its own
ADR.

## Context

`docs/research/broadcast-recording-and-feed-publishing.md` describes the goal:
one live show becomes one podcast episode. The episode has one MP3, one
Podcasting 2.0 RSS item, chapters, and value time splits that pay the same
artists that the live listeners paid. The note also says that the publisher
records the timeline and that v4vmm generates the episode.

The publisher side now exists:

| Publisher ADR | Status | Content |
|---|---|---|
| 0017 | Implemented 2026-10-19 | The show log, `musicindex.showlog/1`. It supersedes ADR 0003. |
| 0014 | Implemented 2026-10-18 | Each producer reports each track, payable or not. |
| 0010 | Accepted | The drop file has `album` and `play_id`. `play_id` is the ID of the Mixxx history row. |
| 0008 | Accepted | The display state carries the art as a URL, or as an embedded image with its SHA-256. |

The publisher generates no RSS, no chapter, no value time split and no art.
That is the subject of this request.

## What Changed Since The Research Note

The research note cites publisher ADR 0003. ADR 0017 superseded it on
2026-10-19. These facts changed:

- `observed_at` is the time at which the publisher reads the drop file. The
  publisher reads the file within milliseconds of the producer write. The
  real-time rule of the research note does not change.
- `aired_at` is gone. Since publisher ADR 0011, the relay applies the stream
  delay. Each entry records `listener_delay_secs` instead, as an estimate for
  a recording made after Icecast.
- Each track has an entry, payable or not. `payable` tells which.
- The art is a reference. The publisher stores no image.
- The display fields are in a separate `display` entry.

## The Log

- Place: on the publisher host, in
  `~/.local/state/musicindex-live-publisher/<instance>/showlog/`, as
  `showlog-YYYY-MM-DD.jsonl`. One file for each UTC day. The file mode is
  `0600`.
- Retention: `show_log_retention_days` in the publisher configuration,
  default 365.
- Format: JSON Lines. One entry on each line. The last line can be damaged
  after a crash.
- Kinds: `track`, `clear` and `display`. Publisher ADR 0017 §Entries lists
  each field.

This is the shape of a `track` entry and a `display` entry. The titles, the
times and the event come from a check on 2026-10-19. The other values are
not shown. A check of each field in a real file is still open:

```json
{"schema":"musicindex.showlog/1","kind":"track","observed_at":"2026-10-19T16:47:56.569Z","target":"default","event_guid":"DFnUwDM1mzcljaF4pI9Q6w","block_guid":"…","payable":true,"play_id":"…","artist":"…","title":"Nothing Is Easy","feed_guid":"…","track_guid":"…","value":{…},"listener_delay_secs":…,"publish_result":"accepted","seq":…}
{"schema":"musicindex.showlog/1","kind":"display","observed_at":"2026-10-19T16:47:56.570Z","target":"default","event_guid":"DFnUwDM1mzcljaF4pI9Q6w","play_id":"…","title":"Nothing Is Easy","song_line":"…","artwork":{"sha256":"…","mime":"image/jpeg"}}
```

## The Requests

1. **An ADR for the episode export.** Record that v4vmm builds the episode
   from `musicindex.showlog/1`, and cite publisher ADR 0017 as the owner of
   the contract.
2. **The read rules.** The ADR states these rules, or cites ADR 0017 for
   them:
   - Read the entries whose `observed_at` is in the recording window. A
     window can cross a UTC day, so read more than one file.
   - Use only the entries with `publish_result: "accepted"`.
   - For each `target`, `event_guid` and `block_guid`, keep the last accepted
     entry, at the `observed_at` of the first entry of that block. That is a
     route revision.
   - Join a `display` entry to a `track` entry by `target` and `play_id`.
   - Ignore a line that does not parse, an unknown `schema`, and an unknown
     `kind`.
3. **The recording start in wall clock.** The research note requires that
   each recorder records its own start time. A chapter start is
   `observed_at` minus that time. Without it, each chapter and each split is
   in the wrong place.
4. **The chapters.** One chapter for each `track` entry, and one for each
   `clear` entry. A chapter ends at the `observed_at` of the next chapter. Do
   not use `duration_secs` for the end, because a crossfade cuts a track
   short. Use `title`, `artist`, `album` and `song_line` for the text.
5. **The value time splits.** Select one form and record why:
   - Explicit recipients from `value`. It is exactly what the live listeners
     paid, with its type, method and `suggested`.
   - A `<podcast:remoteItem>` with `feed_guid` and `track_guid`, and a
     `remotePercentage`. An app then reads the current split of the artist's
     feed, which can differ from the live split.

   A `track` entry with `payable: false` and a `clear` entry have the dead
   block in `value`. Use the podcast's own value for those parts. Never
   publish the dead block in a feed. The research note §Editing Policy says
   that an operator change of a route is an override, kept apart from the
   logged route.
6. **The art.** v4vmm resolves the image of each chapter:
   - The full embedded image of the file, through `play_id` and the Mixxx
     history row. An animated GIF is possible here. The live path cannot send
     a GIF, so the log never names one.
   - `image_url` of the `track` entry, or `artwork.url` of the `display`
     entry.
   - The `sha256` of `artwork` is the hash of the live image. The producer
     reduces an image larger than 512 KiB or 1,000 px to a JPEG. So the hash
     can differ from the hash of the original. Use it to identify the live
     image, not to verify the file.
   - The operator decides whether an image goes into a public feed. A
     commercial cover of a non-V4V track can need a different image.
   - Record where the images of an episode are hosted.
7. **The chapter link.** The log has no page URL for a track, because
   publisher ADR 0010 added no `link`. v4vmm can find one from `feed_guid`
   through the MusicIndex API. A non-V4V track has no link.
8. **The access to the log.** The log is on the publisher host. Select how
   v4vmm reads it, for example a read-only copy through `ssh` (ADR 0059
   §Remote Control Uses SSH First). v4vmm never writes to the log.
9. **The old references.** These v4vmm documents cite publisher ADR 0003:
   - `docs/adr/0061-executable-governance.md`, the table row "Show log"
   - `docs/research/broadcast-recording-and-feed-publishing.md`
   - `docs/plans/broadcast-chain-delivery-order.md`, the show log rows

   Change them to cite publisher ADR 0017.

## Limits That v4vmm Must Know

- **Embedded art needs the file.** If the file or its history row is gone
  before the export, nothing can recover the image.
- **Only the Mixxx producer writes `play_id`.** A future producer without
  `play_id` needs `image_url`, or its chapters have no art.
- **`listener_delay_secs` is an estimate.** No one measured the real delay
  after Icecast. A recording made after Icecast needs a correction before an
  export.

## What The Publisher Gives

- `musicindex.showlog/1`, with the rules of publisher ADR 0017.
- A new kind of entry, when a later publisher ADR adds one, for example a
  talk segment of publisher ADR 0015. A reader ignores a kind that it does
  not know, so no new schema version is needed.

## References

- `musicindex-live-publisher`: `docs/adr/0017-show-log.md`,
  `docs/adr/0008-display-path.md`,
  `docs/adr/0010-live-value-payload-reference-shape.md`,
  `docs/adr/0014-one-assembler-and-track-reporting.md`,
  `docs/adr/0015-live-segments-from-v4vmm.md`
- v4vmm: ADR 0059, `docs/research/broadcast-recording-and-feed-publishing.md`,
  `docs/plans/broadcast-chain-delivery-order.md`,
  `docs/plans/live-segment-request.md`
