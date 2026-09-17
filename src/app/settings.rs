//! Settings composition and persistent input/focus wiring (ADR 0069).

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{AnyElement, Context, Window};

use crate::config;
use crate::theme_profile::ThemeProfile;
use crate::ui::composites::settings::{
    settings_actions, settings_cached_row, settings_field, settings_frame, settings_heading,
    settings_message, settings_text_input, SettingsCallback,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::Button;
use crate::view_models::cached_files::{CachedFileAction, CachedFileActionDisplay, CachedFileRow};
use crate::view_models::settings::{SettingsAction, SettingsContent, SettingsEffect, SettingsVm};
use crate::view_models::startup::StartupAvailability;

use super::TopApp;

impl TopApp {
    pub(super) fn settings_action(
        &mut self,
        action: SettingsAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.settings.dispatch(action) {
            // Group buttons stay mounted and retain their own keyboard/mouse focus.
            SettingsEffect::Navigate => {}
            SettingsEffect::ConfigureConverter => self.capability_action(
                crate::view_models::startup::capabilities::CapabilityAction::Configure(
                    crate::application::capability::Dependency::Converter,
                ),
                window,
                cx,
            ),
            SettingsEffect::Save => self.save_settings(window, cx),
            SettingsEffect::UseDefaults => self.use_default_settings(window, cx),
            SettingsEffect::SetScale(scale) => self.set_ui_scale(scale, window, cx),
            SettingsEffect::SetTheme(profile) => self.set_theme_profile(profile, window, cx),
        }
        cx.notify();
    }

    fn use_default_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.endpoint_input.update(cx, |input, cx| {
            input.set_value(crate::api::DEFAULT_BASE_URL, window, cx);
        });
        self.flac_path_input.update(cx, |input, cx| {
            input.set_value("", window, cx);
        });
        self.set_ui_scale(config::UiScale::Medium, window, cx);
        self.set_theme_profile(ThemeProfile::default(), window, cx);
        self.save_settings(window, cx);
    }

    pub(super) fn apply_corrected_presentation(
        &mut self,
        snapshot: &config::ConfigSnapshot,
        cx: &mut Context<Self>,
    ) {
        let preferences = snapshot.workspace_preferences();
        let preferences = preferences
            .as_ref()
            .and_then(|config| config.layout.as_ref());
        self.content_pane_width = Self::initial_content_pane_width(preferences);
        self.library.update(cx, |library, cx| {
            library.set_content_view_mode(Self::initial_content_list_view_mode(preferences), cx);
        });
        self.workspace_layout = Self::initial_workspace_layout(
            snapshot
                .workspace_layout
                .as_ref()
                .ok()
                .and_then(Option::as_ref),
            super::WorkspaceScreenMount::Music,
        );
        cx.notify();
    }

    pub(super) fn refresh_corrected_settings(
        &mut self,
        snapshot: &config::ConfigSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Form values and appearance share the accepted task 006 owner.
        // Scoped capability checks are queued separately; Save never retries an action.
        if let Ok(endpoint) = &snapshot.musicindex_endpoint {
            self.endpoint_input.update(cx, |input, cx| {
                input.set_value(endpoint.clone(), window, cx);
            });
        }
        if let Ok(path) = &snapshot.flac_path {
            let value = path
                .as_ref()
                .map_or_else(String::new, |path| path.display().to_string());
            self.flac_path_input
                .update(cx, |input, cx| input.set_value(value, window, cx));
        }
        if let Ok(scale) = snapshot.ui_scale {
            self.set_ui_scale(scale, window, cx);
        }
        if let Ok(profile) = snapshot.theme_profile {
            self.set_theme_profile(profile, window, cx);
        }
        cx.notify();
    }
}

