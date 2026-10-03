# v4vmm Design System vs. musicindex.org Look

This note maps the musicindex.org website tokens onto the v4vmm token system.
It is a read-only inventory. The work does not change a file in the v4vmm repository.

Sources read:
- `/home/citizen/build/v4vmm/AGENTS.md` (durable set, token discipline)
- `/home/citizen/build/v4vmm/docs/architecture/source-map.md`
- `/home/citizen/build/v4vmm/src/ui/tokens.rs`
- `/home/citizen/build/v4vmm/src/ui/theme_profiles.rs`
- `/home/citizen/build/v4vmm/src/ui/theme_bridge.rs`
- `/home/citizen/build/v4vmm/src/ui/style.rs`
- `/home/citizen/build/v4vmm/src/ui/control_styles.rs`
- `/home/citizen/build/v4vmm/src/ui/contrast.rs`
- `/home/citizen/build/v4vmm/src/theme_profile.rs`
- `/home/citizen/build/v4vmm/src/config.rs`
- `/home/citizen/build/v4vmm/src/app/bootstrap.rs`
- `/home/citizen/build/v4vmm/src/ui/primitives/*.rs`
- `/home/citizen/build/v4vmm/src/ui/composites/*.rs`
- `/home/citizen/build/v4vmm/src/ui/icons.rs`, `src/ui/layouts.rs`
- `/home/citizen/build/v4vmm/tests/architecture_tests.rs`
- `/home/citizen/build/musicindex/search.html` (this is the live site. `index.html` is discarded)

---

## 1. musicindex.org website tokens (source: `search.html`)

`search.html` is the live app shell, not a marketing page. It has a sidebar
search and browse list on the left, and a detail inspector on the right. It
is the closest web match to v4vmm's own Library/Search two-pane shell. Some
of its screens map onto v4vmm features directly. `.compare-table` runs an
RSS/tag/MusicBrainz-style metadata compare, the same function as
`track_metadata_grid.rs`. `.value-route-list`/`.history-list` show
payment routes, the same area as ADR 0076's stored payment routes.

### 1.1 `:root` custom properties

Dark is the default (`:root`, around line 43). Light overrides live under
`:root[data-theme="light"]` (around line 93). Dark values are shown first
below; the light override follows in the "Light value" column.

| Token | Dark value | Light value | Role |
|---|---|---|---|
| `--bg` | `#0b0b0d` | `#ffffff` | Page canvas |
| `--sidebar` | `#141417` | `#f5f5f7` | Left-pane background |
| `--surface` | `#1c1c1f` | `#ffffff` | Raised surface |
| `--surface2` | `#26262a` | `#ececef` | Higher-tier surface (placeholder art mix base) |
| `--fill` | `rgba(118,118,128,.24)` | `rgba(120,120,128,.14)` | Input/control fill |
| `--fill-soft` | `rgba(118,118,128,.14)` | `rgba(120,120,128,.08)` | Row hover fill |
| `--select` | `rgba(255,255,255,.12)` | `rgba(120,120,128,.18)` | Plain selection wash (where the gloss treatment, below, is not used) |
| `--sep` | `rgba(255,255,255,.1)` | `rgba(60,60,67,.16)` | Hairline border |
| `--bar-bg` | `rgba(11,11,13,.62)` | `rgba(255,255,255,.7)` | Top-bar backing, under a blur |
| `--text` | `#f5f5f7` | `#1d1d1f` | Primary text |
| `--muted` | `#a1a1a6` | `#5c5c62` | Secondary/subtitle text |
| `--accent` | `#2d7bff` | `#0a5bd6` | The one brand accent (blue) |
| `--focus` | `#2d7bff` | `#005bd3` | Focus ring |
| `--art-shadow` | `rgba(0,0,0,.5)` | `rgba(0,0,0,.18)` | Drop-shadow color, artwork only |
| `--gloss-hi` | `rgba(255,255,255,.14)` | `rgba(255,255,255,.7)` | Inset top highlight on the "gloss" selection sheen |
| `--veil-top` / `--veil-mid` | `25%` / `60%` | `55%` / `80%` | Hero scrim gradient stops, fading art into `--bg` |
| `--pill-bg` / `--pill-bg-hover` / `--pill-edge` | `--fill` / `--select` / `transparent` | `rgba(255,255,255,.78)` / `#ffffff` / `rgba(60,60,67,.2)` | Frosted pill-button fill/hover/edge |
| `--grain-opacity` | `.22` | `.28` | Hero noise-texture strength |
| `--brand-filter` | `none` | `brightness(0)` | Forces the color logo SVG to flat black in light mode |
| `--bar-h` | `52px` | `52px` | Top-bar height |
| `--gloss-top` / `--gloss-bottom` / `--gloss-edge` | `rgba(45,123,255,.24/.12/.34)` | `rgba(10,91,214,.2/.1/.4)` | Plain rgba copies of `--accent`, used for the selection sheen (a `color-mix()` gradient breaks in WebKit) |

Two more root values are inline SVG data URIs, not colors: `--mark-mask` (the
Music Index "MI" logomark, used as a CSS mask on detail-section headings and
the empty-state icon) and `--grain` (a `feTurbulence` noise filter, used as
the hero texture layer).

### 1.2 Entity color family (`--c-*`)

One hue per entity kind, set on a per-row custom property (`--entity`, e.g.
`style="--entity:var(--c-track)"`), not read from `--accent`. A code comment
states the design rule: "Feed, track, playlist and artist each have a hue of
their own. Host and author are shades of artist. The business roles are
shades of one violet. Red is only for live now."

| Kind | Dark | Light | Family |
|---|---|---|---|
| `--c-feed` | `#c4965b` | `#8a5e21` | tan/gold, own hue |
| `--c-track` | `#008b99` | `#008e9c` | teal, own hue |
| `--c-playlist` | `#8edfad` | `#006238` | mint/green, own hue |
| `--c-live` | `#ff453a` | `#d70015` | red — reserved for "live" only |
| `--c-artist` | `#fab5e5` | `#723763` | pink, own hue |
| `--c-host` | `#ffb4d9` | `#763559` | pink, shade of artist |
| `--c-author` | `#f3b7f0` | `#6d396b` | pink, shade of artist |
| `--c-label` | `#875ccf` | `#8d62d5` | violet, business-role family |
| `--c-publisher` | `#7c60d4` | `#8267db` | violet, business-role family |
| `--c-producer` | `#9158c8` | `#975ecf` | violet, business-role family |
| `--c-network` | `#9a54c0` | `#a05bc7` | violet, business-role family |
| `--c-hosting` | `#7065d8` | `#756bdf` | violet, business-role family |
| `--c-sponsor` | `#a251b7` | `#a957be` | violet, business-role family |

