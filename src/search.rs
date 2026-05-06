#![allow(dead_code)]
#![warn(clippy::pedantic)]
#![expect(
    clippy::pedantic,
    reason = "legacy discover screen is being migrated incrementally under ADR 0023"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use gpui::{
    prelude::*, size, Application, Bounds, Entity, Image, ScrollHandle, WindowBounds, WindowOptions,
};
use gpui_component::input::InputState;
use gpui_component::Root;
use rusqlite::Connection;

use crate::api::{Artist, Feed, PaymentRoute, Publisher, Track};
use crate::application::ApplicationServices;
use crate::config;
use crate::db;
use crate::media::ImageCache;
use crate::metadata::*;
use crate::presentation::GpuiCommandRunner;
use crate::view_models::search::{LazyPanel, SearchViewModel};
use crate::views::ContributorView;

#[cfg(test)]
use crate::api::EntityDetail;
#[cfg(test)]
use crate::ui::shells::discover::track_inspector_metadata_grid::{
    id3_frame_hint, metadata_data_row, metadata_drag_value, unused_id3v24_frames_for_group,
    used_id3_fields_for_group,
};
#[cfg(test)]
use crate::view_models::search::{
    artist_rows_from_result_rows, search_result_type_is_visible, ResultRow, SearchBatch,
};

#[derive(Clone, Debug)]
pub(crate) enum InspectorDetail {
    Loading(String),
    Error(String),
    Artist(Box<ArtistContext>),
    Feed(Box<Feed>),
    Track(Box<TrackContext>),
    Publisher(Publisher),
}

#[derive(Clone, Debug)]
pub(crate) struct ArtistContext {
    pub(crate) artist: Artist,
    pub(crate) tracks: Vec<Track>,
    pub(crate) feeds: Vec<Feed>,
    pub(crate) has_more_tracks: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct InspectorFrame {
    pub entity_type: String,
    pub entity_id: String,
    pub title: String,
    pub detail: InspectorDetail,
    pub image: Option<Arc<Image>>,
    pub contributors: LazyPanel<Vec<ContributorView>>,
    pub contributors_collapsed: bool,
    pub value_routes: LazyPanel<Vec<PaymentRoute>>,
    pub value_routes_collapsed: bool,
    pub expanded_id3_frame_groups: BTreeSet<String>,
    pub expanded_metadata_cells: BTreeSet<String>,
    pub pending_id3_edits: BTreeMap<String, PendingId3Edit>,
    pub suppressed_auto_id3_edits: BTreeSet<String>,
    pub applying_id3_edits: bool,
    pub id3_apply_error: Option<String>,
    pub local_subscription: Option<bool>,
    pub subscription_busy: bool,
    pub subscription_message: Option<String>,
    pub tag_compare: LazyPanel<TagCompareResult>,
    pub musicbrainz_lookup: LazyPanel<MusicBrainzLookupResult>,
    pub musicbrainz_selected: usize,
    pub podroll: LazyPanel<Vec<Feed>>,
    /// Per-frame scroll handle so popping back to a prior inspector
    /// frame restores the user's scroll position. Without this, every
    /// frame in the stack would share the single element-state-keyed
    /// scroll on the inspector container — and content swaps reset it
    /// to zero, which feels like a fresh navigation rather than a back.
    pub scroll_handle: ScrollHandle,
}

impl InspectorFrame {
    fn loading(entity_type: String, entity_id: String, title: String) -> Self {
        Self {
            entity_type,
            entity_id,
            title: title.clone(),
            detail: InspectorDetail::Loading(SearchViewModel::inspector_loading_message(&title)),
            image: None,
            contributors: LazyPanel::Hidden,
            contributors_collapsed: true,
            value_routes: LazyPanel::Hidden,
            value_routes_collapsed: true,
            expanded_id3_frame_groups: BTreeSet::new(),
            expanded_metadata_cells: BTreeSet::new(),
            pending_id3_edits: BTreeMap::new(),
            suppressed_auto_id3_edits: BTreeSet::new(),
            applying_id3_edits: false,
            id3_apply_error: None,
            local_subscription: None,
            subscription_busy: false,
            subscription_message: None,
            tag_compare: LazyPanel::Hidden,
            musicbrainz_lookup: LazyPanel::Hidden,
            musicbrainz_selected: 0,
            podroll: LazyPanel::Hidden,
            scroll_handle: ScrollHandle::new(),
        }
    }
}

#[derive(Clone)]
enum ThumbnailState {
    Loading,
    Loaded(Option<Arc<Image>>),
}

pub struct SearchApp {
    pub(crate) conn: Arc<Mutex<Connection>>,
    application_services: Arc<ApplicationServices>,
    command_runner: GpuiCommandRunner,
    cache: Arc<ImageCache>,
    musicindex_endpoint: String,
    pub(crate) input: Entity<InputState>,
    /// Stateful screen view-model. Owns all pure UI scalars,
    /// pane-state flags, and loaded snapshots (results, recent feeds,
    /// playlists). Fields kept on `SearchApp` itself are GPUI-bound
    /// (`Entity`, `Subscription`, `FocusHandle`), service
    /// handles, screen-only inspector state, or maps that still hold
    /// `Arc<gpui::Image>`. See ADR 0023.
    pub(crate) vm: SearchViewModel,
    inspector_stack: Vec<InspectorFrame>,
    thumbnails: BTreeMap<String, ThumbnailState>,
    _input_sub: gpui::Subscription,
    list_focus: gpui::FocusHandle,
    /// Reserved for paged search results (ADR 0040 follow-up).
    #[cfg(feature = "async-runtime")]
    #[allow(dead_code)]
    pub(crate) runtime_host: Option<Arc<crate::presentation::RuntimeHost>>,
}

/// Events emitted by [`SearchApp`] to notify peer components (e.g. the
/// library tab) that local library state has changed and they should refresh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchAppEvent {
    LibraryMutated,
}

