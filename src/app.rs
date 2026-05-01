#![warn(clippy::pedantic)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{
    div, img, prelude::*, size, Application, Bounds, Context, Entity, Image, ImageFormat,
    KeyDownEvent, ObjectFit, Render, SharedString, Styled, Window, WindowBounds, WindowOptions,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputState};
use gpui_component::{Root, Size};
use rusqlite::Connection;

use crate::application::{ApplicationEvent, ApplicationEventSubscriber, ApplicationServices};
use crate::config;
use crate::db;
use crate::library::{build_tree, cleanup_empty_parents, LibraryApp, LibraryAppEvent};
use crate::library_service;
use crate::media::ImageCache;
use crate::playback;
use crate::playback_driver::ConfiguredPlaybackDriver;
use crate::playback_owner::{PlaybackOwner, PollOutcome};
use crate::presentation::{GpuiEventBridge, PresentationEventBridge};
use crate::search::{SearchApp, SearchAppEvent};
use crate::ui::composites::{NowPlayingBar, NowPlayingData, NowPlayingState};
use crate::ui::sizable_bridge::SizableScaled;
use crate::ui::theme::layout;
use crate::ui::tokens::{color, FontSize, Radius, SemanticColor, Size as TokenSize, Spacing};
use crate::view_models::library::LibraryTree;

// ---------------------------------------------------------------------------
// Color helpers (same palette)
// ---------------------------------------------------------------------------

fn app_logo() -> Arc<Image> {
    Arc::new(Image::from_bytes(
        ImageFormat::Png,
        include_bytes!("assets/music_network_logo.png").to_vec(),
    ))
}

// ---------------------------------------------------------------------------
// AppTab
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppTab {
    Library,
    Discover,
    Settings,
}

// ---------------------------------------------------------------------------
// TopApp
// ---------------------------------------------------------------------------

pub struct TopApp {
    tab: AppTab,
    search: Entity<SearchApp>,
    library: Entity<LibraryApp>,
    endpoint_input: Entity<InputState>,
    music_dir_input: Entity<InputState>,
    flac_path_input: Entity<InputState>,
    ui_scale: crate::config::UiScale,
    cfg_path: PathBuf,
    settings_status: String,
    library_tab_focus: gpui::FocusHandle,
    discover_tab_focus: gpui::FocusHandle,
    settings_tab_focus: gpui::FocusHandle,
    _search_sub: gpui::Subscription,
    _library_sub: gpui::Subscription,
    playback_owner: PlaybackOwner<ConfiguredPlaybackDriver>,
    conn: Arc<Mutex<Connection>>,
    cached_tree: LibraryTree,
    application_event_bridge: Arc<GpuiEventBridge>,
}

