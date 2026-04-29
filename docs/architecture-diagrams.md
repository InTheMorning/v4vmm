# v4vmm Architecture Diagrams

Two views: the **current state** of the codebase, then an **ideal target** for an app of this kind.

---

## 1 — Current Architecture

### 1.1 System Overview

```mermaid
graph TD
    subgraph External["External Sources"]
        RSS["RSS Feeds"]
        MI["MusicIndex HTTP API"]
        MB["MusicBrainz API"]
        FS["Local Filesystem\n(MP3 / FLAC / WAV)"]
    end

    subgraph App["v4vmm — single binary"]
        main["main.rs"]
        cli["cli.rs\nOne-shot CLI"]
        run_app["app.rs — TopApp\nRoot GPUI entity\nTab bar · Playback bar · Settings"]

        subgraph Tabs["GPUI Views"]
            LibApp["library.rs\nLibraryApp"]
            SrchApp["search.rs\nSearchApp\n(Discover)"]
        end

        subgraph Services["Service Layer\n(no GPUI imports)"]
            FeedSvc["feed_service"]
            LibSvc["library_service"]
            SubSvc["subscribe_service"]
            PLSvc["playlist_service"]
            MetaSvc["metadata_service"]
            PlaySvc["playback.rs"]
        end

        subgraph Playback["Playback"]
            POwner["playback_owner\nPlaybackOwner‹D›"]
            PDriver["playback_driver/\nmpv · null"]
        end

        DB[("SQLite\nArc‹Mutex‹Connection››")]
    end

    main --> run_app
    main --> cli
    run_app --> LibApp
    run_app --> SrchApp
    run_app --> POwner

    LibApp --> FeedSvc & LibSvc & PLSvc & MetaSvc
    SrchApp --> FeedSvc & LibSvc & SubSvc & PLSvc
    cli --> FeedSvc & LibSvc & SubSvc & PLSvc & PlaySvc

    FeedSvc --> RSS & MI
    SubSvc --> MI & MB & FS
    SrchApp --> MI
    LibApp --> MB

    FeedSvc & LibSvc & SubSvc & PLSvc & MetaSvc & PlaySvc --> DB
    POwner --> PlaySvc & PDriver
```

---

### 1.2 Module Relationships

```mermaid
graph LR
    subgraph UI["UI Layer (GPUI)"]
        TopApp["app.rs\nTopApp"]
        LibApp["library.rs\nLibraryApp"]
        SrchApp["search.rs\nSearchApp"]
        UITrack["ui_track.rs"]
        UIFeed["ui_feed.rs"]
        UIArtist["ui_artist.rs"]
        UICommon["ui_common.rs"]
        UICtx["ui_context.rs"]
        Views["views.rs"]
    end

    subgraph UIKit["UI Kit (ui/)"]
        Theme["theme.rs\ncolor · spacing\ntypography · radius"]
        PlaylistPop["playlist_popover.rs\nAddToPlaylistPopover"]
    end

    subgraph Domain["Domain / Service"]
        FeedSvc["feed_service"]
        LibSvc["library_service"]
        SubSvc["subscribe_service"]
        PLSvc["playlist_service"]
        MetaSvc["metadata_service"]
        PlaySvc["playback"]
        TrackCmp["track_compare"]
        Meta["metadata.rs\nTrackContext\nTagCompareResult"]
    end

    subgraph Infra["Infrastructure"]
        DB["db.rs\nrow types · queries"]
        API["api.rs\nMusicIndex client"]
        RSS["rss/\nfetch · parse\nPodcasting 2.0"]
        Tags["audio_tags.rs\nID3v2.4 r/w\nlofty"]
        Fmt["audio_format.rs\nbyte detection"]
        MB["musicbrainz.rs"]
        Cfg["config.rs"]
        Img["media/ImageCache"]
    end

    TopApp --> LibApp & SrchApp
    LibApp --> UITrack & UIFeed & UIArtist & UICommon & Views & UICtx
    SrchApp --> UITrack & UIFeed & UIArtist & UICommon & Views
    UITrack & UIFeed & UIArtist --> PlaylistPop
    UITrack & UIFeed & UIArtist & UICommon & Views --> Theme

    LibApp --> FeedSvc & LibSvc & PLSvc & MetaSvc & TrackCmp
    SrchApp --> FeedSvc & LibSvc & SubSvc & PLSvc & TrackCmp

    FeedSvc --> RSS & API & DB
    LibSvc --> SubSvc & DB
    SubSvc --> Tags & Fmt & MB & MetaSvc & DB & API
    PLSvc --> DB
    MetaSvc --> Meta & Tags & DB
    TrackCmp --> Meta
    PlaySvc --> DB
    Meta -.->|no GPUI| Domain

    DB --> Cfg
```

