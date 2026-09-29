# ADR 0080: Tag Frames Follow Their Owner

## Status

Accepted - 2026-09-29. The operator discussed the direction on 2026-09-26.
The operator decided each proposal on 2026-09-29, and Decisions 5 to 8 record them. The operator accepted this ADR on 2026-09-29.
The [phase plan](../plans/adr-0080-tag-frames-phase-plan.md) registers its packets.

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
- Each `WOAR` value holds label text before the URL, for example `download for free (url, forward): https://example.test/feed`. ID3v2.4 defines a URL frame value as a URL only.

The label text has two sources, recorded on 2026-09-29:

- `release_url_values` in `src/musicbrainz.rs` writes each MusicBrainz URL relation as `<relation type> (<target type>, <direction>): <url>`.
  The relation types, such as "download for free", are the types of the MusicBrainz website. The tag form is a v4vmm format.
- `format_source_value_for_id3v24` in `src/metadata.rs` puts `download for free (url, forward):` before each RSS website. An RSS link is not that relation, so this text is false.

The id3 writer adds one `WOAR` frame for each different value. The same URL with a different label, or with no label, becomes a second frame. Duplicate frames grow in this way.

MusicBrainz Picard, on its `master` branch on 2026-09-29, writes a plain URL in each URL frame:

- `WOAR` holds the `website` tag. Picard deletes each `WOAR` frame, then adds one frame for each valid URL.
- `WCOP` holds one `license` URL. With more than one license URL, Picard writes `TXXX:LICENSE`.
- From the MusicBrainz URL relations, Picard reads only `license`.

The Podcast Namespace states that item values replace channel values for `podcast:value` and `podcast:person`.
v4vmm applies that rule to payment routes in files (ADR 0076 Decision 9) and to the credit list (ADR 0076 packet 006).
The Podcast Namespace states no such rule for `podcast:txt`.

## Decision

### 1. Nostr: One Resolved Value

The frame `TXXX:RSS Nostr Handle` holds one value: the valid Nostr key of the item, otherwise the valid Nostr key of the channel. Decision 7 defines a valid key.
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

### 5. Description: Two Frames

`COMM:MusicIndex Description` holds the item description only.
When the item states no description, `COMM:MusicIndex Album Description` holds the channel description. The frame name states the owner.
A file never holds the channel description as a track description. The operator decided this on 2026-09-29.

### 6. A Write Is Idempotent

A second write with the same inputs changes no frame. A write never adds a duplicate frame.

For each frame that the app owns, a write removes the values that the app supplies, and then adds the current values. The app supplies a value from RSS or from MusicBrainz.
The write recognizes each earlier form of an app value as an app value, and replaces it. An earlier form is a value with a label before the URL, or a track page in `WOAR`.

A write keeps each value that neither RSS nor MusicBrainz supplied, for example a value from another tool.
A write never removes a MusicBrainz value. RSS values and MusicBrainz values stay side by side in a frame. The compare shows each value on its own row with its source (ADR 0075 Decision I).
The operator decided this on 2026-09-29.

### 7. An Invalid Nostr Key Goes Into No Frame

A Nostr key that fails NIP-19 validation goes into no frame. An invalid item key does not block the channel key. With no valid key, the file has no Nostr frame.
ADR 0075 section 3 keeps an invalid value as evidence only. The operator decided this on 2026-09-29.

### 8. A URL Frame Holds A Plain URL

Each URL frame holds one plain URL. It holds no label text. The app writes a URL only when it parses as a URL.
The relation type or link label stays in the database, and the compare shows it.

| Value | Frame |
|---|---|
| The item page (RSS item `<link>`) | `WOAF` |
| The channel website (RSS channel `<link>`) | `WOAR` |
| A MusicBrainz official homepage relation | `WOAR` |
| One MusicBrainz license relation | `WCOP` |
| More than one MusicBrainz license relation | `TXXX:LICENSE`, as Picard does |
| Each other MusicBrainz URL relation type, for example "download for free" | No frame |

The app writes no `WXXX` frame. The operator decided this on 2026-09-29, after the comparison with Picard.

## Relationship To Other Decisions

- ADR 0004 and ADR 0008 own the tag boundary and the explicit ID3v2.4 write boundary. The writer's frame list in `src/audio_tags.rs` already contains `WOAF` and `WCOP`.
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
- The tag edits for a track without its own description hold the channel description in `COMM:MusicIndex Album Description`, and no `COMM:MusicIndex Description`.
- A write of the same edits twice gives equal frames. A file with an earlier labeled `WOAR` value holds only the plain URL after one write.
- A write keeps a MusicBrainz URL and a `WOAR` value that no source supplied.
- An invalid item Nostr key gives the valid channel key. Two invalid keys give no Nostr frame.
- Each URL frame value parses as a URL, with no label text. A MusicBrainz "download for free" relation gives no frame.

Visual criteria, for an operator check after the visual pause ends:

- After "Update n file(s)", an external tag reader shows the item page as the file webpage and the channel website as the artist webpage.
- After the update, the next scan shows no difference for that file.
- A second "Update n file(s)" on the same file adds no frame in an external tag reader.

## Alternatives Considered

- Two Nostr frames, one for each owner. Each player must then resolve the value itself, and the item-over-channel rule of payment routes is not applied.
- The website as one resolved value. The item page and the channel website mean different things, and ID3 has a frame for each.
- No change. The round-trip defect stays, and a file mixes the values of two owners in one frame.
- The channel description in `COMM:MusicIndex Description` when the item has none. Rejected: the file then states album text as track text.
- An item description only, with no album frame. Rejected by the operator: the file then loses the album text.
- `WXXX` frames with the MusicBrainz relation type as the description. Rejected: ID3 permits one `WXXX` frame for each description, players seldom show it, and Picard does not use it.
- The label form `<type> (<target>, <direction>): <url>` for MusicBrainz values only. Rejected: a player cannot open the value, and one frame then holds two formats.
- MusicBrainz values replace RSS values in a frame. Rejected: ADR 0075 Decision I keeps the two sources apart, and no source wins.

## Consequences

- A file holds one Nostr key, the same key that the value routes follow.
- A tag reader can tell the track page from the artist page.
- A file that the app wrote agrees with the next compare.
- Each existing file with the earlier mapping needs one confirmed rewrite.
- MusicBrainz relation types other than official homepage and license leave the file. They stay in the database.
- A file can hold two `WOAR` values: the RSS channel website and the MusicBrainz official homepage.
