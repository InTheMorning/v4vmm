use crate::api::Feed;
use crate::search::{render_feed_list_section, SearchApp};
use crate::ui_common::{optional_row, render_detail_grid, render_detail_header};
use crate::ui_context::ViewContext;
use crate::views::ArtistView;
use gpui::{div, prelude::*, AnyElement, Context};
use std::sync::Arc;

#[expect(
    clippy::too_many_arguments,
    reason = "shared artist view still accepts explicit discover-stage state"
)]
pub(crate) fn render_artist_view(
    view: &ArtistView,
    feeds: &[Feed],
    image: Option<&Arc<gpui::Image>>,
    _ctx: &ViewContext,
    has_more_tracks: bool,
    track_count_override: Option<i32>,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    use crate::ui::theme::spacing;

    let title = view.name.clone().unwrap_or_else(|| "Unknown Artist".into());
    let display_track_count = track_count_override.or(view.track_count).unwrap_or(0);
    let track_count_str = format!(
        "{}{}",
        display_track_count,
        if has_more_tracks { "+" } else { "" }
    );

    let mut rows = vec![
        ("Tracks".to_string(), track_count_str),
        ("Feeds".to_string(), feeds.len().to_string()),
    ];
    optional_row(&mut rows, "Sort Name", view.sort_name.clone());
    optional_row(&mut rows, "Area", view.area.clone());
    optional_row(
        &mut rows,
        "Active",
        artist_active_years(view.begin_year, view.end_year),
    );
    optional_row(&mut rows, "Website", view.url.clone());

    div()
        .flex()
        .flex_col()
        .gap(spacing::LG)
        .child(render_detail_header(
            "artist",
            &title,
            Some("Feeds with tracks by this artist"),
            image,
        ))
        .child(render_detail_grid(rows))
        .when(!feeds.is_empty(), |el| {
            el.child(render_feed_list_section("Feeds", feeds.to_vec(), app, cx))
        })
        .into_any_element()
}

fn artist_active_years(begin_year: Option<i32>, end_year: Option<i32>) -> Option<String> {
    match (begin_year, end_year) {
        (Some(begin), Some(end)) => Some(format!("{begin}-{end}")),
        (Some(begin), None) => Some(format!("{begin}-")),
        (None, Some(end)) => Some(format!("until {end}")),
        (None, None) => None,
    }
}
