use std::sync::{Arc, Mutex};

use gpui::{
    div, prelude::*, px, rgb, size, Application, Bounds, Context, Entity, FontWeight, Render,
    SharedString, Styled, Window, WindowBounds, WindowOptions,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::{Root, Sizable, Size};

use crate::config;
use crate::db;
use crate::library::LibraryApp;
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
fn accent() -> gpui::Rgba {
    rgb(0x8b9bff)
}

// ---------------------------------------------------------------------------
// AppTab
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppTab {
    Library,
    Discover,
}

// ---------------------------------------------------------------------------
// TopApp
// ---------------------------------------------------------------------------

pub struct TopApp {
    tab: AppTab,
    search: Entity<SearchApp>,
    library: Entity<LibraryApp>,
}

impl TopApp {
    fn new(conn: Arc<Mutex<Connection>>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_conn = Arc::clone(&conn);
        let search = cx.new(|cx| SearchApp::new(search_conn, window, cx));
        let library = cx.new(|cx| LibraryApp::new(conn, window, cx));

        Self {
            tab: AppTab::Library,
            search,
            library,
        }
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
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(accent())
                            .mr(px(12.0))
                            .child("stophammer"),
                    )
                    .child(render_app_tab("Library", AppTab::Library, self.tab, cx))
                    .child(render_app_tab("Discover", AppTab::Discover, self.tab, cx)),
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
                    }),
            )
    }
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
        config::ensure_dirs(&cfg).expect("ensure dirs");
        let conn = db::open_db(&cfg).expect("open db");
        let conn = Arc::new(Mutex::new(conn));

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
                let view = cx.new(|cx| TopApp::new(conn, window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("failed to open window");
    });
}