---

### 1.3 UI Component Hierarchy

```mermaid
graph TD
    subgraph Window["GPUI Window"]
        TopApp["TopApp\nwindow root"]

        subgraph TopBar["Top Bar"]
            Tabs["Tab selector\n(Library · Discover · Settings)"]
            PlayBar["Playback bar\ntrack info · controls"]
        end

        subgraph Content["Content Area (active tab)"]
            subgraph LibTab["LibraryApp"]
                LibTree["Artist/Album/Track tree"]
                LibInsp["Inspector stack\n(artist · feed · track)"]
                LibInsp --> MetaGrid["Metadata compare grid\nRSS · ID3 · MusicBrainz"]
                LibInsp --> MBPanel["MusicBrainz lookup panel"]
                LibInsp --> FeedUpd["Feed update panel"]
            end

            subgraph SrchTab["SearchApp (Discover)"]
                SearchBar["Search input"]
                Results["Results list\n(artists · feeds · tracks)"]
                SrchInsp["Inspector stack\n(artist · feed · track · publisher)"]
                SrchInsp --> SrchMetaGrid["Metadata compare grid"]
                SrchInsp --> PodRoll["Podroll section"]
                SrchInsp --> ValueRoutes["Value‑4‑value routes"]
            end

            subgraph SettingsTab["Settings"]
                ConfigFields["Config fields\nendpoint · dirs · flac path"]
            end
        end

        subgraph SharedComponents["Shared render helpers"]
            TrackRow["ui_track.rs\nTrackRow (Discover mode)"]
            FeedRow["ui_feed.rs\nFeed header/row"]
            ArtistRow["ui_artist.rs\nArtist row"]
            Thumb["ui_common.rs\nThumbnail · truncated"]
            AddPLPop["playlist_popover.rs\nAddToPlaylistPopover\n(floating HIG popover)"]
        end
    end

    TopApp --> TopBar & Content
    Results --> TrackRow & FeedRow & ArtistRow
    TrackRow --> AddPLPop & Thumb
    FeedRow --> Thumb
    LibTree --> Thumb
```

---

### 1.4 Key Call Flows

```mermaid
sequenceDiagram
    participant U as User
    participant SR as SearchApp
    participant FS as feed_service
    participant LS as library_service
    participant SS as subscribe_service
    participant PL as playlist_service
    participant DB as SQLite
    participant Disk as Filesystem

    U->>SR: clicks "+ Playlist" on track row
    SR->>SR: AddToPlaylistPopover opens (popover state)
    U->>SR: selects playlist (or creates one)

    alt create new playlist
        SR->>PL: create(name)
        PL->>DB: INSERT playlists
        PL-->>SR: playlist_id
        SR->>SR: load_playlists()
    end

    SR->>FS: ensure_feed_in_db(feed_guid, feed_url)
    FS->>DB: find_feed_id_by_guid
    alt feed not in DB
        FS->>FS: rss::subscribe_feed()
        FS->>DB: INSERT feeds + tracks
    end
    FS-->>SR: feed_id

    SR->>DB: SELECT track_id by feed_id + item_guid
    SR->>SS: subscribe_then_append_to_playlist(playlist_id, [track_id])
    SS->>DB: fetch TrackRow
    alt not yet downloaded
        SS->>Disk: download enclosure
        SS->>SS: detect audio format
        SS->>SS: write ID3v2.4 tags
        SS->>DB: mark_track_downloaded
    end
    SS->>DB: set_track_in_library
    SS->>PL: append track to playlist
    PL->>DB: INSERT playlist_tracks
    SS-->>SR: AppendToPlaylistOutcome
    SR->>SR: update status label, cx.notify()
```

