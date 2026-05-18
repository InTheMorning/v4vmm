//! Recent Feeds route shell.
//!
//! ADR 0048 routes Recent Feeds into the workspace `ContentList` frame as a
//! first-class navigation entry. This shell owns only the page presentation:
//! view-mode control, list/tile rendering, and scroll pagination callbacks.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, AnyElement, App, ClickEvent, FontWeight, Image, InteractiveElement, IntoElement,
    ParentElement, ScrollHandle, ScrollWheelEvent, SharedString, StatefulInteractiveElement,
    Styled, Window,
};

use crate::ui::composites::{EntityKind, Segment, SegmentDisplay, SegmentedControl};
use crate::ui::control_styles::ControlStyle;
use crate::ui::layouts as layout;
use crate::ui::primitives::{
    Button as UiButton, Image as ImagePrimitive, Label, LabelVariant, Skeleton,
};
use crate::ui::shells::search_result_rows::{
    feed_fields, render_pending_result_row, render_result_row, SearchResultSelectHandler,
};
use crate::ui::tokens::{color as token_color, FontSize, Radius, SemanticColor, Spacing};
use crate::view_models::pagination::{should_auto_load_more, AUTO_PAGINATE_THRESHOLD_PX};
use crate::view_models::recent_feeds::{
    RecentFeedResultRow, RecentFeedsPageState, RecentFeedsPageVm, RecentFeedsViewMode,
};
use crate::view_models::search_results::{EmptyStateDisplay, FeedResultDisplay, SearchResultsTab};

type RecentFeedSelectHandler = Rc<dyn Fn(String, &mut Window, &mut App) + 'static>;
type RecentFeedsLoadMoreHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;
type RecentFeedsViewModeSelectHandler =
    Rc<dyn Fn(RecentFeedsViewMode, &mut Window, &mut App) + 'static>;

/// Callback slots supplied by the Recent Feeds route owner.
#[derive(Default)]
#[must_use]
pub(crate) struct RecentFeedsPageSlots {
    result_select: Option<RecentFeedSelectHandler>,
    load_more: Option<RecentFeedsLoadMoreHandler>,
    view_mode_select: Option<RecentFeedsViewModeSelectHandler>,
    scroll_handle: Option<ScrollHandle>,
    thumbnails: BTreeMap<String, Option<Arc<Image>>>,
}

impl RecentFeedsPageSlots {
    /// Creates empty Recent Feeds slots.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Supplies the result-row activation callback.
    pub(crate) fn on_result_select(
        mut self,
        handler: impl Fn(String, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.result_select = Some(Rc::new(handler));
        self
    }

    /// Supplies the view-mode selection callback.
    pub(crate) fn on_view_mode_select(
        mut self,
        handler: impl Fn(RecentFeedsViewMode, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.view_mode_select = Some(Rc::new(handler));
        self
    }

    /// Supplies the load-more callback for scroll and fallback button pagination.
    pub(crate) fn on_load_more(
        mut self,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.load_more = Some(Rc::new(handler));
        self
    }

    /// Supplies the scroll handle used for auto-pagination.
    pub(crate) fn with_scroll_handle(mut self, scroll_handle: ScrollHandle) -> Self {
        self.scroll_handle = Some(scroll_handle);
        self
    }

    /// Supplies pre-resolved thumbnail images keyed by feed result id.
    pub(crate) fn with_thumbnails(
        mut self,
        thumbnails: BTreeMap<String, Option<Arc<Image>>>,
    ) -> Self {
        self.thumbnails = thumbnails;
        self
    }
}

/// Renders the Recent Feeds route.
pub(crate) fn render_recent_feeds_page(
    vm: &RecentFeedsPageVm,
    slots: &RecentFeedsPageSlots,
    cx: &App,
) -> AnyElement {
    let view_mode = vm.view_mode();
    let is_loading = vm.is_loading();
    let has_more = vm.has_more();
    let body = match vm.state() {
        RecentFeedsPageState::Loading => match view_mode {
            RecentFeedsViewMode::Tiles => render_recent_feed_tiles(&[], true, has_more, slots, cx),
            RecentFeedsViewMode::List => render_recent_feed_rows(&[], true, has_more, slots, cx),
        },
        RecentFeedsPageState::Loaded(rows) if rows.is_empty() => render_empty_state(
            &EmptyStateDisplay::new(
                "No recent feeds",
                "MusicIndex did not return recent feeds.",
                None,
            ),
            cx,
        ),
        RecentFeedsPageState::Loaded(rows) => match view_mode {
            RecentFeedsViewMode::Tiles => {
                render_recent_feed_tiles(rows, is_loading, has_more, slots, cx)
            }
            RecentFeedsViewMode::List => {
                render_recent_feed_rows(rows, is_loading, has_more, slots, cx)
            }
        },
        RecentFeedsPageState::Error { message, detail } => {
            render_empty_state(&EmptyStateDisplay::new(message, detail, None), cx)
        }
    };

    div()
        .id("recent-feeds-page")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_hidden()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(Spacing::XS.scaled(cx))
                .p(Spacing::MD.scaled(cx))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(Label::new("Recent Feeds").variant(LabelVariant::Headline))
                        .child(Label::new("Index").variant(LabelVariant::Caption)),
                )
                .child(render_recent_feeds_view_mode_control(
                    view_mode,
                    slots.view_mode_select.as_ref(),
                )),
        )
        .child(body)
        .into_any_element()
}

