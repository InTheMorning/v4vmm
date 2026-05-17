//! Shared workspace frame shell composite.
//!
//! This composite owns frame chrome for ADR 0046: title, subtitle, status,
//! history controls, close, action menu, and a caller-supplied content slot.
//! Screens provide content and command callbacks; they do not render frame
//! chrome locally.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, AnyElement, App, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window,
};

use crate::ui::composites::{filter_chip_strip, BreadcrumbTrail, FilterChipStripSlots};
use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::IconName;
use crate::ui::primitives::{
    Button, ContextMenu, ContextMenuItem, ContextMenuItemDisplay, ContextMenuScope,
};
use crate::ui::tokens::{resolve_color, Appearance, FontSize, SemanticColor, Spacing};
use crate::view_models::workspace::{
    ContentFilter, FilterChipStripDisplay, FrameChromeButtonDisplay, FrameChromeMenuItemDisplay,
    FrameNavigationEntry, FrameShellDisplay,
};

type FrameButtonHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;
type FrameMenuSelectHandler = Rc<dyn Fn(SharedString, &mut Window, &mut App) + 'static>;
type FrameFilterSelectHandler = Rc<dyn Fn(ContentFilter, &mut Window, &mut App) + 'static>;
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
    on_filter_select: Option<FrameFilterSelectHandler>,
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
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "deferred frame action wiring consumes this slot")
    )]
    pub(crate) fn on_forward(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_forward = Some(Rc::new(handler));
        self
    }

    /// Supplies the close-frame callback.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "deferred frame action wiring consumes this slot")
    )]
    pub(crate) fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }

    /// Supplies the action-menu selection callback.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "deferred frame action wiring consumes this slot")
    )]
    pub(crate) fn on_menu_select(
        mut self,
        handler: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_menu_select = Some(Rc::new(handler));
        self
    }

    /// Supplies the frame-local content-filter selection callback.
    pub(crate) fn on_filter_select(
        mut self,
        handler: impl Fn(ContentFilter, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_filter_select = Some(Rc::new(handler));
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

    /// Overrides appearance resolution for this shell.
    #[expect(dead_code, reason = "ADR 0046 Task 008+ wires per-frame appearance")]
    pub(crate) fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = Some(appearance);
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
    let filter_chip_strip_display = display.filter_chip_strip.clone();
    let breadcrumb_display = display.breadcrumb.clone();
    let mut nav = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(Spacing::XXS.scaled(cx));

    if !display.back.disabled {
        nav = nav.child(chrome_button(
            display.back,
            IconName::ChevronLeft,
            slots.on_back,
        ));
    }

    if !display.forward.disabled {
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
    chrome = chrome.child(header);

    if let Some(filter_display) = filter_chip_strip_display {
        chrome = chrome.child(frame_filter_row(filter_display, slots.on_filter_select, cx));
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

fn frame_filter_row(
    filter_display: FilterChipStripDisplay,
    handler: Option<FrameFilterSelectHandler>,
    cx: &App,
) -> AnyElement {
    let mut filter_slots = FilterChipStripSlots::new();
    if let Some(handler) = handler {
        filter_slots = filter_slots.on_select(move |filter, window, cx| {
            handler(filter, window, cx);
        });
    }
    div()
        .px(Spacing::MD.scaled(cx))
        .pb(Spacing::XS.scaled(cx))
        .child(filter_chip_strip(filter_display, filter_slots))
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
            .on_breadcrumb_select(|_, _, _| {});

        assert!(slots.content.is_some());
        assert!(slots.on_back.is_some());
        assert!(slots.on_forward.is_some());
        assert!(slots.on_close.is_some());
        assert!(slots.on_menu_select.is_some());
        assert!(slots.on_breadcrumb_select.is_some());
        assert!(slots.on_filter_select.is_none());
    }

    #[test]
    fn slots_accept_filter_selection_callback() {
        let slots = FrameShellSlots::new().on_filter_select(|_, _, _| {});

        assert!(slots.on_filter_select.is_some());
    }

    #[test]
    fn slots_accept_breadcrumb_selection_callback() {
        let slots = FrameShellSlots::new().on_breadcrumb_select(|_, _, _| {});

        assert!(slots.on_breadcrumb_select.is_some());
    }
}