pub(super) fn render_settings(app: &mut TopApp, cx: &mut Context<TopApp>) -> AnyElement {
    let correction_idle = !app
        .configuration_editor
        .as_ref()
        .is_some_and(|editor| editor.read(cx).vm.is_working());
    let entity = cx.weak_entity();
    let callback: SettingsCallback = Rc::new(move |action, window, cx| {
        let _ = entity.update(cx, |this, cx| this.settings_action(action, window, cx));
    });
    let navigation = settings_actions(app.settings.navigation(), &callback, cx);
    let mut content = vec![settings_heading(app.settings.selected().label(), cx)];
    if app.settings.repair_first() {
        if let Some(editor) = &app.configuration_editor {
            use gpui::IntoElement as _;
            content.push(editor.clone().into_any_element());
        }
    }
    for &field in app.settings.selected().contents() {
        let input = match field {
            SettingsContent::SessionMaintenance => {
                if let Some(callback) = app.session_callback.clone() {
                    let available = app.maintenance_worker.is_some()
                        && !app.capability_vm.is_working()
                        && correction_idle;
                    content.push(crate::ui::composites::maintenance_forms::session_entry(
                        crate::view_models::startup::session::SessionReportVm::entry(available),
                        app.command_runner.session().generation(),
                        &app.previous_session_report,
                        &app.log_frames,
                        callback,
                        cx,
                    ));
                }
                continue;
            }
            SettingsContent::Scale => {
                settings_actions(SettingsVm::scale_choices(app.ui_scale), &callback, cx)
            }
            SettingsContent::Theme => {
                settings_actions(SettingsVm::theme_choices(app.theme_profile), &callback, cx)
            }
            SettingsContent::Endpoint => settings_text_input(&app.endpoint_input, cx),
            SettingsContent::MusicDirectory => {
                settings_message(app.music_dir.display().to_string(), cx)
            }
            SettingsContent::FlacPath => {
                settings_actions(SettingsVm::converter_setup(), &callback, cx)
            }
            SettingsContent::BackgroundReports => {
                content.extend(app.render_capabilities(true, cx));
                continue;
            }
            SettingsContent::CachedFiles => {
                content.extend(render_cached_files(app, cx));
                continue;
            }
        };
        content.push(settings_field(field, input, cx));
    }
    if app.settings.editable() {
        content.push(settings_message(SettingsVm::SAVE_SCOPE, cx));
        content.push(settings_actions(
            app.settings.edit_actions(correction_idle),
            &callback,
            cx,
        ));
    }
    if app.settings.shows_configuration_repair() && !app.settings.repair_first() {
        if let Some(editor) = &app.configuration_editor {
            use gpui::IntoElement as _;
            content.push(editor.clone().into_any_element());
        }
    }
    if !app.settings_status.is_empty() {
        content.push(settings_message(app.settings_status.clone(), cx));
    }
    let page_scroll = app.settings_scroll.handle(app.settings.selected());
    settings_frame(navigation, content, page_scroll, cx)
}

fn render_cached_files(app: &TopApp, cx: &mut Context<TopApp>) -> Vec<AnyElement> {
    let mut content = vec![settings_heading(app.cached_files.title(), cx)];
    let execution = app.command_runner.availability();
    for row in app.cached_files.rows(&app.music_dir, execution) {
        content.push(match row {
            CachedFileRow::Artist(name) => settings_message(name, cx),
            CachedFileRow::Track { title, delete } => {
                settings_cached_row(title, cached_action(delete, cx), cx)
            }
        });
    }
    if let Some(action) = app.cached_files.delete_all_action(execution) {
        use gpui::IntoElement as _;
        content.push(cached_action(action, cx).into_any_element());
    }
    if let Some(message) = app.cached_files.status() {
        content.push(settings_message(message, cx));
    }
    content
}

fn cached_action(display: CachedFileActionDisplay, cx: &mut Context<TopApp>) -> Button {
    let entity = cx.weak_entity();
    Button::styled(
        gpui::SharedString::from(display.id),
        ControlStyle::Destructive,
    )
    .label(display.label)
    .a11y_label(display.a11y_label)
    .disabled(display.availability != StartupAvailability::Available)
    .on_activate(move |_, cx| {
        let _ = entity.update(cx, |this, cx| match &display.action {
            CachedFileAction::Delete(path) => this.delete_cached_file(path.clone(), cx),
            CachedFileAction::DeleteAll => this.delete_all_cached(cx),
        });
    })
}