fn render_recent_feeds_view_mode_control(
    selected: RecentFeedsViewMode,
    on_view_mode_select: Option<&RecentFeedsViewModeSelectHandler>,
) -> SegmentedControl<RecentFeedsViewMode> {
    let mut control = SegmentedControl::new(selected).filter_style().segments(
        [RecentFeedsViewMode::Tiles, RecentFeedsViewMode::List].map(|mode| {
            Segment::new(SegmentDisplay {
                id: SharedString::from(format!("recent-feeds-view-{}", mode.id_suffix())).into(),
                key: mode,
                label: SharedString::from(mode.label()),
                a11y_label: SharedString::from(mode.a11y_label()),
            })
        }),
    );

    if let Some(handler) = on_view_mode_select.cloned() {
        control = control.on_select(move |mode, window, cx| {
            handler(*mode, window, cx);
        });
    }

    control
}

fn render_recent_feed_rows(
    rows: &[RecentFeedResultRow],
    show_loading_placeholders: bool,
    has_more: bool,
    slots: &RecentFeedsPageSlots,
    cx: &App,
) -> AnyElement {
    let on_result_select = slots.result_select.clone().map(|handler| {
        Rc::new(move |_tab, result_id, window: &mut Window, cx: &mut App| {
            handler(result_id, window, cx);
        }) as SearchResultSelectHandler
    });
    let pending_rows = if show_loading_placeholders { 3 } else { 0 };

    let mut container = div()
        .id("recent-feeds-rows")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_y_scroll()
        .px(Spacing::MD.scaled(cx))
        .pb(Spacing::MD.scaled(cx))
        .gap(Spacing::XXS.scaled(cx));

    container =
        attach_recent_feeds_auto_pagination(container, has_more, show_loading_placeholders, slots);

    container
        .children(rows.iter().map(|(_id, row)| {
            render_result_row(
                SearchResultsTab::Feeds,
                EntityKind::Feed,
                feed_fields(row),
                slots.thumbnails.get(&row.id).cloned().flatten(),
                on_result_select.as_ref(),
            )
        }))
        .children((0..pending_rows).map(|index| {
            render_pending_result_row(SearchResultsTab::Feeds, EntityKind::Feed, index)
        }))
        .when(has_more && !show_loading_placeholders, |el| {
            el.child(render_recent_feeds_load_more_footer(slots, cx))
        })
        .into_any_element()
}

fn render_recent_feed_tiles(
    rows: &[RecentFeedResultRow],
    show_loading_placeholders: bool,
    has_more: bool,
    slots: &RecentFeedsPageSlots,
    cx: &App,
) -> AnyElement {
    let pending_tiles = if show_loading_placeholders {
        if rows.is_empty() {
            8
        } else {
            3
        }
    } else {
        0
    };

    let mut container = div()
        .id("recent-feeds-tiles")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_y_scroll()
        .px(Spacing::MD.scaled(cx))
        .pb(Spacing::MD.scaled(cx));

    container =
        attach_recent_feeds_auto_pagination(container, has_more, show_loading_placeholders, slots);

    container
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap(Spacing::MD.scaled(cx))
                .children(
                    rows.iter()
                        .map(|(_id, row)| render_recent_feed_tile(row, slots, cx)),
                )
                .children(
                    (0..pending_tiles).map(|index| render_pending_recent_feed_tile(index, cx)),
                ),
        )
        .when(has_more && !show_loading_placeholders, |el| {
            el.child(render_recent_feeds_load_more_footer(slots, cx))
        })
        .into_any_element()
}

fn attach_recent_feeds_auto_pagination<E>(
    mut container: E,
    has_more: bool,
    is_loading: bool,
    slots: &RecentFeedsPageSlots,
) -> E
where
    E: InteractiveElement + StatefulInteractiveElement,
{
    let Some(scroll_handle) = slots.scroll_handle.as_ref() else {
        return container;
    };

    container = container.track_scroll(scroll_handle);
    let Some(handler) = slots.load_more.clone() else {
        return container;
    };

    let scroll_for_listener = scroll_handle.clone();
    container.on_scroll_wheel(move |_: &ScrollWheelEvent, window, cx| {
        if !has_more {
            return;
        }

        let max_y = f32::from(scroll_for_listener.max_offset().height);
        let offset_y = f32::from(scroll_for_listener.offset().y);
        let remaining = max_y + offset_y;
        if should_auto_load_more(remaining, AUTO_PAGINATE_THRESHOLD_PX, has_more, is_loading) {
            handler(window, cx);
        }
    })
}

