//! Top-level keyboard routing for presentation tabs.

use gpui::{Context, KeyDownEvent, Window};

use crate::library::LibraryApp;
use crate::search::SearchApp;

use super::{AppTab, TopApp};

impl TopApp {
    pub(super) fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let modifiers = event.keystroke.modifiers;
        let key = event.keystroke.key.as_str();

        if modifiers.platform {
            self.handle_platform_key(key, window, cx);
        } else {
            self.handle_plain_key(key, cx);
        }
    }

    fn handle_platform_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        match key {
            "1" => self.select_tab(AppTab::Library, cx),
            "2" => self.select_tab(AppTab::Discover, cx),
            "3" => self.select_tab(AppTab::Settings, cx),
            "f" => self.focus_active_search(window, cx),
            "r" => {
                if self.tab == AppTab::Library {
                    self.library.update(cx, LibraryApp::refresh);
                }
            }
            _ => {}
        }
    }

    fn handle_plain_key(&mut self, key: &str, cx: &mut Context<Self>) {
        match key {
            "escape" => match self.tab {
                AppTab::Library => {
                    self.library.update(cx, LibraryApp::pop_inspector);
                }
                AppTab::Discover => {
                    self.search.update(cx, SearchApp::pop_inspector);
                }
                AppTab::Settings => {}
            },
            "up" => match self.tab {
                AppTab::Library => self.library.update(cx, LibraryApp::move_up),
                AppTab::Discover => self.search.update(cx, SearchApp::move_up),
                AppTab::Settings => {}
            },
            "down" => match self.tab {
                AppTab::Library => self.library.update(cx, LibraryApp::move_down),
                AppTab::Discover => self.search.update(cx, SearchApp::move_down),
                AppTab::Settings => {}
            },
            "enter" => match self.tab {
                AppTab::Library => self.library.update(cx, LibraryApp::confirm),
                AppTab::Discover => self.search.update(cx, SearchApp::confirm),
                AppTab::Settings => {}
            },
            _ => {}
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
