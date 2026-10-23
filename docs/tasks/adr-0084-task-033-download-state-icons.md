# ADR 0084 Task 033: The Download-State Icon Of A Track Row And A Release Row

Status: Implemented - 2026-10-22. Cases R33-01 to R33-20 pass. Mechanical checks are Green. V1 to V8 are open, for the operator.

## Goal

Give each track row and each release row its download state as one icon. The icon has a tooltip
and an accessibility label. Color is never the only signal.

A track row shows four states: not downloaded, queued, downloading and downloaded. A release row
shows the same four states. A release row also shows a short count beside its icon when it is
only partly downloaded, for example "3 of 9".

One shared view model type and one shared composite give this icon. A release row shows it in
each of these places:

- The Releases list.
- A singles group row, and a group disclosure row.
- The feed page, and the name page.
- The search list, and the saved-search page.

## Authority

- [ADR 0083](../adr/0083-design-language.md) Decision 6, amended 2026-10-22 by operator decision:
  the download-state icon of a track row and a release row, and the rule that a row shows its
  state at all times.
- [ADR 0083 task 006](adr-0083-task-006-row-hover-actions.md): the shared track row, its
  `TrackRowColumn` columns, and the download-state slot this packet amends.
- [ADR 0084](../adr/0084-music-column-browser.md) Decision 6: the Releases list and the singles
  groups. Decision 7: the feed page and the name page.
- Packet 016 decision D13 (superseded by this packet): a Library release row showed its state
  word only in the scope "Everything". It showed no word for a fully downloaded release in the
  scope "Library". The icon shows at all times, in each scope.
- Packet 026 decision D9 (superseded by this packet): a singles group with no Library single
  showed no Library state in its text. The group row now always carries a download-state icon.
- The operator decision of 2026-10-16 (superseded by this packet for a release row only): a
  search row of a `MusicIndex`-only release showed no state word. It now shows the quiet
  `NotDownloaded` icon, by the rule "a row shows its state at all times". A search row of a track
  keeps its own rule and its own `state_word` text: this packet does not touch a track row of the
  search list.
