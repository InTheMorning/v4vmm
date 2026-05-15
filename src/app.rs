#![warn(clippy::pedantic)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use gpui::{div, prelude::*, relative, Context, Entity, Render, SharedString, Styled, Window};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::Size;
use rusqlite::Connection;

use crate::application::commands::download::RemoveCachedFiles;
use crate::application::{ApplicationEventSubscriber, ApplicationServices, CommandContext};
use crate::config;
use crate::library::{build_tree, LibraryApp, LibraryAppEvent};
use crate::media::ImageCache;
use crate::playback_driver::ConfiguredPlaybackDriver;
use crate::playback_owner::{PlaybackOwner, PollOutcome};
use crate::presentation::{GpuiCommandRunner, GpuiEventBridge};
use crate::search::{SearchApp, SearchAppEvent};
use crate::theme_profile::ThemeProfile;
use crate::ui::control_styles::ControlStyle;
use crate::ui::layouts as layout;
use crate::ui::primitives::Button as UiButton;
use crate::ui::shells::window_layers::render_window_layers;
use crate::ui::shells::workspace::{render_workspace, WorkspaceSlots};
use crate::ui::sizable_bridge::SizableScaled;
use crate::ui::tokens::{color, FontSize, SemanticColor, Spacing};
use crate::view_models::app_toolbar::{AppToolbarVm, GlobalSearchScope};
use crate::view_models::library::{LibraryTrackRowVm, LibraryTree};
use crate::view_models::workspace::{
    WorkspaceFrameId, WorkspaceFrameKind, WorkspaceFrameState, WorkspaceLayout,
    WorkspaceLayoutConfig,
};

mod bootstrap;
mod events;
mod keyboard;
mod menu;
mod playback_bar;
mod queue_now_playing;
mod tab_bar;

pub use bootstrap::run_app;

use playback_bar::build_playback_bar;
use queue_now_playing::build_queue_now_playing_frame;
use tab_bar::render_tab_bar;

const WORKSPACE_RENDER_ENABLED: bool = true;
const WORKSPACE_CONTENT_FRAME_ID: WorkspaceFrameId = WorkspaceFrameId::new(2);
const WORKSPACE_QUEUE_FRAME_ID: WorkspaceFrameId = WorkspaceFrameId::new(4);

// ---------------------------------------------------------------------------
// AppTab
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppTab {
    Library,
    Search,
    Settings,
}

/// ADR 0046 Phase 3 transitional mount boundary.
///
/// The workspace shell wraps existing screens whole until a later task extracts
/// Library/Search internals into independent source/content/detail frame slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkspaceScreenMount {
    Library,
    Search,
    Settings,
}

impl WorkspaceScreenMount {
    const fn frame_title(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::Search => "Search",
            Self::Settings => "Settings",
        }
    }
}

impl From<AppTab> for WorkspaceScreenMount {
    fn from(tab: AppTab) -> Self {
        match tab {
            AppTab::Library => Self::Library,
            AppTab::Search => Self::Search,
            AppTab::Settings => Self::Settings,
        }
    }
}

// ---------------------------------------------------------------------------
// TopApp
// ---------------------------------------------------------------------------

pub struct TopApp {
    tab: AppTab,
    search: Entity<SearchApp>,
    library: Entity<LibraryApp>,
    global_search_input: Entity<InputState>,
    global_search_scope: GlobalSearchScope,
    endpoint_input: Entity<InputState>,
    music_dir_input: Entity<InputState>,
    flac_path_input: Entity<InputState>,
    workspace_layout: WorkspaceLayout,
    ui_scale: crate::config::UiScale,
    theme_profile: ThemeProfile,
    cfg_path: PathBuf,
    settings_status: String,
    library_tab_focus: gpui::FocusHandle,
    search_tab_focus: gpui::FocusHandle,
    settings_tab_focus: gpui::FocusHandle,
    _global_search_sub: gpui::Subscription,
    _search_sub: gpui::Subscription,
    _library_sub: gpui::Subscription,
    _appearance_sub: gpui::Subscription,
    playback_owner: Arc<Mutex<PlaybackOwner<ConfiguredPlaybackDriver>>>,
    conn: Arc<Mutex<Connection>>,
    cached_tree: LibraryTree,
    application_services: Arc<ApplicationServices>,
    command_runner: GpuiCommandRunner,
    application_event_bridge: Arc<GpuiEventBridge>,
}