**Where the entity color is used — dots, fills, rings, never a colored
text badge:**

- **Dot.** `.kind-dot`: a 7px filled circle (`background: var(--entity)`)
  before a row title, shown only where a list mixes entity kinds. A hollow
  variant (`.kind-dot.hollow`) drops the fill and draws a
  `box-shadow: inset 0 0 0 1.5px var(--entity)` ring instead, meaning
  "inferred, not confirmed" (ADR 0010 in the musicindex repository).
- **Hero eyebrow dot.** The same solid/hollow pattern at 8px, next to the
  uppercase entity-type label above a detail page's hero title.
  `.hero-eyebrow.inferred::before` draws the hollow ring version.
- **Artwork-placeholder tint.** When no real art exists,
  `background: color-mix(in srgb, var(--entity) 45%, var(--surface2))` fills
  the hero frame/cover and `cover-art-placeholder` tiles. The hero's ambient
  background glow (behind blurred/desaturated art) is also entity-tinted:
  `color-mix(in srgb, var(--entity) 40%, transparent)`.
- **No colored text, no filled color badge.** Type identity in a placeholder
  is a plain two-letter monogram (`FD`, `TR`, `PB`, or `MI` as the generic
  fallback) in a neutral `--muted`-colored glyph. The entity hue never
  colors a text label or a filled pill the way a status badge would.

### 1.3 Gloss, grain, and art-shadow (the three "finish" tokens)

- **Gloss (selection sheen).** `background-image: linear-gradient(180deg,
  var(--gloss-top), var(--gloss-bottom))` plus
  `box-shadow: inset 0 1px 0 var(--gloss-hi), inset 0 0 0 1px var(--gloss-edge)`.
  This is the "selected" or "current" treatment for `browse-item`,
  `scope-option`, `result-item`, `recent-feed-tile`, and `copy-seg` — a
  layered glass sheen built only from rgba copies of `--accent`, never a flat
  highlight color.
- **Grain.** `.hero-grain` lays the `--grain` noise-SVG over the hero art at
  `opacity: var(--grain-opacity)` with `mix-blend-mode: overlay` — real film
  grain texture, combined with a blurred/saturated ambient art glow
  (`.hero-glow-art`, `filter: blur(90px) saturate(1.7)`) and a scrim that
  fades into `--bg` using the `--veil-top`/`--veil-mid` stops.
- **Art-shadow.** `--art-shadow` is used **only on artwork**, never on chrome
  (buttons, rows, panels get no shadow at all): `filter: drop-shadow(0 24px
  40px var(--art-shadow))` on the masked hero collage, `box-shadow: 0 24px
  48px var(--art-shadow)` on the 232px hero cover, and the lighter
  `0 12px 28px var(--art-shadow)` on 150px `cover-grid` tiles. The blur grows
  with the artwork's size.

### 1.4 The 52px blurred top bar and the sidebar

`#inspector-bar` (`min-height: var(--bar-h)` = 52px) floats over the detail
pane (`position: absolute`, `z-index: 3`) with
`backdrop-filter: blur(24px) saturate(180%)` over `--bar-bg`. It collapses to
fully transparent, with no blur and no border, when the inspector title is
empty (`#inspector-bar:has(#inspector-title:empty)`) — the bar is
context-sensitive chrome, not a fixed app-wide toolbar.

`#left-pane` is `width: clamp(280px, 24vw, 340px)`, with a faint accent-tinted
gradient wash over the top 320px of the sidebar background. It holds the
brand mark, a theme toggle, a pill search field, then either a "Browse" nav
list (five to seven icon+label rows, which collapses to a single
horizontally-scrolling row on a phone) or — once a query exists — a 3-way
segmented scope control, followed by the results list and a status line.

The hero action pills (`.copy-btn`, `.open-action`) reuse the frosted-glass
recipe at a smaller scale: `backdrop-filter: blur(20px)` over `--pill-bg`,
with an `inset 0 0 0 1px var(--pill-edge)` border standing in for a shadow.

### 1.5 Pills and the radius set in use

Pills are not a fixed-radius token; they are "radius = half of element
height," so a 44px-tall pill gets `border-radius: 22px` (`.theme-toggle`,
`.history-btn`, `.open-action`/`.copy-btn`, `#load-more`,
`.scope-control`'s current item via `44px` rows, `#inspector-back`). The
outcome is the same stadium shape v4vmm reaches with `Radius::Full` (999px),
just computed per control instead of fixed at an oversized constant.

Radii actually in use: **4px** (skeleton blocks), **6px** (40/44px row
thumbnails), **8px** (list rows, 64px `.compare-cover`), **10px** (search
input, scope control, 150px cover-grid art), **12px** (232px hero cover,
copy-segment chip), **22px** (pills, see above), and **50%** (`.kind-dot`
circle). There is no radius above 22px (not counting the 50%-circle case).

### 1.6 The 150px tile grid and artwork treatment

`.cover-grid` is `grid-template-columns: repeat(auto-fill, minmax(150px,
1fr))` with `1.5rem 1.4rem` gaps — related-entity tiles (1:1 art, 10px
radius, the art-shadow above). On a phone it becomes a horizontal
scroll-snap row of 140px tiles instead of wrapping. `.row-grid` is a second,
denser related-entity layout: `repeat(auto-fill, minmax(280px, 1fr))` with a
44px thumbnail per row.

