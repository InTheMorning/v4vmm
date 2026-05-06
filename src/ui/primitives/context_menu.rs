//! Context-menu primitive for row-level commands.
//!
//! GPUI does not currently expose a native arbitrary-element context-menu API,
//! so this primitive uses the shared [`Popover`] infrastructure while owning
//! the menu row chrome and action contract. Screens should pass display-ready
//! menu items instead of hand-rolling floating row action panels.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{div, prelude::*, App, Entity, IntoElement, RenderOnce, SharedString, Window};

use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::IconName;
use crate::ui::primitives::{Button, Popover, PopoverAlignment, PopoverPlacement};
use crate::ui::tokens::{Size, Spacing};

type SelectHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

struct ContextMenuState {
    open: bool,
}

/// Row families expected to expose context-menu commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextMenuScope {
    FeedList,
    TrackList,
    PlaylistTrack,
}

/// Display-ready fields for one context-menu item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextMenuItemDisplay {
    pub id: SharedString,
    pub label: SharedString,
    pub a11y_label: SharedString,
    pub destructive: bool,
    pub disabled: bool,
}

/// One selectable menu item in a [`ContextMenu`].
#[derive(Clone)]
#[must_use]
pub struct ContextMenuItem {
    display: ContextMenuItemDisplay,
    on_select: Option<SelectHandler>,
}

/// Shared context-menu surface.
#[derive(IntoElement)]
#[must_use]
pub struct ContextMenu {
    id: SharedString,
    scope: ContextMenuScope,
    trigger_label: SharedString,
    trigger_a11y_label: SharedString,
    items: Vec<ContextMenuItem>,
}

impl ContextMenuItem {
    pub fn new(display: ContextMenuItemDisplay) -> Self {
        Self {
            display,
            on_select: None,
        }
    }

    pub fn on_select(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl ContextMenu {
    pub fn new(
        id: impl Into<SharedString>,
        scope: ContextMenuScope,
        trigger_a11y_label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            scope,
            trigger_label: SharedString::from("Actions"),
            trigger_a11y_label: trigger_a11y_label.into(),
            items: Vec::new(),
        }
    }

    pub fn trigger_label(mut self, label: impl Into<SharedString>) -> Self {
        self.trigger_label = label.into();
        self
    }

    pub fn item(mut self, item: ContextMenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = ContextMenuItem>) -> Self {
        self.items.extend(items);
        self
    }

    #[must_use]
    pub const fn scope(&self) -> ContextMenuScope {
        self.scope
    }
}

impl RenderOnce for ContextMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state_key = SharedString::from(format!("{}-state", self.id));
        let state: Entity<ContextMenuState> =
            window.use_keyed_state(state_key, cx, |_window, _cx| ContextMenuState {
                open: false,
            });

        let open = state.read(cx).open;
        let items = self.items;
        let trigger_id = SharedString::from(format!("{}-trigger", self.id));

        Popover::new(self.id)
            .placement(PopoverPlacement::Below)
            .alignment(PopoverAlignment::End)
            .surface_padding(Spacing::SM)
            .open(open)
            .on_open_change({
                let state = state.clone();
                move |is_open, _window, cx| {
                    state.update(cx, |state, cx| {
                        state.open = *is_open;
                        cx.notify();
                    });
                }
            })
            .trigger(
                Button::styled(trigger_id, ControlStyle::RowAction)
                    .leading_icon(IconName::More)
                    .label(self.trigger_label)
                    .a11y_label(self.trigger_a11y_label),
            )
            .content(move |_window, cx| build_menu_content(&state, items.clone(), cx))
    }
}

fn build_menu_content(
    state: &Entity<ContextMenuState>,
    items: Vec<ContextMenuItem>,
    cx: &App,
) -> impl IntoElement {
    let is_empty = items.is_empty();
    let mut content = div()
        .w(Size::MenuRegular.scaled(cx))
        .flex()
        .flex_col()
        .gap(Spacing::XXS.scaled(cx));

    for item in items {
        let display = item.display;
        let on_select = item.on_select;
        let state = state.clone();
        let mut button = if display.destructive {
            Button::styled(display.id, ControlStyle::DestructiveRowAction)
        } else {
            Button::plain(display.id)
        }
        .full_width()
        .align_leading()
        .label(display.label)
        .a11y_label(display.a11y_label)
        .disabled(display.disabled);

        if !display.disabled {
            button = button.on_click(move |_, window, cx| {
                state.update(cx, |state, cx| {
                    state.open = false;
                    cx.notify();
                });
                if let Some(handler) = &on_select {
                    handler(window, cx);
                }
            });
        }

        content = content.child(button);
    }

    if is_empty {
        content = content.child(
            div()
                .px(Spacing::MD.scaled(cx))
                .py(Spacing::SM.scaled(cx))
                .child("No actions"),
        );
    }

    content
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_context_menu_names_expected_scopes() {
        assert_eq!(
            [
                ContextMenuScope::FeedList,
                ContextMenuScope::TrackList,
                ContextMenuScope::PlaylistTrack,
            ]
            .len(),
            3
        );
    }

    #[test]
    fn menu_items_keep_display_and_handler_contracts_separate() {
        let item = ContextMenuItem::new(ContextMenuItemDisplay {
            id: "remove".into(),
            label: "Remove".into(),
            a11y_label: "Remove track".into(),
            destructive: true,
            disabled: false,
        })
        .on_select(|_, _| {});

        assert_eq!(item.display.label, SharedString::from("Remove"));
        assert!(item.display.destructive);
        assert!(item.on_select.is_some());
    }

    #[test]
    fn context_menu_carries_row_scope() {
        let menu = ContextMenu::new(
            "playlist-track-actions",
            ContextMenuScope::PlaylistTrack,
            "Track actions",
        );

        assert_eq!(menu.scope(), ContextMenuScope::PlaylistTrack);
    }
}
