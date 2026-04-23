# Cosmetic + Accessibility Revamp Plan

Target: V4V Music Manager desktop app (GPUI, Rust). Goal: tighter visual system, better contrast, keyboard-first usability, fewer ad-hoc styles, aligned (loosely) with Apple HIG accessibility guidance.

## 1. Current state (what we're fixing)

Audit of `src/app.rs`, `src/library.rs`, `src/search.rs`:

- **Palette**: one shared dark theme (`bg/surface/border/text/muted/accent`) plus 10+ ad-hoc hex values sprinkled in status/badge/ID3-frame code. No tokens, no light mode, no contrast guarantees.
- **Typography**: mix of named (`text_xs/sm/lg`) and raw pixel sizes (`px(9.0)`, `px(10.0)`, `px(10.5)`, `px(11.0)`). At least six distinct tiny sizes. No scale, no weights convention.
- **Spacing**: gap/padding values drawn from `{2,4,6,8,10,12,16,24}` arbitrarily. No 4/8 pt grid discipline.
- **Focus / keyboard**: no `on_key`, no actions, no focus rings. Mouse-only. Can't Tab, can't Escape from inspector, can't arrow-key through lists.
- **Labels**: icon-only buttons and small glyph rows lack accessible labels.
- **Hit targets**: 9–10 px text rows used as click targets; below HIG macOS 20 pt minimum.
- **Muted text**: `#9aa0b4` on `#0f1117` ≈ 6.5:1 (OK for 14 pt+, borderline at 9–10 px).
- **State**: selection / disabled conveyed by bg tint or opacity only — color-only affordance.

## 2. Design tokens (introduce a module)

New file: `src/ui/theme.rs` (or add to `app.rs` if kept small). Centralize everything. Nothing below should be hard-coded outside this module.

### 2.1 Color tokens

Dark theme (default). Keep existing spirit, fix contrast.

| Token               | Hex       | Use                                 |
|---------------------|-----------|-------------------------------------|
| `bg.canvas`         | `#0f1117` | window background                   |
| `bg.surface`        | `#1a1d27` | cards, top bar, inspector           |
| `bg.surface_hi`     | `#232735` | hover / elevated surface            |
| `bg.selected`       | `#2a3352` | selected row (accent-tinted)        |
| `border.subtle`     | `#2a2d3a` | hairlines                           |
| `border.strong`     | `#3d4153` | inputs, dividers needing emphasis   |
| `text.primary`      | `#eceef5` | default text (↑ from `#e2e4ed`)     |
| `text.secondary`    | `#b4bacb` | secondary (↑ from `#9aa0b4`)        |
| `text.muted`        | `#8a90a4` | tertiary — ≥12 px only              |
| `text.on_accent`    | `#0b0d13` | text on accent fills                |
| `accent`            | `#8b9bff` | primary action                      |
| `accent.hover`      | `#a5b2ff` | hover state                         |
| `accent.pressed`    | `#7486f5` | active state                        |
| `focus.ring`        | `#a8b6ff` | 2 px focus outline, 2 px offset     |
| `status.success`    | `#7dd67d` | done (↑ from `#6bcc6b`)             |
| `status.warning`    | `#ffd666` | processing (↑ from `#ffcc00`)       |
| `status.danger`     | `#ff8585` | error (↑ from `#ff6b6b`)            |
| `diff.match`        | `#6fd4a3` | tag match                           |
| `diff.different`    | `#ffd27a` | tag differs                         |
| `diff.missing`      | `#ffa07f` | tag missing                         |

Contrast targets (WCAG): normal text ≥ 4.5:1, large text ≥ 3:1, UI/graphics ≥ 3:1. Verify every `text.*` vs every `bg.*` combo in a quick script before shipping.

Entity badge colors (`feed/track/publisher/...`) in `search.rs` stay but move to `theme::badges` map. Each badge's text color auto-picked by luminance (already partly done — formalize).

Retire: `#9298ab`, `#9aa0b4`, `#6bcc6b`, `#ffcc00`, `#ff6b6b`, `#252836`, `#1f2230`, `#2a2d3a` literals scattered across `library.rs` / `search.rs`.

