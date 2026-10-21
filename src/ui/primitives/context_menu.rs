//! Context-menu primitive for row commands and ADR 0063 text selection.
//!
//! GPUI does not currently expose a native arbitrary-element context-menu API,
//! so this primitive uses the shared [`Popover`] infrastructure while owning
//! the menu row chrome and action contract. Screens should pass display-ready
//! menu items instead of hand-rolling floating row action panels.
//! Pointer menus reuse those rows and the shared surface at the click position.
//! An item can open a second level in the same menu surface (ADR 0083 task
//! 006). Both menu forms show that level.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    actions, anchored, deferred, div, prelude::*, AnyElement, App, ElementId, Entity, FocusHandle,
    IntoElement, KeyBinding, MouseButton, Pixels, Point, RenderOnce, SharedString, Window,
};

use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::IconName;
use crate::ui::primitives::{
    Button, Divider, Popover, PopoverAlignment, PopoverPlacement, Surface, SurfaceElevation,
};
use crate::ui::tokens::{Size, Spacing};

type SelectHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;
type OpenLevelHandler = Rc<dyn Fn(usize, &mut Window, &mut App) + 'static>;

/// The builder of the second level of a menu item. It gets the controls of
/// the menu and gives the content of the level.
pub(crate) type MenuLevelBuilder =
    Rc<dyn Fn(&MenuLevelControls, &mut Window, &mut App) -> AnyElement + 'static>;

/// The controls that the second level of a menu item can use.
#[derive(Clone)]
pub(crate) struct MenuLevelControls {
    /// Shows the first level of the menu again.
    pub(crate) back: SelectHandler,
    /// Closes the menu.
    pub(crate) dismiss: SelectHandler,
}

const POINTER_MENU_KEY_CONTEXT: &str = "PointerContextMenu";

actions!(pointer_context_menu, [Dismiss]);

/// Install the pointer menu's Escape binding before opening windows (ADR 0071).
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new(
        "escape",
        Dismiss,
        Some(POINTER_MENU_KEY_CONTEXT),
    )]);
}

struct ContextMenuState {
    open: bool,
    /// The index of the item whose second level shows. `None` shows the
    /// first level.
    level: Option<usize>,
}

/// The second-level state of a [`PointerContextMenu`].
struct PointerMenuLevel {
    level: Option<usize>,
}

/// Row families expected to expose context-menu commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextMenuScope {
    FeedList,
    TrackList,
    PlaylistTrack,
    WorkspaceFrame,
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
    description: Option<SharedString>,
    display: ContextMenuItemDisplay,
    on_select: Option<SelectHandler>,
    level: Option<MenuLevelBuilder>,
}

/// Shared context-menu surface.
#[derive(IntoElement)]
#[must_use]
pub struct ContextMenu {
    id: SharedString,
    scope: ContextMenuScope,
    trigger_label: SharedString,
    trigger_icon: IconName,
    trigger_tooltip: Option<SharedString>,
    disabled: bool,
    trigger_a11y_label: SharedString,
    items: Vec<ContextMenuItem>,
}

/// Controlled menu at a pointer position. The owner removes it on dismissal.
#[derive(IntoElement)]
#[must_use]
pub struct PointerContextMenu {
    id: ElementId,
    position: Point<Pixels>,
    return_focus: FocusHandle,
    items: Vec<ContextMenuItem>,
    on_dismiss: SelectHandler,
}

impl PointerContextMenu {
    pub fn new(
        id: impl Into<ElementId>,
        position: Point<Pixels>,
        return_focus: FocusHandle,
        on_dismiss: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            position,
            return_focus,
            items: Vec::new(),
            on_dismiss: Rc::new(on_dismiss),
        }
    }

    pub fn item(mut self, item: ContextMenuItem) -> Self {
        self.items.push(item);
        self
    }
}