Row/thumbnail sizes in use: 40px (`result-item`, `recent-feed-tile`, radius
6px), 44px (`playlist-row`, `.row-thumb`, radius 6px), 48px
(`playlist-row`'s thumb slot), 36/40px (`track-thumb`, radius 6px), 64px
(`.compare-cover`, radius 8px), 36px (`.compare-thumb`, radius 6px). Content
row heights are taller than a typical list row: `result-item`/`recent-feed-tile`
52px, `track-row` 56px, `playlist-row` 64px.

A missing-art placeholder is a flat `color-mix(entity, surface2)` tile
holding a centered two-letter monogram (`FD`/`TR`/`PB`/`MI`) — never a
gradient and never an emoji.

### 1.7 Typography

Figtree is self-hosted and actually wired in, unlike the discarded
`index.html`:

```css
@font-face {
  font-family: "Figtree";
  font-weight: 300 900;       /* one variable-font file covers the whole range */
  font-display: swap;
  src: url("assets/fonts/figtree-latin.woff2") format("woff2");
  unicode-range: U+0000-00FF, ...;   /* a second @font-face covers Latin Extended */
}
```

Body stack: `-apple-system, BlinkMacSystemFont, "SF Pro Text", "Figtree",
system-ui, sans-serif` — Apple devices get the system San Francisco font
first; Figtree is the actual brand font everywhere else. Base `font-size:
15px`, `line-height: 1.45`. `input[type="search"]` is pinned to `16px` so iOS
does not zoom the page on focus.

Sizes in use: `.7rem`/`.75rem` (11.2/12px, micro labels, placeholder
monograms), `.8rem`–`.875rem` (12.8–14px, row titles/subtitles, most
buttons), `.9rem`–`.95rem` (14.4–15.2px, pills, inspector title), `1rem`
(empty-state heading), `1.05rem` (hero summary body), `1.4rem` (22.4px,
section/empty-state headings), `clamp(2rem, 4.4vw, 4.5rem)` (32–72px, hero
title). Weights in use: 500 (buttons, row titles), 600 (emphasis, selected
state, section headings), 650 (hero summary link, history-kind label), 700
(hero eyebrow, section `h4`, placeholder monograms), 800 (hero title,
`.differs-mark`). Letter-spacing: `-.035em` on the hero title (tight
display), `-.02em` on section headings, `.08em` uppercase on the hero
eyebrow, `.06em` uppercase on comparison-table group headers.

### 1.8 Layout: sidebar + inspector, rows vs. tiles, detail pages

`#app` is a two-pane flex row: `#left-pane` (search/browse/results, fixed
width) and `#right-pane` (the absolutely-positioned inspector). Below 768px,
or in a short landscape, the layout stacks to one column and the inspector
becomes a full-screen overlay (`body.inspector-open`) with a back button.

A detail page is: a hero (ambient glow + grain + either a branded
clip-path-masked art collage — up to four images cut to the "MI" logomark,
used when there is no single square cover — or a plain square `.hero-cover`;
an eyebrow with the kind dot; a large title; a summary; pill actions, the
first of which is inverted to a solid `--text`-colored fill as the primary
action) followed by detail sections: a definition-list `detail-grid`, a
`track-list` (56px rows), a `cover-grid` or `row-grid` of related entities,
`contributor-list`/`value-route-list` (payment routes), a `compare-table`
(RSS/tag/MusicBrainz-style metadata comparison, with a `.differs` cell tinted
`color-mix(accent 9%, transparent)`), a `history-list`, and collapsible
`<details>`-based sections with a rotating chevron.

---

## 2. v4vmm tokens — `src/ui/tokens.rs`

### 2.1 Colors — `SemanticColor` (Apple HIG-style roles)

Every color is a named role, resolved per `Appearance` (`Light` / `Dark`), then
through a `ThemeProfile` (see §3). No raw hex lives outside `tokens.rs` and
`theme_profiles.rs`.

| Role | Dark value | Light value |
|---|---|---|
| `SystemBackground` | `#0f1117` | `#ffffff` |
| `SecondarySystemBackground` | `#1a1d27` | `#f2f2f7` |
| `TertiarySystemBackground` | `#232735` | `#ffffff` |
| `Label` | `#eceef5` | `#000000` |
| `SecondaryLabel` | `#b4bacb` | `#3c3c43` |
| `TertiaryLabel` | `#8a90a4` | `#6c6c70` |
| `QuaternaryLabel` | `#5f6577` | `#a9a9ad` |
| `SystemFill` | `#2a2d3a` | `#e5e5ea` |
| `SecondaryFill` | `#232735` | `#eeeef0` |
| `TertiaryFill` | `#1a1d27` | `#f2f2f7` |
| `SelectedContent` | `#2a3352` | `#d1e0ff` |
| `Separator` | `#2a2d3a` | `#c6c6c8` |
| `OpaqueSeparator` | `#6a708a` | `#8e8e93` |
| `Accent` | `#8b9bff` (blue-violet) | `#007aff` (iOS blue) |
| `AccentHover` | `#a5b2ff` | `#3394ff` |
| `AccentPressed` | `#7486f5` | `#0064d1` |
| `OnAccent` | `#0b0d13` | `#ffffff` |
| `Focus` | `#a8b6ff` | `#3394ff` |
| `Success` / `SuccessLabel` | `#7dd67d` | fill `#34c759`, label `#1f7a3a` |
| `Warning` / `WarningLabel` | `#ffd666` | fill `#ff9500`, label `#a05400` |
| `Danger` / `DangerLabel` | `#ff8585` | fill `#ff3b30`, label `#c2271c` |
| `Info` / `InfoLabel` | `#8b9bff` | fill `#007aff`, label `#005fcc` |
| `OnSuccess/Warning/Danger/Info` | all `#0b0d13` (dark-on-bright) | black for success/warning, white for danger/info |
| `DiffMatch` / `DiffDifferent` / `DiffMissing` | green/amber/orange | dark green/brown/red |
| `Id3FrameV22/V23Only/V24Only/Unknown` | purple/amber/teal/orange | darker variants |

v4vmm's accent in Dark mode is a **blue-violet (`#8b9bff`)**, not an orange.
There is no gold or purple *brand* role — `Id3FrameV22` happens to be purple
but it means "ID3 v2.2 frame", not a brand color.

Two extra `ThemeProfile` variants add high-contrast dark/light palettes
(see §3) with their own hard-coded hex tables in `theme_profiles.rs`.

### 2.2 Spacing — 4pt grid

| Token | Base px |
|---|---|
| `XXS` | 2 |
| `XS` | 4 |
| `SM` | 8 |
| `MD` | 12 |
| `LG` | 16 |
| `XL` | 24 |
| `XXL` | 32 |

### 2.3 Radius

| Token | Base px | Use |
|---|---|---|
| `SM` | 4 | chips, badges |
| `MD` | 6 | buttons, inputs |
| `LG` | 10 | popovers, sheets |
| `XL` | 14 | large overlays |
| `Full` | 999 | pill shape |

The largest named radius (14px) is far below the website's smallest
(`--radius-sm` = 12px is close, but `--radius-lg`/`--radius-xl` = 32/48px have
no v4vmm equivalent at all).

### 2.4 Typography — `FontSize` (type scale) and `Weight`

| Token | Base px |
|---|---|
| `Micro` | 11 |
| `Caption` | 12 |
| `Body` | 13 |
| `Headline` | 15 |
| `Title3` | 17 |
| `Title2` | 20 |
| `Title` | 24 |

`Weight`: `Regular` / `Medium` / `Semibold` / `Bold` (maps to
`FontWeight::NORMAL/MEDIUM/SEMIBOLD/BOLD`). There is no `Weight::Light` or
`Weight::Thin` token — the website leans on `font-weight: 100–300` for its
hero and section headings, which v4vmm's type system cannot express today.

Every `FontSize` also scales through `ScaleFactor` (`XSmall…XLarge`, ADR 0039)
with per-role interpolated multipliers (e.g. `Title` at `XSmall` = 20.40px,
at `XLarge` = 26.88px). This is a dynamic-type ramp, separate from the
website's `clamp()`-based responsive sizing, but serving the same goal
(text that adapts to context).

### 2.5 `Size` (semantic widths/heights)

| Token | px | Use |
|---|---|---|
| `MinHitTarget` / `RowLg` | 44 | minimum tap target, HIG row |
| `ButtonSm` | 28 | compact button |
| `ButtonMd` | 32 | default button |
| `ButtonLg` | 40 | prominent dialog button |
| `MenuCompact` | 160 | compact menu |
| `MenuRegular` | 220 | regular menu/popover |
| `MenuWide` | 280 | wide menu |
| `ColumnShort/Regular/Tall` | 240/320/480 | scrollable columns |
| `RowMd` | 36 | menu row |
| `ContentTileWidth` | 176 | Music content-tile outer width |
| `ContentTileArtwork` | 152 | Music content-tile artwork edge |
| `NoticeWidth` | 600 | empty-state / report measure |

### 2.6 `SkeletonBlock` (loading placeholders)

`InspectorTitle` 220×22, `InspectorSubtitle` 160×14, `InspectorCaption`
120×12, `FeedTileSubtitle` 96×12, `TrackNumber` 12×16, `TrackDuration` 32×16.

### 2.7 Icon sizes — `src/ui/icons.rs` / `src/ui/layouts.rs`

| Token | px | Use |
|---|---|---|
| `IconSize::Action` | 14 (inside an 18px hit target, `ACTION_ICON_SIZE`) | toolbar/action icons |
| `IconSize::Transport` | tracks `FontSize::Body` base (13) | inline glyph aligned to body text |

Icons come from the bundled Lucide SVG set (`gpui-kit-assets`), tinted through
`SemanticColor`, not through per-icon brand colors (except `IconName::Rss`
and `IconName::Nostr`, which carry a fixed brand fill checked for contrast).

### 2.8 Artwork/thumbnail sizes

See §5 for the full inventory. Named sizes:

- `ThumbnailSize` (composite): `Sm` 32, `Md` 48, `Lg` 80.
- `ImageSize` (primitive): `Sm` 32, `Md` 48, `Lg` 80, `Xl` 152, `XXl` 200.
- `Size::ContentTileArtwork`: 152 (Music content-tile grid).

All scale through `ScaleFactor::chrome_multiplier()` (CHROME domain, not TYPE).

### 2.9 `ScaleFactor` and `Environment`

`ScaleFactor` has five steps: `XSmall` (0.85×) … `Medium` (1.0×, default) …
`XLarge` (1.25×). `Spacing`, `Radius`, `Size`, `SkeletonBlock`, icon and
artwork sizes all multiply by this single CHROME coefficient. `FontSize`
uses a separate, per-role TYPE interpolation (§2.4) — the two domains are
kept deliberately apart (ADR 0039).

`Environment` is a single `gpui::Global` struct bundling `profile`
(`ThemeProfile`), `appearance` (`Light`/`Dark`), `scale` (`ScaleFactor`), and
`reduce_motion`. Primitives and composites read it through
`Environment::current(cx)` instead of several separate globals.

---

## 3. Theme construction

### 3.1 `src/theme_profile.rs` — `ThemeProfile` (the persisted choice)

GPUI-free enum, config key **`theme_profile`** in `config.toml`
(`rename_all = "kebab-case"`):

| Value | `as_str()` | Base appearance |
|---|---|---|
| `System` | `"system"` | follows the OS |
| `Dark` (default) | `"dark"` | Dark |
| `Light` | `"light"` | Light |
| `HighContrastDark` | `"high-contrast-dark"` | Dark |
| `HighContrastLight` | `"high-contrast-light"` | Light |

### 3.2 `src/ui/theme_profiles.rs`

Resolves a `(ThemeProfile, SemanticColor)` pair to a concrete `Rgba`.
`System` resolves to the platform's current light/dark state at render time.
`HighContrastDark`/`HighContrastLight` have their own full hex tables (pure
black/white backgrounds, pure-white/black labels, cyan/blue accents) —
separate from the base Dark/Light palettes in `tokens.rs`.

### 3.3 `src/ui/theme_bridge.rs`

`install_theme(profile, scale, cx)` / `install_theme_for_window(...)` is
called once at startup (Dark/Medium, before config loads) and again once the
real profile/scale is known. It:

1. Installs the `Environment` global (profile, appearance, scale).
2. Calls `gpui_component::Theme::change(mode, …)` to reset that crate's
   defaults.
3. Overwrites ~40 fields of `gpui_component`'s `ThemeColor`
   (`background`, `popover`, `primary`, `secondary`, `muted`, `input`,
   `border`, `ring`, `accent`, `link`, `success/danger/warning/info`,
   `sidebar_*`, `list_*`, `table_*`, `title_bar*`, `tab*`, `scrollbar*`,
   `overlay`, …) with the resolved `SemanticColor` values, converted
   `Rgba → Hsla`.
4. Calls `Theme::sync_base(cx)` and `cx.refresh_windows()` so every open
   window repaints immediately.

This is the single bridge point: every third-party `gpui-component` widget
(`Button`, `Popover`, `Input`, `Sidebar`, tables, tabs) inherits v4vmm's
palette through this file, never through per-widget overrides.

### 3.4 `src/ui/style.rs` (legacy compatibility shim)

Pre-token-migration color/spacing/radius/typography helpers
(`color::bg_canvas()`, `spacing::MD`, `radius::SM`,
`typography::type_title(el)`, …). All colors still resolve through
`SemanticColor` via `role()` — no raw hex — but spacing/radius/typography
here are **fixed, unscaled** `Pixels` constants kept only for screens that
have not migrated to the `tokens::*` enums yet. New code should use
`tokens::Spacing` / `tokens::Radius` / `tokens::FontSize` directly.

### 3.5 `src/ui/control_styles.rs` — `ControlStyle` (ADR 0025)

Screens pick a product-intent role; this module owns the concrete button
variant/size/token mapping:

| `ControlStyle` | `ButtonVariant` | Size | Font | Radius | Foreground | Border |
|---|---|---|---|---|---|---|
| `Primary` | Filled | Md | Body | MD | — | — |
| `Secondary` | Tinted | Md | Body | MD | — | — |
| `Ghost` | Plain | Md | Body | MD | Accent | — |
| `Destructive` | Destructive | Md | Body | MD | — | — |
| `ToolbarIcon` / `RowAction` | Plain | Sm | Caption | SM | Accent | — |
| `DestructiveRowAction` | Plain | Sm | Caption | SM | DangerLabel | — |
| `MetadataAction` | Plain | Sm | Micro | SM | Accent | Accent |
| `Pill` | Tinted | Sm | Micro | Full | — | — |

`ToolbarIcon`, `RowAction`, and `DestructiveRowAction` default to showing a
hover tooltip (`prefers_tooltip()`).

### 3.6 `src/ui/primitives/button.rs` — `ButtonVariant` / `ButtonSize`

`ButtonVariant`: `Filled`, `Tinted`, `Plain`, `Destructive`. Tinted buttons
composite the accent color over the surface at two fixed alpha steps:
`TINTED_BUTTON_BG_ALPHA = 0.08`, `TINTED_BUTTON_HOVER_BG_ALPHA = 0.12` — the
closest thing v4vmm has to the website's `--accent-glow` wash, but it is an
alpha-composited tint, not a separate glow token, and it is not used as a
free-standing background glow anywhere else.

`ButtonSize`: `Sm` 28px, `Md` 32px, `Lg` 40px — all wrapped in the shared
44pt minimum hit-target contract.

Buttons never use a `box-shadow`-equivalent; v4vmm has no elevation/shadow
token at all (see §7, gap 1).

### 3.7 `src/ui/primitives/surface.rs` — `SurfaceElevation`

| Elevation | Background | Border | Radius |
|---|---|---|---|
| `Sunken` | `SecondarySystemBackground` | `Separator` (hairline) | `MD` (6px) |
| `Raised` | `SecondarySystemBackground` | `OpaqueSeparator` | `LG` (10px) |
| `Floating` | `TertiarySystemBackground` | `OpaqueSeparator` | `LG` (10px) |

v4vmm expresses "raised" purely through background tier + border weight, not
through shadow/blur. This is the opposite of the website's approach, which
uses a flat near-white surface everywhere and expresses elevation only
through a hover drop-shadow.

### 3.8 `src/ui/contrast.rs`

WCAG 2.1 contrast math (`ratio`, `relative_luminance`, `composite_over`) plus
`REQUIRED_PAIRS`: an explicit allow-list of every (foreground, background,
WCAG level) combination the app actually renders. Tests iterate this list for
`Light`, `Dark`, and both high-contrast profiles, and separately verify the
alpha-composited tinted-button backgrounds and the two brand icon fills
(RSS, Nostr). Any new color pairing a redesign introduces must be added here
and must pass 4.5:1 (normal text) or 3:1 (large text/graphics) before the
suite is green.

---

## 4. Fonts

**Current state:** v4vmm sets no explicit `font_family`. The `gpui-component`
`Theme` default is the literal string `".SystemUIFont"`
(`gpui-component-0.6.1/src/theme/mod.rs:625`). This is a virtual family name.
GPUI's `font-kit` backend (the `font-kit` feature on `gpui_platform` in
`Cargo.toml`) resolves this name to the platform UI font at paint time. On
Linux, this is whatever `fontconfig` names as the default sans family.
v4vmm does not override `theme.font_family` in `theme_bridge.rs`.