---

## 2 — Ideal Architecture

### 2.1 System Overview

```mermaid
graph TD
    subgraph External["External Sources"]
        RSS["RSS Feeds"]
        MI["MusicIndex API"]
        MB["MusicBrainz API"]
        FS["Filesystem"]
    end

    subgraph App["Ideal App — layered binary"]
        subgraph Presentation["Presentation Layer\n(GPUI only)"]
            RootView["RootView\n(window shell, nav)"]
            LibView["LibraryView"]
            DiscoverView["DiscoverView"]
            PlaybackView["PlaybackView (mini-player)"]
            CompSystem["Design System\ntheme · tokens · components"]
        end

        subgraph AppLayer["Application Layer\n(commands · queries · events)"]
            CmdBus["CommandBus\ncreate-playlist · subscribe-track\ndownload · tag-write"]
            QuerySvc["QueryService\nlibrary-tracks · playlists\nsearch-results"]
            EventBus["EventBus\n(typed domain events)"]
            VM["ViewModels\n(pure Rust, no GPUI)"]
        end

        subgraph Domain["Domain Layer\n(pure business logic)"]
            LibDom["Library\nmembership rules"]
            PlaylistDom["Playlist\nordering · dedup"]
            PlaybackDom["Playback\nsession state machine"]
            MetaDom["Metadata\nprovenance model"]
        end

        subgraph Infra["Infrastructure Layer"]
            Repo["Repositories\n(SQLite-backed)"]
            MediaClient["MediaClient\n(MusicIndex + MusicBrainz)"]
            RSSClient["RSSClient\n(fetch + parse)"]
            TagIO["TagIO\n(ID3v2.4 / lofty)"]
            AudioDL["AudioDownloader\n(byte-detect + transcode)"]
            PlayerAdapter["PlayerAdapter\n(mpv / null / future)"]
        end
    end

    RootView --> LibView & DiscoverView & PlaybackView
    LibView & DiscoverView & PlaybackView --> VM
    VM --> QuerySvc
    LibView & DiscoverView --> CmdBus
    CmdBus --> LibDom & PlaylistDom & PlaybackDom & MetaDom
    LibDom & PlaylistDom & PlaybackDom & MetaDom --> Repo
    QuerySvc --> Repo
    EventBus --> VM

    MediaClient --> MI & MB
    RSSClient --> RSS
    TagIO & AudioDL --> FS

    CmdBus --> MediaClient & RSSClient & TagIO & AudioDL & PlayerAdapter
    Repo -.-> EventBus
```

---

### 2.2 Ideal Module Relationships

```mermaid
graph LR
    subgraph DesignSystem["Design System (ui/)"]
        Tokens["tokens.rs\ncolor · spacing · type · radius"]
        Primitives["primitives/\nButton · Input · Divider\nPopover · Badge · Icon"]
        Composites["composites/\nTrackRow · FeedCard\nPlaylistPopover · MetaGrid\nNowPlayingBar"]
        Layouts["layouts/\nInspectorStack · SplitPane\nScrollList"]
    end

    subgraph Views["Views (views/)"]
        LibV["library/\nview + viewmodel"]
        DiscV["discover/\nview + viewmodel"]
        PlayV["playback/\nview + viewmodel"]
        SettV["settings/\nview + viewmodel"]
    end

    subgraph AppSvc["Application Services"]
        LibCmd["library_commands"]
        SubCmd["subscribe_command"]
        PLCmd["playlist_commands"]
        PlayCmd["playback_commands"]
        LibQ["library_queries"]
        SrchQ["search_queries"]
    end

    subgraph DomainMods["Domain"]
        LibDom["library"]
        PLDom["playlist"]
        PlayDom["playback_session"]
        MetaDom["metadata_provenance"]
    end

    subgraph InfraMods["Infrastructure"]
        DB["db/\nrepositories"]
        APIClient["api_client/"]
        RSSMod["rss/"]
        Tags["tag_io/"]
        Audio["audio/\ndetect · download · transcode"]
        Player["player/\nmpv · null"]
        Cfg["config/"]
    end

    Views --> DesignSystem
    Composites --> Primitives
    Layouts --> Primitives
    Views --> AppSvc

    LibCmd & SubCmd & PLCmd --> LibDom & PLDom & MetaDom
    PlayCmd --> PlayDom
    LibQ & SrchQ --> DB

    LibDom & PLDom & MetaDom & PlayDom --> DB
    SubCmd --> APIClient & RSSMod & Tags & Audio
    PlayCmd --> Player
    APIClient --> DB
```

