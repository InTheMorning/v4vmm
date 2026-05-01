# Post-ADR 0026 Task 003 Artwork Source Expansion Review

## Result

Pass for audit and current explicit support boundary - 2026-05-01.

## Scope

- Audited `ArtworkRef` variants.
- Checked Discover and Library rendering paths for release artwork.
- Confirmed whether a new ADR is required before supporting non-URL artwork
  sources.

## Current Variants

`ArtworkRef` currently defines:

- `Url(String)`
- `CacheKey(String)`
- `LocalPath(String)`
- `EmbeddedBytesKey(String)`

Only `ArtworkRef::Url` is constructed today. Existing constructors route
optional image URLs through the URL variant.

## Rendering Paths

| Variant | Constructed Today | Discover Rendering | Library Rendering | Status |
|---|---:|---|---|---|
| `Url` | Yes | Screen-owned fetch downloads the URL image into the inspector frame before rendering `DetailHeader`. | Screen-owned album thumbnail map resolves the URL before rendering `DetailHeader`. | Supported through existing screen adapters |
| `CacheKey` | No | No resolver. | No resolver. | Unsupported, explicit future contract |
| `LocalPath` | No | No resolver. | No resolver. | Unsupported, explicit future contract |
| `EmbeddedBytesKey` | No | No resolver. | No resolver. | Unsupported, explicit future contract |

## Findings

- Shared projections remain GPUI-free.
- `src/ui_entity.rs` accepts `header_image: Option<Arc<Image>>`; it does not
  resolve `ArtworkRef`.
- Image handles and cache access remain screen-owned, which matches ADR 0026.
- Non-URL variants are contract-shaped placeholders. Implementing real support
  for them requires defining cache-key, local-path, or embedded-byte ownership
  semantics first.

## Recommendation

- Do not implement non-URL artwork rendering in this task.
- Keep `Url` as the only supported artwork source until a producer and resolver
  contract exists.
- Require a new ADR before implementing `CacheKey`, `LocalPath`, or
  `EmbeddedBytesKey` if it changes image-cache semantics, database storage,
  file/path ownership, or public artwork contracts.

## Verification

- Documentation audit only.
- No runtime code changed.
- No tests were run.
