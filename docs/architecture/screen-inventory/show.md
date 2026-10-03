# Show Section Screen Inventory

Scope: the Show dashboard and its cards, the live status bar on other
sections, and the built-in player with its transport controls. It also
covers the playback driver condition shown to the operator, the cuelist,
and each Show popup or dialog. Source: a read-only inspection of
`/home/citizen/build/v4vmm` on 2026-10-03. This file supports Phase 2 of
`docs/plans/design-and-cleanup-overhaul-plan.md`.

Show has no popup or dialog of its own. The window-level sheet and dialog
layers (`src/ui/shells/window_layers.rs`) exist for Library flows
(`library_removal_confirmation.rs`, `playlist_removal_confirmation.rs`,
`tag_update_confirmation.rs`). No code path opens one from Show.

| Screen | Reached by | Shell file | View model | Key composites |
|---|---|---|---|---|
| Show Dashboard | Toolbar "Show" tab, `cmd-2`, or "Open Show" in the Live Status Strip | `src/ui/shells/show.rs` | `src/view_models/show.rs` (`ShowPageVm`) | `show_card.rs`, `show_log_pane.rs`, `show_detail_panel.rs` |
| Side Panel: Source detail | Click the Source card | `src/ui/composites/show_detail_panel.rs` | `SourceSectionDisplay`, `SourceReadinessDisplay` | `status_badge.rs` |
| Side Panel: Live Metadata detail | Click the Live Metadata card | `src/ui/composites/show_detail_panel.rs` | `PublisherSectionDisplay`, `EventSectionDisplay`, `PublisherServiceDisplay` | `context_menu.rs`, `status_badge.rs` |
| Side Panel: Stream (Encoder) detail | Click the Stream card | `src/ui/composites/show_detail_panel.rs` | `StreamSectionDisplay` | `status_badge.rs` |
| Side Panel: Cuelist | Default panel mode, or the "Cuelist" back action | `src/ui/shells/queue_now_playing.rs` | `QueueNowPlayingPageVm` (`QueueRowDisplay`) | none beyond primitives |
| Log Pane | Click a "Logs" action in a detail row | `src/ui/composites/show_log_pane.rs` | `ShowLogPaneDisplay` | `log_frame.rs`, `split_pane.rs`, `selectable_text.rs` |
| Transport Deck | Fixed footer on Show. Also `cmd-alt-p`, `cmd-alt-left`, `cmd-alt-right` | `src/ui/shells/queue_now_playing.rs` | `TransportDisplay` | `button.rs` |
| Live Status Strip | Always visible on Music and Settings while a show is active | `src/ui/composites/live_status_strip.rs` | `src/view_models/live_status.rs` (`LiveStatusDisplay`) | none beyond primitives |

## 1. Show Dashboard

1. Name: Show Dashboard. The operator opens it from the toolbar "Show" tab
   (`app-tab-show`), the shortcut `cmd-2` (`SelectShowTab`), or the
   "Open Show" button in the Live Status Strip.
2. Owner files:
   - Shell: `src/ui/shells/show.rs` (`render_show`, `ShowShell`).
   - View model: `src/view_models/show.rs` (`ShowPageVm`).
   - App wiring: `src/app/show.rs` (`build_show_screen`).
   - Composites: `show_card.rs`, `show_log_pane.rs`, `show_detail_panel.rs`,
     `src/ui/shells/queue_now_playing.rs`.
3. Layout, top to bottom: a summary header (title, now-playing line, or
   empty state), then an optional red status line. Below that, a row holds
   the card grid and transport deck on the left, and the trailing detail
   panel on the right.
4. Elements:
   - Caption "Show".
   - A now-playing title, artist, and duration. Or, when idle, "No active
     show" with "Show playback is idle."
   - Three fixed-height cards (Source, Live Metadata, Stream), in one, two,
     or three columns by window width.
   - Each card shows a title, a color badge, and two summary lines.
   - No artwork appears on this screen.
5. Actions: click a card to open its detail in the side panel. No
   destructive action exists on this screen.
6. Data: the now-playing line reads the live `playback_sessions` row through
   `QueueNowPlayingPageVm`. Card badges read live broadcast-service and
   encoder snapshots, not stored MusicIndex or ID3 values. No compare or
   provenance panel exists here.
7. Visual notes:
   - Page background: `SemanticColor::SystemBackground`.
   - Cards use `SecondarySystemBackground`, with an `Accent` border when
     selected.
   - Card height is fixed at `Size::MenuCompact`.
   - Long summary text clips with no wrap and no ellipsis.