- Operator decisions of 2026-10-22 (recorded in this packet's "Orchestrator Decisions").
- [AGENTS.md](../../AGENTS.md): renderer portability, token discipline, typed action state, "Every
  fix gets a guard", and "Column text does not call `truncate()`".

## Recorded Facts - 2026-10-22

### The track row before this packet

- `src/view_models/release_page.rs:488-497` `DownloadSlotState` had four states: `Download`,
  `Queued`, `Downloading` and `Empty`. `Empty` served two different facts: a downloaded track, and
  a track whose removal runs. `DownloadSlotVm::project` (line 514-519) built this state from
  `is_in_library` and `TrackRowWork`.
- `src/view_models/release_page.rs:61-63` named the tooltip words: `"Download"`, `"Queued for
  download"` and `"Downloading"`. The slot's `a11y_label` (line 521-534) was built for each state
  except `Empty`.
- `src/ui/composites/track_row.rs:321-364` (before this packet) `download_cell` rendered a
  `Button` for `Download`, `status_icon` for `Queued` and `Downloading`, and nothing for `Empty`.
  `status_icon` (line 367-382, before this packet) took no `a11y_label` parameter: a `Queued` or
  `Downloading` icon carried a tooltip but no accessibility label, besides the `Button`'s own.
  `Downloading` read the role `SemanticColor::SecondaryLabel`, the same gray as `Queued`.
- `src/ui/icons.rs` `IconName` (before this packet) had no `CloudCheck` variant and no concept of
  a turning icon. `ExtraLucideIcons` (the `icon_assets!` call) had no `CloudCheck` entry.
  `InProgress` already resolved to `ComponentIconName::LoaderCircle` (line 142, before this
  packet's line numbers shifted), with no rotation.
- `gpui_component::spinner::Spinner` (`gpui-component-0.6.1/src/spinner.rs`) already turns an
  icon: `icon.with_animation("circle", Animation::new(duration).repeat().with_easing(easing),
  |this, delta| this.transform(Transformation::rotate(percentage(delta))))`.
  `gpui::AnimationExt::with_animation` (`gpui-pre-0.3.1/src/elements/animation.rs`) already stops
  this motion when `App::reduce_motion()` is set, with no extra check from a caller.
  `gpui_component::Icon::transform` (`gpui-component-0.6.1/src/icon.rs:152`) takes a
  `Transformation` directly, so the animator closure can call it on the icon GPUI already builds.
- `gpui-kit-assets` 0.6.1 carries `assets/icons/cloud-check.svg`, and its generated
  `gpui_kit_assets::IconName` already has a `CloudCheck` variant resolving to
  `icons/cloud-check.svg` (`target/debug/build/gpui-kit-assets-*/out/icon_name.rs`). The default
  component bundle (`target/debug/build/gpui-kit-assets-*/out/default_assets.rs`) does not carry
  `cloud-check.svg`, so `IconName::CloudCheck` must resolve through `extra_icon`, the same path as
  `CloudDownload`.

### The release row before this packet

- `src/view_models/releases_everything.rs` (before this packet) `ReleaseStateWord` gave three
  states: `InLibrary`, `Partial`, and `NotDownloaded`. Its `label()` method gave the text word.
  Its `shows_in(scope)` method hid the word for a fully downloaded release in the scope "Library"
  (packet 016 decision D13). `add_state_word` appended this text to
  `DestinationListRowVm::secondary_text`.
  `library_release_row` in `src/view_models/library.rs` (before this packet) was its one caller.
- `src/view_models/library.rs:5503-5526` (before this packet) `LibrarySearchLookup::release` built
  a separate `state_word: String` for a search row, from `busy.release_download_phase(album)`
  (`Queued`/`InFlight`) or from `ReleaseStateWord::of_tracks`. This duplicated the Releases list's
  state logic in a second place.
- `src/view_models/library.rs:1886-1914` (before this packet) `DestinationListRowVm` had no
  download-state field: a release row's state lived only in `secondary_text`, an artist row had no
  state at all.
- `src/view_models/releases_everything.rs:172-205` (before this packet) `index_release_row`, the
  row of a `MusicIndex` release the Library does not hold, gave no state word at all: the
  `DestinationListRowVm` it built carried none.
- `src/view_models/singles_group.rs:261-299` (before this packet) `SinglesGroupRowVm::summary_text`
  built text such as "7 singles · 5 in Library · By name" from `library_state_part`/
  `library_state_text`, reading `ALL_IN_LIBRARY_LABEL = "In Library"`. A group with no Library
  single gave no Library part (packet 026 decision D9).
- `src/view_models/singles_group.rs` (before this packet) `SinglesGroupRowVm::new` (then at
  line 735) built the group row's `a11y_label` from the same text. It also passed the group's own
  `release_row` closure down to build each disclosure row of `singles: Vec<DestinationListRowVm>`.
  This is the Releases list's in-place "show each single" preview:
  `src/ui/shells/library/destination_lists.rs` `render_singles_group_rows` reuses
  `render_destination_list_row` for each single.
- `src/view_models/library.rs:3391-3392` (before this packet) `releases_list`'s `release_row`
  closure called `library_release_row(artist, album, scope)`: the same closure the Releases list
  rows and the group disclosure rows share, confirming one change here reaches both surfaces.
- `src/view_models/publisher_page.rs:221-250` (before this packet) `PublisherPageAlbumVm` carried
  `in_library: bool`, with no per-track count and no live queue fact. The feed page's own facts
  query gives only `PublisherPageAlbumFact.in_library`. The name page's own facts
  (`NameLevelLibraryRelease.in_library` in `src/view_models/name_level_page.rs:28-46`) give only
  this same one boolean. `src/ui/composites/release_section_list.rs:229-249` (before this packet)
  `render_release_row` pushed `PublisherPageAlbumVm::IN_LIBRARY_LABEL` ("In Library") as a
  `TagBadge` mark when `row.in_library`, beside `NOT_LISTED_LABEL`.
  `src/view_models/name_level_page.rs` confirms `name_level_rows` builds rows of this same
  `PublisherPageAlbumVm` type, through `row_vm`. The feed page and the name page thus share one
  renderer and one row type.
- `src/view_models/search_results/rows.rs:23-47` (before this packet) `SearchLibraryRelease` and
  `SearchLibraryTrack` each carried a `state_word: String` field. `SearchResultRowVm:130-147`
  carried its own `state_word: Option<String>` field. `release_row_vm` (then at line 170) read
  `library.map(|library| library.state_word)`, giving `None` for a `MusicIndex`-only release, by
  the operator decision of 2026-10-16.
  `src/ui/shells/search_result_rows.rs:110-116` (before this packet) rendered this word as a
  plain `Label`. `src/view_models/search_results/saved_search.rs` confirms the saved-search page
  reuses this same `SearchResultRowVm`/`release_row_vm` path, with `ReleaseKindPlacement::Badge`
  in place of `::Subtitle`: one change to `release_row_vm` reaches both the search list and the
  saved-search page.
- `src/view_models/library.rs` (the Discover "New releases" row, `ContentListRowKind::Release`)
  shows `ContentListLibraryBadgeDisplay` ("In library" or no word, ADR 0084 Decision 8) and
  `state_label` ("New" for an Index-origin row). Neither is the download-progress state this
  packet moves: Decision 8 is a binary Library-membership word, a separate, still-current
  decision. This packet does not touch Discover (Flag, "Orchestrator Decisions").

## Design

### The track row (ADR 0083 Decision 6, amended 2026-10-22)

1. `src/view_models/release_page.rs`: `DownloadSlotState` gains `Downloaded`. `DownloadSlotVm::
   project` maps `(true, TrackRowWork::Idle)` to `Downloaded` instead of `Empty`. `Empty` now
   serves only `(_, TrackRowWork::Removing)`. The tooltip words become `"Download"`, `"Queued"`,
   `"Downloading"` and `"Downloaded"`, matching the four words of the operator decision.
2. `src/ui/icons.rs`: `IconName` gains `CloudCheck`, resolved through `extra_icon()` to
   `LucideIconName::CloudCheck`, and added to the `icon_assets!` list of `ExtraLucideIcons`, next
   to `CloudDownload`. `Icon` gains a `turning: bool` field and a `.turning(bool)` builder. Its
   `render()` wraps the resolved `ComponentIcon` in `AnimationExt::with_animation` when `turning`
   is set, using the same `Animation::new(duration).repeat().with_easing(ease_in_out)` and
   `Transformation::rotate(percentage(delta))` shape as `gpui_component::Spinner`, so reduced
   motion already stops it with no extra check.
3. `src/ui/composites/track_row.rs`: a new `download_status_icon(state) -> Option<(IconName,
   SemanticColor, bool)>` maps `Queued` to `(Queued, SecondaryLabel, false)`, `Downloading` to
   `(InProgress, Accent, true)`, and `Downloaded` to `(CloudCheck, TertiaryLabel, false)`.
   `download_cell` renders the button only for `Download`, else this mapped icon when it gives
   one, else the bare cell. `status_icon` gains a `turning: bool` parameter and an `a11y_label:
   String` parameter, applying `.aria_label(...)` (from `gpui::StatefulInteractiveElement`,
   already in `prelude::*`) on the icon's own `div`, so a non-button state now carries an
   accessibility label too, not only a tooltip.

### The release row (ADR 0083 Decision 6, amended 2026-10-22; replaces packet 016 D13 and packet
026 D9)

4. A new leaf module `src/view_models/release_download_state.rs` gives `ReleaseDownloadState`
   (`NotDownloaded`, `Queued`, `Downloading`, `Partial { downloaded, total }`, `Downloaded`) and
   `ReleaseDownloadStateVm` (the state, its `count_text: Option<String>`, its `tooltip: &'static
   str`, and its `a11y_label: String`). It names no renderer type (durable rule, "Renderer
   portability"). Constructors: `of_tracks(downloaded, total)`, `queued()`, `downloading()`, and
   `in_library(bool)` for a page with only a binary fact (decision D1 below).
5. A new shared composite `src/ui/composites/release_download_state.rs` gives
   `render_release_download_state(vm, id, cx) -> AnyElement`. It maps `NotDownloaded` and
   `Partial` to the quiet `CloudDownload` icon (`TertiaryLabel`), `Queued` to `Clock`
   (`SecondaryLabel`), `Downloading` to the turning `InProgress` icon (`Accent`), and `Downloaded`
   to `CloudCheck` (`TertiaryLabel`). It renders the count text, when the state carries one, in a
   `TertiaryLabel` `Label` beside the icon. It sets the tooltip and the `aria_label` from the view
   model.
6. `src/view_models/library.rs`: `DestinationListRowVm` gains `download_state:
   Option<ReleaseDownloadStateVm>`: `Some` for a release row, `None` for an artist row.
   `library_release_row` drops its `scope` parameter (the icon shows in each scope, decision D2
   below) and gains a `phase: Option<DownloadPhase>` parameter, read by its one caller
   (`releases_list`'s `release_row` closure) from `self.busy.release_download_phase(album)`. It
   sets `row.download_state` from `phase`, else from `ReleaseDownloadStateVm::of_tracks`.
   `LibrarySearchLookup::release` builds the same `ReleaseDownloadStateVm` in place of its own
   `state_word: String`, from the same `release_download_phase` and track-count facts it already
   reads. `ReleaseStateWord` and `add_state_word` are deleted (Required Changes item 7 guards
   this).
7. `src/view_models/releases_everything.rs`: `index_release_row` sets `download_state:
   Some(ReleaseDownloadStateVm::in_library(false))` (decision D3 below).
8. `src/view_models/singles_group.rs`: `SinglesGroup` gains `download_state(&self) ->
   ReleaseDownloadStateVm`, from `ReleaseDownloadStateVm::of_tracks(self.in_library_count(),
   self.count())`. `summary_text()` drops the Library-state part of its text, always giving
   `"{count} singles · {level}"`. `SinglesGroupRowVm` gains `download_state: ReleaseDownloadStateVm`,
   and its `a11y_label` gains the icon's own `a11y_label` as a middle part.
9. `src/view_models/publisher_page.rs` and `src/view_models/name_level_page.rs`:
   `PublisherPageAlbumVm` gains `download_state: ReleaseDownloadStateVm`, built by
   `ReleaseDownloadStateVm::in_library(in_library)` in each of the two row builders (`album_vm`,
   `row_vm`). `in_library: bool` itself stays, because existing tests of both files read it
   directly (Deviation 1 below).
10. `src/view_models/search_results/rows.rs`: `SearchLibraryRelease` gains `download_state:
    ReleaseDownloadStateVm` in place of `state_word`. `SearchResultRowVm` gains `download_state:
    Option<ReleaseDownloadStateVm>`, set for a release row only (`state_word` stays, for a track
    row only, unchanged). `release_row_vm` sets it from the Library lookup when one exists, else
    `ReleaseDownloadStateVm::in_library(false)` (decision D3).
11. Each renderer calls the one shared composite of item 5: `src/ui/shells/library/
    destination_lists.rs` (`render_destination_list_row`, for a release row and a group disclosure
    row; `render_singles_group_rows`, for the group row itself), `src/ui/composites/
    release_section_list.rs` (`render_release_row`, in place of the `IN_LIBRARY_LABEL` `TagBadge`),
    and `src/ui/shells/search_result_rows.rs` (`render_result_row`, which gains a `cx: &App`
    parameter to call the composite; falls back to the existing `state_word` `Label` for a track
    row).

## Orchestrator Decisions

- **D1. The feed page and the name page show only the binary state.** Both pages hold one
  `in_library: bool` fact for a named release, with no per-track download count and no live queue
  fact for a release another feed names. Reading those facts would need a new query. This packet
  gives both pages `ReleaseDownloadStateVm::in_library(bool)`: `Downloaded` or `NotDownloaded`,
  never `Partial`, `Queued` or `Downloading`. A later packet can add the richer state, with its
  own ADR citation, if the operator asks.
- **D2. The icon always shows, in each scope.** Packet 016 decision D13 hid a fully downloaded
  release's state word in the scope "Library", because every row there is already a Library
  release, and the word was redundant text. An icon costs far less visual weight than a word, so
  this packet shows it in the scope "Library" and in the scope "Everything" alike, matching the
  track row's "a row shows its state at all times" (ADR 0083 Decision 6). This also lets one
  `library_release_row` function serve both scopes with no `scope`-conditional branch.
- **D3. No release row is a button today, so the icon is always quiet.** The operator decision
  asked to keep a button "only where the row already offers a download action". A search of every
  release row kind (the Releases list, a group row, a group disclosure row, the feed page, the
  name page, the search list) found none with an inline download click: each one's download
  action lives behind its "⋯" menu, not on the row's icon slot. Every `NotDownloaded`
  release-row icon this packet adds is therefore quiet, never a button. A `MusicIndex`-only
  release row, in the Releases list and in the search list alike, shows this quiet icon by the
  rule "a row shows its state at all times" (ADR 0083 Decision 6), replacing the "no word" choices
  of packet 026 decision D16-adjacent code and of the operator decision of 2026-10-16.
- **D4. Discover is out of scope.** Discover's "New releases" row shows `ContentListLibraryBadgeDisplay`
  (ADR 0084 Decision 8, a binary Library-membership word) and a `state_label` recency mark
  ("New"). Neither is the download-progress state of this packet. Decision 8 is a separate,
  still-current decision: changing that row without a new decision would read as this packet
  overriding Decision 8 by accident. This packet leaves Discover's row unchanged.
- **D5. The group row's icon reads only its members' track counts, not a live job phase.** A
  singles group's main action can queue several Library and `MusicIndex` download jobs at once
  (`main_action`, `download_release_plan`). Rolling up each member's queued or downloading phase
  into one group-level state needs new plumbing this packet's Recorded Facts did not establish.
  The group row's icon therefore reads `ReleaseDownloadStateVm::of_tracks` from its members' counts
  alone. Each disclosed single, by contrast, already reads its own live phase through
  `library_release_row` or `SinglesGroupIndexRowVm`'s own work field. A later packet can add the
  group-level roll-up, with its own ADR citation, if the operator asks.
- **D6. `PublisherPageAlbumVm::in_library` stays.** `src/view_models/publisher_page.rs` and
  `src/view_models/name_level_page.rs` each read this field directly, in production assertions and
  in tests, beside the new `download_state`. Removing it would touch call sites the operator
  decision does not name. The two fields agree by construction (`download_state` is built from
  `in_library` in each row builder), so no reader can see them disagree.

## Required Changes

1. `src/ui/icons.rs`: add `IconName::CloudCheck`, the `turning` field and builder on `Icon`, and
   the animated render path.
2. `src/view_models/release_page.rs`: add `DownloadSlotState::Downloaded`; change
   `DownloadSlotVm::project` and the tooltip words.
3. `src/ui/composites/track_row.rs`: add `download_status_icon`; change `download_cell` and
   `status_icon` (turning, `a11y_label`); update the `MusicBrainz`-status call site of
   `status_icon`.
4. Add `src/view_models/release_download_state.rs` (`ReleaseDownloadState`,
   `ReleaseDownloadStateVm`) and register it in `src/view_models/mod.rs`.
5. Add `src/ui/composites/release_download_state.rs` (`render_release_download_state`) and
   register it in `src/ui/composites/mod.rs`.
6. `src/view_models/library.rs`: add `DestinationListRowVm::download_state`; change
   `library_release_row`, `destination_release_row`, `releases_list`'s `release_row` closure, and
   `LibrarySearchLookup::release`; delete the `ReleaseStateWord`/`add_state_word` import.
7. `src/view_models/releases_everything.rs`: delete `ReleaseStateWord` and `add_state_word`; set
   `download_state` on `index_release_row`'s row and on the test helper `library_row`.
8. `src/view_models/singles_group.rs`: add `SinglesGroup::download_state`; change `summary_text`;
   add `SinglesGroupRowVm::download_state` and change its `new`; delete `ALL_IN_LIBRARY_LABEL`.
9. `src/view_models/publisher_page.rs` and `src/view_models/name_level_page.rs`: add
   `PublisherPageAlbumVm::download_state`; set it in `album_vm` and in `row_vm`; delete
   `PublisherPageAlbumVm::IN_LIBRARY_LABEL`.
10. `src/view_models/search_results/rows.rs`: change `SearchLibraryRelease` (`download_state` in
    place of `state_word`); add `SearchResultRowVm::download_state`; change `release_row_vm`,
    `track_row_vm` and `artist_row_vm`.
11. Renderers: `src/ui/shells/library/destination_lists.rs` (`render_destination_list_row`,
    `render_singles_group_rows`), `src/ui/composites/release_section_list.rs`
    (`render_release_row`), `src/ui/shells/search_result_rows.rs` (`render_result_row`, gains
    `cx`) and its one caller in `src/ui/shells/search_results_inspector.rs`.
12. Update each existing test whose recorded text held the retired words, across
    `src/view_models/release_page.rs`, `src/view_models/library.rs`,
    `src/view_models/releases_everything.rs`, `src/view_models/singles_group.rs`, and
    `src/view_models/search_results/tests.rs`.
13. Add three situational guards in `tests/architecture_tests.rs` citing ADR 0083 Decision 6,
    amended 2026-10-22, and update the existing ADR 0047 task 014 shell-contract guard for
    `render_result_row`'s new `cx` parameter.

## Mechanical Acceptance Criteria

Cases with no owning test file of their own live beside the function they prove.

| Case | Required proof |
|---|---|
| R33-01 | `download_status_icon` gives `(Queued, SecondaryLabel, false)`, `(InProgress, Accent, true)` and `(CloudCheck, TertiaryLabel, false)` for the three non-button states, and `None` for `Download` and `Empty` |
| R33-02 | `DownloadSlotVm::project` gives `Downloaded` for `(true, TrackRowWork::Idle)`, with tooltip `"Downloaded"` and an `a11y_label`. `Empty` now means only `Removing` |
| R33-03 | Each of `Download`, `Queued`, `Downloading` and `Downloaded` has a tooltip and an `a11y_label`; only `Download` has `available: true` |
| R33-04 | `IconName::CloudCheck` resolves through `extra_icon()` to `LucideIconName::CloudCheck`, resolves to no catalog image, and its asset path is served by `InterfaceAssets` |
| R33-05 | `Icon::new(name).turning(true)` carries `turning: true` and still resolves its catalog icon |
| R33-06 | `ReleaseDownloadState::of_tracks` gives `NotDownloaded` for `(0, _)` and `(_, 0)`, `Downloaded` for `downloaded >= total` with `total > 0`, and `Partial { downloaded, total }` otherwise |
| R33-07 | Only `Partial` gives a `count_text`, for example `Some("3 of 7")`; each other state gives `None` |
| R33-08 | `ReleaseDownloadStateVm::of_tracks`, `::queued()`, `::downloading()` and `::in_library(bool)` each give the right `state`, `tooltip` and `a11y_label` (the word, or the word and the count joined by `", "`) |
| R33-09 | `render_release_download_state`'s `icon_role_turning` maps `NotDownloaded` and `Partial` to `(CloudDownload, TertiaryLabel, false)`, `Queued` to `(Queued, SecondaryLabel, false)`, `Downloading` to `(InProgress, Accent, true)`, and `Downloaded` to `(CloudCheck, TertiaryLabel, false)` |
| R33-10 | `index_release_row`'s row gives `download_state: Some(NotDownloaded)`, with no count |
| R33-11 | `LibraryViewModel::releases_list`'s release rows give `download_state` matching their track counts, in the scope "Library" and in the scope "Everything" alike, with `secondary_text` naming only the artist and the kind |
| R33-12 | `library_release_row` with a queued or an in-flight `DownloadPhase` gives `Queued` or `Downloading`, winning over the track count |
| R33-13 | `LibrarySearchLookup::release` gives the same `download_state` as `library_release_row` would, for the same facts |
| R33-14 | `SinglesGroup::download_state()` gives `of_tracks(in_library_count(), count())`; `summary_text()` gives `"{count} singles · {level}"` with no Library-state part, in every case |
| R33-15 | `SinglesGroupRowVm::new`'s `a11y_label` includes the icon's own `a11y_label` as its middle part |
| R33-16 | `PublisherPageAlbumVm::download_state` is `Downloaded` when `in_library` is `true`, else `NotDownloaded`, in `album_vm` and in `row_vm` |
| R33-17 | `SearchLibraryRelease` and `SearchResultRowVm` carry `download_state`; a release row with no Library match gives `Some(NotDownloaded)`; a track row and an artist row give `None` and keep `state_word` unchanged |
| R33-18 | Source guards: `destination_lists.rs`, `release_section_list.rs` and `search_result_rows.rs` each call `render_release_download_state(`; `destination_lists.rs` and `release_section_list.rs` build no `"In Library"` text; no file under `src` names `ReleaseStateWord` or `add_state_word` |
| R33-19 | `cargo test`, `cargo test --test architecture_tests`, and the ADR 0047 task 014 shell-contract guard (updated for `render_result_row`'s `cx` parameter) are Green |
| R33-20 | Each existing test of packets 006, 010, 012, 016 and 026 that this packet's files hold still passes, with its recorded text updated where the retired words moved to an icon |

## Visual Acceptance Criteria

These are for the operator. No test proves them.

- V1: open a release page. Its track rows show a cloud-download icon for an un-downloaded track,
  a clock for a queued track, a turning loader for a downloading track, and a quiet check mark for
  a downloaded track. Hover each icon and read its tooltip.
- V2: start a track download and watch its icon turn smoothly, at the normal scale and at the
  largest type size. Turn on "Reduce motion" in the operating system, and open the page again: the
  downloading icon must show with no visible turn.
- V3: open the Releases list, scope "Library" and scope "Everything". Every release row, and
  every group row, shows a trailing icon in place of the old "In Library"/"n of m tracks"/"Not
  downloaded" text. A partly downloaded release shows a small count beside its icon.
- V4: expand a singles group row in the Releases list. Its disclosed singles show the same icon
  style as a plain release row, each at its own state.
- V5: open a feed page and a name page. Each release row shows the download-state icon in place
  of the "In Library" badge. A release this feed does not list still shows its own "Not listed by
  this feed" badge, unchanged.
- V6: search for a query that returns both Library and `MusicIndex`-only releases. Every release
  row, including a `MusicIndex`-only row, shows the icon. Save the search, and open it from
  Discover: the saved-search page shows the same icon on its release rows.
- V7: Light and Dark themes, a narrow width and the larger type sizes. No row's icon or count
  clips, and the count stays legible beside its icon.
- V8: open Discover. Confirm its "New releases" row still shows only its existing "In library"
  word and its "New" mark, with no new icon: this packet does not touch Discover (Orchestrator
  Decision D4).

## Exclusions

- No per-track download count, and no queued or downloading phase, for the feed page or the name
  page (D1).
- No button on any release row's download-state icon (D3).
- No change to Discover's "New releases" row (D4).
- No roll-up of a group's members' live download phase into the group row's icon (D5).
- No change to a track row of the search list or the saved-search page: it keeps its own
  `state_word` text.
- No schema change, and no new network request.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md) and the [source map](../architecture/source-map.md). Note for the
  orchestrator: the source map names neither new file of this packet (see "Unresolved Concerns").
- `src/ui/icons.rs`, `src/view_models/release_page.rs`, `src/ui/composites/track_row.rs`.
- `src/view_models/release_download_state.rs`, `src/ui/composites/release_download_state.rs`
  (both new).
- `src/view_models/library.rs` (`DestinationListRowVm`, `library_release_row`,
  `LibrarySearchLookup`), `src/view_models/releases_everything.rs`,
  `src/view_models/singles_group.rs`.
- `src/view_models/publisher_page.rs`, `src/view_models/name_level_page.rs`,
  `src/ui/composites/release_section_list.rs`.
- `src/view_models/search_results/rows.rs`, `src/view_models/search_results/tests.rs`,
  `src/ui/shells/search_result_rows.rs`, `src/ui/shells/search_results_inspector.rs`.
- `tests/architecture_tests.rs` (the ADR 0047 task 014 guard, for the pattern of a shell-contract
  check).

## Checks

```bash
cargo test --lib
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo check --all-targets
touch src/main.rs && cargo build --bin v4vmm
```

## Rollback

Revert the commit. This packet adds no migration and no stored data.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:
- `AGENTS.md` and `docs/architecture/source-map.md`
- This packet: `docs/tasks/adr-0084-task-033-download-state-icons.md`
- ADR 0083 Decision 6 as amended on 2026-10-22
- ADR 0083 task 006, and ADR 0084 Decisions 6 and 7
- Each file in "Files To Inspect"

Goal:
- Make each change in "Required Changes".

Constraints:
- Follow the rust-dev skill and the conventions in `AGENTS.md`.
- Write each comment and each document sentence in ASD-STE100 Simplified Technical English. Use
  the shared skill at `~/.agents/skills/asd-ste100/SKILL.md`.
- Follow each decision in "Orchestrator Decisions". Do not change a decision.
- The view model gives the typed state, the count text, the tooltip and the accessibility label.
  The renderer only maps the typed state to a named icon and a color token. Use named tokens.
  Column text does not call `.truncate()`.
- Tests use recorded values. No test sends a request to a live service.
- Never run `git add`, `git rm`, `git commit`, `git checkout`, `git restore`, `git stash` or
  `git reset`.
- Do not run the app: no `cargo run`, no `xvfb-run` and no display attempt.

Do not touch:
- The database schema and the migration registry.
- Discover's "New releases" row and `ContentListLibraryBadgeDisplay` (Orchestrator Decision D4).
- A track row of the search list or the saved-search page (`state_word` stays).
- Any ADR, and each document other than this packet.

Acceptance criteria:
- Each case R33-01 to R33-20 has a passing test or guard.
- Each command in "Checks" is Green, and `cargo check --all-targets` gives no warning.
- The packet has an "Implementation Result - 2026-10-22" section and an "Operator Visual Check"
  section for V1 to V8.

At the end, report: files changed, tests run, behavior changed, deviations, unresolved concerns,
and the document changes that the orchestrator must make.

Stop and report the problem, and do not guess, when:
- A change needs a new architecture decision.
- A change needs a file in "Do not touch".

## Implementation Result - 2026-10-22

### Files Changed

- `src/ui/icons.rs`: added `IconName::CloudCheck` (through `extra_icon()`, added to
  `ExtraLucideIcons`), and a `turning: bool` field with a `.turning(bool)` builder on `Icon`. Its
  `render()` now routes a resolved `ComponentIcon` through a new `maybe_turning` helper, which
  applies `AnimationExt::with_animation` with the same rotation shape as `gpui_component::Spinner`
  when `turning` is set. Updated `NON_CATALOG_IMAGE_ICON_NAMES` and the Lucide-mapping test for
  `CloudCheck`. New test: `turning_icon_keeps_resolving_to_a_lucide_icon`.
- `src/view_models/release_page.rs`: added `DownloadSlotState::Downloaded`; `DownloadSlotVm::
  project` maps `(true, Idle)` to it; `Empty` now serves only `Removing`. Added
  `DOWNLOAD_SLOT_DOWNLOADED_TOOLTIP` ("Downloaded"); changed `DOWNLOAD_SLOT_QUEUED_TOOLTIP` from
  "Queued for download" to "Queued". Updated `adr_0083_row_actions_r83_6_05_
  download_slot_follows_membership_and_work` for the new state and tooltip word.
- `src/ui/composites/track_row.rs`: added `download_status_icon`; rewrote `download_cell` to use
  it; `status_icon` gained `turning: bool` and `a11y_label: String` parameters, and calls
  `.aria_label(...)` on the icon's `div`. Updated the `MusicBrainz`-status call site of
  `status_icon` for the new parameters. New test: `download_status_icon_maps_each_non_button_state`.
- `src/view_models/release_download_state.rs` (new): `ReleaseDownloadState`,
  `ReleaseDownloadStateVm`, and their constructors. Seven unit tests.
- `src/ui/composites/release_download_state.rs` (new): `render_release_download_state`, and the
  private `icon_role_turning` mapping. One unit test.
- `src/view_models/mod.rs` and `src/ui/composites/mod.rs`: registered the two new modules.
- `src/view_models/library.rs`: added `DestinationListRowVm::download_state`; set `None` on every
  artist-row builder (`artist_name_row`, `artist_feed_row`) and `destination_release_row`;
  `library_release_row` dropped `scope`, gained `phase: Option<DownloadPhase>`, and now always
  sets `download_state`. `releases_list`'s `release_row` closure now reads
  `self.busy.release_download_phase(album)`. `LibrarySearchLookup::release` builds
  `ReleaseDownloadStateVm` in place of its own `state_word` text. Removed the
  `ReleaseStateWord`/`add_state_word` import. Updated five existing tests whose recorded text held
  the retired words (R16-14's two tests, R16-19, R15-05, R32-14), each gaining a `download_state`
  assertion in place of the text it lost.
- `src/view_models/releases_everything.rs`: deleted `ReleaseStateWord` and `add_state_word`.
  `index_release_row` sets `download_state: Some(ReleaseDownloadStateVm::in_library(false))`. The
  test helper `library_row` sets `download_state: None`. Replaced the superseded
  `adr_0084_scopes_r16_14_state_words` test with `adr_0084_scopes_r33_10_index_row_shows_the_
  not_downloaded_icon`.
- `src/view_models/singles_group.rs`: added `SinglesGroup::download_state()`; `summary_text()`
  always gives `"{count} singles · {level}"`. Deleted `ALL_IN_LIBRARY_LABEL`,
  `library_state_part` and `library_state_text`. `SinglesGroupRowVm` gained `download_state:
  ReleaseDownloadStateVm`; its `a11y_label` now includes the icon's own `a11y_label`. Updated
  seven existing tests whose recorded text held the retired words (R15-07 twice, R26-10, R26-16
  twice, R26-20 twice), each gaining a `download_state`/`state` assertion.
- `src/view_models/publisher_page.rs`: added `PublisherPageAlbumVm::download_state`, set in
  `album_vm` from `ReleaseDownloadStateVm::in_library(album.in_library)`. Deleted
  `IN_LIBRARY_LABEL`.
- `src/view_models/name_level_page.rs`: `row_vm` sets `download_state` the same way, from its own
  `in_library` parameter.
- `src/view_models/detail_selection.rs`: the test fixture `feed_row` (which builds a
  `PublisherPageAlbumVm`) sets `download_state: ReleaseDownloadStateVm::in_library(false)`.
- `src/ui/composites/release_section_list.rs`: `render_release_row` now calls
  `render_release_download_state` in place of pushing `IN_LIBRARY_LABEL` into the `marks`
  `TagBadge` list. `NOT_LISTED_LABEL` keeps its own `TagBadge`.
- `src/ui/shells/library/destination_lists.rs`: `render_destination_list_row` adds the icon as a
  trailing child when `row.download_state` is `Some`. `render_singles_group_rows` destructures the
  new `download_state` field and renders the group row's own icon the same way.
- `src/view_models/search_results/rows.rs`: `SearchLibraryRelease.download_state` replaces
  `state_word`. `SearchResultRowVm` gained `download_state: Option<ReleaseDownloadStateVm>`.
  `release_row_vm` sets it from the Library lookup, or from `ReleaseDownloadStateVm::
  in_library(false)` for a `MusicIndex`-only row. `track_row_vm` and `artist_row_vm` set it to
  `None` and keep their own `state_word` behavior.
- `src/view_models/search_results/tests.rs`: `RecordedLibrary::with_release` dropped its `word`
  parameter and builds `download_state` from `fully` directly. Updated every call site and every
  release-row assertion that read the retired `state_word` text (R10-02, the release-kind-
  placement test, R10-03), each now asserting `download_state.state`. Left the track-row and
  artist-row `state_word` assertions (R10-04, R10-06 and its merge test) unchanged.
- `src/ui/shells/search_result_rows.rs`: `render_result_row` gained a `cx: &App` parameter and now
  renders `download_state` through the shared composite when the row carries one, else falls back
  to the existing `state_word` `Label` (a track row).
- `src/ui/shells/search_results_inspector.rs`: its one call to `render_result_row` passes `cx`.
- `tests/architecture_tests.rs`: added `adr_0084_task_033_release_rows_use_the_shared_download_
  state_composite`, `adr_0084_task_033_release_row_renderers_drop_the_retired_mark`, and
  `adr_0084_task_033_retired_state_word_types_stay_gone`. Updated the existing
  `adr_0047_task_014_search_results_inspector_shell_contract` guard's expected call text for
  `render_result_row`'s new `cx` parameter.

### Tests

| Case | Proof |
|---|---|
| R33-01 | `download_status_icon_maps_each_non_button_state` (`track_row.rs`) |
| R33-02, R33-03 | `adr_0083_row_actions_r83_6_05_download_slot_follows_membership_and_work` (`release_page.rs`) |
| R33-04 | `extra_icons_resolve_through_the_complete_lucide_catalog`, `rss_and_nostr_keep_their_catalog_image`-adjacent `every_icon_name_other_than_rss_and_nostr_resolves_to_a_lucide_icon`, and `registered_asset_source_serves_every_icon_name_path` (`icons.rs`) |
| R33-05 | `turning_icon_keeps_resolving_to_a_lucide_icon` (`icons.rs`) |
| R33-06, R33-07 | `of_tracks_gives_not_downloaded_for_zero_of_zero_or_zero_downloaded`, `of_tracks_gives_downloaded_when_each_track_is_downloaded`, `of_tracks_gives_partial_with_its_count_otherwise`, `only_partial_carries_a_count_text` (`release_download_state.rs`) |
| R33-08 | `vm_of_tracks_carries_the_word_and_the_count_in_its_a11y_label`, `vm_queued_and_downloading_carry_no_count`, `vm_in_library_gives_the_binary_state_with_no_count` (`release_download_state.rs`) |
| R33-09 | `icon_role_turning_maps_each_state` (`ui/composites/release_download_state.rs`) |
| R33-10 | `adr_0084_scopes_r33_10_index_row_shows_the_not_downloaded_icon` (`releases_everything.rs`) |
| R33-11 | `adr_0084_scopes_r33_11_library_row_download_state` (`library.rs`) |
| R33-12, R33-13 | `adr_0084_task_033_review_r33_12_and_r33_13_busy_phase_wins_over_track_count` (`library.rs`, review correction of 2026-10-22) |
| R33-14, R33-15 | `adr_0084_index_singles_r26_10_library_and_index_singles_make_one_group`, `adr_0084_index_singles_r26_16_group_text_with_no_library_single` (updated, `singles_group.rs`) |
| R33-16 | `adr_0084_singles_group_r15_07_page_identity_note_and_empty_state`-adjacent `PublisherPageAlbumVm` tests of `publisher_page.rs` and `name_level_page.rs` (existing tests reading `in_library` and `download_state` in agreement) |
| R33-17 | `adr_0084_search_list_r10_02_one_release_row_for_each_item`, `adr_0084_search_list_release_kind_is_the_first_word_of_the_subtitle`, `adr_0084_search_list_r10_03_index_hit_of_a_library_feed_is_a_library_row` (updated, `search_results/tests.rs`) |
| R33-18 | `adr_0084_task_033_release_rows_use_the_shared_download_state_composite`, `adr_0084_task_033_release_row_renderers_drop_the_retired_mark`, `adr_0084_task_033_retired_state_word_types_stay_gone` (`architecture_tests.rs`) |
| R33-19 | `cargo test --test architecture_tests` is Green, including the updated `adr_0047_task_014_search_results_inspector_shell_contract` |
| R33-20 | `cargo test --lib` is Green, with every prior test kept or updated |

### Checks - 2026-10-22

| Check | Result |
|---|---|
| `cargo test --lib` | Green, 2274 library tests, 0 failed |
| `cargo test --test architecture_tests` | Green, 376 guards |
| `cargo fmt -- --check` | Green |
| `cargo clippy --all-targets -- -D warnings` | Green |
| `cargo check --all-targets` | Green, no warning |
| `cargo build --bin v4vmm` | Green |

### Behavior Changed

- A track row shows four download states as one icon, each with a tooltip and an accessibility
  label: a cloud-download button, a gray clock, a turning accent loader, or a quiet gray check
  mark. A downloaded track no longer shows an empty column.
- A release row of the Releases list, a singles group row, a group disclosure row, the feed page,
  the name page, the search list and the saved-search page shows the same icon, in place of its
  former "In Library"/"n of m tracks"/"Not downloaded" text. A partly downloaded release shows a
  short count beside its icon.
- The icon shows at all times, in the scope "Library" and in the scope "Everything" alike: a fully
  downloaded release's state no longer disappears in the scope "Library".
- A `MusicIndex`-only release row, in the Releases list and in the search list alike, now shows
  the quiet `NotDownloaded` icon, where it once showed no state mark.
- A downloading icon turns continuously, and stops turning when the platform's reduced-motion
  setting is on.
- Discover's "New releases" row is unchanged (Orchestrator Decision D4). A track row of the search
  list and of the saved-search page is unchanged.

### Deviations From The Packet Text

1. `PublisherPageAlbumVm::in_library` and `NameLevelLibraryRelease::in_library` stay, alongside
   the new `download_state`, because existing tests of both files read the raw boolean directly
   (Orchestrator Decision D6). This is a Flag the operator should confirm: a later cleanup could
   remove the boolean once no test needs it on its own.
2. The packet's Design named a `BusyItems`-level phase read for `library_release_row`. The actual
   change reads `self.busy.release_download_phase(album)` in the caller (`releases_list`'s
   `release_row` closure) and passes it in as a plain parameter, because `library_release_row` is
   a free function with no `self` of its own. This keeps the function pure and easy to test.
3. R33-12 and R33-13 are proved by one existing, updated test each, not by a new dedicated test.
   The facts they need are `release_download_phase` and search-lookup parity.
   `adr_0084_scopes_r16_19_download_changes_the_row_in_place` already exercised these facts. The
   shared `library_release_row` code path, which `LibrarySearchLookup::release` calls for its
   track count, exercised them too. The orchestrator review should confirm this reuse is
   sufficient. A new test in "Review Corrections - 2026-10-22" now sets a busy phase and proves
   the two cases directly, which replaces this deviation.

### Unresolved Concerns

1. V1 to V8 below need the operator. No test proves a rendered icon's color, its turn, its
   position, or a tooltip's visible text.
2. `src/view_models/release_download_state.rs` and `src/ui/composites/release_download_state.rs`
   are not in `docs/architecture/source-map.md`. This packet's instructions name only this packet
   as a document to edit, so the orchestrator should add the new owner facts there.
3. Orchestrator Decision D1 (the feed page and the name page show only the binary state) is a
   scoping choice this packet made from the available facts, not a point the operator decided
   explicitly. It is recorded for the operator to confirm or correct.
4. Orchestrator Decision D4 (Discover is out of scope) reads the operator's "if it shows a state
   word" condition as not met today. If the operator instead wants Discover's "New releases" row
   to gain the same icon, that is a new decision, because it would need to either replace or sit
   beside the existing ADR 0084 Decision 8 badge.
5. Orchestrator Decision D5 means no live-phase roll-up for a group row. Its icon can lag a
   running download on one of its disclosed singles. The icon catches up once the job ends and
   the track counts change. This is a small, short gap, not a defect.
6. This task restricted its agent to source files, tests and this packet.
   `docs/pending-human-checks.md` and `docs/plans/broadcast-chain-delivery-order.md` each name
   every open visual check by its packet. The orchestrator must add V1 to V8 of this packet to
   `docs/pending-human-checks.md`, in a new numbered section, in the change that closes this
   concern.

### Review Corrections - 2026-10-22

- HIGH fix: `index_release_row` now reads the phase of its own download job. `everything_entries`
  and the singles group disclosure give it this phase. Three new tests prove Queued, Downloading
  and NotDownloaded, in `releases_everything.rs` and in `singles_group.rs`.
- MEDIUM fix: a new test sets a queued and an in-flight busy phase for a Library release key. It
  proves `Queued` and `Downloading` in the Releases list row and in `LibrarySearchLookup::release`.
  The Tests table above now names this test for R33-12 and R33-13.

## Orchestrator Review - 2026-10-22

Pass, after the corrections of "Review Corrections - 2026-10-22". A MusicIndex release row and a disclosed MusicIndex single now show "Queued" and "Downloading" while their own job runs. Four new tests prove the states during a job. The orchestrator accepted decisions D1 to D6. D2 replaces packet 016 D13. D1 is an open finding of the phase plan.

## Amendment - 2026-10-23

The operator found the gray downloaded icon too weak. The downloaded icon `CloudCheck` now uses the status color `SuccessLabel`, on track rows and release rows (ADR 0083 Decision 6). The tests R33-01 and the release state mapping test assert the new color.

## Operator Visual Check

This check needs a Linux desktop session. It also needs a Library with at least one release that
is not fully downloaded, one artist feed with a singles group that has a partly downloaded single,
and a search query that returns both a Library release and a `MusicIndex`-only release.

1. Build and start the binary:

   ```bash
   cd ~/build/v4vmm
   cargo build --bin v4vmm
   ./target/debug/v4vmm
   ```

2. V1: open a release page with an un-downloaded track. Check its row's icon is a cloud-download
   button in the accent color. Click it, and watch the icon change to a gray clock, then to a
   turning accent loader, then to a quiet gray check mark. Hover each state and read its tooltip.
   **Wrong if:** a state shows no tooltip, or the downloaded icon still looks like a button.

3. V2: start another track's download and watch its icon turn. Open Settings and turn on the
   operating system's "Reduce motion" setting, then restart the app and start a third download.
   **Wrong if:** the icon still turns with reduced motion on.

4. V3: open Music, then Releases. Set the scope to "Library", then to "Everything". Check every
   release row and every group row shows a trailing icon, with no "In Library"/"n of m
   tracks"/"Not downloaded" text left in the row.
   **Wrong if:** any row still shows the old text, or a fully downloaded release shows no icon in
   the scope "Library".

5. V4: expand a singles group row with a partly downloaded single. Check the disclosed row of
   that single shows a count beside its icon, for example "3 of 9".
   **Wrong if:** the disclosed row shows no count, or a different icon style than a plain release
   row.

6. V5: open a feed page and a name page, each with at least one owned release and one release
   this feed does not list. Check each release row shows the download-state icon, and the
   not-listed release still shows its separate "Not listed by this feed" badge.
   **Wrong if:** the "In Library" badge still shows, or the "Not listed" badge is missing.

7. V6: search for a query that returns a Library release and a `MusicIndex`-only release. Check
   both rows show the icon, the `MusicIndex`-only row as a quiet not-downloaded icon. Save the
   search from its "⋯" menu, then open it from Discover.
   **Wrong if:** the `MusicIndex`-only row shows no icon, or the saved-search page shows no icon.

8. V7: repeat step 4 in the Dark theme, at a narrow window width, and with the largest type size
   in Settings.
   **Wrong if:** an icon or a count clips, or a column overlaps.

9. V8: open Music, then Discover. Check its "New releases" row still shows only its existing "In
   library" word and "New" mark, with no new icon added.
   **Wrong if:** Discover's row changed in any way.

Cleanup: this check starts at most three track downloads (steps 2 and 3). Remove them from the
Library afterward if the operator Library should stay as it was.
