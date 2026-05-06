# v4vmm Architecture — Current Snapshot

A focused snapshot of where the codebase actually is **today**, after the
async-runtime / paged VM / layer-consolidation work shipped under ADRs 0040,
0041, and 0042. For the original side-by-side `current-vs-ideal` view, see
[`architecture-diagrams.md`](./architecture-diagrams.md). This file is the
"what we have right now" reference and is the one to keep in sync with
`src/`.

> **Conventions used below**
> - Solid arrows = synchronous call / direct dependency.
> - Dashed arrows = asynchronous publish/subscribe (events).
> - Crate-internal modules are referenced by their `src/` path.

---

## 1. System Overview

```mermaid
graph TD
    subgraph External["External Sources"]
        RSS["RSS Feeds"]
        MI["MusicIndex API"]
        MB["MusicBrainz API"]
        FS["Filesystem<br/>(audio + ID3)"]
        MPV["mpv (IPC)"]
    end

    subgraph Presentation["Presentation Layer (GPUI)"]
        AppShell["app.rs<br/>window + tabs + sidebar"]
        Library["library/<br/>LibraryApp"]
        Search["search/<br/>SearchApp (Discover)"]
        Shells["ui/shells/*<br/>library · discover · playlist · track"]
        Composites["ui/composites/*<br/>SkeletonInspector · TrackRow ·<br/>FeedTile · MetaGrid · NowPlayingBar"]
        Primitives["ui/primitives/*<br/>Button · Input · Skeleton · Divider"]
        Tokens["ui/tokens.rs · ui/style.rs<br/>color · spacing · type"]
    end

    subgraph PresAdapters["Presentation Adapters (presentation/)"]
        RuntimeHost["RuntimeHost<br/>tokio + VmBus owner"]
        GpuiCmd["GpuiCommandRunner"]
        GpuiVm["GpuiVmBridge"]
        EvBridge["GpuiEventBridge"]
    end

    subgraph VMs["ViewModels (view_models/)"]
        SearchVM["SearchViewModel"]
        LibraryVM["LibraryViewModel"]
        PagedPlaylistVM["PagedPlaylistDetailVm"]
        PagedFeedVM["PagedFeedDetailVm"]
        EntityVM["entity_detail · track_detail ·<br/>artist · feed · metadata"]
    end

    subgraph Application["Application Layer (application/)"]
        CmdBus["CommandBus"]
        AsyncRunner["AsyncCommandRunner<br/>publishes VmEvents"]
        QuerySvc["ApplicationQueryService"]
        EvBus["ApplicationEventBus"]
        Cmds["commands/*<br/>subscribe · playlist · playback · tag"]
        Queries["queries/*"]
        Ports["ports/*<br/>(trait boundaries)"]
    end

    subgraph Runtime["Runtime (runtime/)"]
        VmBus["VmBus<br/>typed VmEvent fan-out"]
        PagedActor["PagedTrackListActor<br/>(per playlist/feed)"]
        PagedVm["PagedListVm<br/>windowed cache"]
        Actor["actor.rs · mod.rs<br/>scaffold + lifecycle"]
    end

    subgraph DomainSvc["Domain / Service Modules (no GPUI)"]
        FeedSvc["feed_service"]
        LibSvc["library_service"]
        PlaylistSvc["playlist_service"]
        SubSvc["subscribe_service"]
        MetaSvc["metadata_service"]
        Compare["track_compare"]
        Identity["track_identity ·<br/>identity_ingest · local_identity"]
    end

    subgraph Infra["Infrastructure"]
        DB["db.rs<br/>SQLite + migrations"]
        Api["api.rs<br/>MusicIndex client"]
        MbMod["musicbrainz.rs"]
        RssMod["rss/"]
        Tags["audio_tags.rs +<br/>audio_format.rs"]
        Sources["sources.rs"]
        Playback["playback.rs +<br/>playback_owner.rs"]
        Driver["playback_driver/<br/>mpv IPC"]
        Cfg["config.rs"]
        Media["media/<br/>image cache"]
    end

    AppShell --> Library & Search
    Library & Search --> Shells --> Composites --> Primitives --> Tokens

    AppShell --> RuntimeHost
    Library & Search --> GpuiCmd & GpuiVm & EvBridge
    GpuiCmd --> AsyncRunner
    GpuiVm --> VmBus
    EvBridge --> EvBus

    Library & Search --> VMs
    VMs --> PagedVm
    PagedVm <-->|window requests + invalidations| PagedActor

    AsyncRunner --> CmdBus
    CmdBus --> Cmds
    Cmds --> DomainSvc
    QuerySvc --> Queries --> DomainSvc

    DomainSvc --> DB & Api & MbMod & RssMod & Tags & Sources
    SubSvc --> Tags & RssMod & Api
    Playback --> Driver --> MPV
    Playback --> DB

    Api --> MI
    MbMod --> MB
    RssMod --> RSS
    Tags --> FS
    Media --> FS

    PagedActor -. VmEvent::PageReady .-> VmBus
    AsyncRunner -. VmEvent::TrackChanged / Invalidate .-> VmBus
    Cmds -. MetadataEvent .-> EvBus
    EvBus -. typed events .-> EvBridge

    Cfg --> DB
```