### 2.2 Typography scale

Five sizes only. One module, one weight per role.

| Role         | Size  | Weight    | Use                                       |
|--------------|-------|-----------|-------------------------------------------|
| `title`      | 20 px | SemiBold  | Settings title, section headers           |
| `headline`   | 15 px | SemiBold  | Card titles, inspector title              |
| `body`       | 13 px | Regular   | Default UI text, rows                     |
| `caption`    | 12 px | Regular   | Secondary meta, helper text               |
| `micro`      | 11 px | Medium    | Badges, ID3 frame tags — min size allowed |

Kill all `px(9.0)` and `px(10.0)` text. HIG macOS floor is 11 pt for readable text. For metadata tables, prefer monospace 12 px over 9 px.

Helper: `fn type_body(el) -> Div { el.text_size(px(13.0)).font_weight(FontWeight::NORMAL) }` etc.

### 2.3 Spacing scale (4 pt grid)

Allowed values only: `2, 4, 8, 12, 16, 24, 32` px. Use named consts:

```rust
pub const SPACE_XXS: Pixels = px(2.0);
pub const SPACE_XS:  Pixels = px(4.0);
pub const SPACE_SM:  Pixels = px(8.0);
pub const SPACE_MD:  Pixels = px(12.0);
pub const SPACE_LG:  Pixels = px(16.0);
pub const SPACE_XL:  Pixels = px(24.0);
pub const SPACE_XXL: Pixels = px(32.0);
```

Delete arbitrary `px(6.0)`, `px(10.0)`, `px(10.5)`, `px(14.0)` uses.

### 2.4 Radius + elevation

- `radius.sm = 4`, `radius.md = 6`, `radius.lg = 10`
- No shadows yet — rely on `bg.surface_hi` for elevation. Add `shadow.card` later if needed.

### 2.5 Hit-target minimum

Every clickable region ≥ 28×28 px (macOS pragmatic, above HIG 20 pt, below iOS 44). Enforce via a `fn clickable(el) -> Div { el.min_w(px(28.)).min_h(px(28.)) }` helper applied to every `.on_click()` target.

## 3. Focus + keyboard

HIG: "all interactive elements reachable via full keyboard access." Today: zero.

### 3.1 Focus ring

Add `.focus_ring()` helper: 2 px `focus.ring` outline, 2 px offset, only visible when focus is keyboard-derived (GPUI exposes focus state via `FocusHandle`). Apply to: every `Button`, every `Input`, every clickable row, every tab.

### 3.2 Actions + shortcuts

Register GPUI `Action`s in `app.rs`:

| Shortcut         | Action              | Scope         |
|------------------|---------------------|---------------|
| `Cmd/Ctrl+1/2/3` | Switch tab          | app           |
| `Cmd/Ctrl+F`     | Focus search input  | Discover/Lib  |
| `Cmd/Ctrl+,`     | Open Settings tab   | app           |
| `Esc`            | Pop inspector / clear search | context |
| `↑ / ↓`          | Move selection in lists | list focus|
| `Enter`          | Open inspector for selected row | list focus |
| `Cmd/Ctrl+R`     | Refresh library     | Library tab   |

Tab order: sidebar → list → inspector. Document in code with a comment near the root render.

### 3.3 List selection model

Lists currently show hover but no "selected" concept for keyboard. Add `selected_idx: Option<usize>` to `LibraryApp` and `SearchApp`; render selected row with `bg.selected` + left-edge accent stripe (2 px) so selection isn't color-only.

## 4. Per-region refresh

### 4.1 Top tab bar (`app.rs:169-212`)

- Increase bar height to 44 px (currently ~38 px).
- Tabs as pill buttons, 13 px body text, 8 px vertical padding. Active = filled accent, text on_accent. Inactive = ghost, text secondary.
- Add 1 px `border.subtle` beneath, keep logo + wordmark left.
- Add tab underline on keyboard focus (2 px `focus.ring`, inside the pill).

### 4.2 Library / Discover list rows