The monospace family (`mono_font_family`, used by `tokens::log_font_family`
for all log text, ADR 0063) follows the same pattern. But `gpui-component`
adds one safety step. `theme/mono_font.rs` checks installed fonts once per
process. It replaces the platform-default mono family with an installed
alternative (`Noto Sans Mono`, `Liberation Mono`, or `Ubuntu Mono` on Linux)
when the default is missing. The module comment gives the reason: GPUI stops
with an error on the first line laid out in a family it cannot find. No such
check exists for the sans family.

`gpui-kit-assets` (the crate given to `.with_assets(...)` in
`src/app/bootstrap.rs`) embeds only **icons** (Lucide SVG files). It carries
no fonts and has no font-loading API.

**What loading Figtree would take:**

1. Add the two `.woff2` files as embedded assets. (An alternative is to
   change them to `.ttf`/`.otf`. Format support depends on the
   `font-kit`/text-system backend.) Use a small `rust_embed`-style
   `AssetSource` that v4vmm registers alongside `gpui_kit_assets::Assets`.
   Then read the file bytes at startup and call the GPUI text system's
   font-registration call before the first window opens. Check the
   method name and signature against the pinned `gpui-pre 0.3.1` version.
   A likely candidate is `cx.text_system().add_fonts(...)`.
