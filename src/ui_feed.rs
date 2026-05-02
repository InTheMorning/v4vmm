use crate::api::Track;
use crate::search::{render_action_row, render_track_list_rows, InspectorFrame, SearchApp};
use crate::ui_context::ViewContext;
use crate::ui_entity::{render_release_detail_shell, ReleaseDetailBehaviorSlots};
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
    let projection = ReleaseDetailVm::new(view, EntitySurfaceContext::Discover);
    let page = projection.page();
    let mut slots = ReleaseDetailBehaviorSlots {
        hero_image: frame.image.clone(),
        primary_actions: vec![render_action_row(frame, &BTreeMap::new(), app, cx)],
        identity_actions: render_identity_actions(view),
        ..ReleaseDetailBehaviorSlots::default()
    };

    if vm.has_tracks() {
        let playlists = app.vm.playlists.clone();
        let feed_guid = frame.entity_id.clone();
        let feed_url = view.feed_url.clone();
        let feed_context = Some((
            feed_guid.as_str(),
            feed_url.as_deref(),
            playlists.as_slice(),
        ));
        let rows = render_track_list_rows(
            vm.sorted_tracks(),
            vm.track_list_feed(),
            feed_context,
            app,
            cx,
        );
        slots.track_rows = Some(rows);
    }

    for panel in panels {
        slots.after_section.push(panel);
    }

    render_release_detail_shell("discover-feed-detail", &page, slots)
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

    if let Some(url) = view.feed_url.clone() {
        let url_for_click = url.clone();
        actions.push(
            crate::ui::composites::identity_action_button(
                SharedString::from(format!("discover-feed-rss:{url}")),
                crate::ui::composites::IdentityActionKind::Rss,
            )
            .on_click(move |_, _, _| {
                let _ = open::that(&url_for_click);
            })
            .into_any_element(),
        );
    }

    actions
}
