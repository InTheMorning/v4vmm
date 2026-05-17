//! Discover feed inspector surface.
//!
//! Owns the inspector frame chrome, inspector routing, and feed-specific body.
//! Selection state and mutators stay on `SearchApp`.

#![warn(clippy::pedantic)]

use gpui::{
    div, prelude::*, AnyElement, ClickEvent, Context, FontWeight, InteractiveElement,
    ScrollWheelEvent, SharedString, Styled,
};

use crate::api::Feed;
use crate::discover::{InspectorDetail, InspectorFrame, SearchApp};
use crate::ui::composites::{EntityKind, SkeletonInspector, Thumbnail, ThumbnailSize};
use crate::ui::control_styles::ControlStyle;
use crate::ui::layouts as layout;
use crate::ui::primitives::{Button as UiButton, Label, LoadingMessage};
use crate::ui::shells::discover::feed_lists::{
    render_artist_inspector, render_publisher_inspector, render_recent_feeds_tiles,
};
use crate::ui::shells::discover::track_inspector::{
    render_discover_track_inspector_core, render_discover_track_inspector_lazy_sections,
};
use crate::ui::shells::feed;
use crate::ui::style::{color, radius, spacing, typography};
use crate::ui::tokens::{FontSize, SemanticColor};
use crate::view_models::search::{
    should_auto_load_more, InspectorChromeDisplay, LazyPanel, RecentFeedTileDisplay,
    RecentFeedTileVm, SearchViewModel, AUTO_PAGINATE_THRESHOLD_PX,
};

pub(crate) fn render_inspector(
    frame: Option<&InspectorFrame>,
    show_back: bool,
    show_recents_root: bool,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let chrome = SearchViewModel::inspector_chrome_display();
    let title = SearchViewModel::inspector_title_display(
        show_recents_root,
        frame.map(|frame| frame.title.as_str()),
    );
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_hidden()
        .child(
            div()
                .min_h(layout::ROW_HEIGHT)
                .bg(color::bg_surface())
                .border_b_1()
                .border_color(color::border_subtle())
                .px(spacing::MD)
                .py(spacing::SM)
                .flex()
                .flex_row()
                .items_center()
                .gap(spacing::SM)
                .when(show_back, |el| {
                    el.child(
                        UiButton::styled(chrome.breadcrumb_root_button_id, ControlStyle::Ghost)
                            .label(chrome.breadcrumb_root_label)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.inspector_back(cx);
                            })),
                    )
                    .child(
                        Label::new("/")
                            .size(FontSize::Micro)
                            .color(SemanticColor::TertiaryLabel),
                    )
                })
                .child(
                    div().flex_1().child(
                        Label::new(title)
                            .size(FontSize::Micro)
                            .color(SemanticColor::TertiaryLabel)
                            .truncated(),
                    ),
                ),
        )
        .child({
            let mut scroll_box = div()
                .id(chrome.scroll_id)
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .p(spacing::LG);
            if let Some(frame) = frame {
                scroll_box = scroll_box.track_scroll(&frame.scroll_handle);
            } else if show_recents_root {
                let recents_handle = app.recents_scroll.clone();
                let recents_for_listener = recents_handle.clone();
                scroll_box = scroll_box
                    .track_scroll(&recents_handle)
                    .on_scroll_wheel(cx.listener(
                        move |this: &mut SearchApp, _event: &ScrollWheelEvent, _window, cx| {
                            let max_y = f32::from(recents_for_listener.max_offset().height);
                            let offset_y = f32::from(recents_for_listener.offset().y);
                            let remaining = max_y + offset_y;
                            if should_auto_load_more(
                                remaining,
                                AUTO_PAGINATE_THRESHOLD_PX,
                                this.vm.recent_has_more,
                                this.vm.recent_loading,
                            ) {
                                this.load_recent_feeds(true, cx);
                            }
                        },
                    ));
            }
            scroll_box.child(match frame {
                Some(frame) => render_inspector_body(frame, app, cx),
                None if show_recents_root => render_recent_feeds_tiles(app, cx),
                None => render_inspector_empty(&chrome),
            })
        })
        .into_any_element()
}

