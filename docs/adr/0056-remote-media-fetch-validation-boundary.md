# ADR 0056: Remote Media Fetch Validation Boundary

## Status

Accepted - 2026-08-28. Amended 2026-08-28 after implementation review. The
amendment reverses this ADR's deferral of a shared fetch module, states the
three-layer ownership explicitly, adds the remote transcript path the original
version missed, and requires container validation on enclosures and byte-derived
image typing on every path. Tasks 001-004 track the code work, in order.

## Context

Subscription and download workflows fetch remote media from feed-owned URLs:

- audio enclosures that become local playable track files
- artwork references that become embedded APIC image frames
- transcript references that become embedded tag text
- artwork and cover-art references that are displayed without being stored

The first version of this ADR listed only the first two. The transcript path in
`audio_tags::read_text_reference` was missed entirely: it sits in the same module
as the APIC fetch, was untouched by the implementation commit, and had no
redirect handling, no scheme check, and only `error_for_status` for status. On
the redirect described below it writes the HTML redirect body into the file's
tags as transcript text.

That omission is the strongest argument in this ADR. Two fetch paths in one file
were fixed and the third was not, because the policy lived at each call site
instead of in one owner.

Some feeds return redirect responses before the final media response. The White
Triangles feed exposed a concrete failure mode: bare-domain media URLs redirected
to `www` URLs whose `Location` headers contained spaces. A client path that does
not resolve that redirect can persist the small HTML redirect body instead of the
intended MP3 or image bytes.

Redirect resolution at this boundary cannot be assumed. Reqwest clients follow
redirects by default, but they stop following and hand back the 3xx response
whenever the `Location` value fails to parse, and a 3xx status is not an error
status. The boundary therefore has to treat a redirect response as a case it
handles itself rather than as something the HTTP client has already resolved.

This is a source-boundary problem. The bad artifact is created before playback,
tag comparison, metadata display, or UI rendering sees the local file. Renderer
fallbacks must not hide or reinterpret the problem after the fact.

Not every remote image fetch is part of this boundary. The `download_image`
helper in `subscribe_service` and the fetch path in `media::image_cache` fetch
remote images whose bytes reach `media::image_from_bytes` for display only and
never become a local artifact.

The distinction is validation, not transport. Those paths are outside the
artifact validation rules below, because there is no artifact to corrupt. They
are not outside the redirect and scheme rules: they fetch the same feed-owned
URLs and fail on the same redirects, and their symptom is silently missing
artwork rather than a bad file. Task 004 brings them onto the shared transport
policy while leaving content validation caller-specific.

## Decision

Remote media fetch validation belongs at the boundary where untrusted remote
bytes enter local artifacts, and it is organized in three layers with one owner
each.

**Transport** is owned by a single remote media fetch module. Scheme allowlist,
bounded redirect resolution, `Location` repair, non-success rejection, and client
redirect configuration live there and nowhere else. Every media fetch in the
application routes through it.

**Classification** answers "what are these bytes?" and has one owner per media
kind: `src/audio_format.rs` for audio containers, `src/media/` for image types.
No other module decides what a byte sequence is.

**Artifact policy** is the only layer permitted to vary by caller, because the
artifacts genuinely differ. It stays with the artifact's owner.

The rules below are artifact policy. The transport rules they share are stated
once at the end of this section and are identical for every path.

Audio enclosure downloads must:

- validate the staged file length when the source enclosure declares a positive
  byte count
- fail before promotion when the actual local bytes do not match the source
  enclosure length
- fail before promotion when the staged bytes do not resolve to a supported
  audio container, independent of any declared byte count
- never fall back to the RSS-declared format when container detection fails,
  because that fallback relabels unrecognized bytes as the expected format

Remote APIC artwork downloads must:

- reject empty image responses
- derive image MIME from the actual image bytes before trusting the declared
  response content type
- never accept a remote URL extension as proof that the response body is image
  data

Remote transcript references must:

- reject markup responses, which a redirect landing page produces even on a
  success status
- reject transcripts that are empty after parsing

Display-only image fetches (thumbnails, cover-art lookup) must:

- derive image MIME from the actual image bytes before trusting the declared
  response content type