8. Redesign notes:
   - No artwork appears on this screen. The now-playing title has none
     either.
   - The header's active or idle condition tracks only built-in playback.
     A live broadcast with no local playback still reads "No active
     show."
   - Card text clips silently. It does not wrap (ADR 0063 forbids
     `truncate()` here).

## 2. Side Panel: Source Detail

1. Name: Source detail. It opens inside the trailing side panel when the
   operator clicks the Source card.
2. Owner files:
   - Composite: `src/ui/composites/show_detail_panel.rs`
     (`render_source_detail`).
   - View model: `SourceSectionDisplay`, `SourceReadinessDisplay`,
     `SourceReachabilityDisplay` in `src/view_models/show.rs`.
   - App wiring: `src/app/show.rs` (`open_broadcast_readiness_in_music`).
3. Layout: a panel header (back arrow, title, close chevron), a vertical
   stack of label and value rows, then a row of action buttons.
4. Elements:
   - A "Host" row (name and summary).
   - A "Reachability" row (Reachable, Not reachable, or Unknown, with a
     detail line).
   - A "Readiness" row with a payment-route readiness condition (Checking,
     Empty, Ready, Needs attention, or Failed) and an "Open" button.
5. Actions: "Open" sends the operator to the Music tab and resets its list
   to the readiness-issues view. It is disabled when readiness is
   Unavailable, Ready, or Empty.
6. Data: host and reachability come from a live SSH or systemd service
   read. Readiness counts come from a live scan of local library files for
   payment-route tags, not from one stored database column.
7. Visual notes: plain label, value, and detail-line rows. No badge color
   appears here beyond the dashboard card. No artwork appears here.
8. Redesign notes:
   - "Readiness" here names payment-route readiness of local files. This
     differs from readiness concepts elsewhere in the app. Keep the names
     distinct in the redesign.
   - The "Open" action leaves Show for the Music tab. It is a cross-section
     jump, not an inline fix.

## 3. Side Panel: Live Metadata Detail

1. Name: Live Metadata detail. It opens in the side panel when the operator
   clicks the Live Metadata card. This card carries both the rubric's
   "publisher" surface and its "relay" surface, as one Event section.
2. Owner files:
   - Composite: `src/ui/composites/show_detail_panel.rs`
     (`render_publisher_detail`, `render_event_detail`,
     `render_publisher_service`).
   - View model: `PublisherSectionDisplay`, `EventSectionDisplay`,
     `PublisherServiceDisplay` in `src/view_models/show.rs`.
   - App wiring: `src/app/show.rs` (`run_publisher_service_command`,
     `run_event_control`, `select_show_event`).
   - Primitive: `src/ui/primitives/context_menu.rs`.
3. Layout: a panel header, then the Event block, then one row for each
   service, separated by dividers.
4. Elements, Event block (the relay surface):
   - A picker of registered Nostr relay broadcast events.
   - A primary command (Create, Replace, or Check, by the event's
     condition).
   - A "More" overflow menu (read targets, attach, detach, copy feed tag,
     refresh).
   - A "Logs" button.
   - A liveness badge (No event, Unknown, Live, or Dead).
   - A target-attachment line (Attached, Not attached, Unknown, Commands
     unavailable, Not reachable, or Failed).

   Elements, service rows: Producer (`mixxx-now-playing.service`) and
   Publisher. Each row shows a label, a color badge (Active, Inactive,
   Starting, Stopping, Failed, Not installed, Not reachable, Unknown, or
   Working), the full unit name, and Start, Stop, Reset, and Logs buttons.
5. Actions:
   - Event: Create, Replace, Check, read targets, Attach, Detach, copy feed
     tag, Refresh.
   - Each service: Start, Stop, Reset, open Logs.
   - Reset restarts the unit. No confirmation dialog appears.
6. Data: all condition facts are live, from systemd unit reads over the
   configured transport and from the relay and publisher registry. None of
   it comes from MusicIndex or RSS.
7. Visual notes: the same `render_state_badge` and `render_item_header`
   helpers give this card the same badge colors and row shape as the
   dashboard cards.
8. Redesign notes:
   - "Publisher" and "relay" are one card today. A redesign that splits them
     into separate cards changes this card's scope.
   - Reset has no confirmation step, the one destructive-feeling action on
     this screen.

## 4. Side Panel: Stream (Encoder) Detail

1. Name: Stream detail, the encoder surface. It opens in the side panel
   when the operator clicks the Stream card.
2. Owner files:
   - Composite: `src/ui/composites/show_detail_panel.rs`
     (`render_stream_detail`).
   - View model: `StreamSectionDisplay` and its connection, signal,
     recording, and listener types, in `src/view_models/show.rs`.
   - App wiring: `src/app/show.rs` (`run_stream_encoder_command`).