impl TopApp {
    #[expect(
        clippy::too_many_arguments,
        reason = "top-level app bootstrap still wires shared state explicitly"
    )]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "bootstrap takes ownership of shared app resources before distributing them to child entities"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "top-level app bootstrap still owns one-time child entity and service wiring"
    )]
    fn new(
        conn: Arc<Mutex<Connection>>,
        image_cache: Arc<ImageCache>,
        cfg_path: PathBuf,
        musicindex_endpoint: String,
        music_dir: PathBuf,
        flac_path: Option<PathBuf>,
        workspace_layout_config: Option<WorkspaceLayoutConfig>,
        ui_scale: crate::config::UiScale,
        theme_profile: ThemeProfile,
        playback_owner: Arc<Mutex<PlaybackOwner<ConfiguredPlaybackDriver>>>,
        #[cfg(feature = "async-runtime")] runtime_host: Option<
            Arc<crate::presentation::RuntimeHost>,
        >,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        crate::ui::theme_bridge::install_theme_for_window(
            theme_profile,
            ui_scale.into(),
            window,
            cx,
        );
        let search_conn = Arc::clone(&conn);
        let search_cache = Arc::clone(&image_cache);
        let library_cache = Arc::clone(&image_cache);
        let search_endpoint = musicindex_endpoint.clone();
        let application_services = Arc::new(
            ApplicationServices::local_with_service_adapters()
                .expect("application services are fully wired"),
        );
        let application_event_bridge = Arc::new(GpuiEventBridge::new());
        let application_event_subscriber: Arc<dyn ApplicationEventSubscriber> =
            application_event_bridge.clone();
        application_services
            .event_bus()
            .subscribe(application_event_subscriber);
        let command_runner = GpuiCommandRunner::new(
            application_services.command_bus(),
            application_services.event_bus(),
        );
        let search_services = Arc::clone(&application_services);
        let library_services = Arc::clone(&application_services);
        let search = cx.new(|cx| {
            SearchApp::new(
                search_conn,
                search_cache,
                search_endpoint,
                search_services,
                #[cfg(feature = "async-runtime")]
                runtime_host.clone(),
                window,
                cx,
            )
        });
        let global_search_display = AppToolbarVm::new().display().global_search;
        let global_search_input = cx.new(|cx: &mut Context<InputState>| {
            InputState::new(window, cx).placeholder(global_search_display.placeholder)
        });
        let global_search_sub = cx.subscribe(&global_search_input, Self::on_global_search_event);
        let library = cx.new(|cx| {
            LibraryApp::new(
                conn.clone(),
                library_cache,
                musicindex_endpoint.clone(),
                library_services,
                #[cfg(feature = "async-runtime")]
                runtime_host.clone(),
                window,
                cx,
            )
        });
        let library_for_sub = library.clone();
        let search_sub = cx.subscribe(
            &search,
            move |_this: &mut Self, _search, event: &SearchAppEvent, cx| match event {
                SearchAppEvent::LibraryMutated => {
                    library_for_sub.update(cx, LibraryApp::refresh);
                }
            },
        );
        let library_sub = cx.subscribe(
            &library,
            move |this: &mut Self, _library, event: &LibraryAppEvent, cx| match event {
                LibraryAppEvent::PlayPlaylistAt {
                    playlist_id,
                    playlist_position,
                } => this.play_playlist_at(*playlist_id, *playlist_position, cx),
            },
        );
        let appearance_sub = cx.observe_window_appearance(window, |this, window, cx| {
            if this.theme_profile == ThemeProfile::System {
                crate::ui::theme_bridge::install_theme_for_window(
                    ThemeProfile::System,
                    this.ui_scale.into(),
                    window,
                    cx,
                );
                cx.notify();
            }
        });
        let endpoint_default = musicindex_endpoint.clone();
        let endpoint_input = cx.new(|cx: &mut Context<InputState>| {
            InputState::new(window, cx)
                .placeholder("https://api.musicindex.org")
                .default_value(endpoint_default)
        });
        let music_dir_default = music_dir.display().to_string();
        let music_dir_input = cx.new(|cx: &mut Context<InputState>| {
            InputState::new(window, cx)
                .placeholder("~/V4Vmusic")
                .default_value(music_dir_default)
        });
        let flac_path_default = flac_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let flac_path_input = cx.new(|cx: &mut Context<InputState>| {
            InputState::new(window, cx)
                .placeholder("flac (from $PATH)")
                .default_value(flac_path_default)
        });

        Self {
            tab: AppTab::Library,
            search,
            library,
            global_search_input,
            global_search_scope: GlobalSearchScope::All,
            endpoint_input,
            music_dir_input,
            flac_path_input,
            workspace_layout: WorkspaceLayout::from_config(workspace_layout_config.as_ref()),
            ui_scale,
            theme_profile,
            cfg_path,
            settings_status: String::new(),
            library_tab_focus: cx.focus_handle(),
            search_tab_focus: cx.focus_handle(),
            settings_tab_focus: cx.focus_handle(),
            _global_search_sub: global_search_sub,
            _search_sub: search_sub,
            _library_sub: library_sub,
            _appearance_sub: appearance_sub,
            playback_owner,
            conn,
            cached_tree: LibraryTree::default(),
            application_services,
            command_runner,
            application_event_bridge,
        }
    }

    fn maybe_start_playback_polling(&mut self, cx: &mut Context<Self>) {
        if !self
            .playback_owner
            .lock()
            .expect("lock playback owner")
            .driver()
            .is_live_driver()
        {
            return;
        }
        {
            let conn = self.conn.lock().expect("lock db");
            let mut playback_owner = self.playback_owner.lock().expect("lock playback owner");
            if let Err(error) = playback_owner.load_current_session(&conn) {
                self.settings_status = format!("Playback error: {error:#}");
            }
        }
        cx.spawn(
            async move |this: gpui::WeakEntity<TopApp>, cx: &mut gpui::AsyncApp| loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if this
                    .update(cx, |this, cx| {
                        this.poll_playback_owner();
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            },
        )
        .detach();
    }

    fn on_global_search_event(
        &mut self,
        _entity: Entity<InputState>,
        event: &InputEvent,
        cx: &mut Context<Self>,
    ) {
        if let InputEvent::PressEnter { .. } = event {
            self.submit_global_search(cx);
        }
    }

    pub(super) fn set_global_search_scope(
        &mut self,
        scope: GlobalSearchScope,
        cx: &mut Context<Self>,
    ) {
        self.global_search_scope = scope;
        cx.notify();
    }

    pub(super) fn submit_global_search(&mut self, cx: &mut Context<Self>) {
        let query = self.global_search_input.read(cx).value().to_string();
        self.tab = AppTab::Search;
        let scope = self.global_search_scope;
        self.search
            .update(cx, |search, cx| search.run_global_search(query, scope, cx));
        cx.notify();
    }

    pub(super) fn focus_global_search(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.global_search_input.update(cx, |input, cx| {
            input.focus(window, cx);
        });
    }

    fn poll_playback_owner(&mut self) {
        let conn = self.conn.lock().expect("lock db");
        let mut playback_owner = self.playback_owner.lock().expect("lock playback owner");
        match playback_owner.poll(&conn) {
            Ok(PollOutcome::NoSession | PollOutcome::Reconciled(None)) => {}
            Ok(PollOutcome::Reconciled(Some(_)) | PollOutcome::Advanced(_)) => {
                self.settings_status.clear();
            }
            Err(error) => {
                self.settings_status = format!("Playback error: {error:#}");
            }
        }
    }

    fn set_ui_scale(
        &mut self,
        scale: crate::config::UiScale,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.ui_scale == scale {
            return;
        }
        self.ui_scale = scale;
        crate::ui::theme_bridge::install_theme_for_window(
            self.theme_profile,
            scale.into(),
            window,
            cx,
        );
        cx.notify();
    }

    fn set_theme_profile(
        &mut self,
        profile: ThemeProfile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.theme_profile == profile {
            return;
        }
        self.theme_profile = profile;
        crate::ui::theme_bridge::install_theme_for_window(
            profile,
            self.ui_scale.into(),
            window,
            cx,
        );
        cx.notify();
    }

    fn persist_workspace_layout(&self) -> Result<()> {
        config::save_workspace_layout(&self.cfg_path, &self.workspace_layout.to_config())
    }

    fn save_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let endpoint = self.endpoint_input.read(cx).value().to_string();
        let music_dir = self.music_dir_input.read(cx).value().to_string();
        let flac_path = self.flac_path_input.read(cx).value().to_string();
        let ui_scale = self.ui_scale;
        let theme_profile = self.theme_profile;
        match config::save_app_settings(
            &self.cfg_path,
            &endpoint,
            &music_dir,
            &flac_path,
            ui_scale,
            theme_profile,
        ) {
            Ok((
                normalized_endpoint,
                normalized_music_dir,
                normalized_flac_path,
                saved_scale,
                saved_profile,
            )) => {
                if let Err(error) = self.persist_workspace_layout() {
                    self.settings_status = format!("Error: {error:#}");
                    cx.notify();
                    return;
                }
                let cfg = match config::load_config(&self.cfg_path)
                    .and_then(|cfg| config::ensure_dirs(&cfg).map(|()| cfg))
                {
                    Ok(cfg) => cfg,
                    Err(error) => {
                        self.settings_status = format!("Error: {error:#}");
                        cx.notify();
                        return;
                    }
                };
                self.search.update(cx, |search, cx| {
                    search.set_musicindex_endpoint(normalized_endpoint.clone(), cx);
                });
                self.library.update(cx, |library, cx| {
                    library.set_musicindex_endpoint(normalized_endpoint.clone(), cx);
                });
                self.endpoint_input.update(cx, |input, cx| {
                    input.set_value(normalized_endpoint.clone(), window, cx);
                });
                self.music_dir_input.update(cx, |input, cx| {
                    input.set_value(normalized_music_dir.display().to_string(), window, cx);
                });
                let flac_display = normalized_flac_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                self.flac_path_input.update(cx, |input, cx| {
                    input.set_value(flac_display, window, cx);
                });
                // Apply scale/profile change immediately so the UI reflects it.
                crate::ui::theme_bridge::install_theme_for_window(
                    saved_profile,
                    saved_scale.into(),
                    window,
                    cx,
                );
                self.settings_status = format!(
                    "Saved settings. Music files download under {}/artists",
                    cfg.music_dir.display()
                );
            }
            Err(error) => {
                self.settings_status = format!("Error: {error:#}");
            }
        }
        cx.notify();
    }

    fn reload_cached(&mut self) {
        let conn = self.conn.lock().expect("lock db");
        match self
            .application_services
            .query_service()
            .cached_tracks(&conn)
        {
            Ok(rows) => {
                self.cached_tree = build_tree(&rows, &conn);
            }
            Err(err) => {
                self.settings_status = format!("Error loading cached: {err:#}");
            }
        }
    }

    fn delete_cached_file(&mut self, path: String, cx: &mut Context<Self>) {
        self.delete_cached_files(vec![path], cx);
    }

    fn delete_all_cached(&mut self, cx: &mut Context<Self>) {
        let paths: Vec<String> = self
            .cached_tree
            .artists
            .iter()
            .flat_map(|a| &a.albums)
            .flat_map(|a| &a.tracks)
            .filter_map(|t| t.local_path.clone())
            .collect();
        self.delete_cached_files(paths, cx);
    }

    fn delete_cached_files(&mut self, paths: Vec<String>, cx: &mut Context<Self>) {
        if paths.is_empty() {
            return;
        }
        let command = RemoveCachedFiles::new(Arc::clone(&self.conn), paths);
        self.command_runner.run(
            command,
            CommandContext::next(),
            cx,
            |this, result, _cx| {
                this.settings_status = result.message().to_string();
                this.reload_cached();
            },
            |this, error, _cx| {
                this.settings_status = format!("Error: {error:#}");
            },
        );
    }

    fn active_workspace_screen_mount(&self) -> WorkspaceScreenMount {
        self.tab.into()
    }

    fn render_workspace_screen_mount(
        &mut self,
        mount: WorkspaceScreenMount,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        match mount {
            WorkspaceScreenMount::Library => self.library.clone().into_any_element(),
            WorkspaceScreenMount::Search => self.search.clone().into_any_element(),
            WorkspaceScreenMount::Settings => render_settings(self, cx),
        }
    }

    fn render_legacy_tab_content(
        &mut self,
        mount: WorkspaceScreenMount,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        self.render_workspace_screen_mount(mount, cx)
    }

    fn render_workspace_content(
        &mut self,
        mount: WorkspaceScreenMount,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let active_screen = self.render_workspace_screen_mount(mount, cx);
        let queue_frame = build_queue_now_playing_frame(self, cx);
        render_workspace(
            &Self::transitional_workspace_layout(mount),
            WorkspaceSlots::new()
                .content_list(active_screen)
                .queue_now_playing(queue_frame),
            cx,
        )
        .into_any_element()
    }

    fn transitional_workspace_layout(mount: WorkspaceScreenMount) -> WorkspaceLayout {
        WorkspaceLayout::new(
            vec![
                WorkspaceFrameState::new(
                    WORKSPACE_CONTENT_FRAME_ID,
                    WorkspaceFrameKind::ContentList,
                    mount.frame_title(),
                ),
                WorkspaceFrameState::with_default_title(
                    WORKSPACE_QUEUE_FRAME_ID,
                    WorkspaceFrameKind::QueueNowPlaying,
                ),
            ],
            Some(WORKSPACE_CONTENT_FRAME_ID),
        )
        .expect("transitional workspace layout has stable unique frame ids")
    }
}

