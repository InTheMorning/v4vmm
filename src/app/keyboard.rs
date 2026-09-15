//! Top-level keyboard shortcuts and focused-control precedence (ADRs 0067/0071).

use std::borrow::Cow;

use gpui::{actions, App, Context, KeyBinding, Window};

use crate::library::LibraryApp;

use super::{AppTab, TopApp};

pub(super) const ACTIVE_PANE_KEY_CONTEXT: &str = "ActivePane";
const ACTIVE_PANE_KEY_BINDING_CONTEXT: &str = "ActivePane && !Input";
const ACTIVE_PANE_CONFIRM_KEY_BINDING_CONTEXT: &str = "ActivePane && !Input && !ActionButton";

actions!(
    v4vmm,
    [
        TogglePlayback,
        SkipPlaybackNext,
        SkipPlaybackPrevious,
        FocusSearch,
        NewPlaylist,
        SelectMusicTab,
        SelectShowTab,
        SelectSettingsTab,
        RefreshLibrary,
        CancelActivePane,
        MoveSelectionUp,
        MoveSelectionDown,
        ConfirmSelection,
    ]
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum AppKeyCommand {
    TogglePlayback,
    SkipPlaybackNext,
    SkipPlaybackPrevious,
    FocusSearch,
    NewPlaylist,
    SelectMusicTab,
    SelectShowTab,
    SelectSettingsTab,
    RefreshLibrary,
    CancelActivePane,
    MoveSelectionUp,
    MoveSelectionDown,
    ConfirmSelection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AppKeyScope {
    Global,
    ActivePane,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AppKeyBindingSpec {
    pub(super) command: AppKeyCommand,
    pub(super) keystroke: &'static str,
    pub(super) label: &'static str,
    pub(super) scope: AppKeyScope,
}

pub(super) const APP_KEY_BINDING_SPECS: &[AppKeyBindingSpec] = &[
    AppKeyBindingSpec {
        command: AppKeyCommand::TogglePlayback,
        keystroke: "cmd-alt-p",
        label: "Play/Pause",
        scope: AppKeyScope::Global,
    },
    AppKeyBindingSpec {
        command: AppKeyCommand::SkipPlaybackNext,
        keystroke: "cmd-alt-right",
        label: "Next Track",
        scope: AppKeyScope::Global,
    },
    AppKeyBindingSpec {
        command: AppKeyCommand::SkipPlaybackPrevious,
        keystroke: "cmd-alt-left",
        label: "Previous Track",
        scope: AppKeyScope::Global,
    },
    AppKeyBindingSpec {
        command: AppKeyCommand::FocusSearch,
        keystroke: "cmd-alt-f",
        label: "Focus Search",
        scope: AppKeyScope::Global,
    },
    AppKeyBindingSpec {
        command: AppKeyCommand::FocusSearch,
        keystroke: "cmd-f",
        label: "Find",
        scope: AppKeyScope::Global,
    },
    AppKeyBindingSpec {
        command: AppKeyCommand::NewPlaylist,
        keystroke: "cmd-n",
        label: "New Playlist",
        scope: AppKeyScope::Global,
    },
    AppKeyBindingSpec {
        command: AppKeyCommand::SelectMusicTab,
        keystroke: "cmd-1",
        label: "Music",
        scope: AppKeyScope::Global,
    },
    AppKeyBindingSpec {
        command: AppKeyCommand::SelectShowTab,
        keystroke: "cmd-2",
        label: "Show",
        scope: AppKeyScope::Global,
    },
    AppKeyBindingSpec {
        command: AppKeyCommand::SelectSettingsTab,
        keystroke: "cmd-3",
        label: "Settings",
        scope: AppKeyScope::Global,
    },
    AppKeyBindingSpec {
        command: AppKeyCommand::RefreshLibrary,
        keystroke: "cmd-r",
        label: "Refresh Library",
        scope: AppKeyScope::Global,
    },
    AppKeyBindingSpec {
        command: AppKeyCommand::CancelActivePane,
        keystroke: "escape",
        label: "Back",
        scope: AppKeyScope::ActivePane,
    },
    AppKeyBindingSpec {
        command: AppKeyCommand::MoveSelectionUp,
        keystroke: "up",
        label: "Move Up",
        scope: AppKeyScope::ActivePane,
    },
    AppKeyBindingSpec {
        command: AppKeyCommand::MoveSelectionDown,
        keystroke: "down",
        label: "Move Down",
        scope: AppKeyScope::ActivePane,
    },
    AppKeyBindingSpec {
        command: AppKeyCommand::ConfirmSelection,
        keystroke: "enter",
        label: "Open",
        scope: AppKeyScope::ActivePane,
    },
];

pub(super) fn install_key_bindings(cx: &mut App) {
    cx.bind_keys(app_key_bindings());
}

fn app_key_bindings() -> Vec<KeyBinding> {
    app_key_bindings_for_platform(cfg!(target_os = "macos"))
}

fn app_key_bindings_for_platform(is_macos: bool) -> Vec<KeyBinding> {
    APP_KEY_BINDING_SPECS
        .iter()
        .map(|spec| spec.key_binding(is_macos))
        .collect()
}

/// Resolve the registry's Command notation to the desktop's primary modifier.
pub(super) fn platform_keystroke(keystroke: &str, is_macos: bool) -> Cow<'_, str> {
    if !is_macos {
        if let Some(key) = keystroke.strip_prefix("cmd-") {
            return Cow::Owned(format!("ctrl-{key}"));
        }
    }
    Cow::Borrowed(keystroke)
}

impl AppKeyBindingSpec {
    fn key_binding(&self, is_macos: bool) -> KeyBinding {
        let context = self.binding_context();
        let keystroke = platform_keystroke(self.keystroke, is_macos);
        let keystroke = keystroke.as_ref();

        match self.command {
            AppKeyCommand::TogglePlayback => KeyBinding::new(keystroke, TogglePlayback, context),
            AppKeyCommand::SkipPlaybackNext => {
                KeyBinding::new(keystroke, SkipPlaybackNext, context)
            }
            AppKeyCommand::SkipPlaybackPrevious => {
                KeyBinding::new(keystroke, SkipPlaybackPrevious, context)
            }
            AppKeyCommand::FocusSearch => KeyBinding::new(keystroke, FocusSearch, context),
            AppKeyCommand::NewPlaylist => KeyBinding::new(keystroke, NewPlaylist, context),
            AppKeyCommand::SelectMusicTab => KeyBinding::new(keystroke, SelectMusicTab, context),
            AppKeyCommand::SelectShowTab => KeyBinding::new(keystroke, SelectShowTab, context),
            AppKeyCommand::SelectSettingsTab => {
                KeyBinding::new(keystroke, SelectSettingsTab, context)
            }
            AppKeyCommand::RefreshLibrary => KeyBinding::new(keystroke, RefreshLibrary, context),
            AppKeyCommand::CancelActivePane => {
                KeyBinding::new(keystroke, CancelActivePane, context)
            }
            AppKeyCommand::MoveSelectionUp => KeyBinding::new(keystroke, MoveSelectionUp, context),
            AppKeyCommand::MoveSelectionDown => {
                KeyBinding::new(keystroke, MoveSelectionDown, context)
            }
            AppKeyCommand::ConfirmSelection => {
                KeyBinding::new(keystroke, ConfirmSelection, context)
            }
        }
    }

    fn binding_context(&self) -> Option<&'static str> {
        match self.scope {
            AppKeyScope::Global => None,
            AppKeyScope::ActivePane if self.command == AppKeyCommand::ConfirmSelection => {
                Some(ACTIVE_PANE_CONFIRM_KEY_BINDING_CONTEXT)
            }
            AppKeyScope::ActivePane => Some(ACTIVE_PANE_KEY_BINDING_CONTEXT),
        }
    }
}

impl TopApp {
    pub(super) fn handle_toggle_playback(
        &mut self,
        _: &TogglePlayback,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_playback_paused(cx);
    }

    pub(super) fn handle_skip_playback_next(
        &mut self,
        _: &SkipPlaybackNext,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.skip_playback_next(cx);
    }

    pub(super) fn handle_skip_playback_previous(
        &mut self,
        _: &SkipPlaybackPrevious,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.skip_playback_previous(cx);
    }

    pub(super) fn handle_focus_search(
        &mut self,
        _: &FocusSearch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_global_search(window, cx);
    }

    pub(super) fn handle_new_playlist(
        &mut self,
        _: &NewPlaylist,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_tab(AppTab::Music, window, cx);
        self.library
            .update(cx, |library, cx| library.begin_new_playlist(window, cx));
    }

    pub(super) fn handle_select_music_tab(
        &mut self,
        _: &SelectMusicTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_tab(AppTab::Music, window, cx);
    }

    pub(super) fn handle_select_settings_tab(
        &mut self,
        _: &SelectSettingsTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_tab(AppTab::Settings, window, cx);
    }

    pub(super) fn handle_select_show_tab(
        &mut self,
        _: &SelectShowTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_tab(AppTab::Show, window, cx);
    }

    pub(super) fn handle_refresh_library(
        &mut self,
        _: &RefreshLibrary,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.tab == AppTab::Music {
            self.library.update(cx, LibraryApp::refresh);
        }
    }

    pub(super) fn handle_cancel_active_pane(
        &mut self,
        _: &CancelActivePane,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.tab {
            AppTab::Music => {
                self.library.update(cx, LibraryApp::pop_inspector);
            }
            AppTab::Show | AppTab::Settings => {}
        }
    }

    pub(super) fn handle_move_selection_up(
        &mut self,
        _: &MoveSelectionUp,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.tab {
            AppTab::Music => self.library.update(cx, LibraryApp::move_up),
            AppTab::Show | AppTab::Settings => {}
        }
    }

    pub(super) fn handle_move_selection_down(
        &mut self,
        _: &MoveSelectionDown,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.tab {
            AppTab::Music => self.library.update(cx, LibraryApp::move_down),
            AppTab::Show | AppTab::Settings => {}
        }
    }

    pub(super) fn handle_confirm_selection(
        &mut self,
        _: &ConfirmSelection,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.tab {
            AppTab::Music => self.library.update(cx, LibraryApp::confirm),
            AppTab::Show | AppTab::Settings => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use gpui::{Action, KeyContext, Keymap, Keystroke};

    use super::*;

    /// Situational ADR 0071: app shortcuts must yield Enter to focused buttons.
    #[gpui::test]
    fn adr_0071_button_enter_precedes_active_pane_shortcut(cx: &mut gpui::TestAppContext) {
        use gpui::{div, prelude::*, Entity, FocusHandle, KeyDownEvent, KeyUpEvent, Render};
        use gpui_component::input::{Escape, Textarea, TextareaState};

        use crate::ui::primitives::Button;

        struct ButtonPane {
            input: Entity<TextareaState>,
            button_focus: FocusHandle,
            pane_focus: FocusHandle,
            activations: usize,
            confirmations: usize,
        }

        impl Render for ButtonPane {
            fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
                let entity = cx.entity();
                div()
                    .key_context(ACTIVE_PANE_KEY_CONTEXT)
                    .track_focus(&self.pane_focus)
                    .on_action(cx.listener(|this, _: &ConfirmSelection, _, _| {
                        this.confirmations += 1;
                    }))
                    .on_action(cx.listener(|this, _: &Escape, window, cx| {
                        cx.stop_propagation();
                        this.button_focus.focus(window, cx);
                    }))
                    .child(Textarea::new(&self.input))
                    .child(
                        Button::plain("editor-disclosure")
                            .label(if self.activations.is_multiple_of(2) {
                                "Close editor"
                            } else {
                                "Reopen editor"
                            })
                            .track_focus(&self.button_focus)
                            .on_activate(move |_, cx| {
                                entity.update(cx, |this, cx| {
                                    this.activations += 1;
                                    cx.notify();
                                });
                            }),
                    )
            }
        }

        cx.update(gpui_component::init);
        cx.update(install_key_bindings);
        let mut pane = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let content = cx.new(|cx| ButtonPane {
                input: cx.new(|cx| TextareaState::new(window, cx)),
                button_focus: cx.focus_handle(),
                pane_focus: cx.focus_handle(),
                activations: 0,
                confirmations: 0,
            });
            pane = Some(content.clone());
            gpui_component::Root::new(content, window, cx)
        });
        let pane = pane.unwrap();
        for (index, key) in ["enter", "space"].into_iter().enumerate() {
            cx.update(|window, cx| {
                pane.read(cx).input.clone().update(cx, |input, cx| {
                    input.set_value("draft", window, cx);
                    input.focus(window, cx);
                });
                window.draw(cx).clear(cx);
            });
            cx.simulate_keystrokes("ctrl-end enter escape");
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                assert!(pane.read(cx).button_focus.is_focused(window));
                assert_eq!(pane.read(cx).input.read(cx).value(), "draft\n");
            });

            let press = KeyDownEvent {
                keystroke: Keystroke::parse(key).unwrap(),
                is_held: false,
                prefer_character_input: false,
            };
            cx.simulate_event(press.clone());
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                assert_eq!(pane.read(cx).activations, index * 2 + 1, "{key} press");
                assert_eq!(pane.read(cx).confirmations, 0);
            });
            for _ in 0..2 {
                cx.simulate_event(KeyDownEvent {
                    is_held: true,
                    ..press.clone()
                });
            }
            cx.simulate_event(KeyUpEvent {
                keystroke: press.keystroke.clone(),
            });
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                assert_eq!(pane.read(cx).activations, index * 2 + 1, "{key} release");
                assert_eq!(pane.read(cx).confirmations, 0);
                assert_eq!(pane.read(cx).input.read(cx).value(), "draft\n");
            });
            cx.simulate_keystrokes(key);
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
                assert_eq!(pane.read(cx).activations, index * 2 + 2, "next {key} press");
                assert_eq!(pane.read(cx).confirmations, 0);
            });
        }
        cx.update(|window, cx| {
            pane.read(cx).pane_focus.clone().focus(window, cx);
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("enter");
        cx.update(|_, cx| assert_eq!(pane.read(cx).confirmations, 1));
    }

    #[gpui::test]
    fn adr_0071_log_menu_escape_precedes_active_pane_shortcut(cx: &mut gpui::TestAppContext) {
        use gpui::{
            div, point, prelude::*, px, ClipboardItem, Entity, Modifiers, MouseButton,
            MouseDownEvent, Render,
        };

        use crate::ui::composites::selectable_text::SelectableText;

        struct LogText;
        impl Render for LogText {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                SelectableText::new("test-log", "  café /tmp/👩‍💻.flac  ")
            }
        }

        struct PaneLogTest {
            root: Entity<gpui_component::Root>,
            pane_cancellations: usize,
        }

        impl Render for PaneLogTest {
            fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
                div()
                    .size_full()
                    .on_action(cx.listener(|this, _: &CancelActivePane, _, _| {
                        this.pane_cancellations += 1;
                    }))
                    .child(
                        div()
                            .key_context(ACTIVE_PANE_KEY_CONTEXT)
                            .size_full()
                            .child(self.root.clone()),
                    )
            }
        }

        cx.update(gpui_component::init);
        cx.update(crate::ui::primitives::context_menu::init);
        cx.update(install_key_bindings);
        let expected = "  café /tmp/👩‍💻.flac  ";
        let (pane, cx) = cx.add_window_view(|window, cx| PaneLogTest {
            root: cx.new(|cx| gpui_component::Root::new(cx.new(|_| LogText), window, cx)),
            pane_cancellations: 0,
        });
        cx.update(|window, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string("CLIPBOARD-KEEP".into()));
            window.draw(cx).clear(cx);
        });
        let position = point(px(35.), px(12.));
        cx.simulate_event(MouseDownEvent {
            position,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 3,
            first_mouse: false,
        });
        cx.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
        let log_focus = cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            window.focused(cx).expect("log selection focuses the log")
        });
        cx.simulate_mouse_down(position, MouseButton::Right, Modifiers::default());
        cx.simulate_mouse_up(position, MouseButton::Right, Modifiers::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("context-menu-items").is_some());

        cx.simulate_keystrokes("escape");
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(
            cx.debug_bounds("context-menu-items").is_none(),
            "Escape must dismiss the log menu with app shortcuts installed"
        );
        cx.update(|window, cx| {
            assert_eq!(pane.read(cx).pane_cancellations, 0);
            assert!(log_focus.is_focused(window));
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().as_deref(),
                Some("CLIPBOARD-KEEP")
            );
            #[cfg(target_os = "linux")]
            assert_eq!(
                cx.read_from_primary().unwrap().text().as_deref(),
                Some(expected)
            );
        });
        cx.simulate_keystrokes("ctrl-c");
        cx.update(|_, cx| {
            assert_eq!(
                cx.read_from_clipboard().unwrap().text().as_deref(),
                Some(expected)
            );
        });
        cx.simulate_keystrokes("escape");
        cx.update(|_, cx| {
            assert_eq!(
                pane.read(cx).pane_cancellations,
                1,
                "closed menus must not intercept Escape"
            );
        });
    }

    fn assert_action<A: Action>(keymap: &Keymap, key: &str, contexts: &[KeyContext]) {
        let (bindings, pending) =
            keymap.bindings_for_input(&[Keystroke::parse(key).expect("test key parses")], contexts);
        assert!(!pending, "{key} must be a complete shortcut");
        assert!(
            bindings
                .first()
                .is_some_and(|binding| binding.action().as_any().is::<A>()),
            "ADR 0067: {key} must resolve first to {}",
            std::any::type_name::<A>()
        );
    }

    #[test]
    fn adr_0067_platform_shortcuts_route_with_and_without_input_focus() {
        use super::super::menu::{app_menu_key_bindings_for_platform, OpenPreferences, QuitApp};

        for (is_macos, primary) in [(false, "ctrl"), (true, "cmd")] {
            let mut keymap = Keymap::default();
            // The input widget registers Find before app bootstrap. The app's
            // toolbar search must still win with an input focused.
            keymap.add_bindings([KeyBinding::new(
                &format!("{primary}-f"),
                gpui_component::input::Search,
                Some("Input"),
            )]);
            keymap.add_bindings(app_key_bindings_for_platform(is_macos));
            keymap.add_bindings(app_menu_key_bindings_for_platform(is_macos));
            for contexts in [
                vec![KeyContext::parse("ActivePane").unwrap()],
                vec![
                    KeyContext::parse("ActivePane").unwrap(),
                    KeyContext::parse("Input").unwrap(),
                ],
            ] {
                assert_action::<FocusSearch>(&keymap, &format!("{primary}-f"), &contexts);
                assert_action::<FocusSearch>(&keymap, &format!("{primary}-alt-f"), &contexts);
                assert_action::<RefreshLibrary>(&keymap, &format!("{primary}-r"), &contexts);
                assert_action::<TogglePlayback>(&keymap, &format!("{primary}-alt-p"), &contexts);
                assert_action::<SkipPlaybackNext>(
                    &keymap,
                    &format!("{primary}-alt-right"),
                    &contexts,
                );
                assert_action::<SkipPlaybackPrevious>(
                    &keymap,
                    &format!("{primary}-alt-left"),
                    &contexts,
                );
                assert_action::<NewPlaylist>(&keymap, &format!("{primary}-n"), &contexts);
                assert_action::<SelectMusicTab>(&keymap, &format!("{primary}-1"), &contexts);
                assert_action::<SelectShowTab>(&keymap, &format!("{primary}-2"), &contexts);
                assert_action::<SelectSettingsTab>(&keymap, &format!("{primary}-3"), &contexts);
                assert_action::<OpenPreferences>(&keymap, &format!("{primary}-,"), &contexts);
                assert_action::<QuitApp>(&keymap, &format!("{primary}-q"), &contexts);
            }
        }
    }

    #[test]
    fn adr_0067_linux_shortcuts_preserve_text_editing() {
        use gpui_component::input::{
            Backspace, Copy, Cut, Enter, Escape, MoveDown, MoveToNextWord, MoveToPreviousWord,
            MoveUp, Paste, Redo, SelectAll, Undo,
        };

        let mut keymap = Keymap::default();
        // Representative input-owned bindings; bootstrap installs app bindings
        // later, so accidental collisions would override them in this test.
        keymap.add_bindings([
            KeyBinding::new("ctrl-a", SelectAll, Some("Input")),
            KeyBinding::new("ctrl-c", Copy, Some("Input")),
            KeyBinding::new("ctrl-x", Cut, Some("Input")),
            KeyBinding::new("ctrl-v", Paste, Some("Input")),
            KeyBinding::new("ctrl-z", Undo, Some("Input")),
            KeyBinding::new("ctrl-y", Redo, Some("Input")),
            KeyBinding::new("ctrl-left", MoveToPreviousWord, Some("Input")),
            KeyBinding::new("ctrl-right", MoveToNextWord, Some("Input")),
            KeyBinding::new("ctrl-h", Backspace, Some("Input")),
            KeyBinding::new(
                "enter",
                Enter {
                    secondary: false,
                    shift: false,
                },
                Some("Input"),
            ),
            KeyBinding::new("escape", Escape, Some("Input")),
            KeyBinding::new("up", MoveUp, Some("Input")),
            KeyBinding::new("down", MoveDown, Some("Input")),
        ]);
        keymap.add_bindings(app_key_bindings_for_platform(false));
        keymap.add_bindings(super::super::menu::app_menu_key_bindings_for_platform(
            false,
        ));
        let contexts = [
            KeyContext::parse("ActivePane").unwrap(),
            KeyContext::parse("Input").unwrap(),
        ];
        assert_action::<SelectAll>(&keymap, "ctrl-a", &contexts);
        assert_action::<Copy>(&keymap, "ctrl-c", &contexts);
        assert_action::<Cut>(&keymap, "ctrl-x", &contexts);
        assert_action::<Paste>(&keymap, "ctrl-v", &contexts);
        assert_action::<Undo>(&keymap, "ctrl-z", &contexts);
        assert_action::<Redo>(&keymap, "ctrl-y", &contexts);
        assert_action::<MoveToPreviousWord>(&keymap, "ctrl-left", &contexts);
        assert_action::<MoveToNextWord>(&keymap, "ctrl-right", &contexts);
        assert_action::<Backspace>(&keymap, "ctrl-h", &contexts);
        assert_action::<Enter>(&keymap, "enter", &contexts);
        assert_action::<Escape>(&keymap, "escape", &contexts);
        assert_action::<MoveUp>(&keymap, "up", &contexts);
        assert_action::<MoveDown>(&keymap, "down", &contexts);
    }

    #[test]
    fn adr_0067_linux_has_no_super_or_duplicate_app_bindings() {
        let mut keys = BTreeSet::new();
        for binding in app_key_bindings_for_platform(false).into_iter().chain(
            super::super::menu::app_menu_key_bindings_for_platform(false),
        ) {
            let strokes = binding.keystrokes();
            assert!(
                strokes.iter().all(|stroke| !stroke.modifiers().platform),
                "ADR 0067: Linux app shortcuts must not use Super"
            );
            assert!(
                keys.insert(format!("{strokes:?}")),
                "ADR 0067: app and menu registries must not duplicate shortcuts"
            );
        }
    }

    #[test]
    fn key_binding_taxonomy_covers_core_commands() {
        let commands = APP_KEY_BINDING_SPECS
            .iter()
            .map(|spec| spec.command)
            .collect::<BTreeSet<_>>();

        for command in [
            AppKeyCommand::TogglePlayback,
            AppKeyCommand::SkipPlaybackNext,
            AppKeyCommand::SkipPlaybackPrevious,
            AppKeyCommand::FocusSearch,
            AppKeyCommand::NewPlaylist,
            AppKeyCommand::SelectMusicTab,
            AppKeyCommand::SelectShowTab,
            AppKeyCommand::SelectSettingsTab,
            AppKeyCommand::CancelActivePane,
            AppKeyCommand::MoveSelectionUp,
            AppKeyCommand::MoveSelectionDown,
            AppKeyCommand::ConfirmSelection,
        ] {
            assert!(
                commands.contains(&command),
                "missing key binding for {command:?}"
            );
        }
    }

    #[test]
    fn key_binding_keystrokes_are_unique() {
        let mut seen = BTreeSet::new();

        for spec in APP_KEY_BINDING_SPECS {
            assert!(
                seen.insert(spec.keystroke),
                "duplicate key binding {}",
                spec.keystroke
            );
        }
    }

    #[test]
    fn search_focus_keeps_standard_find_and_jump_variants() {
        let search_keys = APP_KEY_BINDING_SPECS
            .iter()
            .filter(|spec| spec.command == AppKeyCommand::FocusSearch)
            .map(|spec| spec.keystroke)
            .collect::<BTreeSet<_>>();

        assert!(search_keys.contains("cmd-f"));
        assert!(search_keys.contains("cmd-alt-f"));
    }

    #[test]
    fn app_key_bindings_build_gpui_bindings() {
        assert_eq!(app_key_bindings().len(), APP_KEY_BINDING_SPECS.len());
    }

    #[test]
    fn active_pane_key_bindings_are_context_scoped() {
        for spec in APP_KEY_BINDING_SPECS {
            let expected_context = match spec.scope {
                AppKeyScope::Global => None,
                AppKeyScope::ActivePane if spec.command == AppKeyCommand::ConfirmSelection => {
                    Some(ACTIVE_PANE_CONFIRM_KEY_BINDING_CONTEXT)
                }
                AppKeyScope::ActivePane => Some(ACTIVE_PANE_KEY_BINDING_CONTEXT),
            };

            assert_eq!(
                spec.binding_context(),
                expected_context,
                "unexpected context for {command:?}",
                command = spec.command
            );
        }
    }

    #[test]
    fn active_pane_enter_does_not_shadow_text_input_context() {
        let mut keymap = Keymap::default();
        let enter = [Keystroke::parse("enter").expect("enter keystroke parses")];
        let active_pane_context =
            [KeyContext::parse(ACTIVE_PANE_KEY_CONTEXT).expect("active pane context parses")];
        let input_context = [
            KeyContext::parse(ACTIVE_PANE_KEY_CONTEXT).expect("active pane context parses"),
            KeyContext::parse("Input").expect("input context parses"),
        ];

        keymap.add_bindings(app_key_bindings());

        assert!(
            !keymap
                .bindings_for_input(&enter, &active_pane_context)
                .0
                .is_empty(),
            "enter should remain available to active panes"
        );
        assert!(
            keymap
                .bindings_for_input(&enter, &input_context)
                .0
                .is_empty(),
            "enter should be reserved for the focused input"
        );
    }
}
