use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use gpui::{
    div, prelude::*, px, rgb, AnyElement, Context, FontWeight,
    IntoElement, Render, SharedString, Styled, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::Sizable;
use gpui_component::Size;

use crate::db::{
    self, FeedRow, TrackRow,
};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryTab {
    Feeds,
    Tracks,
}

#[derive(Clone, Debug)]
enum LibraryDetail {
    None,
    Feed(FeedRow, Vec<TrackRow>),
    Track(TrackRow),
}

pub struct LibraryApp {
    conn: Arc<Mutex<Connection>>,
    tab: LibraryTab,
    feeds: Vec<FeedRow>,
    tracks: Vec<TrackRow>,
    selected_id: Option<i64>,
    detail: LibraryDetail,
    status: String,
}

// ---------------------------------------------------------------------------
// Color helpers (same palette as search.rs)
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
    rgb(0x9298ab)
}
fn accent() -> gpui::Rgba {
    rgb(0x8b9bff)
}

// ---------------------------------------------------------------------------
// LibraryApp
// ---------------------------------------------------------------------------

impl LibraryApp {
    pub fn new(conn: Arc<Mutex<Connection>>, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let mut app = Self {
            conn,
            tab: LibraryTab::Feeds,
            feeds: Vec::new(),
            tracks: Vec::new(),
            selected_id: None,
            detail: LibraryDetail::None,
            status: String::new(),
        };
        app.reload();
        app
    }

    fn reload(&mut self) {
        let conn = self.conn.lock().expect("lock db");
        match self.tab {
            LibraryTab::Feeds => match db::subscribed_feeds(&conn) {
                Ok(rows) => {
                    let count = rows.len();
                    self.feeds = rows;
                    self.status = format!("{count} subscribed feed{}", if count == 1 { "" } else { "s" });
                }
                Err(err) => {
                    self.status = format!("Error: {err:#}");
                }
            },
            LibraryTab::Tracks => match db::library_tracks(&conn) {
                Ok(rows) => {
                    let count = rows.len();
                    self.tracks = rows;
                    self.status = format!("{count} library track{}", if count == 1 { "" } else { "s" });
                }
                Err(err) => {
                    self.status = format!("Error: {err:#}");
                }
            },
        }
        self.selected_id = None;
        self.detail = LibraryDetail::None;
    }

    fn select_feed(&mut self, feed: &FeedRow) {
        self.selected_id = Some(feed.id);
        let conn = self.conn.lock().expect("lock db");
        match db::feed_tracks(&conn, feed.id) {
            Ok(tracks) => {
                self.detail = LibraryDetail::Feed(feed.clone(), tracks);
            }
            Err(err) => {
                self.status = format!("Error loading tracks: {err:#}");
            }
        }
    }

    fn select_track(&mut self, track: &TrackRow) {
        self.selected_id = Some(track.id);
        self.detail = LibraryDetail::Track(track.clone());
    }

    fn unsubscribe_feed(&mut self, feed_id: i64) {
        let conn = self.conn.lock().expect("lock db");
        if let Err(err) = db::set_feed_subscribed(&conn, feed_id, false) {
            self.status = format!("Error: {err:#}");
            return;
        }
        drop(conn);
        self.reload();
    }

    fn remove_track(&mut self, track_id: i64) {
        let conn = self.conn.lock().expect("lock db");
        if let Err(err) = db::set_track_in_library(&conn, track_id, false) {
            self.status = format!("Error: {err:#}");
            return;
        }
        drop(conn);
        self.reload();
    }

    fn toggle_track_library(&mut self, track_id: i64, currently_in: bool) {
        let conn = self.conn.lock().expect("lock db");
        if let Err(err) = db::set_track_in_library(&conn, track_id, !currently_in) {
            self.status = format!("Error: {err:#}");
            return;
        }
        drop(conn);
        // Refresh detail if viewing a feed
        if let LibraryDetail::Feed(ref feed, _) = self.detail {
            let feed = feed.clone();
            self.select_feed(&feed);
        }
    }
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

impl Render for LibraryApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status_text = self.status.clone();
        let status_color = if status_text.starts_with("Error:") {
            rgb(0xff6b6b)
        } else {
            muted()
        };

        let left_items: Vec<AnyElement> = match self.tab {
            LibraryTab::Feeds => self
                .feeds
                .iter()
                .map(|feed| render_feed_row(feed, self.selected_id, cx))
                .collect(),
            LibraryTab::Tracks => self
                .tracks
                .iter()
                .map(|track| render_track_row(track, self.selected_id, cx))
                .collect(),
        };

        let detail_pane = render_detail(&self.detail, cx);

