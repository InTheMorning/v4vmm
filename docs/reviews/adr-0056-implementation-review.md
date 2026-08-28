# ADR 0056 Implementation Review

Covers task packets 001-004, implemented as one change rather than four
sequential ones. The packets each specify a per-task review doc. This
consolidated review replaces those four. The work landed as a single change, so
per-task provenance would be fabricated after the fact.

## Result

Accepted. All four packets implemented, plus three fixes from post-implementation
review (below).

## Files Changed

New:

- `src/remote_media.rs` - transport owner
- `src/media/image_type.rs` - image classification owner

Modified:

- `src/track_compare.rs` - transport migration, container validation, client
  parameter removed from `download_track` / `download_enclosure`
- `src/audio_tags.rs` - transport migration for APIC and transcript, classifier
  moved out, transcript markup rejection, APIC byte-only typing
- `src/media/image_cache.rs` - transport migration, classification, client field
  removed, JPEG default paths removed
- `src/media/mod.rs` - `image_from_bytes` returns `Option`
- `src/subscribe_service.rs` - `download_image` transport and classification,
  unused client parameters removed
- `src/application/queries/images.rs`, `src/discover.rs`,
  `src/discover/app_impl.rs`, `src/app/bootstrap.rs`, `src/feed_service.rs` -
  constructor and call-site updates
- `src/ui/shells/library/track_detail_metadata.rs`,
  `src/ui/shells/discover/track_inspector_metadata.rs` - `.map` to `.and_then`
  for the now-optional `image_from_bytes`
- `src/lib.rs` - module registration
- `tests/architecture_tests.rs` - seven ADR 0056 guards

## Decisions Made During Implementation

**Client ownership.** The transport module owns its client, built with
`Policy::none()`. Callers no longer pass one. Task 001 left this open. The
alternative, where callers keep passing clients, leaves the boundary redirect loop
unexercised under the configuration production uses, which was the original
defect in the test suite.

**Space repair removed.** Task 001 required an empirical decision. A test
established that `Url::join` percent-encodes raw spaces in a path, so
`base.join(location).or_else(|_| base.join(&location.replace(' ', "%20")))` never
reached the repair path. Removed. Parser behavior is pinned by
`remote_media::tests::location_with_raw_spaces_resolves_without_repair`. The ADR
paragraphs describing the repair were removed in the same change.

**APIC stricter than display.** `image_type::classify` implements
sniff-then-declared for display paths. APIC uses `image_type::from_bytes` alone,
because it writes an artifact. See the post-review fixes below.

## Post-Implementation Review Fixes

Three findings from a review pass after the first implementation:

1. **APIC accepted a declared type alone.** `read_picture_reference` used
   `classify`, whose declared-`image/*` backup type contradicted the ADR invariant
   that no artifact is promoted on declared type alone. A 200 response carrying
   markup under `Content-Type: image/jpeg` would have been embedded. The fix uses
   byte recognition only. The ADR Decision bullet now matches the invariant.
   A regression test covers the rule.
2. **Transport client had a silent backup path.** `unwrap_or_else(|_| Client::new())`
   would have substituted a redirect-following client on builder failure. That
   is the same silent-substitution pattern this ADR removes elsewhere. The code
   now uses `expect`.
3. **Guards had two gaps.** `subscribe_service` was absent from the artifact
   owner guard (it cannot be banned from `reqwest` wholesale, since it builds the
   MusicBrainz client), and nothing pinned APIC's byte-only rule. Added
   `adr_0056_cover_art_fetch_uses_the_transport_module` and
   `adr_0056_apic_does_not_accept_declared_type_alone`.

## Behavior Changes

Intended:

- Transcript, thumbnail, and cover-art fetches now resolve redirects, reject
  non-HTTP(S) schemes, and reject non-success responses.
- Enclosures whose bytes are not a supported container are rejected before
  promotion, including when the feed declares no byte count.
- Transcripts served as markup are rejected.
- APIC artwork requires recognized image bytes.
- Thumbnails resolve for redirecting feeds and for images served under a
  non-image `Content-Type`.

Not changed: GIF animation and static preview handling, downscale behavior, cache
file format, enclosure size validation, local-file APIC extension handling.

## Test Coverage Added

- `remote_media`: URL parser space handling, scheme rejection
- `media::image_type`: magic bytes, declared-type filtering, precedence,
  octet-stream sniffing, markup rejection
- `media::image_cache`: redirect with spaces yields a thumbnail, octet-stream
  yields a thumbnail, markup yields neither image nor cache entry
- `audio_tags`: transcript markup rejection, APIC markup-declared-as-image
  rejection
- `track_compare`: non-container body rejected with no declared byte count,
  staging cleaned

Two pre-existing tests were serving the literal bytes `mp3data` as audio and
passing. They now use real ID3 magic bytes.

## Verification

- `cargo fmt --all -- --check` - clean
- `cargo clippy --quiet -- -D warnings` - clean
- `cargo test` - 1028 lib, 168 architecture, 0 failures

`cargo clippy --all-targets` reports pre-existing findings in `src/app/resize.rs`,
`src/db.rs`, `src/app/search_dispatch.rs`, `src/library/app_impl.rs`,
`src/runtime/musicbrainz_feed_saga.rs`, and two discover test-helper files. None
are in files this change touched.

## Status Reconciliation

Per the status hygiene rule in
`docs/plans/deferred-architecture-work-index.md`:

- ADR 0056 status updated to record the amendment and that Tasks 001-004 are
  implemented.
- Task packets 001-004 remain as the record of per-layer responsibility. This
  document replaces their four per-task review docs.
- Deferred-work index reconciled: ADR 0056 added to Recently Resolved, its three
  conditional follow-ups routed to a new Conditional Follow-Ups section, and
  priority item 3 (non-URL artwork rendering) annotated because the image decode
  path moved. No priority-list item was opened or closed by this work.

## Merge Recommendation

Merge. The guards are the durable part: the defect this ADR exists to fix was a
missed call site, and seven structural guards now fail the build if a media fetch,
image classification, or silent format guess reappears outside its owner.