2. Set `theme.font_family = "Figtree".into()` in
   `src/ui/theme_bridge.rs::install_theme_for_appearance`. Place it next to
   the existing `theme.mono_font_family` handling. Each `gpui-component`
   widget and each `tokens::FontSize`-based label then picks it up through
   the existing bridge. No per-screen font change is necessary.
3. Add a mono-style installed-font fallback check, matching
   `gpui-component`'s `mono_font.rs`. A machine without the embedded font
   registered correctly then falls back to `.SystemUIFont`, and does not
   stop with an error on first layout.
4. Figtree ships only Regular-to-Bold static weights in most distributions.
   Confirm whether the `.woff2` files here are a variable font (one file,
   many weights) or fixed-weight instances. v4vmm's `Weight` enum has only
   four steps (`Regular/Medium/Semibold/Bold`). The website uses weights as
   low as 100–200 for its display type. Figtree's lower weight range would
   need to supply those.

No architecture guard forbids or requires a specific font family today. This
change needs no ADR exception on that point. But the **token discipline**
durable rule in AGENTS.md still applies: set the font family once, in the
theme bridge. Do not hard-code it per screen.

---

## 5. Primitives catalog (`src/ui/primitives/`)

| Name | Purpose | Main tokens |
|---|---|---|
| `button.rs` (`Button`) | HIG button, 4 variants × 3 sizes, native (no `gpui_component::Button`) | `ButtonVariant`, `ButtonSize`, `SemanticColor`, `Radius::MD` |
| `context_menu.rs` | Floating row-action menu, built on `Popover` | `Popover`, `Spacing`, `SemanticColor` |
| `divider.rs` (`Divider`) | 1px hairline | `SemanticColor::Separator` |
| `image.rs` (`Image`) | Lowest-level artwork tile, `ObjectFit::Cover`, GIF frame-id stamping | `ImageSize`, `Radius` |
| `label.rs` (`Label`) | Token-driven text span | `FontSize`, `SemanticColor`, `Weight` |
| `loading.rs` | Muted italic "loading/empty" line | `SemanticColor::TertiaryLabel`, `FontSize` |
| `multiline_text.rs` | SwiftUI-style `lineLimit`-style truncation with `…` | `FontSize` |
| `popover.rs` (`Popover`) | Canonical floating panel + arrow, wraps `gpui_component::popover` | `SurfaceElevation`, `Radius` |
| `primary_selection.rs` | Linux PRIMARY-selection clipboard glue (ADR 0071) | n/a (behavior, not visual) |
| `section_header.rs` | Small bold secondary-colored group heading, optional disclosure chevron | `FontSize`, `SemanticColor::SecondaryLabel` |
| `skeleton.rs` (`Skeleton`) | Muted block placeholder, no shimmer | `SemanticColor::TertiaryFill`/`SystemFill`, `Radius` |
| `stack.rs` (`VStack`/`HStack`/`ZStack`/`Spacer`) | SwiftUI-style flex layout wrappers | `Spacing` |
| `status_badge.rs` | Show-state badge geometry/colors (ADR 0063) | `SemanticColor`, `Radius` |
| `surface.rs` (`Surface`) | Card/panel/popover body container | `SurfaceElevation` → background/border/`Radius` |
| `tooltip.rs` | Compact hover help wrapper | `FontSize::Caption` |