#[derive(Clone, Debug, Default)]
struct PlaybackBarState {
    active: bool,
    paused: bool,
    title: String,
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
    fn new(
        conn: Arc<Mutex<Connection>>,
        image_cache: Arc<ImageCache>,
        cfg_path: PathBuf,
        musicindex_endpoint: String,
        music_dir: PathBuf,
        flac_path: Option<PathBuf>,
        ui_scale: crate::config::UiScale,
        playback_owner: PlaybackOwner<ConfiguredPlaybackDriver>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search_conn = Arc::clone(&conn);
        let search_cache = Arc::clone(&image_cache);
        let library_cache = Arc::clone(&image_cache);
        let search_endpoint = musicindex_endpoint.clone();
        let application_services = Arc::new(
            ApplicationServices::local_without_downloads()
                .expect("application services are fully wired"),
        );
        let application_event_bridge = Arc::new(GpuiEventBridge::new());
        let application_event_subscriber: Arc<dyn ApplicationEventSubscriber> =
            application_event_bridge.clone();
        application_services
            .event_bus()
            .subscribe(application_event_subscriber);
        let search_services = Arc::clone(&application_services);
        let library_services = Arc::clone(&application_services);
        let search = cx.new(|cx| {
            SearchApp::new(
                search_conn,
                search_cache,
                search_endpoint,
                search_services,
                window,
                cx,
            )
        });
        let library = cx.new(|cx| {
            LibraryApp::new(
                conn.clone(),
                library_cache,
                musicindex_endpoint.clone(),
                library_services,
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
            endpoint_input,
            music_dir_input,
            flac_path_input,
            ui_scale,
            cfg_path,
            settings_status: String::new(),
            library_tab_focus: cx.focus_handle(),
            discover_tab_focus: cx.focus_handle(),
            settings_tab_focus: cx.focus_handle(),
            _search_sub: search_sub,
            _library_sub: library_sub,
            playback_owner,
            conn,
            cached_tree: LibraryTree::default(),
            application_event_bridge,
        }
    }

    fn maybe_start_playback_polling(&mut self, cx: &mut Context<Self>) {
        if !self.playback_owner.driver().is_live_driver() {
            return;
        }
        {
            let conn = self.conn.lock().expect("lock db");
            if let Err(error) = self.playback_owner.load_current_session(&conn) {
                self.settings_status = format!("Playback error: {error:#}");
            }
        }
        cx.spawn(
            async move |this: gpui::WeakEntity<TopApp>, cx: &mut gpui::AsyncApp| loop {
                cx.background_executor()
                    .timer(Duration::from_millis(1_000))
                    .await;
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

    fn poll_playback_owner(&mut self) {
        let conn = self.conn.lock().expect("lock db");
        match self.playback_owner.poll(&conn) {
            Ok(PollOutcome::NoSession | PollOutcome::Reconciled(None)) => {}
            Ok(PollOutcome::Reconciled(Some(_)) | PollOutcome::Advanced(_)) => {
                self.settings_status.clear();
            }
            Err(error) => {
                self.settings_status = format!("Playback error: {error:#}");
            }
        }
    }

    fn play_playlist_at(
        &mut self,
        playlist_id: i64,
        playlist_position: i64,
        cx: &mut Context<Self>,
    ) {
        let conn = self.conn.lock().expect("lock db");
        match self
            .playback_owner
            .play_playlist_at(&conn, playlist_id, playlist_position)
        {
            Ok(update) => {
                self.settings_status = format!("Playing {}", update.title);
            }
            Err(error) => {
                self.settings_status = format!("Playback error: {error:#}");
            }
        }
        cx.notify();
    }

    fn skip_playback_next(&mut self, cx: &mut Context<Self>) {
        let conn = self.conn.lock().expect("lock db");
        match self.playback_owner.skip_next(&conn) {
            Ok(update) => {
                self.settings_status = format!("Playing {}", update.title);
            }
            Err(error) => {
                self.settings_status = format!("Playback error: {error:#}");
            }
        }
        cx.notify();
    }

    fn skip_playback_previous(&mut self, cx: &mut Context<Self>) {
        let conn = self.conn.lock().expect("lock db");
        match self.playback_owner.skip_previous(&conn) {
            Ok(update) => {
                self.settings_status = format!("Playing {}", update.title);
            }
            Err(error) => {
                self.settings_status = format!("Playback error: {error:#}");
            }
        }
        cx.notify();
    }

    fn set_playback_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        let conn = self.conn.lock().expect("lock db");
        match self.playback_owner.pause(&conn, paused) {
            Ok(update) => {
                let verb = if paused { "Paused" } else { "Playing" };
                self.settings_status = format!("{verb} {}", update.title);
            }
            Err(error) => {
                self.settings_status = format!("Playback error: {error:#}");
            }
        }
        cx.notify();
    }

    fn stop_playback_owner(&mut self, cx: &mut Context<Self>) {
        let conn = self.conn.lock().expect("lock db");
        match self.playback_owner.stop(&conn) {
            Ok(_) => {
                self.settings_status = "Playback stopped".to_string();
            }
            Err(error) => {
                self.settings_status = format!("Playback error: {error:#}");
            }
        }
        cx.notify();
    }

    fn playback_bar_state(&self) -> PlaybackBarState {
        let conn = self.conn.lock().expect("lock db");
        let Ok(Some(session)) = db::playback_session(&conn, playback::DEFAULT_SESSION_ID) else {
            return PlaybackBarState::default();
        };
        if session.state == "stopped" {
            return PlaybackBarState::default();
        }
        let title = playback::now_playing_update(&conn, playback::DEFAULT_SESSION_ID)
            .ok()
            .flatten()
            .map_or_else(|| "Current track".to_string(), |update| update.title);
        PlaybackBarState {
            active: true,
            paused: session.state == "paused",
            title,
        }
    }

    fn set_ui_scale(&mut self, scale: crate::config::UiScale, cx: &mut Context<Self>) {
        if self.ui_scale == scale {
            return;
        }
        self.ui_scale = scale;
        cx.notify();
    }

    fn save_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let endpoint = self.endpoint_input.read(cx).value().to_string();
        let music_dir = self.music_dir_input.read(cx).value().to_string();
        let flac_path = self.flac_path_input.read(cx).value().to_string();
        let ui_scale = self.ui_scale;
        match config::save_app_settings(&self.cfg_path, &endpoint, &music_dir, &flac_path, ui_scale)
        {
            Ok((normalized_endpoint, normalized_music_dir, normalized_flac_path, saved_scale)) => {
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
                // Apply scale change immediately so the UI reflects it.
                crate::ui::theme_bridge::install_theme(
                    crate::ui::tokens::Appearance::Dark,
                    saved_scale.into(),
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
        match library_service::cached_tracks(&conn) {
            Ok(rows) => {
                self.cached_tree = build_tree(&rows, &conn);
            }
            Err(err) => {
                self.settings_status = format!("Error loading cached: {err:#}");
            }
        }
    }

    fn defer_application_event_drain(&mut self, window: &Window, cx: &mut Context<Self>) {
        if self.application_event_bridge.pending_event_count() == 0 {
            return;
        }
        cx.defer_in(window, |this, _window, cx| {
            this.drain_application_events(cx);
        });
    }

    fn drain_application_events(&mut self, cx: &mut Context<Self>) {
        let events = self.application_event_bridge.drain_events();
        if events.is_empty() {
            return;
        }
        if events.iter().any(affects_library_surfaces) {
            self.reload_cached();
            self.library.update(cx, LibraryApp::refresh);
            self.search.update(cx, SearchApp::refresh_application_state);
        }
        cx.notify();
    }

    fn delete_cached_file(&mut self, path: &str) {
        if let Err(err) = std::fs::remove_file(path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                self.settings_status = format!("Error deleting file: {err:#}");
                return;
            }
        }
        cleanup_empty_parents(std::path::Path::new(path));
        let conn = self.conn.lock().expect("lock db");
        if let Err(err) = library_service::delete_local_file(&conn, path) {
            self.settings_status = format!("Error: {err:#}");
            return;
        }
        drop(conn);
        self.reload_cached();
    }

    fn delete_all_cached(&mut self) {
        let paths: Vec<String> = self
            .cached_tree
            .artists
            .iter()
            .flat_map(|a| &a.albums)
            .flat_map(|a| &a.tracks)
            .filter_map(|t| t.local_path.clone())
            .collect();
        for path in &paths {
            if let Err(err) = std::fs::remove_file(path) {
                if err.kind() != std::io::ErrorKind::NotFound {
                    self.settings_status = format!("Error deleting {path}: {err:#}");
                    return;
                }
            }
            cleanup_empty_parents(std::path::Path::new(path));
        }
        let conn = self.conn.lock().expect("lock db");
        for path in &paths {
            if let Err(err) = library_service::delete_local_file(&conn, path) {
                self.settings_status = format!("Error: {err:#}");
                return;
            }
        }
        drop(conn);
        self.reload_cached();
    }
}

fn affects_library_surfaces(event: &ApplicationEvent) -> bool {
    matches!(
        event,
        ApplicationEvent::Library(_)
            | ApplicationEvent::Playlist(_)
            | ApplicationEvent::Feed(_)
            | ApplicationEvent::Download(_)
    )
}

impl Render for TopApp {
    #[expect(
        clippy::too_many_lines,
        reason = "top-level render consolidates many UI sections"
    )]
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.defer_application_event_drain(window, cx);
        let playback_bar = build_playback_bar(self, cx);
        let bg_canvas = color(cx, SemanticColor::SystemBackground);
        let text_primary = color(cx, SemanticColor::Label);
        let bg_surface = color(cx, SemanticColor::SecondarySystemBackground);
        let border_subtle = color(cx, SemanticColor::Separator);
        let accent_color = color(cx, SemanticColor::Accent);
        let spacing_xs = Spacing::XS.scaled(cx);
        let spacing_sm = Spacing::SM.scaled(cx);
        let spacing_md = Spacing::MD.scaled(cx);
        let tab_bar_height = TokenSize::RowLg.px();
        div()
            .size_full()
            .bg(bg_canvas)
            .text_color(text_primary)
            .text_sm()
            .flex()
            .flex_col()
            .overflow_hidden()
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                let modifiers = event.keystroke.modifiers;
                let key = event.keystroke.key.as_str();

                if modifiers.platform {
                    match key {
                        "1" => {
                            this.tab = AppTab::Library;
                            cx.notify();
                        }
                        "2" => {
                            this.tab = AppTab::Discover;
                            cx.notify();
                        }
                        "3" => {
                            this.tab = AppTab::Settings;
                            cx.notify();
                        }
                        "f" => match this.tab {
                            AppTab::Library => {
                                this.library
                                    .update(cx, |lib, cx| lib.focus_search(window, cx));
                            }
                            AppTab::Discover => {
                                this.search
                                    .update(cx, |search, cx| search.focus_search(window, cx));
                            }
                            AppTab::Settings => {}
                        },
                        "r" => {
                            if this.tab == AppTab::Library {
                                this.library.update(cx, LibraryApp::refresh);
                            }
                        }
                        _ => {}
                    }
                } else {
                    match key {
                        "escape" => match this.tab {
                            AppTab::Library => {
                                this.library.update(cx, LibraryApp::pop_inspector);
                            }
                            AppTab::Discover => {
                                this.search.update(cx, SearchApp::pop_inspector);
                            }
                            AppTab::Settings => {}
                        },
                        "up" => match this.tab {
                            AppTab::Library => this.library.update(cx, LibraryApp::move_up),
                            AppTab::Discover => this.search.update(cx, SearchApp::move_up),
                            AppTab::Settings => {}
                        },
                        "down" => match this.tab {
                            AppTab::Library => this.library.update(cx, LibraryApp::move_down),
                            AppTab::Discover => this.search.update(cx, SearchApp::move_down),
                            AppTab::Settings => {}
                        },
                        "enter" => match this.tab {
                            AppTab::Library => this.library.update(cx, LibraryApp::confirm),
                            AppTab::Discover => this.search.update(cx, SearchApp::confirm),
                            AppTab::Settings => {}
                        },
                        _ => {}
                    }
                }
            }))
            // Top-level tab bar
            .child(
                div()
                    .h(tab_bar_height)
                    .flex_shrink_0()
                    .bg(bg_surface)
                    .border_b_1()
                    .border_color(border_subtle)
                    .px(spacing_md)
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(spacing_xs)
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(spacing_sm)
                            .mr(spacing_md)
                            .child(
                                div()
                                    .w(layout::APP_ICON_SIZE)
                                    .h(layout::APP_ICON_SIZE)
                                    .rounded(spacing_xs)
                                    .overflow_hidden()
                                    .flex_shrink_0()
                                    .child(
                                        img(app_logo())
                                            .w(layout::APP_ICON_SIZE)
                                            .h(layout::APP_ICON_SIZE)
                                            .object_fit(ObjectFit::Cover),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(FontSize::Headline.scaled(cx))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(accent_color)
                                    .child("V4V Music Manager"),
                            ),
                    )
                    .child(render_app_tab(
                        "Library",
                        AppTab::Library,
                        self.tab,
                        &self.library_tab_focus,
                        window,
                        cx,
                    ))
                    .child(render_app_tab(
                        "Discover",
                        AppTab::Discover,
                        self.tab,
                        &self.discover_tab_focus,
                        window,
                        cx,
                    ))
                    .child(render_app_tab(
                        "Settings",
                        AppTab::Settings,
                        self.tab,
                        &self.settings_tab_focus,
                        window,
                        cx,
                    ))
                    .child(div().flex_1())
                    .child(playback_bar),
            )
            // Active tab content
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .when(self.tab == AppTab::Library, |el| {
                        el.child(self.library.clone())
                    })
                    .when(self.tab == AppTab::Discover, |el| {
                        el.child(self.search.clone())
                    })
                    .when(self.tab == AppTab::Settings, |el| {
                        el.child(render_settings(self, cx))
                    }),
            )
    }
}

