use crate::api::Feed;
use crate::ui::composites::{
    DetailGrid, DetailHeader, DetailHeaderDisplay, DetailRow, DetailTextRow, EntityKind,
};
use crate::ui::primitives::VStack;
use crate::ui::tokens::Spacing;
use crate::view_models::artist::ArtistVm;
use crate::view_models::artist_detail::ArtistDetailPageVm;
use crate::views::ArtistView;
use gpui::{prelude::*, AnyElement};
use std::sync::Arc;

#[derive(Default)]
pub struct ArtistDetailBehaviorSlots {
    pub image: Option<Arc<gpui::Image>>,
    pub feed_section: Option<AnyElement>,
}

#[must_use]
pub fn render_artist_view(
    view: &ArtistView,
    feeds: &[Feed],
    image: Option<Arc<gpui::Image>>,
    has_more_tracks: bool,
    track_count_override: Option<i32>,
    feed_section: Option<AnyElement>,
) -> AnyElement {
    let vm = ArtistVm::new(view, feeds, has_more_tracks, track_count_override);
    let page = vm.page();

    render_artist_detail_shell(
        &page,
        ArtistDetailBehaviorSlots {
            image,
            feed_section,
        },
    )
}

#[must_use]
pub fn render_artist_detail_shell(
    page: &ArtistDetailPageVm,
    slots: ArtistDetailBehaviorSlots,
) -> AnyElement {
    let rows: Vec<DetailRow> = page
        .detail_rows
        .iter()
        .map(|entry| {
            DetailRow::text(DetailTextRow {
                key: entry.key.clone().into(),
                value: entry.value.clone(),
                max_lines: entry.max_lines,
            })
        })
        .collect();

    let mut stack = VStack::new()
        .spacing(Spacing::LG)
        .stretch()
        .child(
            DetailHeader::new(DetailHeaderDisplay {
                kind: EntityKind::Artist,
                title: page.title.clone().into(),
                subtitle: page.subtitle.clone().map(Into::into),
                data_rows: Vec::new(),
            })
            .image(slots.image),
        )
        .child(DetailGrid::new(rows));

    if page.shows_feed_section {
        if let Some(feed_section) = slots.feed_section {
            stack = stack.child(feed_section);
        }
    }

    stack.into_any_element()
}