impl Drop for TopApp {
    fn drop(&mut self) {
        if let Err(error) = self.persist_workspace_layout() {
            eprintln!("v4vmm::workspace: failed to save workspace layout on shutdown: {error:#}");
        }
    }
}

impl Render for TopApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.defer_application_event_drain(window, cx);
        let playback_bar = build_playback_bar(self);
        let bg_canvas = color(cx, SemanticColor::SystemBackground);
        let text_primary = color(cx, SemanticColor::Label);
        div()
            .size_full()
            .bg(bg_canvas)
            .text_color(text_primary)
            .text_sm()
            .flex()
            .flex_col()
            .overflow_hidden()
            .on_action(cx.listener(TopApp::handle_toggle_playback))
            .on_action(cx.listener(TopApp::handle_skip_playback_next))
            .on_action(cx.listener(TopApp::handle_skip_playback_previous))
            .on_action(cx.listener(TopApp::handle_open_preferences))
            .on_action(cx.listener(TopApp::handle_focus_search))
            .on_action(cx.listener(TopApp::handle_new_playlist))
            .on_action(cx.listener(TopApp::handle_select_library_tab))
            .on_action(cx.listener(TopApp::handle_select_discover_tab))
            .on_action(cx.listener(TopApp::handle_select_settings_tab))
            .on_action(cx.listener(TopApp::handle_refresh_library))
            .on_action(cx.listener(TopApp::handle_cancel_active_pane))
            .on_action(cx.listener(TopApp::handle_move_selection_up))
            .on_action(cx.listener(TopApp::handle_move_selection_down))
            .on_action(cx.listener(TopApp::handle_confirm_selection))
            .child(render_tab_bar(self, playback_bar, window, cx))
            // ADR 0046 Task 007: workspace render wraps the active whole-screen
            // mount; the legacy branch remains reachable for fallback.
            .child(
                div()
                    .key_context(keyboard::ACTIVE_PANE_KEY_CONTEXT)
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .child({
                        let mount = self.active_workspace_screen_mount();
                        if WORKSPACE_RENDER_ENABLED {
                            self.render_workspace_content(mount, cx)
                        } else {
                            self.render_legacy_tab_content(mount, cx)
                        }
                    }),
            )
            .children(render_window_layers(window, cx))
    }
}

