//! Discover artist, publisher, and feed-list inspector sections.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, ClickEvent, Context, FontWeight, Image, InteractiveElement,
    SharedString, Styled,
};

use crate::api::{Feed, Publisher};
use crate::discover::{ArtistContext, SearchApp};
use crate::ui::composites::{
    DetailGrid, DetailHeader, DetailHeaderDisplay, DetailRow as CompositeDetailRow,
    DetailTextRow as CompositeDetailTextRow, EntityKind, Thumbnail, ThumbnailSize,
};
use crate::ui::layouts as layout;
use crate::ui::primitives::{Label, SectionHeader};
use crate::ui::shells::artist;
use crate::ui::shells::discover::recent::{
    render_discover_recent, DiscoverRecentParams, DiscoverRecentTile,
};
use crate::ui::style::{color, radius, spacing, typography};
use crate::ui::tokens::FontSize;
use crate::view_models::search::{
    PublisherInspectorVm, RecentFeedTileDisplay, RecentFeedTileVm, SearchViewModel,
};

pub(crate) fn render_artist_inspector(
    frame_image: Option<Arc<Image>>,
    artist_context: &ArtistContext,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let view = crate::views::ArtistView::from_api(artist_context.artist.clone());
    let track_count = artist_context
        .artist
        .track_count
        .unwrap_or(i32::try_from(artist_context.tracks.len()).unwrap_or(i32::MAX));

    let feed_section = (!artist_context.feeds.is_empty())
        .then(|| render_feed_list_section(artist_context.feeds.clone(), app, cx));

    artist::render_artist_view(
        &view,
        &artist_context.feeds,
        frame_image,
        artist_context.has_more_tracks,
        Some(track_count),
        feed_section,
    )
}

pub(crate) fn render_publisher_inspector(
    publisher: &Publisher,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let vm = PublisherInspectorVm::new(publisher);

    div()
        .flex()
        .flex_col()
        .gap(spacing::LG)
        .child(DetailHeader::new(DetailHeaderDisplay {
            kind: EntityKind::Publisher,
            title: vm.title().into(),
            subtitle: None,
            data_rows: Vec::new(),
        }))
        .child(DetailGrid::new(
            vm.detail_rows()
                .into_iter()
                .map(|(k, v)| {
                    CompositeDetailRow::text(CompositeDetailTextRow {
                        key: k.into(),
                        value: v,
                        max_lines: 6,
                    })
                })
                .collect::<Vec<_>>(),
        ))
        .when(vm.has_feed_list(), |el| {
            el.child(render_feed_list_section(vm.feeds(), app, cx))
        })
        .into_any_element()
}

pub(crate) fn render_feed_list_section(
    feeds: Vec<Feed>,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let section_display = SearchViewModel::feed_list_section_display();
    let tiles: Vec<AnyElement> = feeds
        .into_iter()
        .map(|feed| {
            let RecentFeedTileDisplay {
                id,
                feed_list_tile_id,
                title,
                episode_note,
                image_url,
                ..
            } = RecentFeedTileVm::new(&feed).display();
            let click_guid = id;
            let click_title = title.clone();
            let thumb = app.thumbnail_for_url(image_url.as_deref(), cx);
            div()
                .id(SharedString::from(feed_list_tile_id))
                .w(layout::FEED_TILE_WIDTH)
                .flex()
                .flex_col()
                .gap(spacing::SM)
                .p(spacing::XS)
                .rounded(radius::MD)
                .cursor_pointer()
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
                .when_some(episode_note, |el, episode_note| {
                    el.child(
                        div()
                            .text_color(color::text_muted())
                            .text_size(typography::SIZE_MICRO)
                            .child(SharedString::from(episode_note)),
                    )
                })
                .into_any_element()
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .gap(spacing::SM)
        .child(SectionHeader::new(section_display.heading))
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap(spacing::MD)
                .children(tiles),
        )
        .into_any_element()
}

pub(crate) fn render_recent_feeds_tiles(
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let snapshot = app.vm.recent_feeds_snapshot();
    let display = snapshot.display;
    let feeds = snapshot.feeds;
    let status = snapshot.status;
    let has_more = snapshot.has_more;
    let loading = snapshot.loading;
    let is_empty = snapshot.empty;

    let mut tiles: Vec<DiscoverRecentTile> = Vec::with_capacity(feeds.len());
    for feed in feeds {
        let tile_vm = RecentFeedTileVm::new(&feed);
        let display = tile_vm.display();
        let target = display.open_target();
        if target.guid.trim().is_empty() {
            continue;
        }
        let thumbnail = app.thumbnail_for_url(display.image_url.as_deref(), cx);
        tiles.push(DiscoverRecentTile::new(display, thumbnail));
    }

    render_discover_recent(
        DiscoverRecentParams {
            tiles,
            display,
            status,
            has_more,
            loading,
            is_empty,
        },
        cx,
    )
}