3. Layout: a panel header, then label and value rows, then Connect and
   Disconnect buttons when available.
4. Elements: rows for Server, Connection (Connected, Connecting,
   Disconnected, Working, Not installed, Not reachable, or Unknown), Signal
   (Present, Absent, or Unknown), Recording (Recording, Stopped, or
   Unknown, with an optional elapsed timer and file path), and Listeners
   (a count). Optional "Encoder song" and "Stream elapsed" rows follow.
5. Actions: Connect, Disconnect. Both buttons are absent when the encoder
   reports no actionable command.
6. Data: all values come from a live encoder control read, not from
   MusicIndex or RSS. "Encoder song" is a live cross-check against the
   track the encoder itself reports.
7. Visual notes: the same row and badge primitives as the other two
   detail panels. No artwork, level meter, or waveform appears here.
8. Redesign notes:
   - "Encoder song" exists only to catch drift between the app's own
     now-playing state and the encoder's own idea of the playing track.
   - Signal is a flat Present, Absent, or Unknown state. No element shows
     audio level over time.

## 5. Side Panel: Cuelist

1. Name: Cuelist. It is the side panel's default mode, reached by opening
   Show, or by the "Cuelist" back action from a card detail view.
2. Owner files:
   - Shell: `src/ui/shells/queue_now_playing.rs` (`QueueCuelistShell`,
     `render_queue_cuelist`, `render_queue_row`).
   - View model: `src/view_models/queue_now_playing.rs`
     (`QueueNowPlayingPageVm`, `QueueRowDisplay`).
   - App wiring: `src/app/queue_now_playing.rs` (`queue_now_playing_vm`).
3. Layout: one scrolling list of rows, or a centered label "Queue is
   empty."
4. Elements: each row has a leading play icon, shown only for the
   now-playing row. Each row also has a title, an optional artist in a
   secondary color, and an optional duration label. The now-playing row
   has a tinted background and a bold title.
5. Actions: none. A row is not clickable in this view, so the operator
   cannot start a track from the list itself.
6. Data: rows come from the active session's playlist or single track,
   read live from SQLite, not from MusicIndex.
7. Visual notes: fixed-height rows. Long titles and artist names clip with
   `.truncate()` inside a flexible column.
8. Redesign notes:
   - The name "Cuelist" implies a cue system. The current list is a
     read-only queue with no cue points and no reordering here. ADR 0068
     (cue and audition isolation) is Proposed and unbuilt.
   - The view model has a text filter (`set_text_filter`), but no input or
     control in this pane calls it. This looks like unused capability.
   - No artwork appears, only the generic play glyph.

## 6. Log Pane

1. Name: Show Log Pane, a bottom pane shared across services. It opens when
   the operator clicks a service's or the Event's "Logs" button, and
   closes from its own close button or the same action again.
2. Owner files:
   - Composite: `src/ui/composites/show_log_pane.rs` (`ShowLogPane`,
     `render_log_output`).
   - Shared: `log_frame.rs` (`LogFrame`, `LogFrames`), `split_pane.rs`
     (`SplitPane`), `selectable_text.rs` (`SelectableText`).
   - View model: `ShowLogPaneDisplay` in `src/view_models/show.rs`.
   - App wiring: `src/app/show.rs` (`open_publisher_logs`,
     `close_publisher_logs`).
3. Layout: a resizable vertical split below the card grid. A header row
   (source name, line-count caption, an optional "Copy tag" button, a
   close button) sits above the scrolling log text.
4. Elements: the unit or source name, a line-count label, an optional
   header status word, an optional feed-tag copy action, and selectable
   log text.
5. Actions: Close. Copy the feed tag to the clipboard. Drag the split
   handle to resize, bounded between 200 and 600 pixels (ADR 0070).
6. Data: raw log text, read live over the configured transport for the
   selected unit (Producer, Publisher, or the relay Event). None of it is
   stored metadata.
7. Visual notes: background `SecondarySystemBackground`. The open pane
   takes height priority over the card grid (ADR 0070, ADR 0073). The
   cards stay reachable by scroll above it.
8. Redesign notes:
   - Only one log source opens at a time. Switching services replaces the
     pane. It does not stack with the pane already open.
   - This is the one dense, verbose surface on Show. AGENTS.md's goal to
     keep verbose reports and logs should treat this pane as a kept
     pattern, not a target for removal.

## 7. Transport Deck (built-in player)

1. Name: Show Transport Deck. It is a fixed footer under the card grid and
   log pane on the Show screen. The shortcuts `cmd-alt-p`, `cmd-alt-left`,
   and `cmd-alt-right` reach the same commands from any tab.
