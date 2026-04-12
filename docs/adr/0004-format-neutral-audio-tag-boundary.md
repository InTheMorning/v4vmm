# ADR 0004: Format-Neutral Audio Tag Boundary

## Status
Accepted

## Context
The download-and-compare workflow needs to read embedded metadata from downloaded audio files and compare it with MusicIndex/Stophammer source facts. The first supported file format is MP3, using the existing `id3` crate. The workflow should still avoid exposing ID3-specific names to the GUI and comparison layers, so later FLAC/MP4/OGG support does not require rewriting the feature.

## Decision
Add a small format-neutral `audio_tags` module that exposes an `AudioTags` struct and a `read_audio_tags` function. The first implementation delegates to the existing MP3/ID3 reader internally.

The existing `v4vmm id3-dump <path>` command remains as a development-oriented CLI command for dumping MP3 ID3 data.

## Consequences
- New comparison code can depend on embedded-metadata concepts rather than ID3 frame details.
- MP3 remains the only supported format in the first implementation.
- Later multi-format support can be added behind the same boundary.
