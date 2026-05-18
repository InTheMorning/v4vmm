# Library / Index Album-Detail Parity Triage

## Status

Triage - 2026-05-17.

## Surfaces compared

- **Library:** `src/ui/shells/library/detail.rs`, `src/ui/shells/library/feed_detail.rs`, `src/library/app_impl.rs`, `src/view_models/library.rs`, `src/views.rs`, `src/view_models/entity_detail.rs`, `src/ui/shells/entity.rs`, `src/ui/composites/release_detail_surface.rs`, `src/db.rs`, `src/local_identity.rs`, `docs/adr/0028-local-identity-source-fact-persistence.md`
- **Index:** `src/ui/shells/search_results_inspector.rs`, `src/app/search_dispatch.rs`, `src/view_models/search_results/index_detail.rs`, `src/view_models/search_results/mod.rs`, `src/view_models/search_results/results.rs`, `src/views.rs`, `src/view_models/entity_detail.rs`, `src/ui/shells/entity.rs`, `src/ui/composites/release_detail_surface.rs`, `src/api.rs`

## Surface inventory

| Field rendered          | Library shows? | Index shows? | Same composite? | Notes |
| ----------------------- | -------------- | ------------ | --------------- | ----- |
| hero artwork            | yes @src/ui/shells/library/feed_detail.rs:74 | yes @src/app/search_dispatch.rs:430 | yes | Both pass image slots into the shared release shell. |
| title                   | yes @src/views.rs:550 | yes @src/views.rs:500 | yes | Shared hero projects `FeedView.title`. |
| artist / subtitle       | yes @src/views.rs:538 | yes @src/views.rs:501 | yes | Library derives from first local track; Index uses release artist. |
| publisher               | no @src/views.rs:560 | yes @src/views.rs:510 | yes | Shared hero can show `Publisher`; Library local VM sets it absent. |
| primary release actions | yes @src/ui/shells/library/feed_detail.rs:107 | yes @src/app/search_dispatch.rs:478 | yes | Same action slot; labels/commands differ by source ownership. |
| identity action buttons | yes @src/ui/shells/library/feed_detail.rs:183 | yes @src/ui/shells/search_results_inspector.rs:232 | yes | Shared identity action renderer. |
| release kind            | yes @src/view_models/entity_detail.rs:836 | yes @src/view_models/entity_detail.rs:836 | yes | Library renders `Unknown` because local VM sets no release kind. |
| release date            | no @src/views.rs:555 | yes @src/views.rs:505 | yes | Shared summary fact renders only when `FeedView.release_date` exists. |
| track count             | yes @src/views.rs:558 | yes @src/views.rs:508 | yes | Both feed `episode_count` into shared summary facts. |
| duration                | yes @src/view_models/entity_detail.rs:856 | yes @src/view_models/entity_detail.rs:856 | yes | Computed from visible track durations on both sides. |
| language                | no @src/views.rs:556 | yes @src/views.rs:506 | yes | Local schema has `feeds.language`, but local VM does not load/project it. |
| explicit state          | no @src/views.rs:557 | yes @src/views.rs:507 | yes | Shared summary fact renders only explicit `true`. |
| description / summary / annotation | yes @src/ui/shells/library/feed_detail.rs:192 | yes @src/app/search_dispatch.rs:432 | yes | Both use a description panel; Index prefers source release-claim text. |
| website identity row    | yes @src/view_models/entity_detail.rs:1004 | yes @src/view_models/entity_detail.rs:1004 | yes | Closed by ADR 0028 local identity hydration. |
| Nostr identity row      | yes @src/view_models/entity_detail.rs:1010 | yes @src/view_models/entity_detail.rs:1010 | yes | Closed by ADR 0028 local identity hydration. |
| feed URL identity row   | yes @src/view_models/entity_detail.rs:1016 | yes @src/view_models/entity_detail.rs:1016 | yes | Both render when `FeedView.feed_url` is non-empty. |
| GUID identity row       | yes @src/view_models/entity_detail.rs:1022 | yes @src/view_models/entity_detail.rs:1022 | yes | Both render when `FeedView.feed_guid` is non-empty. |
| contributor identity    | yes @src/ui/shells/library/feed_detail.rs:187 | yes @src/view_models/entity_detail.rs:970 | yes | Closed, see post-ADR-0028 task 001 and ADR 0028 lines 316-317. |
| track list              | yes @src/ui/shells/library/feed_detail.rs:184 | yes @src/app/search_dispatch.rs:433 | yes | Same release surface track section; row actions differ by source. |
| fallback source metadata | no @src/ui/shells/library/detail.rs:61 | yes @src/ui/shells/search_results_inspector.rs:219 | no | Index-only fallback when rich remote feed payload is unavailable. |