**What changed vs the "current" snapshot in `architecture-diagrams.md`:**

- A real **Application layer** exists: `CommandBus`, `AsyncCommandRunner`,
  `ApplicationQueryService`, `ApplicationEventBus`, `commands/`, `queries/`,
  `ports/`.
- A **Runtime layer** carries async work off the foreground executor:
  `RuntimeHost` owns the tokio runtime and a typed `VmBus`; per-listing
  `PagedTrackListActor`s feed `PagedListVm` windowed caches.
- **Presentation adapters** (`presentation/`) bridge GPUI to the
  Application/Runtime layers without leaking GPUI types into either.
- The **UI is layered**: `tokens → primitives → composites → shells →
  app screens`. Skeletons are first-class composites used while VMs are
  in a loading state.
- The metadata model (`metadata.rs`) is GPUI-free per ADR 0022; cover art
  travels as `ImageBytes`, never `Arc<gpui::Image>`.

---

## 2. Module Relationships (data dependencies)

```mermaid
graph LR
    subgraph UI["ui/"]
        UTokens["tokens · style"]
        UPrim["primitives/"]
        UComp["composites/"]
        UShells["shells/"]
    end

    subgraph Screens["screen entrypoints"]
        AppMod["app/ + app.rs"]
        LibMod["library/ + library.rs"]
        SearchMod["search/ + search.rs"]
    end

    subgraph PresMod["presentation/"]
        RH["runtime_host"]
        GCR["gpui_command_runner"]
        GVB["gpui_vm_bridge"]
        GEB["gpui_event_bridge"]
        EvB["event_bridge"]
    end

    subgraph VmMod["view_models/"]
        SearchVm["search"]
        LibVm["library"]
        EntityVm["entity_detail · track_detail ·<br/>artist · feed · metadata ·<br/>musicbrainz_panel"]
        PagedPlaylistVm["paged_playlist_detail"]
        PagedFeedVm["paged_feed_detail"]
    end

    subgraph RuntimeMod["runtime/"]
        VmBusM["vm_bus"]
        ActorM["actor"]
        PagedVmM["paged_list_vm"]
    end

    subgraph AppMod2["application/"]
        CB["command_bus"]
        ACR["async_command_runner"]
        AppCtx["command_context"]
        AppEvBus["application_event_bus"]
        AppQs["application_query_service"]
        AppSvc["application_services"]
        CmdsMod["commands/"]
        QueriesMod["queries/"]
        EventsMod["events/ (MetadataEvent etc.)"]
        ErrorsMod["errors/"]
        PortsMod["ports/"]
        PagedTL["paged_track_list"]
    end

    subgraph DomMod["service / domain modules (no GPUI)"]
        FeedS["feed_service"]
        LibS["library_service"]
        PlaylistS["playlist_service"]
        SubS["subscribe_service"]
        MetaS["metadata_service"]
        TrackCmp["track_compare"]
        Ident["track_identity · identity_ingest ·<br/>local_identity · sources"]
        Pb["playback · playback_owner"]
    end

    subgraph InfraMod["infrastructure"]
        DBMod["db.rs"]
        ApiMod["api.rs"]
        MbM["musicbrainz.rs"]
        RssM["rss/"]
        TagsM["audio_tags · audio_format · tag_field"]
        DriverM["playback_driver/"]
        CfgM["config.rs"]
        MediaM["media/"]
    end

    AppMod --> LibMod & SearchMod
    LibMod & SearchMod --> UShells --> UComp --> UPrim --> UTokens

    AppMod --> RH
    LibMod & SearchMod --> GCR & GVB & GEB & EvB
    LibMod & SearchMod --> VmMod

    PagedPlaylistVm & PagedFeedVm --> PagedVmM
    PagedVmM <-->|requests / page deliveries| ActorM
    ActorM --> PagedTL
    PagedTL --> DBMod

    GVB --> VmBusM
    ActorM --> VmBusM
    GCR --> ACR --> CB --> CmdsMod
    AppQs --> QueriesMod
    AppSvc --> CmdsMod & QueriesMod

    CmdsMod --> DomMod
    QueriesMod --> DBMod
    DomMod --> DBMod & ApiMod & MbM & RssM & TagsM
    SubS --> TagsM & RssM & ApiMod
    Pb --> DriverM & DBMod
    EventsMod -. emitted by .-> CmdsMod
    AppEvBus -. fans out .-> GEB
    PortsMod -.-> InfraMod

    CfgM --> DBMod
```