        div()
            .size_full()
            .bg(bg())
            .text_color(text())
            .text_sm()
            .flex()
            .flex_col()
            .overflow_hidden()
            // Tab bar
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
                    .gap(px(8.0))
                    .child(render_tab_button("Feeds", LibraryTab::Feeds, self.tab, cx))
                    .child(render_tab_button("Tracks", LibraryTab::Tracks, self.tab, cx))
                    .child(
                        div().flex_1().child(
                            div()
                                .text_right()
                                .child(
                                    Button::new("lib-refresh")
                                        .label("Refresh")
                                        .ghost()
                                        .with_size(Size::XSmall)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.reload();
                                            cx.notify();
                                        })),
                                ),
                        ),
                    ),
            )
            // Two panes
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    // Left pane: list
                    .child(
                        div()
                            .w(px(320.0))
                            .min_w(px(200.0))
                            .flex_shrink_0()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .border_r_1()
                            .border_color(border())
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(status_color)
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .border_b_1()
                                    .border_color(border())
                                    .child(SharedString::from(status_text)),
                            )
                            .child(
                                div()
                                    .id("library-list")
                                    .flex_1()
                                    .overflow_y_scroll()
                                    .p(px(8.0))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.0))
                                            .children(left_items)
                                            .when(
                                                self.feeds.is_empty()
                                                    && self.tracks.is_empty()
                                                    && !self.status.starts_with("Error:"),
                                                |el| {
                                                    el.child(
                                                        div()
                                                            .text_center()
                                                            .p(px(48.0))
                                                            .text_color(muted())
                                                            .child(
                                                                div().mt(px(8.0)).child(
                                                                    match self.tab {
                                                                        LibraryTab::Feeds => "No subscribed feeds yet",
                                                                        LibraryTab::Tracks => "No library tracks yet",
                                                                    },
                                                                ),
                                                            ),
                                                    )
                                                },
                                            ),
                                    ),
                            ),
                    )
                    // Right pane: detail
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(detail_pane),
                    ),
            )
    }
}

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

fn render_tab_button(
    label: &'static str,
    tab: LibraryTab,
    active: LibraryTab,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let is_active = tab == active;
    let mut btn = Button::new(SharedString::from(format!("lib-tab-{label}")))
        .label(label)
        .with_size(Size::Small);

    if is_active {
        btn = btn.primary();
    } else {
        btn = btn.ghost();
    }

    btn.on_click(cx.listener(move |this, _, _, cx| {
        this.tab = tab;
        this.reload();
        cx.notify();
    }))
    .into_any_element()
}

fn render_feed_row(
    feed: &FeedRow,
    selected_id: Option<i64>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let is_selected = selected_id == Some(feed.id);
    let title = feed
        .title
        .as_deref()
        .unwrap_or("[untitled feed]")
        .to_string();
    let feed_clone = feed.clone();

    div()
        .id(SharedString::from(format!("feed-{}", feed.id)))
        .px(px(8.0))
        .py(px(6.0))
        .rounded(px(4.0))
        .cursor_pointer()
        .when(is_selected, |el| el.bg(rgb(0x252836)))
        .hover(|el| el.bg(rgb(0x1f2230)))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.select_feed(&feed_clone);
            cx.notify();
        }))
        .child(
            div()
                .font_weight(FontWeight::MEDIUM)
                .text_color(if is_selected { accent() } else { text() })
                .child(SharedString::from(title.clone())),
        )
        .when(feed.description.is_some(), |el| {
            el.child(
                div()
                    .text_xs()
                    .text_color(muted())
                    .mt(px(2.0))
                    .overflow_hidden()
                    .child(SharedString::from(
                        feed.description
                            .as_deref()
                            .unwrap_or("")
                            .chars()
                            .take(80)
                            .collect::<String>(),
                    )),
            )
        })
        .into_any_element()
}

fn render_track_row(
    track: &TrackRow,
    selected_id: Option<i64>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let is_selected = selected_id == Some(track.id);
    let title = track
        .track_title
        .as_deref()
        .unwrap_or("[untitled]")
        .to_string();
    let subtitle = track
        .feed_title
        .as_deref()
        .or(track.artist_name.as_deref())
        .unwrap_or("")
        .to_string();
    let track_clone = track.clone();

    div()
        .id(SharedString::from(format!("track-{}", track.id)))
        .px(px(8.0))
        .py(px(6.0))
        .rounded(px(4.0))
        .cursor_pointer()
        .when(is_selected, |el| el.bg(rgb(0x252836)))
        .hover(|el| el.bg(rgb(0x1f2230)))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.select_track(&track_clone);
            cx.notify();
        }))
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(6.0))
                .items_baseline()
                .when(track.track_number.is_some(), |el| {
                    el.child(
                        div()
                            .text_xs()
                            .text_color(muted())
                            .w(px(20.0))
                            .text_right()
                            .child(SharedString::from(
                                track.track_number.unwrap().to_string(),
                            )),
                    )
                })
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if is_selected { accent() } else { text() })
                        .child(SharedString::from(title)),
                ),
        )
        .when(!subtitle.is_empty(), |el| {
            el.child(
                div()
                    .text_xs()
                    .text_color(muted())
                    .mt(px(1.0))
                    .child(SharedString::from(subtitle)),
            )
        })
        .when(track.local_path.is_some(), |el| {
            el.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x6bcc6b))
                    .mt(px(1.0))
                    .child("downloaded"),
            )
        })
        .into_any_element()
}

