# ADR 0083: Design Language

## Status

Accepted - 2026-10-03. The operator chose the direction on 2026-10-03 from a private mockup canvas, and accepted this text on the same day. Phase 3 of the [overhaul plan](../plans/design-and-cleanup-overhaul-plan.md) implements Decisions 1 to 4.

Amended 2026-10-07: Decision 5 names the main action until a page can play a track.

It amends [ADR 0025](0025-theme-icon-style-boundary.md) (entity roles get their own palette), [ADR 0062](0062-music-content-surface.md) (a tile is a release) and [ADR 0066](0066-configuration-and-startup-failure-recovery.md) (the normal-shell notice opens from the status bar).

## Context

The [overhaul plan](../plans/design-and-cleanup-overhaul-plan.md) has three design goals. The app looks like the musicindex.org website. The Library feels like a music player. Each setting has one clear place.

The [screen inventory](../architecture/screen-inventory/README.md) found these facts on 2026-10-03:

- The app has one blue-violet accent for tabs, buttons, links and chips.
- `EntityKind::fill_token` in `src/ui/composites/tag_badge.rs` reuses status colors as entity colors. Track and playlist share one color, and publisher is red.
- Artwork has no consistent size. Show has no artwork. The app loads no font of its own.
- The Library tile view shows one tile for each track.
- Album and track pages show feed URLs, GUIDs and the full compare grid at all times.
- Track pages stack many buttons. The live status band uses about 70 pixels on Music and Settings.

The website is `../musicindex/search.html`. Its dark theme is the default. Its tokens are recorded in the [design-system inventory](../architecture/screen-inventory/design-system.md).

The operator reviewed mockups on 2026-10-03 and chose version 2 at size M. The operator also decided these points:

- Browsing follows Apple Music. The app is not a keyboard-driven power tool.
- A cohesive status bar and simple icons come from Raycast.
- Identity links such as a website or a Nostr profile stay visible.
- A payment split never uses entity colors.

## Decision

### 1. The Palette Follows search.html

The theme profiles use these values. Dark is the default. Each value is a named token in `src/ui/tokens.rs`.

| Role | Dark | Light |
|---|---|---|
| Background | `#0b0b0d` | `#ffffff` |
| Sidebar | `#141417` | `#f5f5f7` |
| Surface | `#1c1c1f` | `#ffffff` |
| Raised surface | `#26262a` | `#ececef` |
| Text | `#f5f5f7` | `#1d1d1f` |
| Muted text | `#a1a1a6` | `#5c5c62` |
| Separator | white at 10 % | `#3c3c43` at 16 % |
| Accent | `#2d7bff` | `#0a5bd6` |

The high-contrast profiles keep their own values and their contrast tests.

### 2. Color Roles Do Not Mix

Each color has one meaning:

- **Accent** marks the selected item, a link and the focus ring.
- **Status** colors mark success, warning, danger and information. Danger also marks a destructive action. In the dark profile, the warning color is orange, as in the light profile, because gold is a value color.
- **Entity** colors mark a stated entity kind: feed, track, playlist, artist, publisher, label and the other kinds of the website. They show as a small dot, never as a filled text badge. Live red is reserved for a live item.
- **Value** colors mark the shares of a payment split. The largest share is lightning gold `#ffd666`. Each next share is a darker, less saturated step of the same hue. The steps differ in lightness.

An entity color appears only when the source states the kind. A payment recipient has no stated kind, so its share uses a value color. Text always carries the meaning: a name, a word or a number. Color alone never does.

### 3. Type Is Figtree

The app embeds Figtree and uses it for all interface text. Identifiers such as GUIDs and URLs in an inspection view use a monospace font.

The type ramp of size M stays. One display size of 30 pixels at M is added for the title of an album, artist or publisher page. The UI scale steps scale it like every other size.

### 4. Artwork Leads

- A cover shows at the sizes of the artwork tokens: 152 pixels for a tile and 200 pixels for a page header at M.
- Artwork, and only artwork, has a shadow. A larger image has a larger shadow.
- An album, artist or publisher page shows a backdrop derived from its cover behind the header. The backdrop fades into the background color before the content.
- A missing cover shows a placeholder tinted with the entity color and a two-letter type monogram. It never shows an emoji.
- Show displays the cover of the current track.

The renderer gets the backdrop from a view-model fact, for example derived colors or a prepared image. The view model carries no renderer type.

### 5. Actions Have A Hierarchy

