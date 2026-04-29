use crate::api::Feed;
use crate::search::{render_feed_list_section, SearchApp};
use crate::ui::composites::{DetailGrid, DetailHeader, DetailRow, EntityKind};
use crate::ui::primitives::VStack;
use crate::ui::tokens::Spacing;
use crate::ui_context::ViewContext;
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
    let title = view.name.clone().unwrap_or_else(|| "Unknown Artist".into());
    let display_track_count = track_count_override.or(view.track_count).unwrap_or(0);
    let track_count_str = format!(
        "{}{}",
        display_track_count,
        if has_more_tracks { "+" } else { "" }
    );

    let mut rows: Vec<DetailRow> = Vec::new();
    rows.push(DetailRow::text("Tracks", track_count_str, 1));
    rows.push(DetailRow::text("Feeds", feeds.len().to_string(), 1));
    push_optional(&mut rows, "Sort Name", view.sort_name.clone());
    push_optional(&mut rows, "Area", view.area.clone());
    push_optional(
        &mut rows,
        "Active",
        artist_active_years(view.begin_year, view.end_year),
    );
    push_optional(&mut rows, "Website", view.url.clone());

    let mut stack = VStack::new()
        .spacing(Spacing::LG)
        .stretch()
        .child(
            DetailHeader::new(EntityKind::Artist, title)
                .subtitle("Feeds with tracks by this artist")
                .image(image),
        )
        .child(DetailGrid::new(rows));

    if !feeds.is_empty() {
        stack = stack.child(render_feed_list_section("Feeds", feeds.to_vec(), app, cx));
    }

    stack.into_any_element()
}

fn artist_active_years(begin_year: Option<i32>, end_year: Option<i32>) -> Option<String> {
    match (begin_year, end_year) {
        (Some(begin), Some(end)) => Some(format!("{begin}-{end}")),
        (Some(begin), None) => Some(format!("{begin}-")),
        (None, Some(end)) => Some(format!("until {end}")),
        (None, None) => None,
    }
}

fn push_optional(rows: &mut Vec<DetailRow>, key: &'static str, value: Option<String>) {
    if let Some(value) = value {
        if !value.is_empty() {
            rows.push(DetailRow::text(key, value, 6));
        }
    }
}