- Minimum row height 36 px.
- Layout: thumb (28×28) | title (body) + meta (caption muted) | trailing actions (right-aligned, 28×28 hit targets).
- Hover: `bg.surface_hi`. Selected: `bg.selected` + 2 px accent stripe on leading edge. Focused (keyboard): selected styles + focus ring.
- Replace all 9–10 px meta text with 12 px caption.

### 4.3 Inspector panel

- Fixed width 360 px (up from 320) so titles stop truncating.
- Header row: back button (`←` glyph, aria-label "Back"), title (headline), close (`✕`, aria-label "Close inspector"). Both icons ≥ 28 px hit targets with visible labels on hover.
- Section headers in `title` style, 16 px top margin.
- Metadata tables: caption for keys (muted), body mono for values. Drop the 9 px frame-ID row — move to a tooltip or on-hover reveal.
- Escape key closes top inspector frame.

### 4.4 Settings (`app.rs:232-352`)

- Title: use `title` style.
- Field labels: caption weight Medium, 8 px below title.
- Help text: caption muted, 4 px below input.
- Buttons row: Save (primary) + Use Defaults (ghost), 8 px gap, right-aligned in the 720 px column.
- Status line: add leading icon (✓ success, ⚠ error) so state isn't color-only.

### 4.5 Badges + status chips

- Uniform: 11 px micro, Medium weight, 2 px / 6 px padding, radius 4.
- Auto-contrast text (already partial). Add a thin 1 px outer stroke at 20% accent alpha to improve edge definition on similar-hue backgrounds.

## 5. Accessibility polish

- **Every** `Button::new(id)` gets a `.label(...)` or an explicit `aria_label` equivalent. Audit via grep for `Button::new` with no `.label`.
- Icon-only clickable `div`s get `.tooltip(...)` + an accessible name. GPUI text alternatives via a `labeled(name, el)` wrapper that both sets tooltip and exposes a name (when GPUI a11y hooks land — leave a TODO comment referencing this plan).
- Disabled state: never opacity alone. Add visible "Disabled" affordance (muted text + no hover). Already at 0.45 opacity in `search.rs` — supplement.
- Diff colors (`match/different/missing`) paired with icon glyphs (`=`, `≠`, `∅`) so color isn't required.
- Check for `prefers-reduced-motion` analog in GPUI; gate future animations behind a toggle in Settings until system hook exists.
- Light-mode support deferred, but all colors referenced via tokens so a second palette is a drop-in.

## 6. Rollout

Small PRs, in order. Each one compiles, runs, visible diff.

1. **Theme module** — extract tokens, replace `bg()/surface()/...` fns with `theme::color::*`. No visual change.
2. **Spacing + typography consts** — swap literals. Light visual tightening.
3. **Top bar + tabs** — new pill tabs, height 44. Keyboard `Cmd+1/2/3`.
4. **Focus rings** — helper + apply to Buttons/Inputs/tab pills.
5. **Row selection + keyboard nav** — arrow keys, Enter, Esc in Library list.
6. **Discover search** — same list treatment + `Cmd+F` focus.
7. **Inspector** — widen, keyboard back, labeled icon buttons.
8. **Settings polish** — status icon, label styles.
9. **Badge pass** — unified micro badges, paired diff glyphs.
10. **A11y sweep** — label every Button, tooltip every icon, contrast audit script.

Each step ships independently. Revert-friendly.

## 7. Out of scope (now)

- Light mode (tokens make it trivial later).
- Liquid Glass / materials (GPUI lacks; don't force).
- Full VoiceOver equivalent — blocked on GPUI a11y tree support. Track upstream, revisit.
- Custom SF Symbols replacement — use system fallback glyphs for now; swap to an icon font when chosen.

## 8. References

- Apple HIG — Accessibility (touch targets, Dynamic Type, contrast, keyboard): `~/.claude/skills/apple-hig/summaries/accessibility-complete.md`
- Apple HIG — Color, Typography, Layout specifications under `~/.claude/skills/apple-hig/foundations/`
- WCAG 2.1 AA contrast ratios: 4.5:1 normal, 3:1 large/UI.


## Non-negotiables
- Existing behavior must remain unchanged unless explicitly listed
- Public API must stay compatible
- No new runtime dependencies without justification
- Keep changes as small and local as possible
- All tests must pass