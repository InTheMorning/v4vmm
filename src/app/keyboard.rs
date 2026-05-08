//! Top-level keyboard shortcut taxonomy and routing.

use gpui::{actions, App, Context, KeyBinding, Window};

use crate::library::LibraryApp;
use crate::search::SearchApp;

use super::{AppTab, TopApp};

pub(super) const ACTIVE_PANE_KEY_CONTEXT: &str = "ActivePane";
const ACTIVE_PANE_KEY_BINDING_CONTEXT: &str = "ActivePane && !Input";

actions!(
    v4vmm,
    [
        TogglePlayback,
        SkipPlaybackNext,
        SkipPlaybackPrevious,
        FocusSearch,
        NewPlaylist,
        SelectLibraryTab,
        SelectDiscoverTab,
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
    SelectLibraryTab,
    SelectDiscoverTab,
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
        command: AppKeyCommand::SelectLibraryTab,
        keystroke: "cmd-1",
        label: "Library",
        scope: AppKeyScope::Global,
    },
    AppKeyBindingSpec {
        command: AppKeyCommand::SelectDiscoverTab,
        keystroke: "cmd-2",
        label: "Discover",
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
    APP_KEY_BINDING_SPECS
        .iter()
        .map(AppKeyBindingSpec::key_binding)
        .collect()
}

impl AppKeyBindingSpec {
    fn key_binding(&self) -> KeyBinding {
        let context = self.binding_context();

        match self.command {
            AppKeyCommand::TogglePlayback => {
                KeyBinding::new(self.keystroke, TogglePlayback, context)
            }
            AppKeyCommand::SkipPlaybackNext => {
                KeyBinding::new(self.keystroke, SkipPlaybackNext, context)
            }
            AppKeyCommand::SkipPlaybackPrevious => {
                KeyBinding::new(self.keystroke, SkipPlaybackPrevious, context)
            }
            AppKeyCommand::FocusSearch => KeyBinding::new(self.keystroke, FocusSearch, context),
            AppKeyCommand::NewPlaylist => KeyBinding::new(self.keystroke, NewPlaylist, context),
            AppKeyCommand::SelectLibraryTab => {
                KeyBinding::new(self.keystroke, SelectLibraryTab, context)
            }
            AppKeyCommand::SelectDiscoverTab => {
                KeyBinding::new(self.keystroke, SelectDiscoverTab, context)
            }
            AppKeyCommand::SelectSettingsTab => {
                KeyBinding::new(self.keystroke, SelectSettingsTab, context)
            }
            AppKeyCommand::RefreshLibrary => {
                KeyBinding::new(self.keystroke, RefreshLibrary, context)
            }
            AppKeyCommand::CancelActivePane => {
                KeyBinding::new(self.keystroke, CancelActivePane, context)
            }
            AppKeyCommand::MoveSelectionUp => {
                KeyBinding::new(self.keystroke, MoveSelectionUp, context)
            }
            AppKeyCommand::MoveSelectionDown => {
                KeyBinding::new(self.keystroke, MoveSelectionDown, context)
            }
            AppKeyCommand::ConfirmSelection => {
                KeyBinding::new(self.keystroke, ConfirmSelection, context)
            }
        }
    }

    fn binding_context(&self) -> Option<&'static str> {
        match self.scope {
            AppKeyScope::Global => None,
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
        self.focus_active_search(window, cx);
    }

    pub(super) fn handle_new_playlist(
        &mut self,
        _: &NewPlaylist,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_tab(AppTab::Library, cx);
        self.library
            .update(cx, |library, cx| library.begin_new_playlist(window, cx));
    }

    pub(super) fn handle_select_library_tab(
        &mut self,
        _: &SelectLibraryTab,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_tab(AppTab::Library, cx);
    }

    pub(super) fn handle_select_discover_tab(
        &mut self,
        _: &SelectDiscoverTab,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_tab(AppTab::Discover, cx);
    }

    pub(super) fn handle_select_settings_tab(
        &mut self,
        _: &SelectSettingsTab,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_tab(AppTab::Settings, cx);
    }

    pub(super) fn handle_refresh_library(
        &mut self,
        _: &RefreshLibrary,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.tab == AppTab::Library {
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
            AppTab::Library => {
                self.library.update(cx, LibraryApp::pop_inspector);
            }
            AppTab::Discover => {
                self.search.update(cx, SearchApp::pop_inspector);
            }
            AppTab::Settings => {}
        }
    }

    pub(super) fn handle_move_selection_up(
        &mut self,
        _: &MoveSelectionUp,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.tab {
            AppTab::Library => self.library.update(cx, LibraryApp::move_up),
            AppTab::Discover => self.search.update(cx, SearchApp::move_up),
            AppTab::Settings => {}
        }
    }

    pub(super) fn handle_move_selection_down(
        &mut self,
        _: &MoveSelectionDown,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.tab {
            AppTab::Library => self.library.update(cx, LibraryApp::move_down),
            AppTab::Discover => self.search.update(cx, SearchApp::move_down),
            AppTab::Settings => {}
        }
    }

    pub(super) fn handle_confirm_selection(
        &mut self,
        _: &ConfirmSelection,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.tab {
            AppTab::Library => self.library.update(cx, LibraryApp::confirm),
            AppTab::Discover => self.search.update(cx, SearchApp::confirm),
            AppTab::Settings => {}
        }
    }

    fn select_tab(&mut self, tab: AppTab, cx: &mut Context<Self>) {
        self.tab = tab;
        cx.notify();
    }

    fn focus_active_search(&self, window: &mut Window, cx: &mut Context<Self>) {
        match self.tab {
            AppTab::Library => {
                self.library
                    .update(cx, |library, cx| library.focus_search(window, cx));
            }
            AppTab::Discover => {
                self.search
                    .update(cx, |search, cx| search.focus_search(window, cx));
            }
            AppTab::Settings => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use gpui::{KeyContext, Keymap, Keystroke};

    use super::*;

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
            AppKeyCommand::SelectLibraryTab,
            AppKeyCommand::SelectDiscoverTab,
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