fn render_ui_scale_picker(
    current: crate::config::UiScale,
    cx: &mut Context<TopApp>,
) -> gpui::AnyElement {
    use crate::config::UiScale;
    use crate::ui::composites::{Segment, SegmentedControl};

    let segments = [
        Segment::new("ui-scale-xs", UiScale::XSmall, "XS"),
        Segment::new("ui-scale-s", UiScale::Small, "S"),
        Segment::new("ui-scale-m", UiScale::Medium, "M"),
        Segment::new("ui-scale-l", UiScale::Large, "L"),
        Segment::new("ui-scale-xl", UiScale::XLarge, "XL"),
    ];

    let entity = cx.entity();
    SegmentedControl::new(current)
        .segments(segments)
        .on_select(move |scale, _window, cx| {
            let scale = *scale;
            entity.update(cx, |this, cx| this.set_ui_scale(scale, cx));
        })
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
        .size_full()
        .bg(color(cx, SemanticColor::SystemBackground))
        .p(Spacing::XL.scaled(cx))
        .overflow_y_scroll()
        .child(
            div()
                .max_w(layout::SETTINGS_COLUMN_WIDTH)
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
                .child(
                    Input::new(&endpoint_input)
                        .cleanable(true)
                        .scaled(Size::Small, cx),
                )
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
                .child(
                    Input::new(&music_dir_input)
                        .cleanable(true)
                        .scaled(Size::Small, cx),
                )
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
                .child(
                    Input::new(&flac_path_input)
                        .cleanable(true)
                        .scaled(Size::Small, cx),
                )
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
                            "Scales every dimension token (spacing, radius, font, sizes). Click Save to persist.",
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(Spacing::SM.scaled(cx))
                        .child(
                            Button::new("settings-save")
                                .label("Save")
                                .primary()
                                .scaled(Size::Small, cx)
                                .text_color(color(cx, SemanticColor::OnAccent))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.save_settings(window, cx);
                                })),
                        )
                        .child(
                            Button::new("settings-default")
                                .label("Use Defaults")
                                .ghost()
                                .scaled(Size::Small, cx)
                                .text_color(color(cx, SemanticColor::OnAccent))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.endpoint_input.update(cx, |input, cx| {
                                        input.set_value(crate::api::DEFAULT_BASE_URL, window, cx);
                                    });
                                    this.flac_path_input.update(cx, |input, cx| {
                                        input.set_value("", window, cx);
                                    });
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
                                let title = track.track_title.as_deref().unwrap_or("[untitled]");
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
                                                .child(SharedString::from(title.to_string()))
                                        )
                                        .child(
                                            Button::new(SharedString::from(format!("del-cached-{}", track.id)))
                                                .label("Delete")
                                                .danger()
                                                .scaled(Size::XSmall, cx)
                                                .text_color(color(cx, SemanticColor::OnAccent))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.delete_cached_file(&path_clone);
                                                    cx.notify();
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
                            Button::new("delete-all-cached-settings")
                                .label("Delete All Cached")
                                .danger()
                                .scaled(Size::XSmall, cx)
                                .text_color(color(cx, SemanticColor::OnAccent))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.delete_all_cached();
                                    cx.notify();
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

fn build_playback_bar(app: &TopApp, cx: &mut Context<TopApp>) -> NowPlayingBar {
    let state = app.playback_bar_state();
    let np_state = if !state.active {
        None
    } else if state.paused {
        Some(NowPlayingState::Paused)
    } else {
        Some(NowPlayingState::Playing)
    };
    NowPlayingBar::new()
        .data(NowPlayingData {
            title: state.active.then_some(state.title),
            artist: None,
            state: np_state,
            thumbnail: None,
        })
        .on_prev(cx.listener(|this, _, _, cx| this.skip_playback_previous(cx)))
        .on_play_pause(cx.listener(|this, _, _, cx| {
            let toggle = !this.playback_bar_state().paused;
            this.set_playback_paused(toggle, cx);
        }))
        .on_next(cx.listener(|this, _, _, cx| this.skip_playback_next(cx)))
        .on_stop(cx.listener(|this, _, _, cx| this.stop_playback_owner(cx)))
}

fn render_app_tab(
    label: &'static str,
    tab: AppTab,
    active: AppTab,
    focus_handle: &gpui::FocusHandle,
    window: &gpui::Window,
    cx: &mut Context<TopApp>,
) -> gpui::AnyElement {
    let is_active = tab == active;
    let is_focused = focus_handle.is_focused(window);
    // Pre-compute colors for use in closures
    let accent_color = color(cx, SemanticColor::Accent);
    let text_on_accent_color = color(cx, SemanticColor::OnAccent);
    let text_secondary_color = color(cx, SemanticColor::SecondaryLabel);
    let bg_surface_hi_color = color(cx, SemanticColor::TertiarySystemBackground);
    let focus_ring_color = color(cx, SemanticColor::Focus);
    let spacing_md = Spacing::MD.scaled(cx);
    let hit_target_min = layout::HIT_TARGET_MIN;
    let radius_lg = Radius::LG.scaled(cx);

    div()
        .id(SharedString::from(format!("app-tab-{label}")))
        .track_focus(focus_handle)
        .on_click(cx.listener(move |this, _, _, cx| {
            this.tab = tab;
            if tab == AppTab::Library {
                this.library.update(cx, LibraryApp::refresh);
            }
            cx.notify();
        }))
        .px(spacing_md)
        .min_h(hit_target_min)
        .flex()
        .items_center()
        .rounded(radius_lg)
        .when(is_active, |el| {
            el.bg(accent_color).text_color(text_on_accent_color)
        })
        .when(!is_active, |el| {
            el.text_color(text_secondary_color)
                .hover(move |s| s.bg(bg_surface_hi_color))
        })
        .when(is_focused, |el| {
            el.border_2().border_color(focus_ring_color)
        })
        .child(div().text_size(FontSize::Body.scaled(cx)).child(label))
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Run the desktop GPUI application.
///
/// # Panics
///
/// Panics if the config path, config file, `MusicIndex` endpoint, database,
/// playback driver, or initial window cannot be initialized.
#[expect(
    clippy::too_many_lines,
    reason = "application bootstrap owns one-time GPUI setup and resource wiring"
)]
pub fn run_app() {
    let app = Application::new().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
        // Pre-config: install with default scale so the loading window is
        // themed; we re-install with the user's preference once config is
        // loaded a few lines below.
        crate::ui::theme_bridge::install_theme(
            crate::ui::tokens::Appearance::Dark,
            crate::ui::tokens::ScaleFactor::Medium,
            cx,
        );

        // Load config + open DB
        let cfg_path = config::config_path().expect("config path");
        let cfg = config::load_config(&cfg_path).expect("load config");
        let musicindex_endpoint =
            config::load_musicindex_endpoint(&cfg_path).expect("load MusicIndex endpoint");
        config::ensure_dirs(&cfg).expect("ensure dirs");

        // Re-apply theme now that config has provided the user's UI scale.
        crate::ui::theme_bridge::install_theme(
            crate::ui::tokens::Appearance::Dark,
            cfg.ui_scale.into(),
            cx,
        );
        let conn = db::open_db(&cfg).expect("open db");
        let conn = Arc::new(Mutex::new(conn));
        let playback_driver = ConfiguredPlaybackDriver::from_config(&cfg.playback)
            .expect("configure playback driver");
        let playback_owner = PlaybackOwner::new(playback_driver, playback::DEFAULT_SESSION_ID);

        let thumbnail_cache_dir = cfg_path
            .parent()
            .expect("config path has parent")
            .join("thumbnail-cache");
        let http = reqwest::blocking::Client::new();
        let image_cache = ImageCache::new(http, thumbnail_cache_dir);

        let window_handle = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(layout::WINDOW_WIDTH, layout::WINDOW_HEIGHT),
                        cx,
                    ))),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|cx| {
                        let mut app = TopApp::new(
                            conn,
                            image_cache,
                            cfg_path,
                            musicindex_endpoint,
                            cfg.music_dir,
                            cfg.flac_path,
                            cfg.ui_scale,
                            playback_owner,
                            window,
                            cx,
                        );
                        app.maybe_start_playback_polling(cx);
                        app
                    });
                    let root = cx.new(|cx| Root::new(view, window, cx));
                    window.refresh();
                    root
                },
            )
            .expect("failed to open window");
        let window_handle = gpui::AnyWindowHandle::from(window_handle);
        window_handle
            .update(cx, |_, window, cx| {
                window.activate_window();
                window.refresh();
                cx.refresh_windows();
            })
            .expect("activate initial window");
        cx.activate(true);
        cx.refresh_windows();
        cx.defer(move |cx| {
            let _ = window_handle.update(cx, |_, window, cx| {
                window.activate_window();
                window.refresh();
                cx.refresh_windows();
            });
            cx.activate(true);
            cx.refresh_windows();
        });
        cx.spawn(async move |cx: &mut gpui::AsyncApp| {
            cx.background_executor()
                .timer(Duration::from_millis(16))
                .await;
            let _ = cx.update(|cx| {
                let _ = window_handle.update(cx, |_, window, cx| {
                    window.activate_window();
                    window.refresh();
                    cx.refresh_windows();
                });
                cx.activate(true);
                cx.refresh_windows();
            });
            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;
            let _ = cx.refresh();
        })
        .detach();
    });
}
