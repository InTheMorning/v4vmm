use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use gpui::{
    div, img, prelude::*, px, rgb, size, Application, Bounds, Context, Entity, FontWeight, Image,
    ImageFormat, ObjectFit, Render, SharedString, Styled, Window, WindowBounds, WindowOptions,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputState};
use gpui_component::{Root, Sizable, Size};

use crate::config;
use crate::db;
use crate::library::LibraryApp;
use crate::media::ImageCache;
use crate::search::SearchApp;

// ---------------------------------------------------------------------------
// Color helpers (same palette)
// ---------------------------------------------------------------------------

fn bg() -> gpui::Rgba {
    rgb(0x0f1117)
}
fn surface() -> gpui::Rgba {
    rgb(0x1a1d27)
}
fn border() -> gpui::Rgba {
    rgb(0x2a2d3a)
}
fn text() -> gpui::Rgba {
    rgb(0xe2e4ed)
}
fn muted() -> gpui::Rgba {
    rgb(0x9aa0b4)
}
fn accent() -> gpui::Rgba {
    rgb(0x8b9bff)
}

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
    cfg_path: PathBuf,
    settings_status: String,
}

impl TopApp {
    fn new(
        conn: Arc<Mutex<Connection>>,
        image_cache: Arc<ImageCache>,
        cfg_path: PathBuf,
        musicindex_endpoint: String,
        music_dir: PathBuf,
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
            LibraryApp::new(conn, library_cache, musicindex_endpoint.clone(), window, cx)
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

        Self {
            tab: AppTab::Library,
            search,
            library,
            endpoint_input,
            music_dir_input,
            cfg_path,
            settings_status: String::new(),
        }
    }

    fn save_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let endpoint = self.endpoint_input.read(cx).value().to_string();
        let music_dir = self.music_dir_input.read(cx).value().to_string();
        match config::save_app_settings(&self.cfg_path, &endpoint, &music_dir) {
            Ok((normalized_endpoint, normalized_music_dir)) => {
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
}

use rusqlite::Connection;

impl Render for TopApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(bg())
            .text_color(text())
            .text_sm()
            .flex()
            .flex_col()
            .overflow_hidden()
            // Top-level tab bar
            .child(
                div()
                    .bg(surface())
                    .border_b_1()
                    .border_color(border())
                    .px(px(12.0))
                    .py(px(6.0))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(8.0))
                            .mr(px(12.0))
                            .child(
                                div()
                                    .w(px(26.0))
                                    .h(px(26.0))
                                    .rounded(px(4.0))
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
                                div()
                                    .text_base()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(accent())
                                    .child("V4V Music Manager"),
                            ),
                    )
                    .child(render_app_tab("Library", AppTab::Library, self.tab, cx))
                    .child(render_app_tab("Discover", AppTab::Discover, self.tab, cx))
                    .child(render_app_tab("Settings", AppTab::Settings, self.tab, cx)),
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
    let endpoint_input = app.endpoint_input.clone();
    let music_dir_input = app.music_dir_input.clone();
    let status = app.settings_status.clone();
    let status_color = if status.starts_with("Error:") {
        rgb(0xff6b6b)
    } else {
        muted()
    };

    div()
        .id("settings-scroll")
        .size_full()
        .bg(bg())
        .p(px(24.0))
        .overflow_y_scroll()
        .child(
            div()
                .max_w(px(720.0))
                .flex()
                .flex_col()
                .gap(px(14.0))
                .child(
                    div()
                        .text_2xl()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Settings"),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(muted())
                        .child("MusicIndex endpoint"),
                )
                .child(
                    Input::new(&endpoint_input)
                        .cleanable(true)
                        .with_size(Size::Small),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(muted())
                        .child("Use api.musicindex.org or a full http/https URL."),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(muted())
                        .child("Music directory"),
                )
                .child(
                    Input::new(&music_dir_input)
                        .cleanable(true)
                        .with_size(Size::Small),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(muted())
                        .child("Downloads are organized under an artists subfolder."),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
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
                }),
        )
        .into_any_element()
}

fn render_app_tab(
    label: &'static str,
    tab: AppTab,
    active: AppTab,
    cx: &mut Context<TopApp>,
) -> gpui::AnyElement {
    let is_active = tab == active;
    let mut btn = Button::new(SharedString::from(format!("app-tab-{label}")))
        .label(label)
        .with_size(Size::Small);

    if is_active {
        btn = btn.primary();
    } else {
        btn = btn.ghost();
    }

    btn.text_color(rgb(0xffffff))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.tab = tab;
            if tab == AppTab::Library {
                this.library.update(cx, |lib, cx| lib.refresh(cx));
            }
            cx.notify();
        }))
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

        let thumbnail_cache_dir = cfg_path
            .parent()
            .expect("config path has parent")
            .join("thumbnail-cache");
        let http = reqwest::blocking::Client::new();
        let image_cache = ImageCache::new(http, thumbnail_cache_dir);

        cx.open_window(
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
                    TopApp::new(
                        conn,
                        image_cache,
                        cfg_path,
                        musicindex_endpoint,
                        cfg.music_dir,
                        window,
                        cx,
                    )
                });
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("failed to open window");
    });
}
