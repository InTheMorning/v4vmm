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
        match config::default_music_dir() {
            Ok(default_music_dir) => {
                self.music_dir_input.update(cx, |input, cx| {
                    input.set_value(default_music_dir.display().to_string(), window, cx);
                });
            }
            Err(error) => {
                self.settings_status = format!("Error: {error:#}");
                cx.notify();
                return;
            }
        }
        self.save_settings(window, cx);
    }
}

pub(super) fn render_settings(app: &mut TopApp, cx: &mut Context<TopApp>) -> AnyElement {
    let entity = cx.weak_entity();
    let callback: SettingsCallback = Rc::new(move |action, window, cx| {
        let _ = entity.update(cx, |this, cx| this.settings_action(action, window, cx));
    });
    let navigation = settings_actions(app.settings.navigation(), &callback, cx);
    let mut content = vec![settings_heading(app.settings.selected().label(), cx)];
    for &field in app.settings.selected().contents() {
        let input = match field {
            SettingsContent::SessionMaintenance => {
                if let Some(callback) = app.session_callback.clone() {
                    let available =
                        app.maintenance_worker.is_some() && !app.capability_vm.is_working();
                    content.push(crate::ui::composites::maintenance_forms::session_entry(
                        crate::view_models::startup::session::SessionReportVm::entry(available),
                        app.command_runner.session().generation(),
                        &app.previous_session_report,
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
            SettingsContent::MusicDirectory => settings_text_input(&app.music_dir_input, cx),
            SettingsContent::FlacPath => settings_text_input(&app.flac_path_input, cx),
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
        content.push(settings_actions(app.settings.edit_actions(), &callback, cx));
    }
    if !app.settings_status.is_empty() {
        content.push(settings_message(app.settings_status.clone(), cx));
    }
    settings_frame(navigation, content, cx)
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