| Kind | Presentation | Example |
|---|---|---|
| Main action | One filled button | Play |
| Frequent action | At most two plain buttons | Add to playlist |
| Navigation | A link on the text | Artist name, publisher name |
| Identity and provenance | Quiet icon links, always visible | Website, Nostr, RSS |
| State | A small icon and a word, always visible | Tags differ, removed from feed, ready for a show |
| Rare, technical or destructive action | The "⋯" menu and the right-click menu | MusicBrainz lookup, download again, remove |

A destructive item is last in its menu, after a divider, in the danger color, and its label ends with "…". It asks for confirmation.

Until a page can play a track ([ADR 0068](0068-show-cue-and-audition-isolation.md)), the filled main action is the next curation step for the state of the item. A track that is not in the Library gets "Download album". A Library track that is not downloaded gets "Download track". A downloaded track gets "Add to playlist". Play replaces it when playback exists. The operator decided this on 2026-10-07.

### 6. Rows Show Information And Hide Most Actions

- A row shows its information and its state at all times.
- A row shows its actions on hover and on keyboard focus.
- The right-click menu of a row holds the same items as its "⋯" menu.
- A track can be dragged onto a playlist in the sidebar.
- Each action is reachable with the mouse. A keyboard shortcut is optional ([ADR 0067](archive/0067-platform-shortcut-modifiers.md) stays).

### 7. One Status Bar

The window has one status bar at the bottom, the same on every section. One shared composite owns it.

- **Left:** the latest reported event, in the words of the report owner ([ADRs 0059 and 0063](0059-broadcast-control-surface.md)), with its recorded time.
- **Right:** small items with a simple icon and a short word. Each item opens its detail. The items are feed state, files to update, reports with a count, and show state.
- **The show item appears only while a show is active.** With no active show, Music and Settings show no broadcast state (Current Design Philosophy: curation shows nothing operational).

The status bar replaces the live status band. The reports item opens the normal-shell notice of ADR 0066. Its full subjects, repair actions and retained actions stay reachable there and in Settings.

### 8. Inspection Is One Step Away

- An album or track page has one "Inspect sources" entry.
- The inspection view shows the RSS value, the MusicIndex value and the file tag side by side, with the source element of each row and a result in words.
- RSS is labeled the source of truth, and MusicIndex the cache ([ADR 0075](0075-metadata-ownership-and-completeness.md) Decision I).
- The track compare grid becomes the inspection view of a track. Its content stays.

### 9. Music Has A Home, And A Tile Is A Release

- The Music section opens on a home: the newest release, the releases of the Library, the playlists, and the upkeep state.
- The tile view shows one tile for each release, not for each track.
- ADR 0062 stays for recency order and paging.

### 10. Simple Icons

- One icon set with one stroke style. The app uses the icons of `gpui-kit-assets` (ADR 0025 icon boundary).
- An icon with a word is the default. An icon alone needs a tooltip and an accessibility label.

## Consequences

- The app gets a visual identity close to the website, and its V4V facts become visible.
- `tokens.rs`, the theme profiles and `contrast.rs` get new roles: surfaces, entity colors, value colors, a display size and an artwork shadow. Each needs contrast pairs.
- `EntityKind` stops using status colors. Code that colors a payment recipient by kind must change.
- Pages lose most visible buttons. Users find rare actions in the "⋯" and right-click menus.
- The cover backdrop needs a technique that GPUI can draw. The first packet must prove one.
- The status bar removes the live status band, and the ADR 0066 notice moves behind it.

## Alternatives Considered

- **Version 1 of the mockups:** flat backgrounds and tinted placeholders. The operator chose version 2.
- **Website sizes:** about one step larger than M. Rejected: a desktop app is denser than a web page.
- **A keyboard-driven command palette as the main surface:** rejected by the operator.
- **Hide identity links in "⋯":** rejected. They are facts that listeners use to support the artist.
- **Color payment recipients by entity kind:** rejected. The feed states no kind for a recipient.

## Verification

Mechanical, at the owning layer:

- `tokens.rs` exposes each palette role of Decision 1 for each theme profile, and the contrast tests cover each new pair.
- `EntityKind` resolves no status color. A guard blocks a status token in the entity palette.
- The value palette exposes steps that differ in lightness.
- A status bar view model exposes no show item when no show is active.
- The status bar has one composite owner, and no screen renders a second status band.
- The Library tile view model exposes one tile for each release.
- Each row action exists in the row's "⋯" items and in its right-click items.

Visual, for the operator:

- Each section at M, in Dark and Light, matches the accepted mockups in hierarchy, color roles and artwork treatment.
- The cover backdrop fades into the background before the content.
- The status bar reads clearly at normal and narrow widths.
