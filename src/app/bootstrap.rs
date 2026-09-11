//! Desktop composition and one startup window lifecycle (ADRs 0040, 0066).

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use gpui::{
    size, AppContext, Application, Bounds, Context, Entity, Window, WindowBounds, WindowOptions,
};
use gpui_component::Root;

use crate::application::capability::{
    CapabilityFailure, CapabilityObservation, CapabilityObservations, Dependency,
};
use crate::config;
use crate::media::ImageCache;
use crate::playback_driver::ConfiguredPlaybackDriver;
use crate::playback_owner::PlaybackOwner;
use crate::presentation::startup_presenter::{
    quit_after_window_close, window_disposition, WindowDisposition,
};
use crate::presentation::{
    maintenance_executor::{MaintenanceClient, MaintenanceWorker},
    RuntimeHost,
};
use crate::startup::{CoreCheckOutcome, PreparedCore, StartupIssue, StartupStage};
use crate::ui::layouts as layout;
use crate::view_models::startup::{format_report, normal_startup_status};

use super::{
    keyboard::install_key_bindings, menu::install_app_menu, startup::StartupScreen, TopApp,
};

/// Run one desktop session. False means recovery closed before normal startup.
///
/// Optional constructor failures remain scoped to their dependent operations.
/// Programmer invariants are not caught or converted into recovery results.
#[must_use]
pub fn run_app() -> bool {
    let worker = MaintenanceWorker::start();
    let client = worker.as_ref().ok().map(|worker| worker.client.clone());
    let opened = Arc::new(AtomicBool::new(false));
    let session_opened = opened.clone();
    let app = Application::new().with_assets(gpui_component_assets::Assets);
    app.run(move |cx| {
        gpui_component::init(cx);
        install_key_bindings(cx);
        install_app_menu(cx);
        // Pre-config: install with default scale; ADR 0066 recovery requires
        // Dark/Medium before configuration can be decoded.
        crate::ui::theme_bridge::install_theme(
            crate::theme_profile::ThemeProfile::Dark,
            crate::ui::tokens::ScaleFactor::Medium,
            cx,
        );
        let window = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(layout::WINDOW_WIDTH, layout::WINDOW_HEIGHT),
                    cx,
                ))),
                ..Default::default()
            },
            |window, cx| {
                window.on_window_should_close(cx, |_, cx| {
                    quit_after_window_close(cx);
                    true
                });
                let view = cx.new(|cx| {
                    let mut view = StartupScreen::new(client, session_opened);
                    view.begin(window, cx);
                    view
                });
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        if let Ok(window) = window {
            nudge_window(gpui::AnyWindowHandle::from(window), cx);
            if window
                .update(cx, |_, window, cx| {
                    window.activate_window();
                    window.refresh();
                    cx.activate(true);
                })
                .is_err()
            {
                let issue = StartupIssue::new(
                    StartupStage::Activation,
                    None,
                    "The desktop window was no longer available for activation.",
                    "Relaunch the app if the window was closed.",
                );
                eprintln!("{}", format_report(&CoreCheckOutcome::blocked(issue)));
                if window_disposition(true, !cx.windows().is_empty())
                    == WindowDisposition::ExitUnsuccessful
                {
                    cx.quit();
                }
            }
        } else {
            let issue = StartupIssue::new(
                StartupStage::Window,
                None,
                "The window system rejected the app's initial window. App did not open another error window.",
                "Check the desktop session and display environment, then relaunch the app.",
            );
            eprintln!("{}", format_report(&CoreCheckOutcome::blocked(issue)));
            cx.quit();
        }
    });
    if let Ok(worker) = worker {
        worker.finish();
    }
    opened.load(Ordering::Acquire)
}

pub(super) struct NormalStartup {
    cfg: config::ConfigSnapshot,
    cfg_path: std::path::PathBuf,
    conn: Arc<Mutex<rusqlite::Connection>>,
    musicindex_endpoint: config::MusicIndexEndpoint,
    playback_owner: Option<Arc<Mutex<PlaybackOwner<ConfiguredPlaybackDriver>>>>,
    runtime_host: Option<Arc<RuntimeHost>>,
    image_cache: Arc<ImageCache>,
    capability_observations: CapabilityObservations,
    notices: Vec<StartupIssue>,
}

