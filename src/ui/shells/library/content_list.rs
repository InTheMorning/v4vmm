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
use crate::ui::primitives::{Button as UiButton, Image as ImagePrimitive, Label, Skeleton};
use crate::ui::tokens::{color, FontSize, Radius, SemanticColor, Size, Spacing};
use crate::view_models::library::{
    ContentListEntityKind, ContentListLoadMoreDisplay, ContentListLoadingStateDisplay,
    ContentListPageStateDisplay, ContentListPageVm, ContentListRowDisplay,
};
use crate::view_models::pagination::{should_auto_load_more, AUTO_PAGINATE_THRESHOLD_PX};
use crate::view_models::workspace::ContentViewMode;

struct ContentListRenderState<'a> {
    visible_rows: Vec<&'a ContentListRowDisplay>,
    loading_display: Option<ContentListLoadingStateDisplay>,
    state_display: Option<ContentListPageStateDisplay>,
    load_more_display: Option<ContentListLoadMoreDisplay>,
}

/// Renders the Music content-list page.
pub(crate) fn render_library_content_list(
    page: &ContentListPageVm,
    thumbnails: &BTreeMap<String, Option<Arc<Image>>>,
    scroll_handle: &ScrollHandle,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let render_state = ContentListRenderState {
        visible_rows: page.visible_rows(),
        loading_display: page.loading_display(),
        state_display: page.page_state_display(),
        load_more_display: page.load_more_display(),
    };
    let body = match page.view_mode() {
        ContentViewMode::List => {
            render_content_list_rows(page, render_state, thumbnails, scroll_handle, cx)
        }
        ContentViewMode::Tiles => {
            render_content_list_tiles(page, render_state, thumbnails, scroll_handle, cx)
        }
    };

    div()
        .id(ContentListPageVm::page_id())
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_hidden()
        .child(body)
        .into_any_element()
}

fn render_content_list_rows(
    page: &ContentListPageVm,
    render_state: ContentListRenderState<'_>,
    thumbnails: &BTreeMap<String, Option<Arc<Image>>>,
    scroll_handle: &ScrollHandle,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let pending_rows = render_state
        .loading_display
        .as_ref()
        .map_or(0, |display| display.skeleton_count);
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

    container
        .children(render_state.visible_rows.into_iter().map(|row| {
            render_content_list_row(row, thumbnails.get(&row.id).cloned().flatten(), cx)
        }))
        .children((0..pending_rows).map(|index| {
            SkeletonTrackRow::new(ContentListPageVm::skeleton_row_id(index))
                .show_duration(false)
                .into_any_element()
        }))
        .when_some(render_state.state_display, |el, display| {
            el.child(render_content_list_state(display, cx))
        })
        .when_some(render_state.load_more_display, |el, display| {
            el.child(render_content_list_load_more(display, cx))
        })
        .into_any_element()
}