fn render_ui_scale_picker(
    current: crate::config::UiScale,
    cx: &mut Context<TopApp>,
) -> gpui::AnyElement {
    use crate::config::UiScale;
    use crate::ui::composites::{Segment, SegmentDisplay, SegmentedControl};

    let segments = [
        Segment::new(SegmentDisplay {
            id: "ui-scale-xs".into(),
            key: UiScale::XSmall,
            label: "XS".into(),
            a11y_label: "Extra small UI scale".into(),
        }),
        Segment::new(SegmentDisplay {
            id: "ui-scale-s".into(),
            key: UiScale::Small,
            label: "S".into(),
            a11y_label: "Small UI scale".into(),
        }),
        Segment::new(SegmentDisplay {
            id: "ui-scale-m".into(),
            key: UiScale::Medium,
            label: "M".into(),
            a11y_label: "Medium UI scale".into(),
        }),
        Segment::new(SegmentDisplay {
            id: "ui-scale-l".into(),
            key: UiScale::Large,
            label: "L".into(),
            a11y_label: "Large UI scale".into(),
        }),
        Segment::new(SegmentDisplay {
            id: "ui-scale-xl".into(),
            key: UiScale::XLarge,
            label: "XL".into(),
            a11y_label: "Extra large UI scale".into(),
        }),
    ];

    let entity = cx.entity();
    SegmentedControl::new(current)
        .segments(segments)
        .on_select(move |scale, window, cx| {
            let scale = *scale;
            entity.update(cx, |this, cx| this.set_ui_scale(scale, window, cx));
        })
        .into_any_element()
}