/// Runs on the independent worker after core admission, never in a UI callback.
pub(super) fn prepare_normal(core: PreparedCore) -> Result<NormalStartup, CoreCheckOutcome> {
    let PreparedCore {
        config_path: cfg_path,
        snapshot,
        connection,
        notices,
    } = core;
    let cfg = snapshot;
    let capability_observations = CapabilityObservations::default();
    let music_dir = cfg
        .music_dir
        .as_ref()
        .expect("core admission verified music path");
    let db_path = cfg
        .db_path
        .as_ref()
        .expect("core admission verified database path");
    if crate::startup::prepare_local_paths(&connection, music_dir, db_path)? {
        capability_observations.record(
            CapabilityObservation::new(
                Dependency::LibraryPaths,
                Some(CapabilityFailure::PathRepair),
            )
            .at_path(db_path),
        );
    }
    let playback_owner =
        crate::startup::prepare_playback(&cfg, &cfg_path, &capability_observations)
            .map(|owner| Arc::new(Mutex::new(owner)));
    let musicindex_endpoint =
        config::MusicIndexEndpoint::from_field(cfg.musicindex_endpoint.clone());
    let conn = Arc::new(Mutex::new(connection));
    let runtime = RuntimeHost::for_config(&cfg_path);
    capability_observations.record(CapabilityObservation::new(
        Dependency::BackgroundRuntime,
        runtime
            .as_ref()
            .err()
            .map(|error| CapabilityFailure::RuntimeStart(error.kind())),
    ));
    let runtime_host = runtime.ok();
    let thumbnail_cache_dir = cfg_path
        .parent()
        .expect("config path has parent")
        .join("thumbnail-cache");
    let image_cache = ImageCache::new_observed(
        thumbnail_cache_dir,
        capability_observations.clone(),
        cache_worker_for_config(&cfg_path),
    );
    Ok(NormalStartup {
        cfg,
        cfg_path,
        conn,
        musicindex_endpoint,
        playback_owner,
        runtime_host,
        image_cache,
        capability_observations,
        notices,
    })
}

pub(super) fn mount_normal<T: 'static>(
    prepared: NormalStartup,
    worker: Option<MaintenanceClient>,
    window: &mut Window,
    cx: &mut Context<T>,
) -> Entity<TopApp> {
    let NormalStartup {
        cfg,
        cfg_path,
        conn,
        musicindex_endpoint,
        playback_owner,
        runtime_host,
        image_cache,
        capability_observations,
        notices,
    } = prepared;
    let workspace_layout_prefs = cfg
        .workspace_preferences()
        .and_then(|workspace| workspace.layout);
    let broadcast = cfg.broadcast();
    let theme_profile = cfg.theme_profile.unwrap_or_default();
    let ui_scale = cfg.ui_scale.unwrap_or_default();
    crate::ui::theme_bridge::install_theme_for_window(theme_profile, ui_scale.into(), window, cx);
    cx.new(|cx| {
        let mut app = TopApp::new(
            conn,
            image_cache,
            cfg_path,
            musicindex_endpoint,
            cfg.music_dir.expect("core admission verified music path"),
            cfg.flac_path,
            broadcast,
            cfg.workspace_layout.unwrap_or_default(),
            workspace_layout_prefs.as_ref(),
            ui_scale,
            theme_profile,
            playback_owner,
            runtime_host,
            window,
            cx,
        );
        app.focus_active_tab(window);
        app.install_capability_controls(capability_observations, worker, cx);
        app.maybe_start_broadcast_readiness_watch(cx);
        app.maybe_start_broadcast_service_watch(cx);
        app.refresh_show_page(cx);
        let status = normal_startup_status(&notices);
        if !status.is_empty() {
            app.settings_status = status;
        }
        app
    })
}

/// Called only during background preparation/retry, never by a renderer.
pub(super) fn cache_worker_for_config(
    path: &std::path::Path,
) -> crate::media::image_cache::CacheWorker {
    #[cfg(debug_assertions)]
    if crate::startup::fixture::injected_failure(path, Dependency::ThumbnailMaintenance) {
        return |_| Err(std::io::Error::other("fixture cache worker unavailable"));
    }
    #[cfg(not(debug_assertions))]
    let _ = path;
    crate::media::image_cache::spawn_cache_worker
}