impl gpui::EventEmitter<SearchAppEvent> for SearchApp {}

pub(crate) type FeedTrackListContext<'a> = (&'a str, Option<&'a str>, &'a [db::Playlist]);

mod app_impl;

#[cfg(test)]
use app_impl::{
    feed_rss_url, merge_track_play_fields, persist_musicindex_artist_facts,
    should_show_inspector_back,
};

pub(crate) use crate::ui::shells::discover::actions::{
    discover_inspector_action_row, render_play_icon_button_with_id, render_track_download_button,
};
pub(crate) use crate::ui::shells::discover::track_rows::render_track_list_rows;

#[cfg(test)]
#[allow(dead_code)]
fn fmt_ms(ms: i64) -> String {
    crate::view_models::track::fmt_dur((ms / 1000).try_into().unwrap_or(i32::MAX))
}

#[cfg(test)]
#[allow(dead_code)]
fn join_values(values: &[String]) -> Option<String> {
    if values.is_empty() {
        None
    } else {
        Some(values.join(" · "))
    }
}

use crate::ui::layouts as layout;

pub fn run_search_app() {
    let app = Application::new().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
        crate::ui::theme_bridge::install_theme(
            crate::theme_profile::ThemeProfile::Dark,
            crate::ui::tokens::ScaleFactor::Medium,
            cx,
        );
        let cfg_path = config::config_path().expect("config path");
        let cfg = config::load_config(&cfg_path).expect("load config");
        config::ensure_dirs(&cfg).expect("ensure dirs");
        let conn = db::open_db(&cfg).expect("open db");
        let conn = Arc::new(Mutex::new(conn));
        let musicindex_endpoint =
            config::load_musicindex_endpoint(&cfg_path).expect("load MusicIndex endpoint");

        crate::ui::theme_bridge::install_theme(cfg.theme_profile, cfg.ui_scale.into(), cx);

        let thumbnail_cache_dir = cfg_path
            .parent()
            .expect("config path has parent")
            .join("thumbnail-cache");
        let http = reqwest::blocking::Client::new();
        let image_cache = ImageCache::new(http, thumbnail_cache_dir);
        let application_services = Arc::new(
            ApplicationServices::local_with_service_adapters()
                .expect("application services are fully wired"),
        );

        cx.open_window(
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
                    SearchApp::new(
                        conn,
                        image_cache,
                        musicindex_endpoint,
                        application_services,
                        #[cfg(feature = "async-runtime")]
                        None,
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

#[cfg(test)]
mod tests;