## Composites catalog (`src/ui/composites/`)

| Name | Purpose | Main tokens |
|---|---|---|
| `action_button.rs` | Metadata-action button via `ControlStyle` | `ControlStyle::MetadataAction` |
| `action_row.rs` | Inspector action-row stack, neutral/danger messages | `Spacing`, `SemanticColor` |
| `breadcrumb_trail.rs` | Breadcrumb nav, typed navigation targets | `FontSize`, `SemanticColor` |
| `confirmation_dialog.rs` | Confirm/Cancel dialog body | `Button`, `Spacing` |
| `detail_grid.rs` | Scale-aware key/value inspector table | `Spacing`, `FontSize` |
| `detail_header.rs` | Thumbnail + badge + title + subtitle header | `Thumbnail`, `ThumbnailSize::Lg`, `TagBadge` |
| `disclosure_group.rs` | Collapsible section header | `SectionHeader::disclosure` |
| `file_header.rs` | Embedded-tag file header (artwork, badge, actions, path) | `Thumbnail::Lg`, `TagBadge` |
| `filter_chip_strip.rs` | Frame content-filter chips (segmented or menu) | `SegmentedControl` |
| `frame_shell.rs` | Workspace frame chrome (title, history, close, menu) | `Spacing`, `SemanticColor` |
| `identity_action.rs` | Website/Nostr identity action buttons | `ControlStyle` |
| `library_filter_control.rs` | Library-membership tri-state filter | `SegmentedControl` |
| `list_row.rs` (`ListRow`) | Generic horizontal row shape | `Spacing`, `Radius` |
| `live_status_strip.rs` | Compact "open Show" glance strip | `SemanticColor`, `IconSize` |
| `log_frame.rs` | Framed, scrollable log viewport | `LOG_TEXT_SIZE`, mono font |
| `maintenance_forms.rs` | Session-maintenance explanation/actions/report | `Spacing`, `Surface` |
| `maintenance_page.rs` | Separate repair/diagnostics actions + report views | `Spacing`, `Surface` |
| `musicbrainz_panel.rs` | MusicBrainz release picker + artwork | `Thumbnail::Lg`, `EntityKind::Track` |
| `page_scroll_content.rs` | Scroll clearance wrapper for pages with nested scrollbars | `Spacing` |
| `playlist_popover.rs` | "Add to Playlist" popover + inline create | `Popover`, `Button`, `Divider` |
| `release_detail_surface.rs` | Shared release/feed detail structure | `Spacing` (scale-aware, guarded) |
| `segmented_control.rs` (`SegmentedControl`) | Mutually-exclusive selector on `Button` | `Button`, `Spacing` |
| `selectable_text.rs` | Selectable read-only log text | shares `ListRow`/log tokens |
| `settings.rs` | Settings form rows and bounded content | `Spacing`, `Surface` |
| `show_card.rs` | Show dashboard summary card | `SemanticColor` (state colors) |
| `show_detail_panel.rs` | Show side panel (detail + cuelist + actions) | `Spacing`, `Button` |
| `show_log_pane.rs` | Show cards + bottom log pane, scroll priority | `Spacing`, `log_frame` |
| `skeleton_inspector.rs` | Loading placeholder shaped like a populated inspector | `Skeleton`, `SkeletonBlock` |
| `skeleton_track_row.rs` | Loading placeholder shaped like `TrackRow` | `Skeleton`, `SkeletonBlock` |
| `split_pane.rs` | Resizable split-pane shell | `Spacing`, drag handle styling |
| `startup_report.rs` | Responsive startup/recovery report + actions | `Spacing`, `Surface` |
| `tag_badge.rs` (`TagBadge`) | Entity-type uppercase pill (artist/feed/track/…) | `EntityKind`, `Radius`, `SemanticColor` |
| `thumbnail.rs` (`Thumbnail`) | Square artwork tile + emoji fallback | `ThumbnailSize`, `Radius`, `EntityKind::emoji()` |
| `track_detail_surface.rs` | Shared track detail layout (header/summary/sections) | `Spacing`, surfaces |
| `track_header.rs` | Track inspector header (artwork, badge, title, artist) | `Thumbnail::Lg`, `EntityKind::Track` |
| `track_metadata_grid.rs` | RSS/tag/MusicBrainz comparison grid shell | `Spacing`, grid columns |
| `track_row.rs` (`TrackRowVm`) | Standard track list-row shape | `ListRow`, `Thumbnail::Sm`, `Label` |
| `view_mode_control.rs` | Content view-mode selector (list/tile) | `SegmentedControl` |

---

## 6. Artwork / thumbnail render sites

| Call site | Component | Size | Fallback | Radius |
|---|---|---|---|---|
| `src/ui/composites/track_row.rs:193` | `Thumbnail::new(EntityKind::Track, Sm)` | 32px | 🎶 emoji on `SystemFill` | `Radius::SM` (4px) |
| `src/ui/composites/detail_header.rs:126` | `Thumbnail::new(kind, Lg)` | 80px | emoji on `SystemFill` | `Radius::MD` (6px) |
| `src/ui/composites/track_header.rs:105` | `Thumbnail::new(Track, Lg)` | 80px | 🎶 emoji | `Radius::MD` |
| `src/ui/composites/file_header.rs:93` | `Thumbnail::new(Track, Lg)` | 80px | 🎶 emoji | `Radius::MD` |
| `src/ui/composites/musicbrainz_panel.rs:78` | `Thumbnail::new(Track, Lg)` | 80px | 🎶 emoji | `Radius::MD` |
| `src/ui/shells/library/track_detail_metadata_values.rs:227` | `Thumbnail::new(Track, Lg)` | 80px | 🎶 emoji | `Radius::MD` |
| `src/ui/shells/entity.rs:283` | `Thumbnail::new(Artist, Sm)` | 32px | 🎤 emoji | `Radius::SM` |
| `src/ui/shells/publisher.rs:213` | `Thumbnail::new(Release, Sm)` | 32px | 💿 emoji | `Radius::SM` |
| `src/ui/shells/library/feed_list.rs:73` | `Thumbnail::new(Feed, Sm)` | 32px | 📡 emoji | `Radius::SM` |
| `src/ui/shells/library/content_list.rs:215` (list mode) | `Thumbnail::new(entity_kind, Sm)` | 32px | emoji | `Radius::SM` |
| `src/ui/shells/library/content_list.rs:315` (tile mode) | `ImagePrimitive::new(image).dimension(ContentTileArtwork)` | 152px (`Size::ContentTileArtwork`) | plain `SystemFill` square with `Separator` border (`render_empty_content_tile_artwork`, no emoji) | `Radius::MD` |
| `src/ui/shells/library/content_list.rs:384` (tile skeleton) | `Skeleton::block(artwork_size, artwork_size)` | 152px | n/a (loading state) | `Radius::MD` |
| `src/ui/shells/search_results_inspector.rs:251` | `Thumbnail::new(kind, Lg)` (no `.image(...)`, always fallback) | 80px | emoji | `Radius::MD` |
| `src/ui/shells/search_result_rows.rs:86` / `:115` | `Thumbnail::new(kind, Sm)` | 32px | emoji | `Radius::SM` |
| `src/ui/shells/library/thumbnail.rs` (`render_album_thumb`) | legacy `ImagePrimitive` / fallback glyph div, caller-chosen `size: f32` | caller-supplied | generic glyph (`LibraryViewModel::album_thumb_display().fallback_icon`), not an `EntityKind` emoji | `Radius::SM` |