## Gap analysis

### Field: publisher

- Library renderer: `src/ui/shells/entity.rs:311`
- Index renderer: `src/ui/shells/entity.rs:317`
- Library VM source: `FeedView::publisher_text`, `src/views.rs:122`; local projection sets it to `None`, `src/views.rs:560`
- Index VM source: `api::Feed::publisher_text`, `src/api.rs:118`; projected by `FeedView::from_api`, `src/views.rs:510`
- Local persistence today: not persisted as a feed publisher column; `feeds` has no publisher column in `src/db.rs:2354`
- Hydration path: local `AlbumNode` receives no publisher field in `src/view_models/library.rs:317`; `render_library_feed_detail` builds a `FeedRow` without publisher in `src/ui/shells/library/feed_detail.rs:53`
- Routing: persistence
- Rationale: The shared hero already supports publisher data rows, and Index feeds hydrate the value from MusicIndex. Library local feed detail cannot render an equivalent publisher because there is no local feed publisher field or `AlbumNode` field to carry it.

### Field: release kind

- Library renderer: `src/view_models/entity_detail.rs:836`
- Index renderer: `src/view_models/entity_detail.rs:836`
- Library VM source: `FeedView::release_kind`, `src/views.rs:121`; local projection sets it to `None`, `src/views.rs:559`
- Index VM source: `api::Feed::release_kind`, `src/api.rs:116`; projected by `FeedView::from_api`, `src/views.rs:509`
- Local persistence today: not persisted as MusicIndex release kind; `feeds.podcast_medium` exists in `src/db.rs:2362` but no `release_kind` column is present in `src/db.rs:2354`
- Hydration path: `build_tree` constructs `AlbumNode` without release-kind data, `src/library/app_impl.rs:2576`; `AlbumNode` has no release-kind field, `src/view_models/library.rs:317`
- Routing: persistence
- Rationale: This field is structurally present on both surfaces, but Library renders the shared fallback `Unknown` while Index can render MusicIndex `release_kind`. The local RSS-era `podcast_medium` column may be related, but the MusicIndex release-kind source fact is not persisted or routed through the Library album VM today.

### Field: release date

- Library renderer: not rendered; shared summary only renders when `FeedView.release_date` is present, `src/view_models/entity_detail.rs:844`
- Index renderer: `src/view_models/entity_detail.rs:844`
- Library VM source: `FeedView::release_date`, `src/views.rs:117`; local projection sets it to `None`, `src/views.rs:555`
- Index VM source: `api::Feed::release_date`, `src/api.rs:117`; projected by `FeedView::from_api`, `src/views.rs:505`
- Local persistence today: not persisted as feed release date; `tracks.pub_date` exists in `src/db.rs:2382` but `feeds` has no release-date column in `src/db.rs:2354`
- Hydration path: local `AlbumNode` carries no release-date field in `src/view_models/library.rs:317`; `build_tree` copies only feed id/guid/url/description/image/identity/tracks into `AlbumNode`, `src/library/app_impl.rs:2576`
- Routing: persistence
- Rationale: Index has a feed-level release date from MusicIndex and the shared summary grid can render it. Library only has track-level `pub_date` persistence and never derives or persists a feed-level release date for the album detail VM.

