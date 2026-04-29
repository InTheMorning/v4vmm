# Copilot Instructions for v4vmm

## Source Minimap

When you need to look something up, consult this map first to find the right file.

### v4vmm source (`src/`)

| What you need | File |
|---------------|------|
| SQLite schema, row types, all queries | `src/db.rs` |
| Config struct, TOML paths | `src/config.rs` |
| MusicIndex HTTP client | `src/api.rs` |
| RSS fetch + Podcasting 2.0 parse | `src/rss/` |
| ID3v2.4 read/write, `AudioTags`, `Id3v24Edit`, `apply_id3v24_edits` | `src/audio_tags.rs` |
| File-byte format detection, `AudioFormat` | `src/audio_format.rs` |
| RSS vs ID3 vs MusicBrainz diff, `TrackContext` compare | `src/track_compare.rs` |
| `TrackContext`, `TagCompareResult`, `ImageBytes` — no GPUI | `src/metadata.rs` |
| Pure metadata derivation, `id3_edits_for_track_context` | `src/metadata_service.rs` |
| Download + tag + library-membership pipeline | `src/subscribe_service.rs` |
| `ensure_feed_in_db`, feed subscription glue | `src/feed_service.rs` |
| Library membership, `subscribe_then_append_to_playlist` | `src/library_service.rs` |
| Playlist CRUD and track append | `src/playlist_service.rs` |
| Now-playing session state | `src/playback.rs` |
| mpv driver, IPC socket lifecycle | `src/playback_driver/` |
| Long-running playback owner (desktop/daemon) | `src/playback_owner.rs` |
| MusicBrainz query client | `src/musicbrainz.rs` |
| Track identity, local file binding | `src/track_identity.rs` |
| Source enclosure selection | `src/sources.rs` |
| ID3 frame field types | `src/tag_field.rs` |
| GPUI library view (view code only) | `src/library.rs` |
| GPUI discover/search view (view code only) | `src/search.rs` |
| Top-level GPUI window, tab switching | `src/app.rs` |
| GPUI view composition helpers | `src/views.rs` |
| Artist inspector view | `src/ui_artist.rs` |
| Feed inspector view | `src/ui_feed.rs` |
| Track inspector view | `src/ui_track.rs` |
| Shared UI helpers (status, layout) | `src/ui_common.rs` |
| GPUI context helpers | `src/ui_context.rs` |
| Theme + scrollbar | `src/ui/theme.rs` |
| Image/media cache helpers | `src/media/` |
| CLI commands | `src/cli.rs` |
| CLI debug JSON contracts | `src/debug_contracts.rs` |

### GPUI framework (`~/dev/vcs-codebases/github.com/zed-industries/zed/crates/gpui/`)

| What you need | File | Key lines |
|---------------|------|-----------|
| `Render` trait, `IntoElement`, `RenderOnce`, `ParentElement` | `src/element.rs` | 112–177 |
| `Entity<T>`, `WeakEntity<T>`, view creation | `src/view.rs` | 28–74 |
| Ownership & data-flow mental model (read this first) | `src/_ownership_and_data_flow.rs` | 1–139 |
| `Context<T>` — `notify`, `spawn`, `listener`, `observe`, `subscribe`, `emit` | `src/app/context.rs` | 19–260 |
| `Context<T>` — window callbacks, focus listeners, `on_action`, `spawn_in` | `src/app/context.rs` | 425–749 |
| `App` — `open_window`, `quit`, clipboard, displays | `src/app.rs` | 1071–1110 |
| `FocusHandle`, focus tab-index, `track_focus` | `src/window.rs` | 349–419 |
| `actions!` macro, `Action` trait, `build`, `name` | `src/action.rs` | 11–173 |
| Working focus + key-binding example | `examples/focus_visible.rs` | full file |
| Working entity + async example | `examples/testing.rs` | full file |
| Window open/close example | `examples/window.rs` | full file |

### gpui-component library (`~/dev/vcs-codebases/github.com/longbridge/gpui-component/`)

