use crate::api::Feed;
use crate::ui::composites::{DetailGrid, DetailHeader, DetailHeaderDisplay, DetailRow, EntityKind};
use crate::ui::primitives::VStack;
use crate::ui::tokens::Spacing;
use crate::view_models::artist::ArtistVm;
use crate::views::ArtistView;
use gpui::{prelude::*, AnyElement};
use std::sync::Arc;

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

    let rows: Vec<DetailRow> = vm
        .detail_rows()
        .into_iter()
        .map(|entry| DetailRow::text(entry.key, entry.value, entry.max_lines))
        .collect();

    let mut stack = VStack::new()
        .spacing(Spacing::LG)
        .stretch()
        .child(
            DetailHeader::new(DetailHeaderDisplay {
                kind: EntityKind::Artist,
                title: vm.title().into(),
                subtitle: Some(vm.subtitle().into()),
                data_rows: Vec::new(),
            })
            .image(image),
        )
        .child(DetailGrid::new(rows));

    if vm.has_feeds() {
        if let Some(feed_section) = feed_section {
            stack = stack.child(feed_section);
        }
    }

    stack.into_any_element()
}
