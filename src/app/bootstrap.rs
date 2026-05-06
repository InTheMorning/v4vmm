//! Public GPUI application bootstrap.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{size, AppContext, Application, Bounds, WindowBounds, WindowOptions};
use gpui_component::Root;

use crate::config;
use crate::db;
use crate::media::ImageCache;
use crate::playback;
use crate::playback_driver::ConfiguredPlaybackDriver;
use crate::playback_owner::PlaybackOwner;
use crate::ui::layouts as layout;

use super::{keyboard::install_key_bindings, menu::install_app_menu, TopApp};

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
        install_key_bindings(cx);
        install_app_menu(cx);
        // Pre-config: install with default scale so the loading window is
        // themed; we re-install with the user's preference once config is
        // loaded a few lines below.
        crate::ui::theme_bridge::install_theme(
            crate::theme_profile::ThemeProfile::Dark,
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
        crate::ui::theme_bridge::install_theme(cfg.theme_profile, cfg.ui_scale.into(), cx);
        let conn = db::open_db(&cfg).expect("open db");
        let conn = Arc::new(Mutex::new(conn));
        let playback_driver = ConfiguredPlaybackDriver::from_config(&cfg.playback)
            .expect("configure playback driver");
        let playback_owner = Arc::new(Mutex::new(PlaybackOwner::new(
            playback_driver,
            playback::DEFAULT_SESSION_ID,
        )));

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
                            cfg.theme_profile,
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