| What you need | File | Key lines |
|---------------|------|-----------|
| `gpui_component::init(cx)` — call at startup | `crates/ui/src/lib.rs` | 102–125 |
| `Root::new(view, window, cx)` — required window wrapper | `crates/ui/src/root.rs` | 27–89 |
| `cx.theme()` / `Theme::global` / `Theme::change` / token fields | `crates/ui/src/theme/mod.rs` | 24–180 |
| `Button` — variants, `primary()`, `ghost()`, `on_click` | `crates/ui/src/button/button.rs` | 39–260 |
| `ButtonGroup`, `DropdownButton`, `Toggle` | `crates/ui/src/button/` | — |
| `Input`, `InputState` | `crates/ui/src/input/input.rs` | 33–49 |
| `DataTable`, `TableState` | `crates/ui/src/table/data_table.rs` | 77–163 |
| `TableDelegate` trait — columns, rows, `render_td`, selection | `crates/ui/src/table/delegate.rs` | 16–230 |
| `List`, `ListState` | `crates/ui/src/list/list.rs` | 90–120 |
| `ListDelegate` trait — `items_count`, `render_item`, sections | `crates/ui/src/list/delegate.rs` | 10–171 |
| `Dialog` — `title`, `on_ok`, `on_cancel`, `window.close_dialog` | `crates/ui/src/dialog/dialog.rs` | 34–260 |
| `Sheet` / drawer — `window.open_sheet`, overlay dismiss | `crates/ui/src/sheet.rs` | 44–205 |
| `Sidebar`, `SidebarItem` | `crates/ui/src/sidebar/mod.rs` | 27–153 |
| Scrollable areas — `.overflow_scrollbar()` etc. | `crates/ui/src/scroll/scrollable.rs` | 13–208 |
| Full working example app | `examples/hello_world/src/main.rs` | full file |
| Full dialog + sheet example | `examples/dialog_overlay/src/main.rs` | full file |

## Build, Test, and Lint

```bash
cargo build                          # build the binary
cargo run                            # start the desktop UI
cargo test --lib                     # run all unit tests
cargo test --lib <module>::<test>    # run a single test
cargo clippy -- -D warnings          # lint (must pass clean)
cargo fmt -- --check                 # format check
```

Each ADR phase must pass all four checks above before proceeding.

## Architecture Overview

`v4vmm` is a single-binary Rust desktop app (GPUI frontend) with a small CLI surface. It is **not** a general-purpose media player or tag editor — it manages a MusicIndex-backed music library stored in a local SQLite database.

### Data sources (kept separate, not merged)

1. **RSS** — source-of-record for subscription import; stored in SQLite
2. **MusicIndex HTTP API** — discovery, feed/track detail hydration, `updated_at` freshness checks
3. **Embedded ID3 tags** — read/write on local MP3 files; other formats are downloaded but not fully editable
4. **MusicBrainz** — optional metadata enrichment via metadata queries (no fingerprinting)

### Module roles

| Module | Role |
|--------|------|
| `db.rs` | All SQLite schema, row types (`FeedRow`, `TrackRow`, `PlaybackSessionRow`), and queries |
| `config.rs` | `Config` struct, `~/.config/v4vmm/config.toml` read/write |
| `api.rs` | MusicIndex HTTP client |
| `rss/` | RSS fetch, parse, Podcasting 2.0 extensions |
| `audio_tags.rs` | ID3v2.4 read/write; `lofty` for non-MP3 formats |
| `audio_format.rs` | File-byte format detection |
| `track_compare.rs` | Side-by-side provenance diff (RSS vs ID3 vs MusicBrainz) |
| `metadata.rs` | `TrackContext`, `TagCompareResult` — must have **no GPUI imports** (ADR 0022) |
| `metadata_service.rs` | Pure metadata derivation: `id3_edits_for_track_context` |
| `subscribe_service.rs` | Canonical download + tag + library-membership pipeline |
| `feed_service.rs` | `ensure_feed_in_db`, feed subscription glue |
| `library_service.rs` | Library membership, `subscribe_then_append_to_playlist` |
| `playlist_service.rs` | Playlist CRUD and track append |
| `playback.rs` / `playback_owner.rs` / `playback_driver/` | Now-playing session state and mpv integration |
| `library.rs` / `search.rs` | **GPUI view code only** — no domain logic (ADR 0022 in progress) |
| `ui_*.rs`, `ui/`, `views.rs`, `app.rs` | GPUI rendering helpers and top-level window |
| `cli.rs` | One-shot CLI commands; shares service modules with UI |

### Service boundary rules (ADR 0015 / ADR 0022)

- Service modules (`*_service.rs`, `db`, `api`, `musicbrainz`, `audio_tags`, `track_compare`, `rss`, `playback`) must **not** import `gpui` or `gpui_component`.
- Service modules must **not** import `library`, `search`, `app`, or any `ui_*` module.
- Service functions are **blocking**; the UI moves them off the foreground executor.
- `library.rs` and `search.rs` must contain only `*App` state structs, event handlers, `Render` impls, and async glue. Domain logic belongs in service modules.

## Key Conventions

### Provenance model