fn render_inspector_body(
    frame: &InspectorFrame,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    match &frame.detail {
        InspectorDetail::Loading(_) => skeleton_for_entity(&frame.entity_type).into_any_element(),
        InspectorDetail::Error(error) => {
            LoadingMessage::new(SearchViewModel::inspector_error_message(error)).into_any_element()
        }
        InspectorDetail::Artist(artist_context) => {
            render_artist_inspector(frame.image.clone(), artist_context, app, cx)
        }
        InspectorDetail::Feed(feed) => render_discover_feed_inspector(frame, feed, app, cx),
        InspectorDetail::Track(track_context) => {
            render_discover_track_inspector_core(frame, track_context, app, cx)
        }
        InspectorDetail::Publisher(publisher) => render_publisher_inspector(publisher, app, cx),
    }
}

/// Body-row count for the inspector skeleton, chosen to roughly mirror the
/// populated layout for each entity kind so the placeholder occupies the
/// same vertical footprint as the real content.
fn skeleton_for_entity(entity_type: &str) -> SkeletonInspector {
    match entity_type {
        "track" => SkeletonInspector::new().body_rows(8),
        "feed" => SkeletonInspector::new().body_rows(6),
        "artist" => SkeletonInspector::new().body_rows(5),
        "publisher" => SkeletonInspector::new().body_rows(4),
        _ => SkeletonInspector::new(),
    }
}

fn render_discover_feed_inspector(
    frame: &InspectorFrame,
    feed: &Feed,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let view = crate::views::FeedView::from_api(feed.clone());
    let tracks = SearchViewModel::feed_inspector_tracks(feed);
    let ctx = crate::ui_context::ViewContext::Discover;
    let mut panels = Vec::new();
    if let Some(section) = podroll_section(frame, app, cx) {
        panels.push(section);
    }
    panels.push(render_discover_track_inspector_lazy_sections(
        frame, app, cx,
    ));

    feed::render_feed_view(&view, &tracks, &ctx, frame, panels, app, cx)
}

fn podroll_section(
    frame: &InspectorFrame,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> Option<AnyElement> {
    let feeds = match &frame.podroll {
        LazyPanel::Loaded(feeds) if !feeds.is_empty() => feeds.clone(),
        _ => return None,
    };

    let mut tiles: Vec<AnyElement> = Vec::with_capacity(feeds.len());
    for feed in feeds {
        let RecentFeedTileDisplay {
            id,
            podroll_tile_id,
            title,
            image_url,
            ..
        } = RecentFeedTileVm::new(&feed).display();
        if id.trim().is_empty() {
            continue;
        }
        let click_title = title.clone();
        let click_guid = id;
        let thumb = app.thumbnail_for_url(image_url.as_deref(), cx);
        let tile = div()
            .id(SharedString::from(podroll_tile_id))
            .flex_shrink_0()
            .w(layout::FEED_TILE_WIDTH)
            .flex()
            .flex_col()
            .gap(spacing::SM)
            .p(spacing::XS)
            .rounded(radius::MD)
            .cursor_pointer()
            .hover(|el| el.bg(color::bg_surface()))
            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                this.push_inspector("feed".into(), click_guid.clone(), click_title.clone(), cx);
            }))
            .child(Thumbnail::new(EntityKind::Feed, ThumbnailSize::Lg).image(thumb.clone()))
            .child(
                div().line_height(typography::LINE_COMPACT).child(
                    Label::new(title)
                        .size(FontSize::Caption)
                        .weight(FontWeight::MEDIUM)
                        .truncated(),
                ),
            )
            .into_any_element();
        tiles.push(tile);
    }

    if tiles.is_empty() {
        return None;
    }
    let section_display = SearchViewModel::podroll_section_display(&frame.entity_id);

    Some(
        div()
            .flex()
            .flex_col()
            .gap(spacing::SM)
            .child(
                div()
                    .text_size(typography::SIZE_HEADLINE)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(section_display.heading_label),
            )
            .child(
                div()
                    .id(SharedString::from(section_display.scroll_id))
                    .flex()
                    .flex_row()
                    .gap(spacing::MD)
                    .overflow_x_scroll()
                    .pb(spacing::XS)
                    .children(tiles),
            )
            .into_any_element(),
    )
}

fn render_inspector_empty(display: &InspectorChromeDisplay) -> AnyElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .text_color(color::text_muted())
        .gap(spacing::SM)
        .child(div().text_3xl().opacity(0.4).child(display.empty_icon))
        .child(display.empty_label)
        .into_any_element()
}