- treat an unrecognized type as no image, never as an assumed format
- write no cache entry for a failed fetch

Every path above shares one transport contract, owned by the transport module:
resolve HTTP redirects before the response body is used, reject unsupported URL
schemes, and reject non-success final HTTP statuses.

Redirect resolution may repair invalid but common feed/server `Location` values
by percent-encoding raw spaces before resolving relative to the previous URL.
This repair is defensive: the WHATWG URL parser already percent-encodes raw
spaces in a path, so the repair only covers `Location` values the parser rejects
outright. It must be kept only if a regression test fails without it.

Redirect depth must be bounded by one named constant in the transport module,
not by per-caller literals.

Boundary fetches must not depend on the calling HTTP client's implicit redirect
policy. Regression tests for this boundary must exercise the same client
configuration production uses, so that a passing test proves the production path
resolves the redirect.

Artifact owners keep their content rules: `track_compare` for downloaded track
files, `audio_tags` for embedded artwork and transcripts, `media` and
`subscribe_service` for display images. They call the transport module for bytes
and the classification owners for type. They do not implement transport, and they
do not classify.

Feed and API fetches (`src/rss/**`, `src/musicbrainz.rs`, `src/api.rs`,
`src/discover.rs`) are outside this boundary. They fetch documents, not media
bytes, and have different error and retry needs. Consolidating them is a separate
decision.

## Invariants

- UI renderers, view models, and metadata comparison surfaces do not hide,
  reinterpret, or relabel corrupted local media artifacts.
- A small redirect body must not be promoted as a playable audio file, including
  when the source declares no enclosure byte count.
- A small redirect body must not be embedded as APIC artwork.
- A redirect landing page must not be embedded as transcript text.
- Transport policy has exactly one implementation. A second redirect loop, scheme
  check, or status policy anywhere in `src/` is a defect regardless of whether it
  currently behaves correctly.
- Image classification has exactly one implementation, owned by `src/media/`.
  Audio container classification has exactly one implementation, owned by
  `src/audio_format.rs`.
- No path guesses a format for unrecognized bytes. Neither
  `unwrap_or(ImageFormat::Jpeg)` nor `unwrap_or(declared_format)` may return.
- Source-declared enclosure byte counts are treated as a validation fact when
  present and positive, and are never the only content check.
- Enclosure and APIC paths reject unrecognized bytes with comparable strength:
  neither promotes an artifact on the strength of a declared type alone.
- Display-only remote image fetches (`subscribe_service::download_image`,
  `media::image_cache`) stay outside the artifact validation rules only while
  their bytes never become a local artifact. Routing either into an artifact
  write requires moving it behind the full boundary first.
- Every remote media fetch, artifact-writing or display-only, resolves redirects
  and rejects unsupported schemes through the same transport policy. No caller
  relies on the HTTP client's implicit redirect behavior.
- Remote image type is derived from response bytes first on every path. A
  declared `image/*` type is a fallback, never a substitute, and no path
  silently assumes JPEG for an unrecognized type.
- Local APIC image files may still use path extension as a MIME hint because the
  operator controls the file path; remote APIC image responses may not.
- Remote media URL schemes are limited to HTTP and HTTPS until a later ADR
  expands the supported transport set.
- Redirect behavior is covered by regression tests for both enclosure downloads
  and APIC image downloads, using the client redirect configuration production
  actually passes in.
- Redirect depth is bounded by one shared named constant.

## Alternatives Considered

### Let Reqwest Handle Redirects Implicitly

Rejected as the durable boundary contract. Default redirect handling silently
gives up and returns the 3xx response when `Location` will not parse, and a 3xx
status passes `error_for_status`, so the failure mode reappears as a persisted
redirect body. An explicit loop in the transport module also states the rule in
one readable place, rather than encoding it in client construction that any
caller can change without noticing what it governs.

### Validate Only During Playback

Rejected. Playback can detect some corrupt files, but by then the bad bytes have
already been promoted into the local library. The failure should be caught while
the file is still staged.

### Trust Remote Artwork URL Extensions

Rejected. A redirected `.jpg` URL can return an HTML redirect body. Remote MIME
classification must be based on the final response bytes or a valid final
response content type, not the original URL suffix.