---

## 3. UI Composition (after ADR 0042)

```mermaid
graph TD
    subgraph Tokens["Design tokens (ui/tokens.rs · style.rs)"]
        TColor["color (semantic + WCAG-tested)"]
        TSpace["spacing scale"]
        TType["typography"]
        TRadius["radius / borders"]
    end

    subgraph Prims["Primitives (ui/primitives/)"]
        PBtn["Button"]
        PInput["Input"]
        PDiv["Divider"]
        PSkel["Skeleton<br/>(redacted-style placeholder)"]
        PIcon["Icon"]
    end

    subgraph Comps["Composites (ui/composites/)"]
        CSkelTrack["SkeletonTrackRow"]
        CSkelFeed["SkeletonFeedTile"]
        CSkelIns["SkeletonInspector<br/>(hero + title + body rows)"]
        CTrackRow["TrackRow"]
        CFeedTile["FeedTile / RecentFeedTile"]
        CMetaGrid["MetaGrid (RSS · ID3 · MB)"]
        CNPBar["NowPlayingBar"]
        CPlPop["PlaylistPopover"]
    end

    subgraph Shells["Shells (ui/shells/*)"]
        ShDiscInsp["discover/feed_inspector +<br/>track_inspector"]
        ShDiscRecent["discover/recent_*"]
        ShLibDetail["library/detail · feed_detail ·<br/>track_detail · playlist_detail"]
        ShLibSidebar["library/sidebar"]
        ShPlaylist["playlist.rs<br/>(slot-driven, used by library)"]
        ShTrack["track.rs · feed.rs · artist.rs · entity.rs"]
    end

    subgraph AppScreens["App screens"]
        AppRoot["app/ · app.rs<br/>(window, tabs, sidebar, playback bar)"]
        LibScr["LibraryApp"]
        DiscScr["SearchApp (Discover)"]
    end

    Tokens --> Prims --> Comps --> Shells --> AppScreens
    PSkel --> CSkelTrack & CSkelFeed & CSkelIns
    ShPlaylist -. consumed by .-> ShLibDetail
```