Every image-present path uses `ObjectFit::Cover` (`src/ui/primitives/image.rs`)
so artwork always fills its box without letterboxing. Every fallback is a
flat `SystemFill`-colored square (or, for the tile grid, an outlined empty
square) with a centered emoji or glyph — there is no gradient placeholder
anywhere in v4vmm, unlike the website's gradient-filled `.artist-visual` and
`.player-thumb` stand-ins.

---

## 7. Architecture guards that constrain tokens/colors

A redesign must keep these green (`cargo test --test architecture_tests`):

| Guard (function) | ADR | What it forbids/requires |
|---|---|---|
| `screens_do_not_reintroduce_raw_color_or_numeric_px_literals` | ADR 0023 | No `rgb(...)` or bare numeric `px(...)` literal in screen code — must be a named token |
| `composites_do_not_reintroduce_raw_color_or_numeric_px_literals` | ADR 0034 | Same rule, scoped to `src/ui/composites/` |
| `ui_style_resolves_colors_through_token_layer` | ADR 0038 task 004 | `src/ui/style.rs` may not contain `gpui::rgb(...)` / `rgb(0x...)` — every color must resolve via `SemanticColor`/`role(...)` |
| `ui_components_do_not_bypass_theme_profile_resolution` | — | Call sites must use `tokens::color`/`resolve_color` (so the active `ThemeProfile` is honored), not a cached/raw value |
| `shared_ui_render_paths_use_scale_aware_tokens` | ADR 0034 | Shared UI render paths must call `.scaled(cx)` token accessors, not unscaled `.px()` |
| `interactive_surfaces_route_through_minimum_hit_target_token` | — | The 44px minimum hit target must stay a named `Size::MinHitTarget` constant |
| `playlist_rows_scale_through_design_tokens` | ADR 0044 | Playlist rows must scale through tokens, not fixed pixels |
| `release_detail_surface_uses_scale_aware_spacing_tokens` | — | `release_detail_surface.rs` must use scale-aware `Spacing`, not legacy fixed constants |
| `playlist_popover_menu_rows_use_leading_alignment_and_token_padding` | — | Popover menu rows must use `Spacing::SM` padding |
| Several screen-local `rgb(`/`gpui::rgb(` forbids (e.g. `adr_0047_phase_d_filter_controls_render_through_frame_shell`, `adr_0047_task_013_frame_shell_renders_breadcrumb_chrome`, `adr_0047_task_014_search_results_inspector_shell_contract`) | ADR 0047 | Situational per-screen bans on raw color literals, tied to specific migrations |
| `dark_palette_meets_wcag` / `light_palette_meets_wcag` (contrast.rs) | — | Every pair in `REQUIRED_PAIRS` must clear 4.5:1 (text) or 3:1 (large/graphic) |
| `high_contrast_dark_profile_meets_wcag` / `high_contrast_light_profile_meets_wcag` | — | Same, for the two high-contrast profiles |
| `pressable_button_token_pairs_meet_wcag` | — | Filled/destructive/plain/tinted button foreground-on-background pairs, including the two tint alpha steps, must clear WCAG |
| `brand_protocol_icon_fills_contrast_on_current_dark_canvas` | — | RSS and Nostr brand-colored icon fills must clear 3:1 on the dark canvas |
| `high_contrast_profiles_are_distinct_from_base_profiles` | — | High-contrast tokens must differ from the base Dark/Light values, so the profile is not a no-op |

**Result for a Figtree/accent-orange redesign:** a new accent color, glow, or
gradient must route through a named `SemanticColor`. This can be a new role
added to that enum. It must also route through `theme_bridge.rs`. Add it to
`contrast.rs`'s `REQUIRED_PAIRS` list, and it must pass WCAG. A new geometry
value, such as a large radius, must be a named `Radius`/`Size` token. The
token must resolve through `.scaled(cx)`. It must not be a literal `px(...)`
value in a screen or composite file.

---

## 8. Mapping table — website token → closest v4vmm token → gap