2. Owner files:
   - Shell: `src/ui/shells/queue_now_playing.rs` (`QueueTransportShell`,
     `render_control_deck`).
   - View model: `TransportDisplay`, `TransportState`, in
     `src/view_models/queue_now_playing.rs`.
   - App wiring: `src/app/playback_bar.rs`, `src/app/keyboard.rs`.
3. Layout: one centered row with three icon buttons, Previous, Play or
   Pause, and Next.
4. Elements: the center icon switches between Play and Pause by state. All
   three buttons disable together when transport is Stopped. Previous and
   Next also disable independently when the queue has no playable
   neighbor in that direction.
5. Actions: Previous track, Play or Pause, Next track. None are
   destructive.
6. Data: transport state and skip availability come from the live
   `playback_sessions` row and the active playlist, read through
   `PlaybackOwner` and `ConfiguredPlaybackDriver` (mpv or a null driver).
7. Visual notes: icon-only buttons. No track title, artwork, elapsed time,
   scrub bar, or volume control appears in this deck.
8. Redesign notes:
   - The mpv driver reports a playback position every poll
     (`src/playback_driver/mod.rs`, field `position_ms`), but no screen
     shows elapsed time or lets the operator seek. This is unused driver
     capability.
   - This deck is the whole of the built-in player today: three buttons,
     no scrubbing, no volume, no artwork. AGENTS.md and the overhaul plan
     both record the player as not built.

## 8. Live Status Strip

1. Name: Live Status Strip. It is a thin bar at the top of the Music and
   Settings screens, shown only while show playback is active. It does not
   appear on the Show screen itself.
2. Owner files:
   - Composite: `src/ui/composites/live_status_strip.rs`
     (`live_status_strip`, `LiveStatusStrip`).
   - View model: `src/view_models/live_status.rs` (`LiveStatusDisplay`,
     `LiveStatusHealthState`).
   - App wiring: `src/app/show.rs` (`build_live_status_strip`), mounted
     from `src/app.rs`.
3. Layout: one row, with a summary block on the left, and the health
   badge, an optional recording badge, and an "Open Show" button on the
   right.
4. Elements:
   - Heading "Live status".
   - A summary line: the now-playing "Artist - Title" text, or "Show
     active," or "No active show."
   - A health badge, always reading "Health unknown" in the shipped app.
   - An optional red "Recording HH:MM:SS" badge.
5. Actions: "Open Show" switches the active tab to Show. No other control
   exists on this strip.
6. Data: the now-playing text reprojects the same `ShowPageVm` snapshot the
   Show screen uses. Health and recording facts come from fields
   (`broadcast_active`, `broadcast_degraded`, a recording elapsed label)
   that production code never sets to anything but false or none.
7. Visual notes: `SecondarySystemBackground` bar with a bottom border. The
   health icon color follows the Info, Success, or Warning role. Only the
   Info, "Health unknown" branch is reachable today.
8. Redesign notes:
   - The health badge and the recording badge are wired end to end but
     have no live data source. The code confirms this is a stub, not an
     assumption.
   - AGENTS.md states the live status has shown one song for months. The
     code explains why: it reprojects the same playback row on every
     refresh, and that row does not change outside tests.

## Cross-screen observations

- No artwork appears anywhere in Show: not on its cards, not in the
  Cuelist rows, not in the Live Status Strip, and not with the
  now-playing title. Of all the areas in scope, Show has the least
  existing material for the "purposeful color and artwork" goal.
- One shared color and badge system (`ShowCardStateKind` to
  `render_state_badge`) serves the dashboard cards, the Live Metadata
  service rows, and the Event badge. This gives Show one consistent state
  language, worth keeping in a redesign.
- The dashboard header tracks only built-in playback for its active or
  idle state. The three cards track only broadcast or publisher state. A
  live broadcast with idle local playback reads "No active show" at the
  top while the cards below show live activity.
- "Readiness" names two different facts in two places: Source-card
  payment-route readiness on Show, and other readiness concepts in
  Library. A shared token or label should not merge these meanings.
- The Log Pane is the one place with dense operator text, by design.
  Everywhere else on Show, text is one clipped line. These two densities
  should coexist, not merge into one.
- One error field drives two screens: a failed broadcast command shows as
  a status line on Show and as a status line on Settings, from the same
  stored text. A wording change to that text changes both screens at
  once.
- The built-in player and the cue system named in scope are, in the
  current code, three transport buttons and a read-only queue list. ADR
  0068 (cue and audition isolation) is Proposed and unbuilt, so any cue
  mockup is new design, not a reskin of existing screens.
- The mpv driver already reports playback position every second
  (`DriverStatus.position_ms`), but no Show surface renders it. A future
  scrub bar needs a new projection and renderer, not new driver work.