The library detail surface currently has its **own** dispatcher
(`ui/shells/library/detail.rs`) and only its `playlist_detail` variant
goes through the slot-driven `playlist.rs` shell. Bringing the rest of
the library variants onto that shell is filed as
`library-uses-playlist-shell`.

---

## 4. Sequence — Discover open + inspector with eager prefetch

This is the path users see today on the Discover surface, including the
ADR 0040/0041 reactive loop and the recent perceived-latency work.

```mermaid
sequenceDiagram
    autonumber
    participant U as User
    participant V as SearchApp / shell (GPUI)
    participant VM as SearchViewModel
    participant GCR as GpuiCommandRunner
    participant ACR as AsyncCommandRunner
    participant CB as CommandBus
    participant Q as QueryService / db
    participant API as MusicIndex API
    participant MB as MusicBrainz
    participant Bus as VmBus / EventBus
    participant GVB as GpuiVmBridge

    Note over V,VM: cold open
    U->>V: open Discover
    V->>VM: snapshot()
    VM-->>V: skeleton state
    V-->>U: SkeletonFeedTile rows render

    V->>GCR: load_recent_feeds(page=1)
    GCR->>ACR: enqueue
    ACR->>CB: dispatch
    CB->>Q: fetch recents page 1
    Q->>API: GET /recent
    API-->>Q: page 1
    Q-->>CB: rows
    CB-->>Bus: VmEvent::RecentsLoaded(p1)
    Bus-->>GVB: notify
    GVB->>VM: apply snapshot
    VM->>V: cx.notify()
    V-->>U: real tiles render

    Note over V,ACR: eager prefetch of page 2 (kicked off after p1)
    V->>GCR: load_recent_feeds(page=2, append)
    GCR->>ACR: enqueue (idempotent — VM guards in-flight)

    Note over U,V: user clicks a tile
    U->>V: click tile (entity_type, entity_id)
    V->>VM: push_inspector_frame(loading)
    V-->>U: SkeletonInspector renders

    V->>GCR: load_inspector(detail)
    GCR->>ACR: dispatch
    ACR->>CB: fetch_inspector_detail
    CB->>Q: structured fetch
    Q->>API: GET feed/track/artist
    API-->>Q: detail + image_url
    Q-->>CB: (InspectorDetail, Option<image_url>)
    CB-->>Bus: VmEvent::InspectorReady
    Bus-->>GVB: notify
    GVB->>VM: slot detail
    VM->>V: cx.notify()
    V-->>U: text/meta paints (image still empty)

    Note over V,ACR: image loads on a 2nd task — frame-identity guarded
    V->>GCR: load_inspector_image(url)
    GCR->>ACR: GET image bytes + decode
    ACR-->>VM: slot frame.image (only if frame still matches)
    VM->>V: cx.notify()
    V-->>U: artwork paints in

    Note over V,ACR: contributors + value-routes prefetch in background
    V->>GCR: prefetch_contributors / prefetch_value_routes
    GCR->>ACR: dispatch (Hidden → Loading)
    ACR->>API: fetch contributors + value routes
    API-->>ACR: payloads
    ACR-->>VM: panels.from_items_result(...)
    VM->>V: cx.notify()
    Note over V,U: disclosures stay closed click opens instantly into Loaded data

    U->>V: scroll near bottom
    V->>VM: should_auto_load_more?
    VM-->>V: true
    V->>GCR: load_recent_feeds(append)
```

---

## 5. Sequence — Library cold open via paged actor

Library detail goes through the `RuntimeHost` + `PagedTrackListActor`
path under `--features async-runtime` (default).