---

### 2.3 Ideal UI Component Hierarchy

```mermaid
graph TD
    subgraph Tokens["Design Tokens (single source of truth)"]
        Color["color palette\ndark/light semantic aliases"]
        Space["spacing scale\n4pt grid"]
        Type["typography\nsize · weight · family"]
        Rad["border radius\nSM · MD · LG · full"]
    end

    subgraph Primitives["Primitive Components\n(stateless, token-driven)"]
        Btn["Button\nvariant · size · icon"]
        Inp["Input\nstate · placeholder · validation"]
        Div2["Divider"]
        Badge["Badge\nlabel · color"]
        Ico["Icon\nSF Symbol wrapper"]
    end

    subgraph Composites["Composite Components\n(own local state)"]
        TrackRow["TrackRow\nthumb · title · dur · actions"]
        FeedCard["FeedCard\nartwork · meta · subscribe btn"]
        MetaGrid["MetaGrid\nRSS · ID3 · MB columns"]
        PLPop["PlaylistPopover\nlist mode · create mode"]
        NPBar["NowPlayingBar\nartwork · scrubber · transport"]
        InspStack["InspectorStack\nbreadcrumb navigation"]
    end

    subgraph Screens["Screens\n(compose composites + layouts)"]
        LibScr["LibraryScreen"]
        DiscScr["DiscoverScreen"]
        SettScr["SettingsScreen"]
    end

    Tokens --> Primitives
    Primitives --> Composites
    Composites --> Screens
```

---

### 2.4 Ideal Data Flow (Reactive)

```mermaid
sequenceDiagram
    participant U as User
    participant V as View (GPUI)
    participant VM as ViewModel
    participant CB as CommandBus
    participant D as Domain
    participant R as Repository
    participant EB as EventBus
    participant Ex as External (API/RSS/Disk)

    Note over V,VM: UI reads from ViewModels (snapshots, no direct DB access)

    U->>V: interaction (click, input)
    V->>CB: dispatch Command (typed, validated)
    CB->>D: apply business rule
    D->>R: persist change
    R->>EB: emit DomainEvent
    EB->>VM: update snapshot
    VM->>V: cx.notify() → re-render

    Note over CB,Ex: Side effects are isolated to commands

    CB->>Ex: fetch RSS / download file / write ID3
    Ex-->>CB: Result
    CB->>R: persist outcome
    R->>EB: emit DomainEvent
    EB->>VM: update snapshot
    VM->>V: re-render with new state
```

---

## Key Differences: Current vs Ideal

| Concern | Current | Ideal |
|---|---|---|
| **State location** | `SearchApp` / `LibraryApp` hold all state | ViewModels hold read snapshots; domain owns write state |
| **Service calls** | Direct `&mut self` methods dispatch inline | CommandBus decouples dispatch from execution |
| **UI reuse** | Ad-hoc render functions, 4 duplicate inline panels | Design system: tokens → primitives → composites → screens |
| **Reactivity** | `cx.notify()` called manually after each mutation | EventBus propagates domain events to all interested VMs |
| **GPUI boundary** | Partially enforced (ADR 0022 in progress) | Hard boundary: no `gpui` import below Application layer |
| **Error handling** | `self.status = format!(...)` strings | Typed error events surfaced via EventBus → toast/banner |
| **Playback** | `PlaybackOwner<D>` generic in root | `PlayerAdapter` trait in Infra; domain state machine independent |
| **CLI** | Shares service functions with UI directly | Shares CommandBus and QueryService — same path as UI |
| **Theme** | Constants in `ui/theme.rs` | Design tokens with semantic aliases + dark/light switching |
