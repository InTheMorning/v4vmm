//! Library content-list surface.
//!
//! ADR 0062 makes the Music default content region a paged list of recent
//! music. This shell owns only GPUI layout and interaction wiring; row labels,
//! state labels, load-more labels, and empty/error copy come from the view
//! model.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{
    div, prelude::FluentBuilder, AnyElement, App, ClickEvent, Context, FontWeight, Image,
    InteractiveElement, IntoElement, ParentElement, ScrollHandle, ScrollWheelEvent, SharedString,
    StatefulInteractiveElement, Styled,
};

use crate::library::LibraryApp;
use crate::ui::composites::{
    EntityKind, ListRow, ListRowA11yLabel, SkeletonTrackRow, TagBadge, TagBadgeDisplay, Thumbnail,
    ThumbnailSize,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::primitives::{Button as UiButton, Label};
use crate::ui::tokens::{FontSize, SemanticColor, Spacing};
use crate::view_models::library::{
    ContentListEntityKind, ContentListLoadMoreDisplay, ContentListPageStateDisplay,
    ContentListPageVm, ContentListRowDisplay,
};
use crate::view_models::pagination::{should_auto_load_more, AUTO_PAGINATE_THRESHOLD_PX};

/// Renders the Music content-list page.
pub(crate) fn render_library_content_list(
    page: &ContentListPageVm,
    thumbnails: &BTreeMap<String, Option<Arc<Image>>>,
    scroll_handle: &ScrollHandle,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let visible_rows = page.visible_rows();
    let loading_display = page.loading_display();
    let state_display = page.page_state_display();
    let load_more_display = page.load_more_display();
    let mut container = div()
        .id(ContentListPageVm::rows_id())
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_y_scroll()
        .px(Spacing::MD.scaled(cx))
        .pb(Spacing::MD.scaled(cx))
        .gap(Spacing::XXS.scaled(cx));

    container = attach_content_list_auto_pagination(
        container,
        page.has_more(),
        page.is_loading(),
        scroll_handle,
        cx,
    );

    div()
        .id(ContentListPageVm::page_id())
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_hidden()
        .child(
            container
                .children(visible_rows.into_iter().map(|row| {
                    render_content_list_row(row, thumbnails.get(&row.id).cloned().flatten(), cx)
                }))
                .children(
                    loading_display
                        .iter()
                        .flat_map(|display| 0..display.skeleton_count)
                        .map(|index| {
                            SkeletonTrackRow::new(ContentListPageVm::skeleton_row_id(index))
                                .show_duration(false)
                                .into_any_element()
                        }),
                )
                .when_some(state_display, |el, display| {
                    el.child(render_content_list_state(display, cx))
                })
                .when_some(load_more_display, |el, display| {
                    el.child(render_content_list_load_more(display, cx))
                }),
        )
        .into_any_element()
}

fn attach_content_list_auto_pagination<E>(
    mut container: E,
    has_more: bool,
    is_loading: bool,
    scroll_handle: &ScrollHandle,
    cx: &mut Context<LibraryApp>,
) -> E
where
    E: InteractiveElement + StatefulInteractiveElement,
{
    container = container.track_scroll(scroll_handle);
    let scroll_for_listener = scroll_handle.clone();
    container.on_scroll_wheel(cx.listener(move |this, _: &ScrollWheelEvent, _window, cx| {
        if !has_more {
            return;
        }

        let max_y = f32::from(scroll_for_listener.max_offset().height);
        let offset_y = f32::from(scroll_for_listener.offset().y);
        let remaining = max_y + offset_y;
        if should_auto_load_more(remaining, AUTO_PAGINATE_THRESHOLD_PX, has_more, is_loading) {
            this.start_recent_music_load(true, cx);
        }
    }))
}

fn render_content_list_row(
    row: &ContentListRowDisplay,
    thumbnail: Option<Arc<Image>>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let row_id = row.id.clone();
    let entity_kind = entity_kind_for_content(row.entity_kind());
    let mut list_row = ListRow::new(SharedString::from(row.element_id()))
        .a11y_label(ListRowA11yLabel {
            label: SharedString::from(row.a11y_label().to_string()),
        })
        .child(Thumbnail::new(entity_kind, ThumbnailSize::Sm).image(thumbnail))
        .child(render_content_list_row_text(row))
        .child(
            Label::new(row.library_badge.label)
                .size(FontSize::Micro)
                .color(SemanticColor::TertiaryLabel)
                .truncated(),
        );

    if let Some(state_label) = row.state_label {
        list_row = list_row.child(
            Label::new(state_label)
                .size(FontSize::Micro)
                .color(SemanticColor::Accent)
                .truncated(),
        );
    }

    list_row
        .child(TagBadge::new(TagBadgeDisplay {
            kind: entity_kind,
            label: Some(SharedString::from(row.entity_badge.label)),
        }))
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            this.open_content_list_row(&row_id, cx);
        }))
        .into_any_element()
}

fn render_content_list_row_text(row: &ContentListRowDisplay) -> AnyElement {
    let mut text = div().flex_1().min_w_0().child(
        Label::new(row.title().to_string())
            .size(FontSize::Micro)
            .weight(FontWeight::MEDIUM)
            .truncated(),
    );

    if !row.secondary_text().is_empty() {
        text = text.child(
            Label::new(row.secondary_text().to_string())
                .size(FontSize::Micro)
                .color(SemanticColor::TertiaryLabel)
                .truncated(),
        );
    }

    text.into_any_element()
}

fn render_content_list_state(display: ContentListPageStateDisplay, cx: &App) -> AnyElement {
    match display {
        ContentListPageStateDisplay::Loading(display) => div()
            .text_center()
            .p(Spacing::LG.scaled(cx))
            .child(
                Label::new(display.label)
                    .size(FontSize::Caption)
                    .color(SemanticColor::TertiaryLabel),
            )
            .into_any_element(),
        ContentListPageStateDisplay::Empty(display) => render_content_list_message(
            display.title,
            display.secondary,
            SemanticColor::TertiaryLabel,
            cx,
        ),
        ContentListPageStateDisplay::Failed(display) => {
            render_content_list_message(display.title, display.detail, SemanticColor::Danger, cx)
        }
    }
}

fn render_content_list_message(
    title: impl Into<SharedString>,
    detail: impl Into<SharedString>,
    color: SemanticColor,
    cx: &App,
) -> AnyElement {
    div()
        .text_center()
        .p(Spacing::XXL.scaled(cx))
        .child(Label::new(title).size(FontSize::Caption).color(color))
        .child(
            Label::new(detail)
                .size(FontSize::Micro)
                .color(SemanticColor::TertiaryLabel),
        )
        .into_any_element()
}

fn render_content_list_load_more(
    display: ContentListLoadMoreDisplay,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    div()
        .pt(Spacing::SM.scaled(cx))
        .child(
            UiButton::styled(display.button_id, ControlStyle::Ghost)
                .label(display.label)
                .a11y_label(display.a11y_label)
                .disabled(display.disabled)
                .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                    this.start_recent_music_load(true, cx);
                })),
        )
        .into_any_element()
}

const fn entity_kind_for_content(kind: ContentListEntityKind) -> EntityKind {
    match kind {
        ContentListEntityKind::Artist => EntityKind::Artist,
        ContentListEntityKind::Release => EntityKind::Release,
        ContentListEntityKind::Track => EntityKind::Track,
    }
}
