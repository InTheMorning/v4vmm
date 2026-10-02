//! Platform application shortcuts and macOS menu bar bootstrap (ADR 0067).
//!
//! GPUI exposes platform menus as action-backed menu items. This module keeps
//! the app-menu contract centralized so standard macOS commands stay visible
//! and pick up their key equivalents from the same keymap as the rest of the
//! application.
//!
//! `NavigateBack` and `NavigateForward` are ADR 0046 Task 015's frame-history
//! keys, added by the operator's decision of 2026-10-02. They are app menu
//! commands only in the sense that they share this module's binding table
//! and platform split. They carry no macOS menu bar item (the task's own
//! Exclusions name "no menu").

use std::borrow::Cow;

use gpui::{actions, App, Context, KeyBinding, Menu, MenuItem, SystemMenuType, Window};

use super::{keyboard::platform_keystroke, AppTab, TopApp};

const APP_NAME: &str = "Application";

/// Context predicate that keeps a macOS Back or Forward bind out of a
/// focused text box, so the box keeps its own key at the same keystroke
/// (ADR 0046 Task 015, operator decision 2026-10-02). `Input` is the
/// context name gpui-base's input state sets on its text box
/// (`gpui-base-0.6.1/src/input/base/state.rs`, constant `CONTEXT`).
const TEXT_BOX_EXCLUDED_CONTEXT: &str = "!Input";

actions!(
    v4vmm,
    [
        OpenPreferences,
        HideApp,
        HideOtherApps,
        ShowAllApps,
        QuitApp,
        NavigateBack,
        NavigateForward,
    ]
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum AppMenuCommand {
    OpenPreferences,
    HideApp,
    HideOtherApps,
    QuitApp,
    NavigateBack,
    NavigateForward,
}

/// Keystroke data for one binding spec entry.
///
/// `Shared` keeps every original binding's shape: one keystroke, converted
/// for the current platform by [`platform_keystroke`], with no context.
/// `PerPlatform` is for a bind whose macOS keystroke and context differ from
/// its keystroke and context on every other platform (ADR 0046 Task 015).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AppMenuKeystroke {
    /// One keystroke, run through [`platform_keystroke`].
    Shared(&'static str),
    /// A distinct keystroke and context for macOS, and for every other
    /// platform.
    PerPlatform {
        macos_keystroke: &'static str,
        macos_context: Option<&'static str>,
        other_keystroke: &'static str,
        other_context: Option<&'static str>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AppMenuBindingSpec {
    pub(super) command: AppMenuCommand,
    pub(super) keys: AppMenuKeystroke,
    pub(super) label: &'static str,
}

pub(super) const APP_MENU_BINDING_SPECS: &[AppMenuBindingSpec] = &[
    AppMenuBindingSpec {
        command: AppMenuCommand::OpenPreferences,
        keys: AppMenuKeystroke::Shared("cmd-,"),
        label: "Preferences...",
    },
    AppMenuBindingSpec {
        command: AppMenuCommand::HideApp,
        keys: AppMenuKeystroke::Shared("cmd-h"),
        label: "Hide Application",
    },
    AppMenuBindingSpec {
        command: AppMenuCommand::HideOtherApps,
        keys: AppMenuKeystroke::Shared("cmd-alt-h"),
        label: "Hide Others",
    },
    AppMenuBindingSpec {
        command: AppMenuCommand::QuitApp,
        keys: AppMenuKeystroke::Shared("cmd-q"),
        label: "Quit Application",
    },
    AppMenuBindingSpec {
        command: AppMenuCommand::NavigateBack,
        keys: AppMenuKeystroke::PerPlatform {
            macos_keystroke: "cmd-[",
            macos_context: Some(TEXT_BOX_EXCLUDED_CONTEXT),
            other_keystroke: "alt-left",
            other_context: None,
        },
        label: "Back",
    },
    AppMenuBindingSpec {
        command: AppMenuCommand::NavigateForward,
        keys: AppMenuKeystroke::PerPlatform {
            macos_keystroke: "cmd-]",
            macos_context: Some(TEXT_BOX_EXCLUDED_CONTEXT),
            other_keystroke: "alt-right",
            other_context: None,
        },
        label: "Forward",
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
                    AppMenuCommand::OpenPreferences
                        | AppMenuCommand::QuitApp
                        | AppMenuCommand::NavigateBack
                        | AppMenuCommand::NavigateForward
                )
        })
        .map(|spec| spec.key_binding(is_macos))
        .collect()
}

