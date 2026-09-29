# ADR 0080: Tag Frames Follow Their Owner

## Status

Proposed - 2026-09-26. The operator discussed the direction on 2026-09-26.
The details in "Proposals for operator review" are not decided. This ADR is not binding until the operator accepts it.

## Context

An ID3 frame has no owner field. v4vmm writes values of two owners into one frame:

- `id3_frame_hint` in `src/metadata.rs` maps the "Website" row (the track) and the "RSS feed website" row (the feed) to `WOAR`.
- It maps the "Nostr handle" row (the track) and the "RSS feed nostr handle" row (the feed) to `TXXX:RSS Nostr Handle`.
- `api::track_with_feed_defaults` copies the feed links, the feed identifiers and the feed description into a track that has none. The tag rows then read feed values as track values.

Each ID3v2.4 frame has a specified meaning. `WOAR` is the official artist or performer webpage. `WOAF` is the official audio file webpage.
The item `<link>` is the page of one track, so it matches `WOAF`. The channel `<link>` is the page of the album or the artist, so it matches `WOAR`.

The app reads embedded tags back as a separate source and compares them with the stored values.
The compare checks a tag against the track's own fact. A frame that holds a feed value then differs from the track fact, and the difference never resolves.
The operator confirmed on 2026-09-26 that this round-trip defect is old.

The fixed edits of ADR 0075 packet 022 (R22-06, 2026-09-28) record the present output:

- A track with its own Nostr key gets the feed key in `TXXX:RSS Nostr Handle`. The track key is lost.
- A track with its own website gets two `WOAR` frames: the feed website and the track page.
- Each `WOAR` value holds link label text before the URL, for example `download for free (url, forward): https://example.test/feed`. ID3v2.4 defines a URL frame value as a URL only.

The Podcast Namespace states that item values replace channel values for `podcast:value` and `podcast:person`.
v4vmm applies that rule to payment routes in files (ADR 0076 Decision 9) and to the credit list (ADR 0076 packet 006).
The Podcast Namespace states no such rule for `podcast:txt`.

## Decision

### 1. Nostr: One Resolved Value

The frame `TXXX:RSS Nostr Handle` holds one value: the Nostr key of the item when the item states one, otherwise the Nostr key of the channel.
This follows the item-over-channel rule of payment routes and credits. v4vmm states this rule for `podcast:txt` itself.

The database keeps both keys with their owners. A screen shows them apart (ADR 0075 Decision B). Only a file holds the resolved value.

### 2. Website: Two Frames

The item page goes to `WOAF`. The channel website goes to `WOAR`.
A frame receives only the value of its own owner. No write copies a value from one owner to the other frame.

### 3. The Compare Uses The Resolution Of The Writer

The tag compare checks each frame against the value that the writer would write for the same track and feed.
A file that the app wrote shows no difference at the next compare.

### 4. Existing Files

The ADR 0076 tag scan uses this mapping. A file with an earlier mapping shows as a difference.
"Update n file(s)" rewrites it after the operator confirms (ADR 0076 Decision 8). No write occurs without that confirmation.

## Proposals For Operator Review

| Detail | Proposal |
|---|---|
| The description frame | `COMM:MusicIndex Description` holds the item description only. A track without its own description gets no description frame. This matches the display rule of 2026-09-20. The alternative writes the channel description when the item has none, and labels the frame as the album description |
| An earlier track page in `WOAR` | A rewrite removes a track page from `WOAR` and writes it to `WOAF` |
| A key that fails validation | A Nostr key that fails NIP-19 validation goes into no frame. ADR 0075 section 3 keeps an invalid value as evidence only |

## Relationship To Other Decisions

- ADR 0004 and ADR 0008 own the tag boundary and the explicit ID3v2.4 frame list. `WOAF` is already in that list.
- ADR 0075 section 4: a feed fact never becomes a track assertion in storage. This ADR applies the same rule to frames.
- ADR 0076 Decisions 8 and 9: the scan, the confirmation and the route frame stay as they are.
- ADR 0075 packet 022 stops the copy of feed links, identifiers and description, and keeps the tag output equal. This ADR then changes the frames.

## Verification

Mechanical criteria, phrased at the owning layer:

- The tag edits for a track with its own Nostr key hold the track key. Without it, they hold the feed key.
- The tag edits hold the item page in `WOAF` and the channel website in `WOAR`, and neither value in the other frame.
- A round-trip test: for each frame, the compare of freshly built edits against the same track and feed reports no difference.
  This guard answers the defect that the operator confirmed on 2026-09-26.
- The scan reports a file with an earlier mapping as a difference.

Visual criteria, for an operator check after the visual pause ends:

- After "Update n file(s)", an external tag reader shows the item page as the file webpage and the channel website as the artist webpage.
- After the update, the next scan shows no difference for that file.

## Alternatives Considered

- Two Nostr frames, one for each owner. Each player must then resolve the value itself, and the item-over-channel rule of payment routes is not applied.
- The website as one resolved value. The item page and the channel website mean different things, and ID3 has a frame for each.
- No change. The round-trip defect stays, and a file mixes the values of two owners in one frame.

## Consequences

- A file holds one Nostr key, the same key that the value routes follow.
- A tag reader can tell the track page from the artist page.
- A file that the app wrote agrees with the next compare.
- Each existing file with the earlier mapping needs one confirmed rewrite.
