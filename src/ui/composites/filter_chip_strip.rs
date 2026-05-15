//! Frame-local content filter chip strip.
//!
//! ADR 0047 moves All / Library / Index filters into frame chrome. This
//! composite renders the shared VM display contract as either a segmented
//! control or, for narrow frame chrome, the existing context-menu primitive.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
};

use crate::ui::composites::{Segment, SegmentDisplay, SegmentedControl};
use crate::ui::primitives::{
    ContextMenu, ContextMenuItem, ContextMenuItemDisplay, ContextMenuScope,
};
use crate::ui::tokens::Spacing;
use crate::view_models::workspace::{ContentFilter, FilterChipOption, FilterChipStripDisplay};

type FilterSelectHandler = Rc<dyn Fn(ContentFilter, &mut Window, &mut App) + 'static>;

/// Callback slots for [`FilterChipStrip`].
#[derive(Default)]
#[must_use]
pub(crate) struct FilterChipStripSlots {
    on_select: Option<FilterSelectHandler>,
}

impl FilterChipStripSlots {
    /// Creates empty filter-chip-strip slots.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Supplies the filter selection callback.
    pub(crate) fn on_select(
        mut self,
        handler: impl Fn(ContentFilter, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

/// Shared frame-local filter chip strip composite.
#[derive(IntoElement)]
#[must_use]
pub(crate) struct FilterChipStrip {
    display: FilterChipStripDisplay,
    slots: FilterChipStripSlots,
}

impl FilterChipStrip {
    /// Creates a filter chip strip from display data and slots.
    pub(crate) fn new(display: FilterChipStripDisplay, slots: FilterChipStripSlots) -> Self {
        Self { display, slots }
    }
}

/// Creates a shared frame-local filter chip strip.
pub(crate) fn filter_chip_strip(
    display: FilterChipStripDisplay,
    slots: FilterChipStripSlots,
) -> FilterChipStrip {
    FilterChipStrip::new(display, slots)
}

impl RenderOnce for FilterChipStrip {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { display, slots } = self;
        let on_select = slots.on_select.as_ref();
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(Spacing::XS.scaled(cx))
            .child(if display.narrow_collapse_to_pulldown {
                render_narrow_menu(&display, on_select).into_any_element()
            } else {
                render_segmented_control(display, on_select).into_any_element()
            })
    }
}

fn render_segmented_control(
    display: FilterChipStripDisplay,
    on_select: Option<&FilterSelectHandler>,
) -> SegmentedControl<ContentFilter> {
    let selected = display.selected;
    let mut control = SegmentedControl::new(selected).filter_style().segments(
        display
            .options
            .into_iter()
            .map(|option| Segment::new(segment_display(&display.id, &option))),
    );

    if let Some(handler) = on_select.cloned() {
        control = control.on_select(move |filter, window, cx| {
            handler(*filter, window, cx);
        });
    }

    control
}

fn render_narrow_menu(
    display: &FilterChipStripDisplay,
    on_select: Option<&FilterSelectHandler>,
) -> ContextMenu {
    let active_label = display
        .options
        .iter()
        .find(|option| option.value == display.selected)
        .map_or_else(|| display.selected.label(), |option| option.label);
    let mut menu = ContextMenu::new(
        format!("{}-menu", display.id),
        ContextMenuScope::WorkspaceFrame,
        "Select content filter",
    )
    .trigger_label(active_label);

    for option in &display.options {
        menu = menu.item(filter_menu_item(option, on_select));
    }

    menu
}

fn segment_display(id_seed: &str, option: &FilterChipOption) -> SegmentDisplay<ContentFilter> {
    SegmentDisplay {
        id: option_id(id_seed, option.value),
        key: option.value,
        label: SharedString::from(option.label),
        a11y_label: SharedString::from(option.a11y_label),
    }
}

fn filter_menu_item(
    option: &FilterChipOption,
    on_select: Option<&FilterSelectHandler>,
) -> ContextMenuItem {
    let filter = option.value;
    let disabled = option.disabled;
    let item = ContextMenuItem::new(ContextMenuItemDisplay {
        id: option_shared_id("filter-chip", filter),
        label: SharedString::from(option.label),
        a11y_label: SharedString::from(option.a11y_label),
        destructive: false,
        disabled,
    });

    if disabled {
        return item;
    }

    on_select.cloned().map_or(item.clone(), |handler| {
        item.on_select(move |window, cx| {
            handler(filter, window, cx);
        })
    })
}

fn option_id(id_seed: &str, filter: ContentFilter) -> ElementId {
    option_shared_id(id_seed, filter).into()
}

fn option_shared_id(id_seed: &str, filter: ContentFilter) -> SharedString {
    SharedString::from(format!("{id_seed}-{}", filter_id(filter)))
}

const fn filter_id(filter: ContentFilter) -> &'static str {
    match filter {
        ContentFilter::All => "all",
        ContentFilter::Library => "library",
        ContentFilter::Index => "index",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_ids_are_stable_and_semantic() {
        assert_eq!(filter_id(ContentFilter::All), "all");
        assert_eq!(filter_id(ContentFilter::Library), "library");
        assert_eq!(filter_id(ContentFilter::Index), "index");
    }

    #[test]
    fn slots_accept_filter_selection_callback() {
        let slots = FilterChipStripSlots::new().on_select(|_, _, _| {});

        assert!(slots.on_select.is_some());
    }
}
