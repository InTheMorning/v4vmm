use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{
    div, img, prelude::*, px, rgb, size, Application, Bounds, Context, Entity, Image, ImageFormat,
    KeyDownEvent, ObjectFit, Render, SharedString, Styled, Window, WindowBounds, WindowOptions,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputState};
use gpui_component::{Root, Sizable, Size};
use rusqlite::Connection;

use crate::config;
use crate::db;
use crate::library::{build_tree, cleanup_empty_parents, LibraryApp, LibraryAppEvent, LibraryTree};
use crate::library_service;
use crate::media::ImageCache;
use crate::playback;
use crate::playback_driver::ConfiguredPlaybackDriver;
use crate::playback_owner::{PlaybackOwner, PollOutcome};
use crate::search::{SearchApp, SearchAppEvent};
use crate::ui::theme::color;
use crate::ui::theme::layout;
#[allow(unused_imports)]
use crate::ui::theme::radius;
use crate::ui::theme::spacing;
use crate::ui::theme::typography;

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
}

impl TopApp {
    #[expect(
        clippy::too_many_arguments,
        reason = "top-level app bootstrap still wires shared state explicitly"
    )]
    fn new(
        conn: Arc<Mutex<Connection>>,
        image_cache: Arc<ImageCache>,
        cfg_path: PathBuf,
        musicindex_endpoint: String,
        music_dir: PathBuf,
        flac_path: Option<PathBuf>,
        playback_owner: PlaybackOwner<ConfiguredPlaybackDriver>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search_conn = Arc::clone(&conn);
        let search_cache = Arc::clone(&image_cache);
        let library_cache = Arc::clone(&image_cache);
        let search_endpoint = musicindex_endpoint.clone();
        let search =
            cx.new(|cx| SearchApp::new(search_conn, search_cache, search_endpoint, window, cx));
        let library = cx.new(|cx| {
            LibraryApp::new(
                conn.clone(),
                library_cache,
                musicindex_endpoint.clone(),
                window,
                cx,
            )
        });
        let library_for_sub = library.clone();
        let search_sub = cx.subscribe(
            &search,
            move |_this: &mut Self, _search, event: &SearchAppEvent, cx| match event {
                SearchAppEvent::LibraryMutated => {
                    library_for_sub.update(cx, |lib, cx| lib.refresh(cx));
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

    fn save_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let endpoint = self.endpoint_input.read(cx).value().to_string();
        let music_dir = self.music_dir_input.read(cx).value().to_string();
        let flac_path = self.flac_path_input.read(cx).value().to_string();
        match config::save_app_settings(&self.cfg_path, &endpoint, &music_dir, &flac_path) {
            Ok((normalized_endpoint, normalized_music_dir, normalized_flac_path)) => {
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

    fn delete_cached_file(&mut self, path: String) {
        if let Err(err) = std::fs::remove_file(&path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                self.settings_status = format!("Error deleting file: {err:#}");
                return;
            }
        }
        cleanup_empty_parents(std::path::Path::new(&path));
        let conn = self.conn.lock().expect("lock db");
        if let Err(err) = library_service::delete_local_file(&conn, &path) {
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

impl Render for TopApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(color::bg_canvas())
            .text_color(color::text_primary())
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
                            AppTab::Library => this
                                .library
                                .update(cx, |lib, cx| lib.focus_search(window, cx)),
                            AppTab::Discover => {
                                this.search.update(cx, |s, cx| s.focus_search(window, cx))
                            }
                            _ => {}
                        },
                        "r" => {
                            if this.tab == AppTab::Library {
                                this.library.update(cx, |lib, cx| lib.refresh(cx));
                            }
                        }
                        _ => {}
                    }
                } else {
                    match key {
                        "escape" => match this.tab {
                            AppTab::Library => {
                                this.library.update(cx, |lib, cx| lib.pop_inspector(cx))
                            }
                            AppTab::Discover => this.search.update(cx, |s, cx| s.pop_inspector(cx)),
                            _ => {}
                        },
                        "up" => match this.tab {
                            AppTab::Library => this.library.update(cx, |lib, cx| lib.move_up(cx)),
                            AppTab::Discover => this.search.update(cx, |s, cx| s.move_up(cx)),
                            _ => {}
                        },
                        "down" => match this.tab {
                            AppTab::Library => this.library.update(cx, |lib, cx| lib.move_down(cx)),
                            AppTab::Discover => this.search.update(cx, |s, cx| s.move_down(cx)),
                            _ => {}
                        },
                        "enter" => match this.tab {
                            AppTab::Library => this.library.update(cx, |lib, cx| lib.confirm(cx)),
                            AppTab::Discover => this.search.update(cx, |s, cx| s.confirm(cx)),
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }))
            // Top-level tab bar
            .child(
                div()
                    .h(layout::TAB_BAR_HEIGHT)
                    .flex_shrink_0()
                    .bg(color::bg_surface())
                    .border_b_1()
                    .border_color(color::border_subtle())
                    .px(spacing::MD)
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(spacing::XS)
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(spacing::SM)
                            .mr(spacing::MD)
                            .child(
                                div()
                                    .w(px(26.0))
                                    .h(px(26.0))
                                    .rounded(spacing::XS)
                                    .overflow_hidden()
                                    .flex_shrink_0()
                                    .child(
                                        img(app_logo())
                                            .w(px(26.0))
                                            .h(px(26.0))
                                            .object_fit(ObjectFit::Cover),
                                    ),
                            )
                            .child(
                                typography::type_headline(div())
                                    .text_color(color::accent())
                                    .child("V4V Music Manager"),
                            ),
                    )
                    .child(render_app_tab(
                        "Library",
                        AppTab::Library,
                        self.tab,
                        &self.library_tab_focus,
                        _window,
                        cx,
                    ))
                    .child(render_app_tab(
                        "Discover",
                        AppTab::Discover,
                        self.tab,
                        &self.discover_tab_focus,
                        _window,
                        cx,
                    ))
                    .child(render_app_tab(
                        "Settings",
                        AppTab::Settings,
                        self.tab,
                        &self.settings_tab_focus,
                        _window,
                        cx,
                    )),
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

fn render_settings(app: &mut TopApp, cx: &mut Context<TopApp>) -> gpui::AnyElement {
    app.reload_cached();

    let endpoint_input = app.endpoint_input.clone();
    let music_dir_input = app.music_dir_input.clone();
    let flac_path_input = app.flac_path_input.clone();
    let status = app.settings_status.clone();
    let status_color = if status.starts_with("Error:") {
        color::status_danger()
    } else {
        color::text_muted()
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
        .bg(color::bg_canvas())
        .p(spacing::XL)
        .overflow_y_scroll()
        .child(
            div()
                .max_w(px(720.0))
                .flex()
                .flex_col()
                .gap(spacing::LG)
                .child(
                    typography::type_title(div())
                        .child("Settings"),
                )
                .child(
                    typography::type_caption_strong(div())
                        .text_color(color::text_muted())
                        .child("MusicIndex endpoint"),
                )
                .child(
                    Input::new(&endpoint_input)
                        .cleanable(true)
                        .with_size(Size::Small),
                )
                .child(
                    typography::type_caption(div())
                        .text_color(color::text_muted())
                        .child("Use api.musicindex.org or a full http/https URL."),
                )
                .child(
                    typography::type_caption_strong(div())
                        .text_color(color::text_muted())
                        .child("Music directory"),
                )
                .child(
                    Input::new(&music_dir_input)
                        .cleanable(true)
                        .with_size(Size::Small),
                )
                .child(
                    typography::type_caption(div())
                        .text_color(color::text_muted())
                        .child("Downloads are organized under an artists subfolder."),
                )
                .child(
                    typography::type_caption_strong(div())
                        .text_color(color::text_muted())
                        .child("flac binary (optional)"),
                )
                .child(
                    Input::new(&flac_path_input)
                        .cleanable(true)
                        .with_size(Size::Small),
                )
                .child(
                    typography::type_caption(div())
                        .text_color(color::text_muted())
                        .child(
                            "Used to silently upgrade WAV downloads to FLAC. Leave blank to resolve `flac` via $PATH.",
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(spacing::SM)
                        .child(
                            Button::new("settings-save")
                                .label("Save")
                                .primary()
                                .with_size(Size::Small)
                                .text_color(rgb(0xffffff))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.save_settings(window, cx);
                                })),
                        )
                        .child(
                            Button::new("settings-default")
                                .label("Use Defaults")
                                .ghost()
                                .with_size(Size::Small)
                                .text_color(rgb(0xffffff))
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
                .child(div().border_t_1().border_color(color::border_subtle()))
                .child(
                    typography::type_caption_strong(div())
                        .text_color(color::text_muted())
                        .child(format!("Cached files ({})", cached_count)),
                )
                .when(!cached_is_empty, |el| {
                    let cached_tree = &app.cached_tree;
                    let mut cached_items = Vec::new();
                    for artist in &cached_tree.artists {
                        cached_items.push(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(color::text_primary())
                                .child(SharedString::from(artist.name.clone()))
                                .into_any_element()
                        );
                        for album in &artist.albums {
                            for track in &album.tracks {
                                let title = track.track_title.as_deref().unwrap_or("[untitled]");
                                let path_clone = track.local_path.clone().unwrap_or_default();
                                cached_items.push(
                                    div()
                                        .pl(spacing::MD)
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .gap(spacing::XS)
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_xs()
                                                .text_color(color::text_primary())
                                                .child(SharedString::from(title.to_string()))
                                        )
                                        .child(
                                            Button::new(SharedString::from(format!("del-cached-{}", track.id)))
                                                .label("Delete")
                                                .danger()
                                                .with_size(Size::XSmall)
                                                .text_color(rgb(0xffffff))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.delete_cached_file(path_clone.clone());
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
                            .gap(spacing::XXS)
                            .children(cached_items)
                    )
                })
                .when(!cached_is_empty, |el| {
                    el.child(
                        div().pt(spacing::SM).child(
                            Button::new("delete-all-cached-settings")
                                .label("Delete All Cached")
                                .danger()
                                .with_size(Size::XSmall)
                                .text_color(rgb(0xffffff))
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
                            .text_color(color::text_muted())
                            .child("No cached files"),
                    )
                }),
        )
        .into_any_element()
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

    div()
        .id(SharedString::from(format!("app-tab-{label}")))
        .track_focus(focus_handle)
        .on_click(cx.listener(move |this, _, _, cx| {
            this.tab = tab;
            if tab == AppTab::Library {
                this.library.update(cx, |lib, cx| lib.refresh(cx));
            }
            cx.notify();
        }))
        .px(spacing::MD)
        .min_h(layout::HIT_TARGET_MIN)
        .flex()
        .items_center()
        .rounded(radius::LG)
        .when(is_active, |el| {
            el.bg(color::accent()).text_color(color::text_on_accent())
        })
        .when(!is_active, |el| {
            el.text_color(color::text_secondary())
                .hover(|s| s.bg(color::bg_surface_hi()))
        })
        .when(is_focused, |el| {
            el.border_2().border_color(color::focus_ring())
        })
        .child(typography::type_body(div()).child(label))
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run_app() {
    let app = Application::new().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);

        // Load config + open DB
        let cfg_path = config::config_path().expect("config path");
        let cfg = config::load_config(&cfg_path).expect("load config");
        let musicindex_endpoint =
            config::load_musicindex_endpoint(&cfg_path).expect("load MusicIndex endpoint");
        config::ensure_dirs(&cfg).expect("ensure dirs");
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
                        size(px(1120.0), px(760.0)),
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
