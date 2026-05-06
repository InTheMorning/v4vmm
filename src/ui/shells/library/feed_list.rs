//! Library feed-list surface.
//!
//! Renders the artist-selected feed list and routes feed-row selection back to
//! `LibraryApp`; the screen keeps selected-entity state and hydration.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, Context, FontWeight, Image, InteractiveElement, SharedString,
    Styled,
};

use crate::library::{LibraryApp, LibraryArtistDetail};
use crate::ui::composites::{
    DisclosureSupplementDisplay, DisclosureSupplementLabel, EntityKind, Thumbnail, ThumbnailSize,
};
use crate::ui::shells::artist::{render_artist_detail_shell, ArtistDetailBehaviorSlots};
use crate::ui::style::{color, radius, spacing, typography};
use crate::view_models::library::{
    ArtistFeedSummaryDisplay, LibraryArtistDetailVm, LibraryChromeDisplay,
};

pub(crate) fn render_library_feed_list(
    detail: &LibraryArtistDetail,
    album_thumbs: &BTreeMap<String, Option<Arc<Image>>>,
    chrome: &LibraryChromeDisplay,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let vm = LibraryArtistDetailVm::new(&detail.name, &detail.tracks);
    let page = vm.page();

    let feed_rows: Vec<AnyElement> = vm
        .feed_summaries()
        .into_iter()
        .map(|summary| {
            let ArtistFeedSummaryDisplay {
                element_id,
                title,
                thumb_url,
                track_count_label,
            } = summary.display();
            let thumb_image = thumb_url
                .as_ref()
                .and_then(|url| album_thumbs.get(url.as_str()))
                .cloned()
                .flatten();
            let feed_name_for_click = title.clone();

            div()
                .id(SharedString::from(element_id))
                .flex()
                .flex_row()
                .items_center()
                .gap(spacing::SM)
                .px(spacing::SM)
                .py(spacing::XS)
                .rounded(radius::SM)
                .hover(|el| el.bg(color::bg_surface_hi()))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.select_album_by_name(&feed_name_for_click, cx);
                }))
                .child(Thumbnail::new(EntityKind::Feed, ThumbnailSize::Sm).image(thumb_image))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(spacing::XXS)
                        .child(
                            div()
                                .text_size(typography::SIZE_MICRO)
                                .font_weight(FontWeight::MEDIUM)
                                .truncate()
                                .child(SharedString::from(title)),
                        )
                        .child(DisclosureSupplementLabel::new(
                            DisclosureSupplementDisplay {
                                label: track_count_label.into(),
                            },
                        )),
                )
                .into_any_element()
        })
        .collect();

    div()
        .id(chrome.artist_detail_scroll_id)
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_y_scroll()
        .p(spacing::LG)
        .child(render_artist_detail_shell(
            &page,
            ArtistDetailBehaviorSlots {
                image: None,
                feed_section: Some(
                    div()
                        .flex()
                        .flex_col()
                        .gap(spacing::XXS)
                        .children(feed_rows)
                        .into_any_element(),
                ),
            },
        ))
        .into_any_element()
}
