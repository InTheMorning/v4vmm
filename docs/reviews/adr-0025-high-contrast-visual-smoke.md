# ADR 0025 High-Contrast Visual Smoke

## Result

Pass - 2026-05-01.

## Scope

- High Contrast Dark via isolated config.
- High Contrast Light via isolated config.
- Library, Discover, Settings, and header playback controls.
- Settings selector exposure for System, Dark, Light, High Contrast Dark, and
  High Contrast Light.
- Runtime repaint from High Contrast Dark to High Contrast Light.

## Findings

No blocking contrast, layout, text-overlap, or repaint issues were observed.

## Notes

- The smoke pass used temporary XDG config and data directories under `/tmp` so
  the user's normal v4vmm config and database were not touched.
- Screenshots were captured locally during review for inspection but are not
  committed.
- Follow-up System smoke confirmed the selector is exposed and repaints through
  GPUI window appearance.