fn render_detail(detail: &LibraryDetail, cx: &mut Context<LibraryApp>) -> AnyElement {
    match detail {
        LibraryDetail::None => div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_color(muted())
                    .text_center()
                    .child("Select an item to view details"),
            )
            .into_any_element(),

        LibraryDetail::Feed(feed, tracks) => render_feed_detail(feed, tracks, cx),

        LibraryDetail::Track(track) => render_track_detail(track, cx),
    }
}

fn render_feed_detail(
    feed: &FeedRow,
    tracks: &[TrackRow],
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let feed_id = feed.id;
    let title = feed.title.as_deref().unwrap_or("[untitled feed]");

    let track_rows: Vec<AnyElement> = tracks
        .iter()
        .map(|track| {
            let track_id = track.id;
            let in_library = track.is_in_library;
            let track_title = track
                .track_title
                .as_deref()
                .unwrap_or("[untitled]")
                .to_string();
            let num_str = track
                .track_number
                .map(|n| format!("{n}. "))
                .unwrap_or_default();
            let dur = track
                .duration_seconds
                .map(|s| format!("  ({}:{:02})", s / 60, s % 60))
                .unwrap_or_default();

            div()
                .id(SharedString::from(format!("feed-track-{track_id}")))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .px(px(8.0))
                .py(px(4.0))
                .rounded(px(4.0))
                .hover(|el| el.bg(rgb(0x1f2230)))
                .child(
                    div()
                        .flex_1()
                        .child(SharedString::from(format!(
                            "{num_str}{track_title}{dur}"
                        ))),
                )
                .child(
                    Button::new(SharedString::from(format!("lib-toggle-{track_id}")))
                        .label(if in_library {
                            "In Library"
                        } else {
                            "Add"
                        })
                        .with_size(Size::XSmall)
                        .when(in_library, |btn| btn.primary())
                        .when(!in_library, |btn| btn.ghost())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.toggle_track_library(track_id, in_library);
                            cx.notify();
                        })),
                )
                .when(
                    track.local_path.is_some(),
                    |el| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x6bcc6b))
                                .child("dl'd"),
                        )
                    },
                )
                .into_any_element()
        })
        .collect();

    div()
        .id("feed-detail-scroll")
        .size_full()
        .overflow_y_scroll()
        .p(px(16.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(12.0))
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::BOLD)
                        .text_color(accent())
                        .child(SharedString::from(title.to_string())),
                )
                .child(
                    Button::new("unsub-btn")
                        .label("Unsubscribe")
                        .danger()
                        .with_size(Size::XSmall)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.unsubscribe_feed(feed_id);
                            cx.notify();
                        })),
                ),
        )
        .when(feed.description.is_some(), |el| {
            el.child(
                div()
                    .text_xs()
                    .text_color(muted())
                    .child(SharedString::from(
                        feed.description.as_deref().unwrap_or("").to_string(),
                    )),
            )
        })
        .child(
            div()
                .mt(px(8.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(text())
                .child(SharedString::from(format!(
                    "{} track{}",
                    tracks.len(),
                    if tracks.len() == 1 { "" } else { "s" }
                ))),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .children(track_rows),
        )
        .into_any_element()
}

fn render_track_detail(track: &TrackRow, cx: &mut Context<LibraryApp>) -> AnyElement {
    let track_id = track.id;
    let in_library = track.is_in_library;
    let title = track
        .track_title
        .as_deref()
        .unwrap_or("[untitled]");

    let rows: Vec<(&str, String)> = [
        ("Title", track.track_title.clone()),
        ("Artist", track.artist_name.clone()),
        ("Album", track.album_title.clone().or_else(|| track.feed_title.clone())),
        (
            "Track #",
            track.track_number.map(|n| n.to_string()),
        ),
        (
            "Duration",
            track
                .duration_seconds
                .map(|s| format!("{}:{:02}", s / 60, s % 60)),
        ),
        ("Feed", track.feed_title.clone()),
        ("Local file", track.local_path.clone()),
    ]
    .into_iter()
    .filter_map(|(k, v)| v.map(|val| (k, val)))
    .collect();

    div()
        .id("track-detail-scroll")
        .size_full()
        .overflow_y_scroll()
        .p(px(16.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .text_color(accent())
                .child(SharedString::from(title.to_string())),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(8.0))
                .child(
                    Button::new("lib-remove-btn")
                        .label(if in_library {
                            "Remove from Library"
                        } else {
                            "Add to Library"
                        })
                        .with_size(Size::Small)
                        .when(in_library, |btn| btn.danger())
                        .when(!in_library, |btn| btn.primary())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.remove_track(track_id);
                            cx.notify();
                        })),
                ),
        )
        .children(rows.into_iter().map(|(key, value)| {
            div()
                .flex()
                .flex_row()
                .gap(px(8.0))
                .child(
                    div()
                        .w(px(80.0))
                        .text_xs()
                        .font_weight(FontWeight::BOLD)
                        .text_color(muted())
                        .child(SharedString::from(key.to_string())),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(text())
                        .child(SharedString::from(value)),
                )
                .into_any_element()
        }))
        .into_any_element()
}