### Introduce A Shared Fetch Module Immediately

Originally deferred, now adopted. This is the main correction in the amendment.

The deferral said a shared helper was justified once a third call site needed the
same policy. That test was already met when the ADR was written and nobody
counted: five media fetch sites existed, two had the policy and three did not.
The deferral also mis-framed the threshold as a duplication concern, when the
real cost is divergence. Duplication is visible in review; divergence is not,
which is how the commit that added redirect handling twice left the third path in
the same file untouched, and nothing reported a gap.

The extraction covers transport only: scheme, bounded redirects, `Location`
repair, status. Enclosure size and container rules stay in `track_compare`,
APIC and transcript rules stay in `audio_tags`, and the display paths keep their
own content policy. Those four callers genuinely disagree about what a valid
response body is, and that disagreement is the reason to keep artifact policy
separate rather than to keep transport scattered.

### Keep Image Classification In `audio_tags`

Rejected. The byte sniffer was private to the tag module, so the thumbnail cache
and cover-art lookup could not use it and instead required a correct
`Content-Type`, producing no artwork for images served as
`application/octet-stream`. Exporting it from `audio_tags` would have made a
tag-writing module the classification owner for display surfaces. It moves to
`src/media/`, next to `ImageCache` and `image_from_bytes`, which already own
image handling.

## Consequences

Positive:

- Failed downloads stop before corrupt files are promoted.
- Adding a media fetch means calling one module, so a new path cannot silently
  ship with weaker rules than the paths beside it.
- Pre-download artwork appears for redirecting feeds and for servers that send a
  wrong `Content-Type`, because classification stops depending on the header.
- Missing artwork caused by redirect bodies is fixed at the tagging boundary.
- Regression tests capture the real redirect-with-spaces failure mode.
- Metadata and UI layers remain source-preserving and do not need compensating
  display logic.

Negative / risks:

- The extraction touches five call sites across four modules before any user
  visible fix lands. Task 001 is the largest diff in the sequence and changes no
  content rules.
- Transcript, thumbnail, and cover-art fetches begin rejecting responses they
  previously accepted. That is the point, but it converts some silent successes
  into visible failures.
- Servers that declare incorrect positive enclosure byte counts will cause a
  download failure instead of accepting the bytes opportunistically.
- Remote image responses with unsupported magic bytes and no valid image content
  type remain rejected even if their URL suffix looks like an image.
- Container detection recognizes only the formats in `AudioFormat`. A valid
  enclosure in an unsupported container now fails at download instead of being
  promoted under its declared format. That is intended: an unsupported container
  is not playable by this application either way.
- The boundary loop and the client's own redirect policy both count redirects, so
  the effective redirect budget for a default client is the product of the two,
  not the shared constant alone.

## Follow-Up Work

Open task packets, in order. Each depends on the one before it.

1. `docs/tasks/adr-0056-task-001-remote-media-transport-module.md` - create the
   transport module and migrate all five media fetch sites, including the missed
   transcript path.
2. `docs/tasks/adr-0056-task-002-image-classification-owner.md` - move image
   classification to `src/media/`, apply one MIME precedence rule everywhere, and
   remove the silent JPEG fallbacks.
3. `docs/tasks/adr-0056-task-003-artifact-content-policy.md` - enclosure
   container validation and transcript markup rejection.
4. `docs/tasks/adr-0056-task-004-remote-fetch-boundary-guard.md` - guard the
   layering last, once it exists.

The order matters. Applying content policy before the extraction would edit code
that the extraction then moves, and guarding before both would lock in the
current shape.

Task 002 is the one that restores missing album art on pre-download surfaces.
Task 001 is a prerequisite but fixes only the redirect half.

Later:

- Add operator-facing diagnostics if repeated size mismatches indicate a feed
  publisher has stale enclosure byte counts.
- Revisit the sniffer's format coverage if real feed artwork appears in formats
  outside PNG, JPEG, GIF, and WebP.

## References

- ADR 0004 - Format-neutral audio tag boundary
- ADR 0008 - Explicit ID3v24 write boundary
- ADR 0015 - Non-UI service boundaries
- ADR 0054 - Local metadata source-fact persistence
