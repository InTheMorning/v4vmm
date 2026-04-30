use crate::api::Track;
use crate::search::{
    detail_rows_from_strings, render_action_row, render_collapsed_text_section, render_feed_header,
    render_publisher_link_value, render_track_list_section, InspectorFrame, SearchApp,
};
use crate::ui::composites::DetailGrid;
use crate::ui::detail_row::DetailRow;
use crate::ui::primitives::VStack;
use crate::ui::tokens::Spacing;
use crate::ui_context::ViewContext;
use crate::view_models::feed::FeedVm;
use crate::views::FeedView;
use gpui::{prelude::*, AnyElement, Context};
use std::collections::BTreeMap;

pub(crate) fn render_feed_view(
    view: &FeedView,
    tracks: &[Track],
    _ctx: &ViewContext,
    frame: &InspectorFrame,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let vm = FeedVm::new(view, tracks);

    let scalar_pairs: Vec<(String, String)> = vm
        .scalar_detail_entries()
        .into_iter()
        .map(|e| (e.key.to_string(), e.value))
        .collect();
    let mut rows = detail_rows_from_strings(scalar_pairs);

    let publisher_row = match vm.publisher_text() {
        Some(publisher) => DetailRow {
            key: "Publisher".into(),
            value: render_publisher_link_value(publisher, cx),
        },
        None => detail_rows_from_strings(vec![("Publisher".into(), "Unknown".into())]).remove(0),
    };
    rows.insert(vm.publisher_row_index(), publisher_row);

    let header_feed = vm.header_feed();
    let title = vm.title();
    let artist = vm.artist_label();

    let mut stack = VStack::new()
        .spacing(Spacing::LG)
        .stretch()
        .child(render_feed_header(
            frame,
            &header_feed,
            &title,
            Some(artist.as_str()),
            cx,
        ))
        .child(render_action_row(frame, &BTreeMap::new(), app, cx))
        .child(DetailGrid::new(
            rows.into_iter().map(Into::into).collect::<Vec<_>>(),
        ));

    if let Some(description) = vm.description() {
        stack = stack.child(render_collapsed_text_section("Description", description));
    }

    if vm.has_tracks() {
        let playlists = app.vm.playlists.clone();
        let open_guid = frame.add_to_playlist_open_track_guid.clone();
        let feed_guid = frame.entity_id.clone();
        let feed_url = view.feed_url.clone();
        let feed_context = Some((
            feed_guid.as_str(),
            feed_url.as_deref(),
            open_guid.as_deref(),
            playlists.as_slice(),
        ));
        stack = stack.child(render_track_list_section(
            "Tracks",
            vm.track_list_summary(),
            vm.sorted_tracks(),
            vm.track_list_feed(),
            feed_context,
            app,
            cx,
        ));
    }

    stack.into_any_element()
}
