//! Music destinations sidebar (ADR 0084 Decisions 1 and 2).
//!
//! Renders the collapse button, the six destinations, Saved Searches, and
//! the feed area. Callbacks dispatch back through `cx.listener(...)` calls
//! into `LibraryApp` mutator methods. The Library tree sidebar is gone:
//! each destination now owns its own list, in the list column.

#![warn(clippy::pedantic)]

use gpui::{
    div, prelude::*, AnyElement, App, ClickEvent, Context, DragMoveEvent, FontWeight, SharedString,
};

use crate::library::LibraryApp;
use crate::ui::composites::track_drag::{PlaylistTrackDragPayload, TrackDragPayload};
use crate::ui::icons::{Icon, IconName, IconSize};
use crate::ui::primitives::Label;
use crate::ui::style::{color, spacing};
use crate::ui::tokens::SemanticColor;
use crate::view_models::library::{
    DestinationRowVm, MusicDestinationIcon, SidebarCollapseButtonVm,
};
use crate::view_models::workspace::{FrameNavigationEntry, MusicSidebarState, NavigationPathStart};

/// Maps a destination's semantic icon to a named `IconName` (ADR 0084
/// Decision 1). The renderer owns this mapping; the view model names only
/// the semantic icon.
const fn icon_name(icon: MusicDestinationIcon) -> IconName {
    match icon {
        MusicDestinationIcon::Discover => IconName::Discover,
        MusicDestinationIcon::Release => IconName::Release,
        MusicDestinationIcon::Artist => IconName::Artist,
        MusicDestinationIcon::Track => IconName::Track,
        MusicDestinationIcon::Playlist => IconName::Playlist,
        MusicDestinationIcon::Warning => IconName::Warning,
    }
}

/// Renders the collapse button (ADR 0084 Decision 1).
pub(crate) fn render_sidebar_collapse_button(
    button: SidebarCollapseButtonVm,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    div()
        .id(button.element_id)
        .flex()
        .items_center()
        .justify_center()
        .w_full()
        .py(spacing::XS)
        .cursor_pointer()
        .rounded(spacing::XS)
        .hover(|el| el.bg(color::bg_surface_hi()))
        .tooltip({
            let tooltip = crate::ui::primitives::Tooltip::new(button.tooltip);
            move |window, cx| tooltip.build(window, cx)
        })
        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
            this.toggle_sidebar_collapse(window, cx);
        }))
        .child(Icon::new(IconName::ChevronLeft).size(IconSize::Action))
        .into_any_element()
}

/// Renders one destination row, expanded (icon, label, and count) or
/// collapsed (icon only) (ADR 0084 Decision 1).
pub(crate) fn render_destination_row(
    row: &DestinationRowVm,
    state: MusicSidebarState,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let destination = row.destination;
    let icon = icon_name(row.icon);
    let selected = row.selected;
    let tooltip_text = row.tooltip.clone();
    let count_label = row.count.map(|count| count.to_string());
    let shows_attention_mark = row.shows_attention_mark;

    let mut el = div()
        .id(SharedString::from(row.element_id))
        .flex()
        .items_center()
        .cursor_pointer()
        .rounded(spacing::XS)
        .when(selected, |el| el.bg(color::bg_selected()))
        .hover(|el| el.bg(color::bg_surface_hi()))
        .tooltip({
            let tooltip = crate::ui::primitives::Tooltip::new(tooltip_text);
            move |window, cx| tooltip.build(window, cx)
        })
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            this.open_page(
                FrameNavigationEntry::Destination(destination),
                NavigationPathStart::NewPath,
                cx,
            );
        }));

    // ADR 0083 Decision 6, task 008 decision D1: a spring-loaded row opens
    // its destination while a dragged track stays over it. The row accepts
    // no drop.
    if row.spring_loaded {
        el = el
            .on_drag_move(cx.listener(
                move |this, event: &DragMoveEvent<TrackDragPayload>, _window, cx| {
                    let on_row = event.bounds.contains(&event.event.position);
                    this.spring_load_pointer(destination, on_row, cx);
                },
            ))
            .on_drag_move(cx.listener(
                move |this, event: &DragMoveEvent<PlaylistTrackDragPayload>, _window, cx| {
                    let on_row = event.bounds.contains(&event.event.position);
                    this.spring_load_pointer(destination, on_row, cx);
                },
            ));
    }

    el = match state {
        MusicSidebarState::Expanded => el
            .w_full()
            .gap(spacing::SM)
            .px(spacing::SM)
            .py(spacing::XS)
            .child(render_destination_icon(
                icon,
                selected,
                shows_attention_mark,
                cx,
            ))
            .child(
                Label::new(row.label)
                    .color(if selected {
                        SemanticColor::Accent
                    } else {
                        SemanticColor::Label
                    })
                    .weight(if selected {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .truncated(),
            )
            .when_some(count_label, |el, count| {
                el.child(div().flex_1()).child(
                    Label::new(count)
                        .size(crate::ui::tokens::FontSize::Micro)
                        .color(SemanticColor::TertiaryLabel),
                )
            }),
        MusicSidebarState::Collapsed => {
            el.justify_center()
                .py(spacing::XS)
                .child(render_destination_icon(
                    icon,
                    selected,
                    shows_attention_mark,
                    cx,
                ))
        }
    };

    el.into_any_element()
}

/// Renders a destination icon, with the attention mark when one is called
/// for (ADR 0084 Decision 1). The mark never relies on color alone: the
/// row's tooltip and accessibility label already give the count in text.
fn render_destination_icon(
    icon: IconName,
    selected: bool,
    shows_attention_mark: bool,
    cx: &App,
) -> AnyElement {
    let color = if selected {
        color::accent()
    } else {
        color::text_primary()
    };
    let glyph = Icon::new(icon).size(IconSize::Action).color(color);
    if !shows_attention_mark {
        return glyph.into_any_element();
    }
    div()
        .relative()
        .child(glyph)
        .child(
            div()
                .absolute()
                .top_0()
                .right_0()
                .size(spacing::XS)
                .rounded_full()
                .bg(crate::ui::tokens::resolve_color(
                    cx,
                    crate::ui::tokens::SemanticColor::Danger,
                    None,
                )),
        )
        .into_any_element()
}