impl RenderOnce for PointerContextMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus = window.use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle());
        let focus = focus.read(cx).clone();
        if !focus.contains_focused(window, cx) {
            focus.focus(window, cx);
        }
        let level_key = SharedString::from(format!("{}-level", self.id));
        let level_state =
            window.use_keyed_state(level_key, cx, |_, _| PointerMenuLevel { level: None });
        let dismiss: SelectHandler = {
            let level_state = level_state.clone();
            Rc::new(move |window, cx| {
                level_state.update(cx, |state, _| state.level = None);
                self.return_focus.focus(window, cx);
                (self.on_dismiss)(window, cx);
            })
        };
        let open_level: OpenLevelHandler = {
            let level_state = level_state.clone();
            Rc::new(move |index, _, cx| {
                level_state.update(cx, |state, cx| {
                    state.level = Some(index);
                    cx.notify();
                });
            })
        };
        let back: SelectHandler = {
            let level_state = level_state.clone();
            Rc::new(move |_, cx| {
                level_state.update(cx, |state, cx| {
                    state.level = None;
                    cx.notify();
                });
            })
        };
        let level = level_state.read(cx).level;
        let outside_dismiss = dismiss.clone();
        let escape_dismiss = dismiss.clone();
        let menu = div()
            .id(self.id)
            .key_context(POINTER_MENU_KEY_CONTEXT)
            .track_focus(&focus)
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down_out(move |_, window, cx| {
                outside_dismiss(window, cx);
                cx.stop_propagation();
            })
            // Bound app actions run before raw key handlers. Dismiss in the
            // menu's own context so Escape cannot reach the pane (ADR 0071).
            .on_action(move |_: &Dismiss, window, cx| {
                escape_dismiss(window, cx);
                cx.stop_propagation();
            })
            .child(
                Surface::new(SurfaceElevation::Floating)
                    .padding(Spacing::SM)
                    .child(build_menu_level(
                        &MenuLevelControls {
                            back,
                            dismiss: dismiss.clone(),
                        },
                        &open_level,
                        self.items,
                        level,
                        window,
                        cx,
                    )),
            );

        deferred(
            anchored().child(
                div()
                    .w(window.bounds().size.width)
                    .h(window.bounds().size.height)
                    .occlude()
                    .child(
                        anchored()
                            .position(self.position)
                            .snap_to_window_with_margin(Spacing::SM.scaled(cx))
                            .child(menu),
                    ),
            ),
        )
    }
}

impl ContextMenuItem {
    pub fn new(display: ContextMenuItemDisplay) -> Self {
        Self {
            display,
            description: None,
            on_select: None,
            level: None,
        }
    }

    /// Gives the item a second level. A selection of the item shows that
    /// level in the same menu surface instead of closing the menu.
    pub(crate) fn level(
        mut self,
        builder: impl Fn(&MenuLevelControls, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.level = Some(Rc::new(builder));
        self
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
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
            trigger_icon: IconName::More,
            trigger_tooltip: None,
            disabled: false,
            trigger_a11y_label: trigger_a11y_label.into(),
            items: Vec::new(),
        }
    }

    pub fn trigger_icon(mut self, icon: IconName) -> Self {
        self.trigger_icon = icon;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The hover help of the trigger, for a trigger with an icon and no
    /// text (ADR 0083 Decision 10).
    pub fn trigger_tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.trigger_tooltip = Some(tooltip.into());
        self
    }

    /// `true` while the menu with the id `id` is open. The caller must call
    /// it at the same element position as the menu, so that it reads the
    /// same keyed state. A row uses it to keep its menu button visible
    /// while the menu is open (ADR 0083 Decision 6).
    pub(crate) fn is_open(id: &SharedString, window: &mut Window, cx: &mut App) -> bool {
        let state =
            window.use_keyed_state(Self::state_key(id), cx, |_, _| ContextMenuState::closed());
        state.read(cx).open
    }