The app is **provenance-first**: RSS, ID3, and MusicBrainz values stay visible as separate columns in the compare view. Conflicts are surfaced, not auto-resolved. Never silently merge metadata sources.

### ID3 write boundary (ADR 0008)

All ID3v2.4 writes go through `audio_tags::apply_id3v24_edits`. Drag/drop staging is in-memory only. The write path is only called after an explicit operator action. Duplicate writes to the same effective ID3 target are treated as conflicts and must be resolved before apply.

### Schema migrations (ADR 0016)

New durable tables and columns must go through the migration registry in `db.rs` (monotonically increasing versions recorded in `schema_migrations`). The inline `CREATE TABLE IF NOT EXISTS` block initializes fresh databases; migrations handle existing ones. Both paths must be idempotent.

### Audio format handling

- Format is detected from **file bytes** after download, not from MIME type or URL extension alone.
- WAV downloads are silently re-encoded to FLAC via the `flac` CLI (`flac_path` config key or `$PATH`). If `flac` is absent, the WAV is kept and left untagged.
- The richest compare/edit workflows require MP3/ID3 files. Other formats may be stored locally but do not have symmetric tag-edit support.

### Enclosure selection priority

1. Primary source enclosure (when present)
2. First supported source enclosure
3. Track's direct enclosure URL

### File layout

```
<music_dir>/artists/<artist>/<feed-or-release>/<filename>
```

Path segments are sanitized for invalid characters, reserved names, and length.

### `metadata.rs` image handling (ADR 0022)

Cover art is carried as `ImageBytes { data: Vec<u8>, mime_type: String }` — not `Arc<gpui::Image>`. The UI layer converts at render-preparation time. Do not introduce `gpui::ImageFormat` or `SharedString` into `metadata.rs`.

### CLI JSON output

All CLI inspection commands output structured JSON and call the same non-UI service functions as the UI. Add new CLI commands only when the backing service exists.

### Playback session

`playback_sessions` is the authoritative now-playing state. Player adapters report transport facts (position, pause) into this model — they do not define metadata identity. One-shot CLI commands do not spawn or reuse mpv; that belongs to the long-running desktop/daemon owner.

## GPUI Reference

### Rendering model

`Render` is the core view trait. GPUI calls `render()` every frame, builds an element tree, lays it out, paints it, then drops the tree.

```rust
impl Render for MyView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().gap_2().child("hello")
    }
}
```

### Entity / View system

A "view" is `Entity<T>` where `T: Render`. Entities are owned by `App`; all access goes through context methods.

```rust
// create
let view = cx.new(|cx| MyView::new(cx));

// update from outside
view.update(cx, |this, cx| {
    this.count += 1;
    cx.notify();   // triggers re-render
});
```

Use `cx.weak_entity()` / `WeakEntity<T>` in async closures and listeners — call `.upgrade()` before touching state to avoid dangling references.

### Context types

| Type | When you have it | Key methods |
|------|-----------------|-------------|
| `Context<T>` | Inside entity methods | `cx.notify()`, `cx.spawn()`, `cx.emit()`, `cx.listener()`, `cx.observe()`, `cx.subscribe()`, `cx.focus_handle()` |
| `App` | App-level callbacks | `cx.open_window()`, `cx.quit()` |
| `Window` | Render / event handlers | `window.focus()`, `window.open_sheet()`, `window.close_dialog()`, `window.remove_window()` |

`Context<T>` derefs to `App`, so app-level methods are always available inside entity methods.

### Element / layout system

`div()` is the primary building block. All style methods are fluent chains:

```rust
div()
    .flex()
    .flex_col()
    .gap_4()
    .p_2()
    .bg(cx.theme().background)
    .child(Label::new("title"))
    .children(items.iter().map(|i| render_item(i)))
```

`RenderOnce` is for stateless one-shot components that don't need to be retained as entities.

### Event handling

```rust
// inside render(), wire a closure back to self
.on_click(cx.listener(|this, _event, _window, cx| {
    this.count += 1;
    cx.notify();
}))
```

For entity-to-entity communication use `EventEmitter` + `cx.subscribe()`:

```rust
// define
impl EventEmitter<MyEvent> for MyView {}

// emit
cx.emit(MyEvent::Selected(id));

// subscribe in parent
cx.subscribe(&child_entity, |this, _child, event, cx| {
    // handle event
});
```

### Actions

```rust
actions!(my_app, [Confirm, Cancel]);

// bind keys (usually in main or app setup)
cx.bind_keys([KeyBinding::new("enter", Confirm, None)]);

// handle in render
div()
    .track_focus(&self.focus_handle)
    .on_action(cx.listener(Self::on_confirm))

fn on_confirm(&mut self, _: &Confirm, _window: &mut Window, cx: &mut Context<Self>) { ... }
```

