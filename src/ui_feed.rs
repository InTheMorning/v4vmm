use crate::api::Track;
use crate::search::{
    detail_rows_from_strings, render_action_row, render_collapsed_text_section, render_feed_header,
    render_publisher_link_value, render_track_list_rows, InspectorFrame, SearchApp,
};
use crate::ui::composites::DetailGrid;
use crate::ui::detail_row::DetailRow;
use crate::ui_context::ViewContext;
use crate::ui_entity::{render_release_detail_shell, ReleaseDetailSlots, TrackSectionSlot};
use crate::view_models::entity_detail::{EntitySurfaceContext, ReleaseDetailVm};
use crate::view_models::feed::FeedVm;
use crate::views::FeedView;
use gpui::{prelude::*, AnyElement, ClipboardItem, Context, SharedString};
use std::collections::BTreeMap;

pub(crate) fn render_feed_view(
    view: &FeedView,
    tracks: &[Track],
    _ctx: &ViewContext,
    frame: &InspectorFrame,
    panels: Vec<AnyElement>,
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

    let projection = ReleaseDetailVm::new(view, EntitySurfaceContext::Discover);
    let mut slots = ReleaseDetailSlots {
        header: Some(render_feed_header(
            frame,
            &header_feed,
            &title,
            Some(artist.as_str()),
            cx,
        )),
        action_row: Some(render_action_row(frame, &BTreeMap::new(), app, cx)),
        identity_actions: render_identity_actions(view),
        details: Some(
            DetailGrid::new(rows.into_iter().map(Into::into).collect::<Vec<_>>())
                .into_any_element(),
        ),
        ..ReleaseDetailSlots::default()
    };

    if let Some(description) = vm.description() {
        slots
            .panels
            .push(render_collapsed_text_section("Description", description));
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
        let rows = render_track_list_rows(
            vm.sorted_tracks(),
            vm.track_list_feed(),
            feed_context,
            app,
            cx,
        );
        slots.track_section = Some(TrackSectionSlot {
            summary: vm.track_list_summary().into(),
            rows,
        });
    }

    for panel in panels {
        slots.after_section.push(panel);
    }

    render_release_detail_shell("discover-feed-detail", &projection, slots)
}

fn render_identity_actions(view: &FeedView) -> Vec<AnyElement> {
    let mut actions = Vec::new();

    if let Some(url) = view.identity.website_url.clone() {
        let url_for_click = url.clone();
        actions.push(
            crate::ui::composites::identity_action_button(
                SharedString::from(format!("discover-feed-website:{url}")),
                crate::ui::composites::IdentityActionKind::Website,
            )
            .on_click(move |_, _, _| {
                let _ = open::that(&url_for_click);
            })
            .into_any_element(),
        );
    }

    if let Some(npub) = view.identity.nostr_npub.clone() {
        let npub_for_click = npub.clone();
        actions.push(
            crate::ui::composites::identity_action_button(
                SharedString::from(format!("discover-feed-nostr:{npub}")),
                crate::ui::composites::IdentityActionKind::Nostr,
            )
            .on_click(move |_, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(npub_for_click.clone()));
            })
            .into_any_element(),
        );
    }

    actions
}