```mermaid
sequenceDiagram
    autonumber
    participant U as User
    participant V as LibraryApp + shells
    participant VM as PagedPlaylistDetailVm
    participant Cache as PagedListVm (window cache)
    participant PA as PagedTrackListActor
    participant Host as RuntimeHost (tokio)
    participant DB as db (SQLite)
    participant Bus as VmBus

    U->>V: select playlist
    V->>VM: select_playlist(id)
    VM->>Cache: ensure_window(visible_range)
    Cache-->>VM: pending pages
    VM->>V: snapshot (Pending rows)
    V-->>U: SkeletonTrackRow renders

    V->>Host: spawn / reuse actor for playlist
    Host->>PA: start
    PA->>DB: query window
    DB-->>PA: rows
    PA-->>Bus: VmEvent::PageReady(playlist_id, range)
    Bus-->>VM: invalidate window
    VM->>Cache: insert page
    Cache-->>VM: ready rows
    VM->>V: cx.notify()
    V-->>U: real rows render

    Note over Host,PA: re-selecting same playlist short-circuits respawn

    U->>V: scroll
    V->>VM: ensure_window(new_range)
    VM->>PA: request page
    PA->>DB: query
    DB-->>PA: rows
    PA-->>Bus: VmEvent::PageReady
    Bus-->>VM: apply
    VM->>V: cx.notify()
```

---

## 6. Where the ideal architecture is **already true** vs **still pending**

| Concern | Ideal (target) | Current state |
|---|---|---|
| Layered package structure | `presentation / application / domain / infrastructure` | ✅ All four layers exist as crates-of-modules; `application/` and `runtime/` are real |
| GPUI boundary | No `gpui` import below presentation | ✅ Enforced for `metadata.rs`, services, application, runtime; ❗ `library.rs` and `search.rs` still hold non-trivial logic next to render code |
| CommandBus | All UI writes go through a typed bus | ✅ `CommandBus` + `AsyncCommandRunner` in place; ❗ a few legacy direct service calls remain in screens |
| EventBus | Domain events fan out to VMs | ✅ `ApplicationEventBus` + `VmBus`; `MetadataEvent::TrackTagged` → `VmEvent::TrackChanged` shipped |
| ViewModels as snapshot source | Views read VM snapshots only | ✅ Discover and library go through VMs; ❗ a few inspector-frame fields are still mutated from screen impl files |
| Paged / windowed lists | Windowed VMs over actors | ✅ `PagedListVm` + `PagedTrackListActor` for playlist & feed listings |
| Skeleton-first rendering | Loading states render placeholders, never raw text | ✅ Recents, track lists, and inspectors all render skeletons; auto-pagination wired |
| Eager prefetch | Latency hidden by background hydration | ✅ Recents page 2, inspector image, inspector contributors + value routes |
| Sidebar full-collapse + reflow | HIG-compliant split view | ❌ Tracked as `sidebar-full-collapse` and `sidebar-content-reflows-on-resize` |
| Unified library/playlist shell | One slot-driven shell | ⚠️ Playlist variant uses `playlist.rs` shell; other library variants still use bespoke renderers (`library-uses-playlist-shell`) |
| Artist N+1 fetch | Parallel feed hydration | ❌ Filed as `discover-artist-parallel-feed-fetch` |

---

## File map (where things live)

| Concern | Files |
|---|---|
| Application layer | `src/application/{command_bus,async_command_runner,application_event_bus,application_query_service,application_services,command_context,paged_track_list}.rs`, `src/application/{commands,queries,events,errors,ports}/` |
| Runtime layer | `src/runtime/{actor,vm_bus,paged_list_vm,mod}.rs` |
| Presentation adapters | `src/presentation/{runtime_host,gpui_command_runner,gpui_vm_bridge,gpui_event_bridge,event_bridge}.rs` |
| ViewModels | `src/view_models/*.rs` |
| Screens | `src/app.rs`, `src/app/`, `src/library.rs`, `src/library/`, `src/search.rs`, `src/search/` |
| UI design system | `src/ui/{tokens,style}.rs`, `src/ui/{primitives,composites,shells,layouts}/` |
| Domain / services | `src/{feed,library,playlist,subscribe,metadata}_service.rs`, `src/{track_compare,track_identity,identity_ingest,local_identity,sources,playback,playback_owner}.rs` |
| Infrastructure | `src/{db,api,musicbrainz,audio_tags,audio_format,tag_field,config}.rs`, `src/{rss,playback_driver,media}/` |
