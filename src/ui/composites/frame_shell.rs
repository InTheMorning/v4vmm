//! Shared workspace frame shell composite.
//!
//! This composite owns frame chrome for ADR 0046: title, subtitle, status,
//! history controls, close, action menu, and a caller-supplied content slot.
//! Screens provide content and command callbacks; they do not render frame
//! chrome locally.
//!
//! ADR 0048 records the composite call-site exception: `frame_shell` remains
//! the canonical home for frame chrome even with a single caller
//! (`WorkspaceShell::render`), and any future frame-chrome consumer must route
//! through this composite instead of re-creating the chrome locally.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, AnyElement, App, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window,
};

use crate::ui::composites::{
    view_mode_control, BreadcrumbTrail, Segment, SegmentDisplay, SegmentedControl,
    ViewModeControlSlots,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::IconName;
use crate::ui::primitives::{
    Button, ContextMenu, ContextMenuItem, ContextMenuItemDisplay, ContextMenuScope,
};
use crate::ui::tokens::{resolve_color, Appearance, FontSize, SemanticColor, Spacing};
use crate::view_models::index_name_list::{MusicListScope, MusicScopeControlDisplay};
use crate::view_models::workspace::{
    ContentViewMode, ContentViewModeControlDisplay, FrameChromeButtonDisplay,
    FrameChromeMenuItemDisplay, FrameHeaderActionDisplay, FrameNavigationEntry, FrameShellDisplay,
};

type FrameButtonHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;
type FrameMenuSelectHandler = Rc<dyn Fn(SharedString, &mut Window, &mut App) + 'static>;
type FrameViewModeSelectHandler = Rc<dyn Fn(ContentViewMode, &mut Window, &mut App) + 'static>;
type FrameScopeSelectHandler = Rc<dyn Fn(MusicListScope, &mut Window, &mut App) + 'static>;
type FrameBreadcrumbSelectHandler =
    Rc<dyn Fn(FrameNavigationEntry, &mut Window, &mut App) + 'static>;

/// Callback and content slots supplied by a frame-shell caller.
#[derive(Default)]
#[must_use]
pub(crate) struct FrameShellSlots {
    content: Option<AnyElement>,
    on_back: Option<FrameButtonHandler>,
    on_forward: Option<FrameButtonHandler>,
    on_close: Option<FrameButtonHandler>,
    on_menu_select: Option<FrameMenuSelectHandler>,
    on_header_action: Option<FrameButtonHandler>,
    on_view_mode_select: Option<FrameViewModeSelectHandler>,
    on_scope_select: Option<FrameScopeSelectHandler>,
    on_breadcrumb_select: Option<FrameBreadcrumbSelectHandler>,
    appearance: Option<Appearance>,
}

impl FrameShellSlots {
    /// Creates empty frame-shell slots.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Supplies the body content rendered inside the frame.
    pub(crate) fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    /// Supplies the back-navigation callback.
    pub(crate) fn on_back(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_back = Some(Rc::new(handler));
        self
    }

    /// Supplies the forward-navigation callback.
    pub(crate) fn on_forward(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_forward = Some(Rc::new(handler));
        self
    }

    /// Supplies the trailing header action's click callback (ADR 0084
    /// Decision 10 Amendment, 2026-10-19).
    pub(crate) fn on_header_action(
        mut self,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_header_action = Some(Rc::new(handler));
        self
    }

    /// Supplies the frame-local view-mode selection callback.
    pub(crate) fn on_view_mode_select(
        mut self,
        handler: impl Fn(ContentViewMode, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_view_mode_select = Some(Rc::new(handler));
        self
    }

    /// Supplies the scope selection callback of a Music list (ADR 0084
    /// packet 016).
    pub(crate) fn on_scope_select(
        mut self,
        handler: impl Fn(MusicListScope, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_scope_select = Some(Rc::new(handler));
        self
    }

    /// Supplies the frame-local breadcrumb selection callback.
    pub(crate) fn on_breadcrumb_select(
        mut self,
        handler: impl Fn(FrameNavigationEntry, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_breadcrumb_select = Some(Rc::new(handler));
        self
    }
}

/// `on_close` and `on_menu_select` are ADR 0046 frame chrome slots with no
/// production caller yet: the workspace shell wires only `on_back` and
/// `on_forward` today. `frame_shell`'s own rendering test needs each builder
/// as a direct helper, so each one stays here, compiled only for tests.
#[cfg(test)]
impl FrameShellSlots {
    /// Supplies the close-frame callback.
    pub(crate) fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }

    /// Supplies the action-menu selection callback.
    pub(crate) fn on_menu_select(
        mut self,
        handler: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_menu_select = Some(Rc::new(handler));
        self
    }
}

/// Workspace frame shell that renders shared frame chrome around content.
#[derive(IntoElement)]
#[must_use]
pub(crate) struct FrameShell {
    display: FrameShellDisplay,
    slots: FrameShellSlots,
}

impl FrameShell {
    /// Creates a frame shell from display data and caller-owned slots.
    pub(crate) fn new(display: FrameShellDisplay, slots: FrameShellSlots) -> Self {
        Self { display, slots }
    }
}

/// Creates a shared workspace frame shell.
pub(crate) fn frame_shell(display: FrameShellDisplay, slots: FrameShellSlots) -> FrameShell {
    FrameShell::new(display, slots)
}

impl RenderOnce for FrameShell {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut slots = self.slots;
        let appearance = slots.appearance;
        let border = resolve_color(cx, SemanticColor::Separator, appearance);
        let title_color = resolve_color(cx, SemanticColor::Label, appearance);
        let secondary_color = resolve_color(cx, SemanticColor::SecondaryLabel, appearance);
        let content = slots
            .content
            .take()
            .unwrap_or_else(|| div().into_any_element());
        let shell_id = SharedString::from(format!(
            "workspace-frame-{}-shell",
            self.display.frame_id.value()
        ));

        div()
            .id(shell_id)
            .size_full()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_hidden()
            .border_1()
            .border_color(border)
            .child(render_chrome(
                self.display,
                slots,
                title_color,
                secondary_color,
                cx,
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .child(content),
            )
    }
}

#[allow(clippy::too_many_lines)]
fn render_chrome(
    display: FrameShellDisplay,
    slots: FrameShellSlots,
    title_color: gpui::Rgba,
    secondary_color: gpui::Rgba,
    cx: &App,
) -> impl IntoElement {
    let content_slot_id = display.content_slot_id.clone();
    let view_mode_control_display = display.view_mode_control.clone();
    let scope_control_display = display.scope_control;
    let breadcrumb_display = display.breadcrumb.clone();
    let header_visible = display.header_visible();
    let mut nav = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(Spacing::XXS.scaled(cx));

    let back_shown = display.back_shown();
    let forward_shown = display.forward_shown();
    if back_shown {
        nav = nav.child(chrome_button(
            display.back,
            IconName::ChevronLeft,
            slots.on_back,
        ));
    }

    if forward_shown {
        nav = nav.child(chrome_button(
            display.forward,
            IconName::ChevronRight,
            slots.on_forward,
        ));
    }

    if let Some(close) = display.close {
        nav = nav.child(chrome_button(close, IconName::Close, slots.on_close));
    }

    let mut title_stack = div().flex().flex_col().min_w_0().flex_1().child(
        div()
            .text_size(FontSize::Body.scaled(cx))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(title_color)
            .truncate()
            .child(SharedString::from(display.title)),
    );

    if let Some(subtitle) = display.subtitle {
        title_stack = title_stack.child(
            div()
                .text_size(FontSize::Micro.scaled(cx))
                .text_color(secondary_color)
                .truncate()
                .child(SharedString::from(subtitle)),
        );
    }

    let mut trailing = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(Spacing::XS.scaled(cx));

    if let Some(status) = display.status {
        trailing = trailing.child(
            div()
                .text_size(FontSize::Micro.scaled(cx))
                .text_color(secondary_color)
                .truncate()
                .child(SharedString::from(status)),
        );
    }

    if !display.action_menu_items.is_empty() {
        trailing = trailing.child(action_menu(
            &content_slot_id,
            display.action_menu_items,
            slots.on_menu_select.as_ref(),
        ));
    }

    if let Some(header_action) = display.header_action {
        trailing = trailing.child(header_action_button(header_action, slots.on_header_action));
    }

    let header = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(Spacing::SM.scaled(cx))
        .px(Spacing::MD.scaled(cx))
        .py(Spacing::XS.scaled(cx))
        .border_b_1()
        .border_color(secondary_color)
        .child(nav)
        .child(title_stack)
        .child(trailing);

    let mut chrome = div()
        .flex()
        .flex_col()
        .border_b_1()
        .border_color(secondary_color);
    if header_visible {
        chrome = chrome.child(header);
    }

    if view_mode_control_display.is_some() || scope_control_display.is_some() {
        chrome = chrome.child(frame_filter_row(
            FrameFilterRowDisplays {
                view_mode: view_mode_control_display,
                scope: scope_control_display,
            },
            FrameFilterRowHandlers {
                view_mode: slots.on_view_mode_select,
                scope: slots.on_scope_select,
            },
            cx,
        ));
    }

    if let Some(breadcrumb) = breadcrumb_display {
        let mut breadcrumb_trail =
            BreadcrumbTrail::new(breadcrumb).appearance(slots.appearance.unwrap_or_default());
        if let Some(handler) = slots.on_breadcrumb_select {
            breadcrumb_trail = breadcrumb_trail.on_select(move |entry, window, cx| {
                handler(entry, window, cx);
            });
        }
        chrome = chrome.child(
            div()
                .px(Spacing::MD.scaled(cx))
                .pb(Spacing::XS.scaled(cx))
                .child(breadcrumb_trail)
                .into_any_element(),
        );
    }

    chrome
}

/// The controls of the frame filter row.
struct FrameFilterRowDisplays {
    view_mode: Option<ContentViewModeControlDisplay>,
    scope: Option<MusicScopeControlDisplay>,
}

/// The callbacks of the frame filter row.
struct FrameFilterRowHandlers {
    view_mode: Option<FrameViewModeSelectHandler>,
    scope: Option<FrameScopeSelectHandler>,
}

fn frame_filter_row(
    displays: FrameFilterRowDisplays,
    handlers: FrameFilterRowHandlers,
    cx: &App,
) -> AnyElement {
    let FrameFilterRowDisplays {
        view_mode: view_mode_display,
        scope: scope_display,
    } = displays;
    let FrameFilterRowHandlers {
        view_mode: view_mode_handler,
        scope: scope_handler,
    } = handlers;
    let mut row = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(Spacing::XS.scaled(cx));

    // ADR 0084 packet 016 decision D1: the scope control sits at the
    // leading edge of the row.
    if let Some(display) = scope_display {
        row = row.child(scope_control(display, scope_handler));
    }

    row = row.child(div().flex_1().min_w_0());
    if let Some(display) = view_mode_display {
        let mut view_mode_slots = ViewModeControlSlots::new();
        if let Some(handler) = view_mode_handler {
            view_mode_slots = view_mode_slots.on_select(move |view_mode, window, cx| {
                handler(view_mode, window, cx);
            });
        }
        row = row.child(view_mode_control(display, view_mode_slots));
    }

    div()
        .px(Spacing::MD.scaled(cx))
        .pb(Spacing::XS.scaled(cx))
        .child(row)
        .into_any_element()
}

/// The scope control of a Music list: a `SegmentedControl` with the
/// `Filter` style (ADR 0084 packet 016).
fn scope_control(
    display: MusicScopeControlDisplay,
    handler: Option<FrameScopeSelectHandler>,
) -> AnyElement {
    let mut control = SegmentedControl::new(display.selected)
        .filter_style()
        .segments(display.options.map(|option| {
            Segment::new(SegmentDisplay {
                id: SharedString::from(option.id).into(),
                key: option.scope,
                label: SharedString::from(option.label),
                a11y_label: SharedString::from(option.a11y_label),
            })
        }));
    if let Some(handler) = handler {
        control = control.on_select(move |scope, window, cx| {
            handler(*scope, window, cx);
        });
    }
    div()
        .id(SharedString::from(display.control_id))
        .child(control)
        .into_any_element()
}

fn chrome_button(
    display: FrameChromeButtonDisplay,
    icon: IconName,
    handler: Option<FrameButtonHandler>,
) -> Button {
    let disabled = display.disabled;
    let mut button = Button::styled(SharedString::from(display.id), ControlStyle::ToolbarIcon)
        .leading_icon(icon)
        .a11y_label(display.a11y_label)
        .tooltip(display.a11y_label)
        .disabled(disabled);

    if !disabled {
        if let Some(handler) = handler {
            button = button.on_click(move |_, window, cx| {
                handler(window, cx);
            });
        }
    }

    button
}

/// Renders the frame shell's trailing header action (ADR 0084 Decision
/// 10 Amendment, 2026-10-19): "Select" or "Done" for the Music list
/// column today. The shell renders the given label; it decides none of
/// its own.
fn header_action_button(
    display: FrameHeaderActionDisplay,
    handler: Option<FrameButtonHandler>,
) -> Button {
    let a11y_label = display.a11y_label.clone();
    let mut button = Button::styled(SharedString::from(display.id), ControlStyle::Ghost)
        .label(display.label)
        .a11y_label(a11y_label.clone())
        .tooltip(a11y_label);

    if let Some(handler) = handler {
        button = button.on_click(move |_, window, cx| {
            handler(window, cx);
        });
    }

    button
}

fn action_menu(
    id_seed: &str,
    items: Vec<FrameChromeMenuItemDisplay>,
    on_select: Option<&FrameMenuSelectHandler>,
) -> ContextMenu {
    let mut menu = ContextMenu::new(
        format!("{id_seed}-actions"),
        ContextMenuScope::WorkspaceFrame,
        "Frame actions",
    )
    .trigger_label("");

    for item in items {
        menu = menu.item(frame_menu_item(item, on_select.cloned()));
    }

    menu
}

fn frame_menu_item(
    item: FrameChromeMenuItemDisplay,
    on_select: Option<FrameMenuSelectHandler>,
) -> ContextMenuItem {
    let item_id = SharedString::from(item.id);
    let disabled = item.disabled;
    let menu_item = ContextMenuItem::new(ContextMenuItemDisplay {
        id: item_id.clone(),
        label: SharedString::from(item.label),
        a11y_label: SharedString::from(item.a11y_label),
        destructive: false,
        disabled,
    });

    if disabled {
        return menu_item;
    }

    on_select.map_or(menu_item.clone(), |handler| {
        menu_item.on_select(move |window, cx| {
            handler(item_id.clone(), window, cx);
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_accept_callbacks_and_content() {
        let slots = FrameShellSlots::new()
            .content(div())
            .on_back(|_, _| {})
            .on_forward(|_, _| {})
            .on_close(|_, _| {})
            .on_menu_select(|_, _, _| {})
            .on_view_mode_select(|_, _, _| {})
            .on_breadcrumb_select(|_, _, _| {});

        assert!(slots.content.is_some());
        assert!(slots.on_back.is_some());
        assert!(slots.on_forward.is_some());
        assert!(slots.on_close.is_some());
        assert!(slots.on_menu_select.is_some());
        assert!(slots.on_view_mode_select.is_some());
        assert!(slots.on_breadcrumb_select.is_some());
    }

    /// ADR 0084 Decision 10 Amendment: the frame shell accepts the
    /// trailing header action's click callback.
    #[test]
    fn slots_accept_header_action_callback() {
        let slots = FrameShellSlots::new().on_header_action(|_, _| {});

        assert!(slots.on_header_action.is_some());
    }

    #[test]
    fn slots_accept_view_mode_selection_callback() {
        let slots = FrameShellSlots::new().on_view_mode_select(|_, _, _| {});

        assert!(slots.on_view_mode_select.is_some());
    }

    #[test]
    fn slots_accept_breadcrumb_selection_callback() {
        let slots = FrameShellSlots::new().on_breadcrumb_select(|_, _, _| {});

        assert!(slots.on_breadcrumb_select.is_some());
    }
}
