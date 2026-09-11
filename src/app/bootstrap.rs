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
use crate::view_models::startup::format_report;
use crate::{config, db, playback};

use super::{
    keyboard::install_key_bindings, menu::install_app_menu, startup::StartupScreen, TopApp,
};

/// Run one desktop session. False means recovery closed before normal startup.
///
/// Optional constructor failures retain their existing policy until 003/004.
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
    cfg: config::Config,
    cfg_path: std::path::PathBuf,
    conn: Arc<Mutex<rusqlite::Connection>>,
    musicindex_endpoint: String,
    playback_owner: Arc<Mutex<PlaybackOwner<ConfiguredPlaybackDriver>>>,
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
    // The strict optional adapter and second endpoint read are task 004's boundary.
    let cfg = snapshot
        .legacy_config()
        .expect("load optional configuration (task 004)");
    let musicindex_endpoint =
        config::load_musicindex_endpoint(&cfg_path).expect("load MusicIndex endpoint");
    let repair =
        db::repair_local_file_paths(&connection, &cfg.music_dir).expect("repair local paths");
    if repair.skipped == Some(db::LocalPathRepairSkip::MusicFolderMissing) {
        return Err(CoreCheckOutcome::blocked(StartupIssue::new(
            StartupStage::MusicInspect,
            Some(&cfg.music_dir),
            "Music storage disappeared before normal startup. App preserved the library bindings.",
            "Mount or correct the music directory, then choose Check again.",
        )));
    }
    let conn = Arc::new(Mutex::new(connection));
    let playback_driver =
        ConfiguredPlaybackDriver::from_config(&cfg.playback).expect("configure playback driver");
    let drop_file_producer = cfg
        .broadcast
        .drop_file_producer()
        .expect("configure broadcast drop-file producer");
    let playback_owner = Arc::new(Mutex::new(
        PlaybackOwner::new(
            playback_driver,
            playback::DEFAULT_SESSION_ID,
            cfg.music_dir.clone(),
        )
        .with_drop_file_producer(drop_file_producer),
    ));
    let capability_observations = CapabilityObservations::default();
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
        .workspace
        .as_ref()
        .and_then(|workspace| workspace.layout.clone());
    crate::ui::theme_bridge::install_theme_for_window(
        cfg.theme_profile,
        cfg.ui_scale.into(),
        window,
        cx,
    );
    cx.new(|cx| {
        let mut app = TopApp::new(
            conn,
            image_cache,
            cfg_path,
            musicindex_endpoint,
            cfg.music_dir,
            cfg.flac_path,
            cfg.broadcast,
            cfg.workspace_layout,
            workspace_layout_prefs.as_ref(),
            cfg.ui_scale,
            cfg.theme_profile,
            playback_owner,
            runtime_host,
            window,
            cx,
        );
        app.focus_active_tab(window);
        app.install_capability_controls(capability_observations, worker, cx);
        app.maybe_start_playback_polling(cx);
        app.maybe_start_broadcast_readiness_watch(cx);
        app.maybe_start_broadcast_service_watch(cx);
        app.refresh_show_page(cx);
        if !notices.is_empty() {
            app.settings_status = notices
                .iter()
                .map(|issue| issue.cause.as_str())
                .collect::<Vec<_>>()
                .join("\n");
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
