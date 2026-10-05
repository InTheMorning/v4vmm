# Request: Write The MusicIndex Image Tag

## Status

Decided - 2026-10-04. `musicindex-live-publisher` made this request to v4vmm.
v4vmm accepted it as [ADR 0080](../adr/0080-tag-frames-follow-their-owner.md) Decision 9.
[ADR 0080 task 005](../tasks/adr-0080-task-005-musicindex-image-frame.md) implements it.

This document binds nothing in v4vmm. v4vmm records its decision in its own
ADR. If the decision changes the frames of ADR 0080, that ADR gets an
amendment or a successor.

## The Request

When v4vmm writes the MusicIndex tags of a track, it also writes the frame
`TXXX:MusicIndex Image`. The frame value is the `http` or `https` URL of the
track artwork, with a maximum of 2,048 characters. A track that has no such
URL gets no frame.

## Why The Chain Needs It

Two producers read this frame. Today no software writes it.

- `mixxx-now-playing` reads it into the field `image` of the drop file
  (publisher ADR 0002). Publisher tag-read task 001 added the read on
  2026-10-04. Podcast apps receive `image` in the live value payload.
- The `mpv` producer of v4vmm reads it into the same field.
  `src/broadcast/producer.rs` has the key `Image` in its vocabulary.
- Publisher ADR 0008, the optional display path, uses the URL as the artwork
  of a V4V track. The producer sends the URL and no image file. Thus an
  animated GIF stays animated in the listener app.

Without the frame, a V4V track uses its embedded picture. ADR 0008 accepts
only a JPEG or a PNG embedded picture. A track with an embedded GIF thus gets
no artwork.

## Evidence

A probe on 2026-10-04 read each MP3 file in `~/V4Vmusic/artists` with the
`mixxx-now-playing` tag reader. The probe found 81 files.

- No file has the frame `TXXX:MusicIndex Image`.
- 78 files have an embedded JPEG or PNG that passes the ADR 0008 rules.
- 3 files have an embedded GIF and get no artwork:
  - The Doerfels, "Disco Swag" (5.0 MB GIF)
  - HeyCitizen, "19 - ZZXX" (382 KB GIF)
  - HeyCitizen, "10 - How Bout You" (33 MB GIF)

On 2026-10-04 the display state of "Disco Swag" reached the relay with
`"artwork": null`.

## What v4vmm Has Now

Read from the source on 2026-10-04:

- `metadata::artwork_url` gives the image URL of the track, else the image
  URL of its feed.
- `id3_frame_hint` maps the "Artwork" row to `APIC`, the embedded picture. No
  row maps to `TXXX:MusicIndex Image`.

## Questions For v4vmm

1. **The owner of the value.** `artwork_url` falls back to the feed image.
   ADR 0080 Decision 1 lets one frame hold a resolved value, the item value
   else the channel value. Does the Image frame follow that rule, or does it
   hold only the track image?
2. **Which tracks get the frame.** The request covers each track with a
   payment route. A frame on other tracks does no harm, because the producers
   read it for each track.
3. **A URL that is not `http` or `https`.** The producers do not use such a
   value. Is it written, or left out?

## Answers From v4vmm - 2026-10-04

The operator decided each question on 2026-10-04. ADR 0080 Decision 9 owns the rule.

1. **The owner of the value.** The frame holds the item image, else the channel image. This is the resolution of the embedded picture and of ADR 0080 Decision 1.
2. **Which tracks get the frame.** Each track with an artwork URL, with or without a payment route.
3. **A URL that is not `http` or `https`.** It goes into no frame. A URL longer than 2,048 characters also goes into no frame.

MusicIndex on 2026-10-04 shows that each GIF of the evidence is an item image. Thus each of the three tracks gets its GIF URL.

## How The Publisher Checks The Result

After v4vmm writes the frame, the publisher repository repeats the probe:

- Each file with an image URL in the database shows the URL in the probe.
- The display state of "How Bout You" reaches the relay with
  `"artwork": {"url": …}`, and the private app shows the animated GIF.

## References

- `musicindex-live-publisher`: `docs/adr/0002-nowplaying-drop-file-contract.md`,
  `docs/adr/0008-display-path.md`, `docs/tasks/tag-read-task-001-image-tag.md`
- `musicindex-live-relay`: `docs/adr/0003-display-state-and-artwork.md`
- v4vmm: `docs/adr/0080-tag-frames-follow-their-owner.md`,
  `docs/plans/broadcast-chain-delivery-order.md`