fn nudge_window(window_handle: gpui::AnyWindowHandle, cx: &mut gpui::App) {
    cx.defer(move |cx| {
        let _ = window_handle.update(cx, |_, window, cx| {
            window.activate_window();
            window.refresh();
            cx.refresh_windows();
        });
        cx.activate(true);
        cx.refresh_windows();
    });
    // Window-activation nudge: GPUI needs this 16ms/100ms deferred refresh after creation; presentation lifecycle, not domain work, exempted by `cx_spawn_is_restricted_to_presentation_runtime_and_bootstrap`.
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adr_0066_normal_factory_scopes_each_optional_group_and_keeps_config_bytes() {
        let cases = [
            "musicindex_endpoint = false\n",
            "playback = false\n",
            "broadcast = false\n",
            "theme_profile = 12\n",
            "ui_scale = false\n",
            "workspace_layout = []\n",
            "flac_path = false\n",
            "[workspace]\nlayout = false\n",
            "[playback]\ndriver = 'broken'\n",
            "[broadcast]\nhosts = []\n",
            "[broadcast]\nselected_host = 12\n",
            "[broadcast]\ndrop_directory = false\n",
            "[broadcast]\nencoder = false\n",
            "[playback]\nmpv_path = false\n",
        ];
        for case in cases {
            let temp = tempfile::tempdir().unwrap();
            let music = temp.path().join("music");
            std::fs::create_dir(&music).unwrap();
            let path = temp.path().join("config.toml");
            let bytes = format!(
                "music_dir = {:?}\ndb_path = {:?}\n{case}",
                music,
                temp.path().join("library.sqlite")
            );
            std::fs::write(&path, &bytes).unwrap();
            let mut backend = crate::startup::StartupBackend::new(Some(path.clone()));
            let crate::startup::CoreResult::Prepared(core) =
                backend.execute(crate::startup::CheckIntent::Initial)
            else {
                panic!("optional setting blocked core admission: {case}");
            };
            let prepared = prepare_normal(*core)
                .unwrap_or_else(|_| panic!("optional setting blocked normal construction: {case}"));
            assert!(
                prepared
                    .capability_observations
                    .snapshot()
                    .values()
                    .any(|entry| entry.failure.is_some()),
                "{case}"
            );
            if case.contains("musicindex_endpoint") {
                assert!(prepared.musicindex_endpoint.require().is_err());
            }
            if case == "playback = false\n" || case.contains("driver = 'broken'") {
                assert!(prepared.playback_owner.is_none());
            }
            if case.contains("mpv_path = false") {
                assert!(
                    prepared.playback_owner.is_some(),
                    "unused mpv setting cannot disable configured Null"
                );
            }
            assert!(normal_startup_status(&prepared.notices).is_empty());
            drop(prepared);
            assert_eq!(std::fs::read_to_string(path).unwrap(), bytes);
        }
    }

    #[test]
    fn adr_0066_optional_notices_have_one_normal_report_owner() {
        let temp = tempfile::tempdir().unwrap();
        let music = temp.path().join("music");
        std::fs::create_dir(&music).unwrap();
        let artists = music.join("artists");
        std::fs::write(&artists, b"preserve download-directory blocker").unwrap();
        let config_path = temp.path().join("config.toml");
        let bytes = format!(
            "music_dir = {:?}\ndb_path = {:?}\ntheme_profile = 42\nui_scale = false\n[workspace.layout]\ncontent_list_view_mode = false\n",
            music, temp.path().join("library.sqlite"),
        );
        std::fs::write(&config_path, &bytes).unwrap();
        let mut backend = crate::startup::StartupBackend::new(Some(config_path.clone()));
        let crate::startup::CoreResult::Prepared(core) =
            backend.execute(crate::startup::CheckIntent::Initial)
        else {
            panic!("optional failures must permit normal startup");
        };
        assert_eq!(core.notices.len(), 4);
        let download_notice = core
            .notices
            .iter()
            .find(|issue| issue.stage == StartupStage::Artists)
            .unwrap()
            .cause
            .clone();
        let prepared = prepare_normal(*core).unwrap_or_else(|_| panic!("core remains usable"));
        assert_eq!(normal_startup_status(&prepared.notices), download_notice);
        let reports = crate::view_models::startup::capabilities::CapabilityReportVm::new(
            prepared.capability_observations.snapshot(),
            true,
        );
        for field in [
            "theme_profile",
            "ui_scale",
            "workspace.layout.content_list_view_mode",
        ] {
            assert_eq!(
                reports
                    .issues()
                    .filter(|issue| issue.dependency == Dependency::Configuration(field))
                    .count(),
                1
            );
            assert!(reports.report().contains(field));
        }
        assert_eq!(
            std::fs::read(&artists).unwrap(),
            b"preserve download-directory blocker"
        );
        assert_eq!(std::fs::read_to_string(config_path).unwrap(), bytes);
    }
}
