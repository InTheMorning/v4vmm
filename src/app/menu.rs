//! macOS menu bar bootstrap.
//!
//! GPUI exposes platform menus as action-backed menu items. This module keeps
//! the app-menu contract centralized so standard macOS commands stay visible
//! and pick up their key equivalents from the same keymap as the rest of the
//! application.

use gpui::{actions, App, Context, KeyBinding, Menu, MenuItem, SystemMenuType, Window};

use super::{AppTab, TopApp};

const APP_NAME: &str = "v4vmm";

actions!(
    v4vmm,
    [
        OpenPreferences,
        HideApp,
        HideOtherApps,
        ShowAllApps,
        QuitApp,
    ]
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum AppMenuCommand {
    OpenPreferences,
    HideApp,
    HideOtherApps,
    QuitApp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AppMenuBindingSpec {
    pub(super) command: AppMenuCommand,
    pub(super) keystroke: &'static str,
    pub(super) label: &'static str,
}

pub(super) const APP_MENU_BINDING_SPECS: &[AppMenuBindingSpec] = &[
    AppMenuBindingSpec {
        command: AppMenuCommand::OpenPreferences,
        keystroke: "cmd-,",
        label: "Preferences...",
    },
    AppMenuBindingSpec {
        command: AppMenuCommand::HideApp,
        keystroke: "cmd-h",
        label: "Hide v4vmm",
    },
    AppMenuBindingSpec {
        command: AppMenuCommand::HideOtherApps,
        keystroke: "cmd-alt-h",
        label: "Hide Others",
    },
    AppMenuBindingSpec {
        command: AppMenuCommand::QuitApp,
        keystroke: "cmd-q",
        label: "Quit v4vmm",
    },
];

pub(super) fn install_app_menu(cx: &mut App) {
    cx.bind_keys(app_menu_key_bindings());
    cx.on_action(handle_hide_app);
    cx.on_action(handle_hide_other_apps);
    cx.on_action(handle_show_all_apps);
    cx.on_action(handle_quit_app);
    cx.set_menus(app_menus());
}

fn app_menu_key_bindings() -> Vec<KeyBinding> {
    APP_MENU_BINDING_SPECS
        .iter()
        .map(AppMenuBindingSpec::key_binding)
        .collect()
}

fn app_menus() -> Vec<Menu> {
    vec![Menu {
        name: APP_NAME.into(),
        items: vec![
            MenuItem::action("Preferences...", OpenPreferences),
            MenuItem::separator(),
            MenuItem::os_submenu("Services", SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action("Hide v4vmm", HideApp),
            MenuItem::action("Hide Others", HideOtherApps),
            MenuItem::action("Show All", ShowAllApps),
            MenuItem::separator(),
            MenuItem::action("Quit v4vmm", QuitApp),
        ],
    }]
}

impl AppMenuBindingSpec {
    fn key_binding(&self) -> KeyBinding {
        match self.command {
            AppMenuCommand::OpenPreferences => {
                KeyBinding::new(self.keystroke, OpenPreferences, None)
            }
            AppMenuCommand::HideApp => KeyBinding::new(self.keystroke, HideApp, None),
            AppMenuCommand::HideOtherApps => KeyBinding::new(self.keystroke, HideOtherApps, None),
            AppMenuCommand::QuitApp => KeyBinding::new(self.keystroke, QuitApp, None),
        }
    }
}

impl TopApp {
    pub(super) fn handle_open_preferences(
        &mut self,
        _: &OpenPreferences,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.tab = AppTab::Settings;
        cx.notify();
    }
}

fn handle_hide_app(_: &HideApp, cx: &mut App) {
    cx.hide();
}

fn handle_hide_other_apps(_: &HideOtherApps, cx: &mut App) {
    cx.hide_other_apps();
}

fn handle_show_all_apps(_: &ShowAllApps, cx: &mut App) {
    cx.unhide_other_apps();
}

fn handle_quit_app(_: &QuitApp, cx: &mut App) {
    cx.quit();
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn app_menu_binding_specs_cover_standard_app_menu_commands() {
        let commands = APP_MENU_BINDING_SPECS
            .iter()
            .map(|spec| spec.command)
            .collect::<BTreeSet<_>>();

        for command in [
            AppMenuCommand::OpenPreferences,
            AppMenuCommand::HideApp,
            AppMenuCommand::HideOtherApps,
            AppMenuCommand::QuitApp,
        ] {
            assert!(
                commands.contains(&command),
                "missing app-menu binding for {command:?}"
            );
        }
    }

    #[test]
    fn app_menu_uses_standard_macos_key_equivalents() {
        let keys = APP_MENU_BINDING_SPECS
            .iter()
            .map(|spec| (spec.command, spec.keystroke))
            .collect::<BTreeSet<_>>();

        assert!(keys.contains(&(AppMenuCommand::OpenPreferences, "cmd-,")));
        assert!(keys.contains(&(AppMenuCommand::HideApp, "cmd-h")));
        assert!(keys.contains(&(AppMenuCommand::HideOtherApps, "cmd-alt-h")));
        assert!(keys.contains(&(AppMenuCommand::QuitApp, "cmd-q")));
    }

    #[test]
    fn app_menu_key_bindings_build_gpui_bindings() {
        assert_eq!(app_menu_key_bindings().len(), APP_MENU_BINDING_SPECS.len());
    }
}