fn app_menus() -> Vec<Menu> {
    vec![Menu {
        name: APP_NAME.into(),
        disabled: false,
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
        let (keystroke, context): (Cow<'_, str>, Option<&'static str>) = match self.keys {
            AppMenuKeystroke::Shared(keystroke) => (platform_keystroke(keystroke, is_macos), None),
            AppMenuKeystroke::PerPlatform {
                macos_keystroke,
                macos_context,
                ..
            } if is_macos => (Cow::Borrowed(macos_keystroke), macos_context),
            AppMenuKeystroke::PerPlatform {
                other_keystroke,
                other_context,
                ..
            } => (Cow::Borrowed(other_keystroke), other_context),
        };
        let keystroke = keystroke.as_ref();
        match self.command {
            AppMenuCommand::OpenPreferences => KeyBinding::new(keystroke, OpenPreferences, context),
            AppMenuCommand::HideApp => KeyBinding::new(keystroke, HideApp, context),
            AppMenuCommand::HideOtherApps => KeyBinding::new(keystroke, HideOtherApps, context),
            AppMenuCommand::QuitApp => KeyBinding::new(keystroke, QuitApp, context),
            AppMenuCommand::NavigateBack => KeyBinding::new(keystroke, NavigateBack, context),
            AppMenuCommand::NavigateForward => KeyBinding::new(keystroke, NavigateForward, context),
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

    /// Routes the Back key to the same path the Back button uses (ADR 0046
    /// Task 015). The key does nothing when Back has no history, because
    /// `handle_content_list_back_select` itself does nothing then.
    pub(super) fn handle_navigate_back(
        &mut self,
        _: &NavigateBack,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_content_list_back_select(cx);
    }

    /// Routes the Forward key to the same path the Forward button uses (ADR
    /// 0046 Task 015). The key does nothing when Forward has no history,
    /// because `handle_content_list_forward_select` itself does nothing
    /// then.
    pub(super) fn handle_navigate_forward(
        &mut self,
        _: &NavigateForward,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_content_list_forward_select(cx);
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
            .filter_map(|spec| match spec.keys {
                AppMenuKeystroke::Shared(keystroke) => Some((spec.command, keystroke)),
                AppMenuKeystroke::PerPlatform { .. } => None,
            })
            .collect::<BTreeSet<_>>();

        assert!(keys.contains(&(AppMenuCommand::OpenPreferences, "cmd-,")));
        assert!(keys.contains(&(AppMenuCommand::HideApp, "cmd-h")));
        assert!(keys.contains(&(AppMenuCommand::HideOtherApps, "cmd-alt-h")));
        assert!(keys.contains(&(AppMenuCommand::QuitApp, "cmd-q")));
    }

    #[test]
    fn app_menu_key_bindings_build_gpui_bindings() {
        let expected = if cfg!(target_os = "macos") { 6 } else { 4 };
        assert_eq!(app_menu_key_bindings().len(), expected);
    }

    #[test]
    fn adr_0067_hide_commands_remain_macos_only() {
        for is_macos in [false, true] {
            let bindings = app_menu_key_bindings_for_platform(is_macos);
            assert_eq!(bindings.len(), if is_macos { 6 } else { 4 });
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

    fn assert_action<A: gpui::Action>(
        keymap: &gpui::Keymap,
        key: &str,
        contexts: &[gpui::KeyContext],
    ) {
        let (bindings, pending) = keymap.bindings_for_input(
            &[gpui::Keystroke::parse(key).expect("test key parses")],
            contexts,
        );
        assert!(!pending, "{key} must be a complete shortcut");
        assert!(
            bindings
                .first()
                .is_some_and(|binding| binding.action().as_any().is::<A>()),
            "ADR 0046 Task 015: {key} must resolve first to {}",
            std::any::type_name::<A>()
        );
    }

    fn active_pane_context() -> Vec<gpui::KeyContext> {
        vec![gpui::KeyContext::parse("ActivePane").expect("test context parses")]
    }

    fn active_pane_with_input_context() -> Vec<gpui::KeyContext> {
        vec![
            gpui::KeyContext::parse("ActivePane").expect("test context parses"),
            gpui::KeyContext::parse("Input").expect("test context parses"),
        ]
    }

    /// R15-04 (ADR 0046 Task 015): the macOS pair resolves to Back and
    /// Forward outside a text box.
    #[test]
    fn adr_0046_forward_macos_navigation_bindings_resolve_outside_a_text_box() {
        let mut keymap = gpui::Keymap::default();
        keymap.add_bindings(app_menu_key_bindings_for_platform(true));

        assert_action::<NavigateBack>(&keymap, "cmd-[", &active_pane_context());
        assert_action::<NavigateForward>(&keymap, "cmd-]", &active_pane_context());
    }

    /// R15-04 (ADR 0046 Task 015): the macOS pair carries the negated
    /// `Input` context, so a focused text box keeps its own `cmd-[` and
    /// `cmd-]` keys (here stood in by `gpui_component::input::Outdent` and
    /// `Indent`, the same actions gpui-base's own text box binds at these
    /// keystrokes).
    #[test]
    fn adr_0046_forward_macos_navigation_bindings_stay_out_of_a_text_box() {
        let mut keymap = gpui::Keymap::default();
        keymap.add_bindings([
            KeyBinding::new("cmd-[", gpui_component::input::Outdent, Some("Input")),
            KeyBinding::new("cmd-]", gpui_component::input::Indent, Some("Input")),
        ]);
        keymap.add_bindings(app_menu_key_bindings_for_platform(true));

        let text_box_context = active_pane_with_input_context();
        assert_action::<gpui_component::input::Outdent>(&keymap, "cmd-[", &text_box_context);
        assert_action::<gpui_component::input::Indent>(&keymap, "cmd-]", &text_box_context);
    }

    /// R15-04 (ADR 0046 Task 015): the pair for every other platform
    /// resolves to Back and Forward, both outside and inside a text box.
    /// This system's own text box moves by word on `ctrl-left`/`ctrl-right`,
    /// not `alt-left`/`alt-right`, so this pair carries no context.
    #[test]
    fn adr_0046_forward_other_platform_navigation_bindings_resolve_in_and_out_of_a_text_box() {
        let mut keymap = gpui::Keymap::default();
        keymap.add_bindings(app_menu_key_bindings_for_platform(false));

        for contexts in [active_pane_context(), active_pane_with_input_context()] {
            assert_action::<NavigateBack>(&keymap, "alt-left", &contexts);
            assert_action::<NavigateForward>(&keymap, "alt-right", &contexts);
        }
    }

    /// R15-04 (ADR 0046 Task 015): the new Back and Forward keystrokes
    /// repeat no keystroke the app's own key bindings already use, on
    /// either platform.
    #[test]
    fn adr_0046_forward_navigation_keys_do_not_duplicate_an_existing_app_binding() {
        use super::super::keyboard::APP_KEY_BINDING_SPECS;

        for is_macos in [false, true] {
            let mut keystrokes: Vec<String> = APP_KEY_BINDING_SPECS
                .iter()
                .map(|spec| platform_keystroke(spec.keystroke, is_macos).into_owned())
                .collect();
            keystrokes.extend(APP_MENU_BINDING_SPECS.iter().map(|spec| match spec.keys {
                AppMenuKeystroke::Shared(keystroke) => {
                    platform_keystroke(keystroke, is_macos).into_owned()
                }
                AppMenuKeystroke::PerPlatform {
                    macos_keystroke,
                    other_keystroke,
                    ..
                } => {
                    if is_macos {
                        macos_keystroke.to_string()
                    } else {
                        other_keystroke.to_string()
                    }
                }
            }));

            let unique = keystrokes.iter().cloned().collect::<BTreeSet<_>>();
            assert_eq!(
                unique.len(),
                keystrokes.len(),
                "ADR 0046 Task 015: a binding keystroke repeats on {}: {:?}",
                if is_macos {
                    "macOS"
                } else {
                    "the other platforms"
                },
                keystrokes
            );
        }
    }
}