### Field: explicit state

- Library renderer: not rendered; shared summary only renders explicit state when `FeedView.explicit == Some(true)`, `src/view_models/entity_detail.rs:868`
- Index renderer: `src/view_models/entity_detail.rs:868`
- Library VM source: `FeedView::explicit`, `src/views.rs:119`; local projection sets it to `None`, `src/views.rs:557`
- Index VM source: `api::Feed::explicit`, `src/api.rs:120`; projected by `FeedView::from_api`, `src/views.rs:507`
- Local persistence today: not persisted as feed explicit state; `tracks.itunes_explicit` exists in `src/db.rs:2391` but `feeds` has no explicit column in `src/db.rs:2354`
- Hydration path: local `AlbumNode` carries no explicit field in `src/view_models/library.rs:317`; `FeedView::from_local_with_identity` sets `explicit: None`, `src/views.rs:557`
- Routing: persistence
- Rationale: The shared detail VM can render explicit state without renderer changes, but the Library album path has no feed-level explicit source persisted or projected. Track-level RSS explicit data is a different scope and is not a feed-detail parity source today.

### Field: language

- Library renderer: not rendered; shared summary only renders when `FeedView.language` is non-empty, `src/view_models/entity_detail.rs:862`
- Index renderer: `src/view_models/entity_detail.rs:862`
- Library VM source: `FeedView::language`, `src/views.rs:118`; local projection sets it to `None`, `src/views.rs:556`
- Index VM source: `api::Feed::language`, `src/api.rs:119`; projected by `FeedView::from_api`, `src/views.rs:506`
- Local persistence today: `feeds.language`, `src/db.rs:2360`
- Hydration path: `subscribed_feeds` selects id/url/guid/title/description/image/subscribed but not language, `src/db.rs:68`; `FeedRow` has no language field, `src/db.rs:10`; `build_tree` therefore cannot put language into `AlbumNode`, `src/library/app_impl.rs:2540`
- Routing: loading-shape
- Rationale: Unlike release date and explicit state, the local schema already has a feed-language column. The gap is in the local read model and VM loading shape: `FeedRow`, `AlbumNode`, and `FeedView::from_local_with_identity` do not carry that persisted value to the shared summary facts.

### Field: fallback source metadata

- Library renderer: not rendered; Library album detail dispatch always routes `LibraryDetail::Album` into `render_library_feed_detail`, `src/ui/shells/library/detail.rs:61`
- Index renderer: `src/ui/shells/search_results_inspector.rs:219`
- Library VM source: n/a; no Library fallback-detail VM is used for album detail, `src/ui/shells/library/detail.rs:61`
- Index VM source: `IndexDetailDisplay::id`, `src/view_models/search_results/index_detail.rs:45`; constructed by fallback `index_feed_detail`, `src/view_models/search_results/mod.rs:228`
- Local persistence today: not persisted; fallback source metadata is navigation/source chrome, not local album data, `src/view_models/search_results/index_detail.rs:40`
- Hydration path: Index fallback is used when `IndexDetailDisplay.feed` is absent, `src/ui/shells/search_results_inspector.rs:157`; rich rows attach `remote_feed` when `index_feed_display` has fetched detail, `src/app/search_dispatch.rs:1295`
- Routing: intentional asymmetry
- Rationale: This is not album metadata parity. It is an Index-only fallback surface for a missing rich MusicIndex feed payload, showing source and id so the user is not left with an empty drill-down. Library album detail has a local entity and routes through the release shell instead.

## Open questions

- Should local Library album detail derive a feed-level release date from persisted `tracks.pub_date`, or should parity wait for a true feed/release-date source fact?
- Should `feeds.podcast_medium` be treated as the local equivalent of MusicIndex `release_kind`, or should MusicIndex `release_kind` be persisted separately?

## Out of scope (handled by sibling triage tasks)

- Track detail fields → Task 002
- Artist + Playlist detail fields → Task 003