    fn state_key(id: &SharedString) -> SharedString {
        SharedString::from(format!("{id}-state"))
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

impl ContextMenuState {
    const fn closed() -> Self {
        Self {
            open: false,
            level: None,
        }
    }
}

impl RenderOnce for ContextMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<ContextMenuState> =
            window.use_keyed_state(Self::state_key(&self.id), cx, |_window, _cx| {
                ContextMenuState::closed()
            });

        let open = state.read(cx).open && !self.disabled;
        let items = self.items;
        let trigger_id = SharedString::from(format!("{}-trigger", self.id));
        let mut trigger = Button::styled(trigger_id, ControlStyle::RowAction)
            .leading_icon(self.trigger_icon)
            .disabled(self.disabled)
            .label(self.trigger_label)
            .a11y_label(self.trigger_a11y_label);
        if let Some(tooltip) = self.trigger_tooltip {
            trigger = trigger.tooltip(tooltip);
        }

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
                        state.level = None;
                        cx.notify();
                    });
                }
            })
            .trigger(trigger)
            .content(move |window, cx| {
                let dismiss: SelectHandler = {
                    let state = state.clone();
                    Rc::new(move |_, cx| {
                        state.update(cx, |state, cx| {
                            state.open = false;
                            state.level = None;
                            cx.notify();
                        });
                    })
                };
                let back: SelectHandler = {
                    let state = state.clone();
                    Rc::new(move |_, cx| {
                        state.update(cx, |state, cx| {
                            state.level = None;
                            cx.notify();
                        });
                    })
                };
                let open_level: OpenLevelHandler = {
                    let state = state.clone();
                    Rc::new(move |index, _, cx| {
                        state.update(cx, |state, cx| {
                            state.level = Some(index);
                            cx.notify();
                        });
                    })
                };
                let level = state.read(cx).level;
                build_menu_level(
                    &MenuLevelControls { back, dismiss },
                    &open_level,
                    items.clone(),
                    level,
                    window,
                    cx,
                )
            })
    }
}

/// The content of a menu: the second level of the item at `level`, or
/// the first level when `level` is `None` or names no such level.
fn build_menu_level(
    controls: &MenuLevelControls,
    open_level: &OpenLevelHandler,
    items: Vec<ContextMenuItem>,
    level: Option<usize>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    if let Some(builder) = level
        .and_then(|index| items.get(index))
        .and_then(|item| item.level.clone())
    {
        return builder(controls, window, cx);
    }
    build_menu_content(&controls.dismiss, open_level, items, cx).into_any_element()
}

fn build_menu_content(
    on_dismiss: &SelectHandler,
    open_level: &OpenLevelHandler,
    items: Vec<ContextMenuItem>,
    cx: &App,
) -> impl IntoElement {
    let is_empty = items.is_empty();
    let mut content = div()
        .id("context-menu-items")
        .max_h(Size::MenuRegular.scaled(cx))
        .overflow_y_scroll()
        .w(Size::MenuRegular.scaled(cx))
        .flex()
        .flex_col()
        .gap(Spacing::XXS.scaled(cx));

    let mut previous_destructive = true;
    for (index, item) in items.into_iter().enumerate() {
        let display = item.display;
        // ADR 0083 Decision 5: a divider separates a destructive item from
        // the items before it.
        if display.destructive && !previous_destructive {
            content = content.child(Divider::horizontal());
        }
        previous_destructive = display.destructive;
        let on_select = item.on_select;
        let on_dismiss = on_dismiss.clone();
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

        if let Some(description) = item.description {
            button = button.description(description);
        }
        if !display.disabled && item.level.is_some() {
            // An item with a second level shows that level. The menu stays
            // open.
            let open_level = open_level.clone();
            button = button.on_activate(move |window, cx| open_level(index, window, cx));
        } else if !display.disabled {
            button = button.on_activate(move |window, cx| {
                on_dismiss(window, cx);
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

    #[cfg(test)]
    let content = content.debug_selector(|| "context-menu-items".to_owned());

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
                ContextMenuScope::WorkspaceFrame,
            ]
            .len(),
            4
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