| Website token/effect | Value | Closest v4vmm token today | Gap |
|---|---|---|---|
| `--accent` (one blue) | `#2d7bff` dark / `#0a5bd6` light | `SemanticColor::Accent` (`#8b9bff` dark / `#007aff` light) | **Smallest gap in this document.** Both are blue. v4vmm's dark accent leans more violet and lighter; its light accent (`#007aff`, iOS blue) is close to the website's `#0a5bd6`. This needs a value tune, not a new role. |
| `--c-feed`/`--c-track`/`--c-playlist`/`--c-artist`/`--c-label`/`--c-publisher`, and seven more (13-hue entity family) | see §1.2 table | `EntityKind::fill_token()` (`tag_badge.rs`), which reuses `Success`/`Warning`/`Info`/`Danger`/`Accent`/`SystemFill` | **Role conflict, not a missing token.** v4vmm already colors entities by kind, but it borrows the 6-color status palette to do it. `Track` and `Playlist` both resolve to `Info` (one hue for two kinds); `Publisher` resolves to `Danger` (red), which the website reserves for "Live" only. v4vmm has no dedicated identity-color family independent of status meaning, and no `Host`/`Author`/`Producer`/`Network`/`Hosting`/`Sponsor` equivalents at all. |
| `.kind-dot` (7–8px solid/hollow entity dot) | `background: var(--entity)` or an inset ring | none | **No equivalent.** v4vmm shows entity kind through `TagBadge`, a filled text pill (`"artist"`, `"track"`, …), never a small color dot, and has no "confirmed vs. inferred" visual (solid vs. hollow). |
| Placeholder monogram (`FD`/`TR`/`PB`/`MI`) | 2-letter glyph on a tinted tile | `EntityKind::emoji()` (🎤 🎶 📡 💿 …) | **Different content language**, same job (identify a missing-art row/tile). A redesign following the website would swap emoji for short letter codes — a `Thumbnail`/`Image` fallback change, not a new token. |
| `--gloss-top`/`--gloss-bottom`/`--gloss-edge`/`--gloss-hi` (selection sheen) | rgba copies of `--accent` | `SemanticColor::SelectedContent` (flat fill, `#2a3352` dark / `#d1e0ff` light) | **Different mechanism for the same state.** Both mark "selected/current," but the website layers a gradient + two inset `box-shadow` highlights from the accent; v4vmm uses one flat color. No gradient or inset-highlight primitive exists to reproduce the sheen. |
| `--grain` (SVG noise texture) | `mix-blend-mode: overlay`, hero only | none | **No equivalent.** No texture/noise concept exists in v4vmm's token system. |
| `--art-shadow` (artwork-only drop shadow, 12–48px blur scaled by art size) | `rgba(0,0,0,.5)` dark / `rgba(0,0,0,.18)` light | none | **No shadow token**, but note the website itself shadows only artwork, never chrome — closer to v4vmm's existing restraint (`SurfaceElevation` uses border, not shadow, for chrome) than the discarded marketing page was. The gap is narrower than it first looks: v4vmm would need one new artwork-only shadow token, not a general elevation system. |
| `backdrop-filter: blur(24px) saturate(180%)` (52px top bar) | `--bar-bg` | none (`FrameShell`/`frame_shell.rs` renders an opaque bar) | **No equivalent**, and no context-sensitive collapse: the website's bar goes fully transparent when the inspector title is empty; v4vmm's frame chrome does not currently vary by content state. |
| `--bar-h: 52px` | — | `TAB_BAR_HEIGHT` (44px, `src/ui/layouts.rs`) | **Close, not exact.** v4vmm's nearest named bar-height constant is smaller; a redesign would add or adjust a token rather than invent a new family. |
| Pills, radius = half of control height (22px at 44px tall) | `border-radius: 22px`/`100px` | `Radius::Full` (999px, fixed) | **Matches in outcome.** Both always draw a stadium shape; v4vmm forces it with an oversized constant instead of computing half the height, which is a simpler, already-solved case. |
| Radii 4/6/8/10/12px (chrome and small art) | see §1.5 | `Radius::SM` 4, `MD` 6, `LG` 10, `XL` 14 | **Close fit.** 4, 6, and 10 already exist as named tokens. Only 8px (list rows) and 12px (the 232px hero cover) have no matching step; `XL` (14) is unused by anything in `search.html`. |
| Sidebar `width: clamp(280px, 24vw, 340px)` | — | `INSPECTOR_WIDTH` 360px (a different pane, v4vmm's right-hand detail rail) | **No matching left-rail width token.** v4vmm's closest named width governs the opposite pane (its inspector sits on the right, not a fixed-width search rail on the left). |
| `cover-grid` 150px auto-fill tiles / `row-grid` 280px auto-fill rows | — | `Size::ContentTileArtwork` 152px / `FEED_TILE_WIDTH` 140px / `SEARCH_TILE_WIDTH` 168px | **Close fit.** v4vmm's existing tile tokens are within a few px of the website's; this is a value check, not a new token. |
| Content row heights: 52px (`result-item`), 56px (`track-row`), 64px (`playlist-row`) | — | `Size::RowMd` 36px, `Size::RowLg` 44px | **v4vmm's rows run shorter.** Even the tallest named row token (44px) is below the website's shortest content row (52px); a redesign needs a new, taller row-height step. |
| Figtree, self-hosted, one variable file, weight range 300–900 | `assets/fonts/figtree-latin*.woff2` | `.SystemUIFont` (platform default; no custom font loaded) | **No font loaded.** Unlike the discarded marketing page, the real site both ships and uses Figtree, so there is no open question about whether it is a variable font — it is one, confirmed by `font-weight: 300 900` in the `@font-face` rule. See §4 for what loading it would take. |
| Hero title `font-weight: 800`, section heading `700` | — | `Weight::Bold` (`FontWeight::BOLD`) | **Matches well.** Unlike the discarded marketing page's 100–300 ultralight display type, the real site's heaviest text sits at 700–800, inside v4vmm's existing `Weight` range. |

---

## Three biggest gaps between the website look and v4vmm's token system

1. **v4vmm's entity colors carry status meaning; the website's do not.**
   `EntityKind::fill_token()` reuses `Success`/`Warning`/`Info`/`Danger`/`Accent`
   for identity color, so `Track` and `Playlist` share one hue (`Info`), and
   `Publisher` inherits `Danger` (red) — a color the website keeps reserved
   for "Live" alone. The website's 13-member `--c-*` family is a dedicated
   identity palette, independent of status, with its own grouping rule
   (feed/track/playlist/artist each get an own hue; host and author are
   shades of artist; the six business roles share one violet family; red
   means "live" and nothing else). Carrying this forward needs a new color
   family in `tokens.rs`, not a remap of the existing status roles, plus new
   `contrast.rs` pairs for every hue against every surface tier.

2. **v4vmm has no selection "sheen," no texture, and no artwork-only
   shadow.** The website marks a selected or current row with a layered
   gradient and two inset highlights built from `--accent`
   (`--gloss-top/bottom/edge/hi`), textures its hero with a real SVG noise
   layer (`--grain`), and shadows artwork alone, scaling the blur to the art's
   size (`--art-shadow`, 12–48px). `SelectedContent` in v4vmm is one flat
   color, there is no texture concept, and no artwork-only shadow token
   exists (`SurfaceElevation` shadows nothing at all). The artwork-shadow gap
   is narrow — one new token, scoped to `Thumbnail`/`Image` — but the sheen
   and the grain are each a new rendering primitive, not a value change.

3. **v4vmm's identity visuals (dot, monogram, pill) and row heights differ
   from the website's.** The website marks entity kind with a small color
   dot (hollow when inferred) and a two-letter monogram placeholder; v4vmm
   uses a filled text `TagBadge` pill and an emoji fallback — a different
   content language for the same job, which a redesign could swap without a
   new token family. Separately, the website's content rows (52–64px) run
   taller than v4vmm's tallest named row token (`Size::RowLg`, 44px), so
   adopting the look also means a new, taller row-height step. Both changes
   stay inside the existing kinds of token (`SemanticColor` for the dot,
   `Size` for the row), unlike gap 2's new primitives.
