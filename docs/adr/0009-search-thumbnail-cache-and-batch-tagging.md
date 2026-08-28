# ADR 0009: Search Thumbnail Cache and Feed Batch Tagging

## Status

Accepted - 2026-04-16.

## Context

Search results include remote artwork URLs, but the GUI previously rendered placeholder thumbnails. Subscribing to a feed also downloads multiple tracks, so it is the natural operator action for applying the same source-fact ID3 staging rules to a complete feed.

## Decision

Add a small on-disk thumbnail cache under the application config directory. The search GUI downloads image URLs in the background, validates image content types, stores the bytes and MIME type by URL hash, and renders the cached image at thumbnail size in result rows.

Feed subscription will batch-process feed tracks by hydrating available item metadata, computing the same automatic ID3 edits used by the track compare table, downloading each MP3 into the configured music directory, and writing those edits through the existing ID3v2.4 writer boundary.

## Consequences

- Search result thumbnails are available after the first successful image download and do not need to be fetched on every render.
- Feed subscription remains the explicit write action for batch downloads and tag writes.
- ID3 writes stay behind the audio tag boundary from ADR 0008.
- Failed image downloads or missing image content types produce placeholders instead of blocking search.
