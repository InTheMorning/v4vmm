use crate::api::Track;
use crate::search::{
    discover_inspector_action_row, render_track_list_rows, InspectorFrame, SearchApp,
};
use crate::ui::composites::ReleaseSurfaceElement;
use crate::ui::shells::entity::{
    render_feed_identity_actions, render_release_detail_shell, ReleaseDetailBehaviorSlots,
};
use crate::ui_context::ViewContext;
use crate::view_models::entity_detail::{EntitySurfaceContext, ReleaseDetailVm};
use crate::view_models::feed::FeedVm;
use crate::views::FeedView;
use gpui::{AnyElement, Context};

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
        primary_actions: vec![ReleaseSurfaceElement::from_element(
            discover_inspector_action_row(frame, app, cx),
        )],
        identity_actions: render_feed_identity_actions(&page),
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
        )
        .into_iter()
        .map(ReleaseSurfaceElement::from_element)
        .collect();
        slots.track_rows = Some(rows);
    }

    for panel in panels {
        slots
            .after_section
            .push(ReleaseSurfaceElement::from_element(panel));
    }

    render_release_detail_shell(&page, slots)
}
