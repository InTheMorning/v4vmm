//! Platform application shortcuts and macOS menu bar bootstrap (ADR 0067).
//!
//! GPUI exposes platform menus as action-backed menu items. This module keeps
//! the app-menu contract centralized so standard macOS commands stay visible
//! and pick up their key equivalents from the same keymap as the rest of the
//! application.

use gpui::{actions, App, Context, KeyBinding, Menu, MenuItem, SystemMenuType, Window};

use super::{keyboard::platform_keystroke, AppTab, TopApp};

const APP_NAME: &str = "Application";

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
        label: "Hide Application",
    },
    AppMenuBindingSpec {
        command: AppMenuCommand::HideOtherApps,
        keystroke: "cmd-alt-h",
        label: "Hide Others",
    },
    AppMenuBindingSpec {
        command: AppMenuCommand::QuitApp,
        keystroke: "cmd-q",
        label: "Quit Application",
    },
];

pub(super) fn install_app_menu(cx: &mut App) {
    cx.bind_keys(app_menu_key_bindings());
    cx.on_action(handle_quit_app);
    if cfg!(target_os = "macos") {
        cx.on_action(handle_hide_app);
        cx.on_action(handle_hide_other_apps);
        cx.on_action(handle_show_all_apps);
        cx.set_menus(app_menus());
    }
}

fn app_menu_key_bindings() -> Vec<KeyBinding> {
    app_menu_key_bindings_for_platform(cfg!(target_os = "macos"))
}

pub(super) fn app_menu_key_bindings_for_platform(is_macos: bool) -> Vec<KeyBinding> {
    APP_MENU_BINDING_SPECS
        .iter()
        .filter(|spec| {
            is_macos
                || matches!(
                    spec.command,
                    AppMenuCommand::OpenPreferences | AppMenuCommand::QuitApp
                )
        })
        .map(|spec| spec.key_binding(is_macos))
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
            MenuItem::action("Hide Application", HideApp),
            MenuItem::action("Hide Others", HideOtherApps),
            MenuItem::action("Show All", ShowAllApps),
            MenuItem::separator(),
            MenuItem::action("Quit Application", QuitApp),
        ],
    }]
}

impl AppMenuBindingSpec {
    fn key_binding(&self, is_macos: bool) -> KeyBinding {
        let keystroke = platform_keystroke(self.keystroke, is_macos);
        let keystroke = keystroke.as_ref();
        match self.command {
            AppMenuCommand::OpenPreferences => KeyBinding::new(keystroke, OpenPreferences, None),
            AppMenuCommand::HideApp => KeyBinding::new(keystroke, HideApp, None),
            AppMenuCommand::HideOtherApps => KeyBinding::new(keystroke, HideOtherApps, None),
            AppMenuCommand::QuitApp => KeyBinding::new(keystroke, QuitApp, None),
        }
    }
}

impl TopApp {
    pub(super) fn handle_open_preferences(
        &mut self,
        _: &OpenPreferences,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_tab(AppTab::Settings, window, cx);
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
        let expected = if cfg!(target_os = "macos") { 4 } else { 2 };
        assert_eq!(app_menu_key_bindings().len(), expected);
    }

    #[test]
    fn adr_0067_hide_commands_remain_macos_only() {
        for is_macos in [false, true] {
            let bindings = app_menu_key_bindings_for_platform(is_macos);
            assert_eq!(bindings.len(), if is_macos { 4 } else { 2 });
            assert_eq!(
                bindings
                    .iter()
                    .any(|binding| binding.action().as_any().is::<HideApp>()),
                is_macos,
                "ADR 0067: Hide must not claim Ctrl+H on Linux"
            );
            assert_eq!(
                bindings
                    .iter()
                    .any(|binding| binding.action().as_any().is::<HideOtherApps>()),
                is_macos
            );
        }
    }
}