fn render_content_list_tiles(
    page: &ContentListPageVm,
    render_state: ContentListRenderState<'_>,
    thumbnails: &BTreeMap<String, Option<Arc<Image>>>,
    scroll_handle: &ScrollHandle,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let pending_tiles = render_state
        .loading_display
        .as_ref()
        .map_or(0, |display| display.skeleton_count);
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
        .gap(Spacing::SM.scaled(cx));

    container = attach_content_list_auto_pagination(
        container,
        page.has_more(),
        page.is_loading(),
        scroll_handle,
        cx,
    );

    container
        .child(
            div()
                .id(ContentListPageVm::tiles_id())
                .flex()
                .flex_row()
                .flex_wrap()
                .gap(Spacing::MD.scaled(cx))
                .children(render_state.visible_rows.into_iter().map(|row| {
                    render_content_list_tile(row, thumbnails.get(&row.id).cloned().flatten(), cx)
                }))
                .children((0..pending_tiles).map(|index| render_pending_content_tile(index, cx))),
        )
        .when_some(render_state.state_display, |el, display| {
            el.child(render_content_list_state(display, cx))
        })
        .when_some(render_state.load_more_display, |el, display| {
            el.child(render_content_list_load_more(display, cx))
        })
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

fn render_content_list_tile(
    row: &ContentListRowDisplay,
    thumbnail: Option<Arc<Image>>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let row_id = row.id.clone();
    let entity_kind = entity_kind_for_content(row.entity_kind());
    let artwork_size = Size::ContentTileArtwork.scaled(cx);
    let hover_bg = color(cx, SemanticColor::SecondarySystemBackground);

    div()
        .id(SharedString::from(row.element_id()))
        .flex()
        .flex_col()
        .gap(Spacing::SM.scaled(cx))
        .w(Size::ContentTileWidth.scaled(cx))
        .p(Spacing::SM.scaled(cx))
        .rounded(Radius::MD.scaled(cx))
        .cursor_pointer()
        .hover(move |el| el.bg(hover_bg))
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            this.open_content_list_row(&row_id, cx);
        }))
        .child(render_content_list_tile_artwork(
            thumbnail,
            artwork_size,
            cx,
        ))
        .child(
            div().w(artwork_size).min_w_0().child(
                Label::new(row.title().to_string())
                    .size(FontSize::Caption)
                    .weight(FontWeight::MEDIUM)
                    .truncated(),
            ),
        )
        .when(!row.secondary_text().is_empty(), |el| {
            el.child(
                div().w(artwork_size).min_w_0().child(
                    Label::new(row.secondary_text().to_string())
                        .size(FontSize::Micro)
                        .color(SemanticColor::TertiaryLabel)
                        .truncated(),
                ),
            )
        })
        .child(render_content_list_tile_badges(row, entity_kind, cx))
        .into_any_element()
}

fn render_content_list_tile_artwork(
    thumbnail: Option<Arc<Image>>,
    artwork_size: gpui::Pixels,
    cx: &App,
) -> AnyElement {
    match thumbnail {
        Some(image) => ImagePrimitive::new(image)
            .dimension(artwork_size)
            .radius(Radius::MD)
            .into_any_element(),
        None => render_empty_content_tile_artwork(artwork_size, cx),
    }
}

fn render_empty_content_tile_artwork(artwork_size: gpui::Pixels, cx: &App) -> AnyElement {
    div()
        .w(artwork_size)
        .h(artwork_size)
        .rounded(Radius::MD.scaled(cx))
        .overflow_hidden()
        .flex_shrink_0()
        .bg(color(cx, SemanticColor::SystemFill))
        .border_1()
        .border_color(color(cx, SemanticColor::Separator))
        .into_any_element()
}

fn render_content_list_tile_badges(
    row: &ContentListRowDisplay,
    entity_kind: EntityKind,
    cx: &App,
) -> AnyElement {
    let mut badges = div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(Spacing::XS.scaled(cx))
        .child(TagBadge::new(TagBadgeDisplay {
            kind: entity_kind,
            label: Some(SharedString::from(row.entity_badge.label)),
        }))
        .child(
            Label::new(row.library_badge.label)
                .size(FontSize::Micro)
                .color(SemanticColor::TertiaryLabel)
                .truncated(),
        );

    if let Some(state_label) = row.state_label {
        badges = badges.child(
            Label::new(state_label)
                .size(FontSize::Micro)
                .color(SemanticColor::Accent)
                .truncated(),
        );
    }

    badges.into_any_element()
}

fn render_pending_content_tile(index: usize, cx: &App) -> AnyElement {
    let artwork_size = Size::ContentTileArtwork.scaled(cx);

    div()
        .id(ContentListPageVm::skeleton_tile_id(index))
        .flex()
        .flex_col()
        .gap(Spacing::SM.scaled(cx))
        .w(Size::ContentTileWidth.scaled(cx))
        .p(Spacing::SM.scaled(cx))
        .rounded(Radius::MD.scaled(cx))
        .child(
            div()
                .flex_shrink_0()
                .child(Skeleton::block(artwork_size, artwork_size).radius(Radius::MD)),
        )
        .child(div().w(artwork_size).child(Skeleton::row().full_width()))
        .child(div().w(artwork_size).child(Skeleton::row().full_width()))
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