### Async tasks

Keep service calls off the foreground executor. The standard pattern:

```rust
// in an event handler
cx.spawn(async move |this, mut cx| {
    // cx is AsyncContext here — cannot call notify directly
    let result = cx.background_executor()
        .spawn(async move { blocking_service_call() })
        .await;

    this.update(&mut cx, |view, cx| {
        view.result = result;
        cx.notify();
    }).ok();
})
.detach();
```

- `cx.spawn(...)` → entity-scoped, use `this.update(...)` to write back
- `cx.spawn_in(window, ...)` → window-scoped
- `cx.background_executor().spawn(...)` → pure background, no UI access

### Focus and keyboard

```rust
struct MyView { focus_handle: FocusHandle }

impl MyView {
    fn new(cx: &mut Context<Self>) -> Self {
        Self { focus_handle: cx.focus_handle() }
    }
}

impl Render for MyView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            // focus listeners
            .on_focus(cx.listener(|this, _, _, cx| { ... }))
    }
}

// to focus programmatically
window.focus(&self.focus_handle, cx);
```

### Window management

```rust
// open
let handle = cx.open_window(WindowOptions { .. }, |window, cx| {
    let view = cx.new(|cx| MyView::new(cx));
    cx.new(|cx| Root::new(view, window, cx))  // Root is required (see below)
});

// close from inside
window.remove_window();
```

---

## gpui-component Reference

### Required initialization

Call once at app startup before any component is used:

```rust
gpui_component::init(cx);
```

### Root wrapper — required

Every top-level window view must be wrapped in `Root`:

```rust
cx.open_window(opts, |window, cx| {
    let content = cx.new(|cx| MyApp::new(cx));
    cx.new(|cx| Root::new(content, window, cx))
});
```

`Root` manages dialog, sheet, and dock overlay layers. Without it, `Dialog` and `Sheet` will not render.

### Theme

```rust
// access current theme tokens in render
let bg = cx.theme().background;
let border = cx.theme().border;

// change theme mode
Theme::change(ThemeMode::Dark, None, cx);
```

Tokens include `.background`, `.foreground`, `.border`, `.sidebar`, `.table`, and family-specific palettes.

### Common components

All components use a **builder / method-chain** pattern and are `RenderOnce`:

```rust
// Button
Button::new("save").primary().label("Save").on_click(|_, _, _| { ... })
Button::new("del").ghost().icon(Icon::Trash).on_click(cx.listener(...))

// Input — requires an Entity<InputState>
let state = cx.new(|cx| InputState::new(window, cx));
Input::new(&state).placeholder("Search…").cleanable(true)

// DataTable — requires Entity<TableState<D>> where D: TableDelegate
DataTable::new(&table_state).stripe(true).bordered(true)

// Sidebar
Sidebar::new("main-sidebar")
    .header(...)
    .child(SidebarItem::new(...))
    .footer(...)
```

### Table delegate pattern

```rust
struct MyDelegate { rows: Vec<MyRow> }

impl TableDelegate for MyDelegate {
    fn columns_count(&self) -> usize { 3 }
    fn rows_count(&self) -> usize { self.rows.len() }
    fn column(&self, ix: usize) -> Arc<dyn TableColumn> { ... }
    fn render_td(&self, row: usize, col: usize, _window: &mut Window, cx: &mut App)
        -> impl IntoElement { ... }
}

// create state
let state = cx.new(|cx| TableState::new(Box::new(MyDelegate { .. }), window, cx));
```

### List delegate pattern

```rust
impl ListDelegate for MyDelegate {
    fn items_count(&self, _section: usize, _cx: &App) -> usize { self.items.len() }
    fn render_item(&self, ix: IndexPath, _window: &mut Window, _cx: &mut App)
        -> Option<impl IntoElement> { Some(div().child(self.items[ix.item].clone())) }
}

let state = cx.new(|cx| ListState::new(Box::new(MyDelegate { .. }), window, cx));
```

### Dialog

```rust
// open a dialog — Root must be present
Dialog::new(cx)
    .title("Confirm")
    .child(Label::new("Are you sure?"))
    .on_ok(|window, cx| {
        // return true to close, false to keep open
        true
    })
    .open(window, cx);

// close manually
window.close_dialog(cx);
```

### Sheet / drawer

```rust
window.open_sheet(cx, |sheet, window, cx| {
    sheet.title("Details").child(MyDetailView::new(cx))
});
```
