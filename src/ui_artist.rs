use crate::api::Feed;
use crate::search::{render_feed_list_section, SearchApp};
use crate::ui::composites::{DetailGrid, DetailHeader, DetailRow, EntityKind};
use crate::ui::primitives::VStack;
use crate::ui::tokens::Spacing;
use crate::ui_context::ViewContext;
use crate::view_models::artist::ArtistVm;
use crate::views::ArtistView;
use gpui::{prelude::*, AnyElement, Context};
use std::sync::Arc;

#[expect(
    clippy::too_many_arguments,
    reason = "shared artist view still accepts explicit discover-stage state"
)]
pub fn render_artist_view(
    view: &ArtistView,
    feeds: &[Feed],
    image: Option<Arc<gpui::Image>>,
    _ctx: &ViewContext,
    has_more_tracks: bool,
    track_count_override: Option<i32>,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
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
            DetailHeader::new(EntityKind::Artist, vm.title())
                .subtitle(vm.subtitle())
                .image(image),
        )
        .child(DetailGrid::new(rows));

    if vm.has_feeds() {
        stack = stack.child(render_feed_list_section("Feeds", feeds.to_vec(), app, cx));
    }

    stack.into_any_element()
}