fn render_recent_feeds_load_more_footer(slots: &RecentFeedsPageSlots, cx: &App) -> AnyElement {
    let mut load_more = UiButton::styled("recent-feeds-load-more", ControlStyle::Ghost)
        .label("Load more")
        .a11y_label("Load more recent feeds");

    if let Some(handler) = slots.load_more.clone() {
        load_more = load_more.on_click(move |_: &ClickEvent, window, cx| {
            handler(window, cx);
        });
    } else {
        load_more = load_more.disabled(true);
    }

    div()
        .pt(Spacing::SM.scaled(cx))
        .child(load_more)
        .into_any_element()
}

fn render_recent_feed_tile(
    row: &FeedResultDisplay,
    slots: &RecentFeedsPageSlots,
    cx: &App,
) -> AnyElement {
    let tile_id = row.id.clone();
    let radius_lg = Radius::LG.scaled(cx);
    let radius_md = Radius::MD.scaled(cx);
    let hover_bg = token_color(cx, SemanticColor::SecondarySystemBackground);
    let fallback_bg = token_color(cx, SemanticColor::SystemFill);
    let thumbnail = slots.thumbnails.get(&row.id).cloned().flatten();
    let mut tile = div()
        .id(SharedString::from(format!("recent-feed-tile-{tile_id}")))
        .flex()
        .flex_col()
        .gap(Spacing::SM.scaled(cx))
        .w(layout::SEARCH_TILE_WIDTH)
        .p(Spacing::SM.scaled(cx))
        .rounded(radius_lg);

    if let Some(handler) = slots.result_select.clone() {
        tile = tile
            .cursor_pointer()
            .hover(move |el| el.bg(hover_bg))
            .on_click(move |_: &ClickEvent, window, cx| {
                handler(tile_id.clone(), window, cx);
            });
    }

    tile.child(render_recent_feed_tile_artwork(
        thumbnail,
        fallback_bg,
        radius_md,
        cx,
    ))
    .child(
        div().w(layout::THUMBNAIL_XL).min_w_0().child(
            Label::new(row.label.clone())
                .size(FontSize::Caption)
                .weight(FontWeight::MEDIUM)
                .truncated(),
        ),
    )
    .when(!row.secondary_text.is_empty(), |el| {
        el.child(
            div().w(layout::THUMBNAIL_XL).min_w_0().child(
                Label::new(row.secondary_text.clone())
                    .size(FontSize::Micro)
                    .color(SemanticColor::TertiaryLabel)
                    .truncated(),
            ),
        )
    })
    .into_any_element()
}

fn render_recent_feed_tile_artwork(
    thumbnail: Option<Arc<Image>>,
    fallback_bg: gpui::Rgba,
    radius_md: gpui::Pixels,
    cx: &App,
) -> AnyElement {
    let has_thumbnail = thumbnail.is_some();
    div()
        .w(layout::THUMBNAIL_XL)
        .h(layout::THUMBNAIL_XL)
        .rounded(radius_md)
        .overflow_hidden()
        .flex_shrink_0()
        .when_some(thumbnail, |el, image| {
            el.child(
                ImagePrimitive::new(image)
                    .dimension(layout::THUMBNAIL_XL)
                    .radius(Radius::MD),
            )
        })
        .when(!has_thumbnail, |el| {
            el.bg(fallback_bg)
                .flex()
                .items_center()
                .justify_center()
                .text_size(FontSize::Title2.scaled(cx))
                .child(SharedString::from(EntityKind::Feed.emoji()))
        })
        .into_any_element()
}

fn render_pending_recent_feed_tile(index: usize, cx: &App) -> AnyElement {
    div()
        .id(SharedString::from(format!(
            "recent-feed-tile-pending-{index}"
        )))
        .flex()
        .flex_col()
        .gap(Spacing::SM.scaled(cx))
        .w(layout::SEARCH_TILE_WIDTH)
        .p(Spacing::SM.scaled(cx))
        .rounded(Radius::LG.scaled(cx))
        .child(
            div().flex_shrink_0().child(
                Skeleton::block(layout::THUMBNAIL_XL, layout::THUMBNAIL_XL).radius(Radius::MD),
            ),
        )
        .child(
            div()
                .w(layout::THUMBNAIL_XL)
                .child(Skeleton::row().full_width()),
        )
        .child(
            div()
                .w(layout::THUMBNAIL_XL)
                .child(Skeleton::row().full_width()),
        )
        .into_any_element()
}

fn render_empty_state(empty: &EmptyStateDisplay, cx: &App) -> AnyElement {
    div()
        .flex()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .items_center()
        .justify_center()
        .p(Spacing::MD.scaled(cx))
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(Spacing::XS.scaled(cx))
                .child(Label::new(empty.title.clone()).variant(LabelVariant::Headline))
                .child(
                    Label::new(empty.secondary.clone())
                        .variant(LabelVariant::Caption)
                        .truncated(),
                ),
        )
        .into_any_element()
}