fn render_theme_profile_picker(
    current: ThemeProfile,
    cx: &mut Context<TopApp>,
) -> gpui::AnyElement {
    use crate::ui::composites::{Segment, SegmentDisplay, SegmentedControl};

    let segments = ThemeProfile::USER_SELECTABLE.map(|profile| {
        Segment::new(SegmentDisplay {
            id: profile.as_str().into(),
            key: profile,
            label: profile.settings_label().into(),
            a11y_label: format!("{} theme profile", profile.settings_label()).into(),
        })
    });

    let entity = cx.entity();
    SegmentedControl::new(current)
        .segments(segments)
        .on_select(move |profile, window, cx| {
            let profile = *profile;
            entity.update(cx, |this, cx| this.set_theme_profile(profile, window, cx));
        })
        .into_any_element()
}

fn render_settings_text_input(
    input: &Entity<InputState>,
    cx: &mut Context<TopApp>,
) -> gpui::AnyElement {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_row()
        .child(
            Input::new(input)
                .cleanable(true)
                .scaled(Size::Small, cx)
                .flex_1()
                .min_w_0(),
        )
        .into_any_element()
}

#[expect(
    clippy::too_many_lines,
    reason = "settings screen remains a single legacy render function during ADR 0023 migration"
)]
fn render_settings(app: &mut TopApp, cx: &mut Context<TopApp>) -> gpui::AnyElement {
    app.reload_cached();

    let endpoint_input = app.endpoint_input.clone();
    let music_dir_input = app.music_dir_input.clone();
    let flac_path_input = app.flac_path_input.clone();
    let status = app.settings_status.clone();
    let status_color = if status.starts_with("Error:") {
        color(cx, SemanticColor::Danger)
    } else {
        color(cx, SemanticColor::TertiaryLabel)
    };
    let settings_column_width = layout::scaled_dimension(layout::SETTINGS_COLUMN_WIDTH, cx);

    let cached_count = app
        .cached_tree
        .artists
        .iter()
        .flat_map(|a| &a.albums)
        .flat_map(|a| &a.tracks)
        .count();
    let cached_is_empty = cached_count == 0;

    div()
        .id("settings-scroll")
        .flex_1()
        .min_h_0()
        .min_w_0()
        .bg(color(cx, SemanticColor::SystemBackground))
        .p(Spacing::XL.scaled(cx))
        .overflow_y_scroll()
        .child(
            div()
                .w(settings_column_width)
                .max_w(relative(1.0))
                .flex()
                .flex_col()
                .gap(Spacing::LG.scaled(cx))
                .child(
                    div()
                        .text_size(FontSize::Title2.scaled(cx))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child("Settings"),
                )
                .child(
                    div()
                        .text_size(FontSize::Caption.scaled(cx))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(color(cx, SemanticColor::TertiaryLabel))
                        .child("MusicIndex endpoint"),
                )
                .child(render_settings_text_input(&endpoint_input, cx))
                .child(
                    div()
                        .text_size(FontSize::Caption.scaled(cx))
                        .text_color(color(cx, SemanticColor::TertiaryLabel))
                        .child("Use api.musicindex.org or a full http/https URL."),
                )
                .child(
                    div()
                        .text_size(FontSize::Caption.scaled(cx))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(color(cx, SemanticColor::TertiaryLabel))
                        .child("Music directory"),
                )
                .child(render_settings_text_input(&music_dir_input, cx))
                .child(
                    div()
                        .text_size(FontSize::Caption.scaled(cx))
                        .text_color(color(cx, SemanticColor::TertiaryLabel))
                        .child("Downloads are organized under an artists subfolder."),
                )
                .child(
                    div()
                        .text_size(FontSize::Caption.scaled(cx))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(color(cx, SemanticColor::TertiaryLabel))
                        .child("flac binary (optional)"),
                )
                .child(render_settings_text_input(&flac_path_input, cx))
                .child(
                    div()
                        .text_size(FontSize::Caption.scaled(cx))
                        .text_color(color(cx, SemanticColor::TertiaryLabel))
                        .child(
                            "Used to silently upgrade WAV downloads to FLAC. Leave blank to resolve `flac` via $PATH.",
                        ),
                )
                .child(
                    div()
                        .text_size(FontSize::Caption.scaled(cx))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(color(cx, SemanticColor::TertiaryLabel))
                        .child("UI scale"),
                )
                .child(render_ui_scale_picker(app.ui_scale, cx))
                .child(
                    div()
                        .text_size(FontSize::Caption.scaled(cx))
                        .text_color(color(cx, SemanticColor::TertiaryLabel))
                        .child(
                            "Scales every dimension token. Applies immediately; click Save to persist.",
                        ),
                )
                .child(
                    div()
                        .text_size(FontSize::Caption.scaled(cx))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(color(cx, SemanticColor::TertiaryLabel))
                        .child("Theme"),
                )
                .child(render_theme_profile_picker(app.theme_profile, cx))
                .child(
                    div()
                        .text_size(FontSize::Caption.scaled(cx))
                        .text_color(color(cx, SemanticColor::TertiaryLabel))
                        .child("Applies immediately. Click Save to persist."),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(Spacing::SM.scaled(cx))
                        .child(
                            UiButton::styled("settings-save", ControlStyle::Primary)
                                .label("Save")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.save_settings(window, cx);
                                })),
                        )
                        .child(
                            UiButton::styled("settings-default", ControlStyle::Ghost)
                                .label("Use Defaults")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.endpoint_input.update(cx, |input, cx| {
                                        input.set_value(crate::api::DEFAULT_BASE_URL, window, cx);
                                    });
                                    this.flac_path_input.update(cx, |input, cx| {
                                        input.set_value("", window, cx);
                                    });
                                    this.set_ui_scale(
                                        crate::config::UiScale::Medium,
                                        window,
                                        cx,
                                    );
                                    this.set_theme_profile(ThemeProfile::default(), window, cx);
                                    match config::default_music_dir() {
                                        Ok(default_music_dir) => {
                                            this.music_dir_input.update(cx, |input, cx| {
                                                input.set_value(
                                                    default_music_dir.display().to_string(),
                                                    window,
                                                    cx,
                                                );
                                            });
                                        }
                                        Err(error) => {
                                            this.settings_status = format!("Error: {error:#}");
                                            cx.notify();
                                            return;
                                        }
                                    }
                                    this.save_settings(window, cx);
                                })),
                        ),
                )
                .when(!status.is_empty(), |el| {
                    el.child(
                        div()
                            .text_xs()
                            .text_color(status_color)
                            .child(SharedString::from(status)),
                    )
                })
                .child(div().border_t_1().border_color(color(cx, SemanticColor::Separator)))
                .child(
                    div()
                        .text_size(FontSize::Caption.scaled(cx))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(color(cx, SemanticColor::TertiaryLabel))
                        .child(format!("Cached files ({cached_count})")),
                )
                .when(!cached_is_empty, |el| {
                    let cached_tree = &app.cached_tree;
                    let mut cached_items = Vec::new();
                    for artist in &cached_tree.artists {
                        cached_items.push(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(color(cx, SemanticColor::Label))
                                .child(SharedString::from(artist.name.clone()))
                                .into_any_element()
                        );
                        for album in &artist.albums {
                            for track in &album.tracks {
                                let title = LibraryTrackRowVm::new(track, None).compact_title();
                                let path_clone = track.local_path.clone().unwrap_or_default();
                                cached_items.push(
                                    div()
                                        .pl(Spacing::MD.scaled(cx))
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .gap(Spacing::XS.scaled(cx))
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_xs()
                                                .text_color(color(cx, SemanticColor::Label))
                                                .child(SharedString::from(title.clone()))
                                        )
                                        .child(
                                            UiButton::styled(
                                                SharedString::from(format!("del-cached-{}", track.id)),
                                                ControlStyle::Destructive,
                                            )
                                                .label("Delete")
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.delete_cached_file(path_clone.clone(), cx);
                                                }))
                                        )
                                        .into_any_element()
                                );
                            }
                        }
                    }
                    el.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(Spacing::XXS.scaled(cx))
                            .children(cached_items)
                    )
                })
                .when(!cached_is_empty, |el| {
                    el.child(
                        div().pt(Spacing::SM.scaled(cx)).child(
                            UiButton::styled("delete-all-cached-settings", ControlStyle::Destructive)
                                .label("Delete All Cached")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.delete_all_cached(cx);
                                })),
                        )
                    )
                })
                .when(cached_is_empty, |el| {
                    el.child(
                        div()
                            .text_xs()
                            .text_color(color(cx, SemanticColor::TertiaryLabel))
                            .child("No cached files"),
                    )
                }),
        )
        .into_any_element()
}
