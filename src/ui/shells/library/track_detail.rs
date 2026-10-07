//! Library track detail core surface.
//!
//! Renders the Library track header, hero artwork, primary actions, external
//! identity links, and summary rows. Metadata editing remains in `src/library.rs`
//! until Slice L6.

#![warn(clippy::pedantic)]

use std::sync::Arc;

use gpui::{div, prelude::*, AnyElement, Context, Image, SharedString};

use crate::api::{Feed, Track};
use crate::db;
use crate::feed_service::track_row_to_track_context;
use crate::library::{InspectorFrame, LazyPanel, LibraryApp};
use crate::metadata::{TagCompareResult, TrackContext};
use crate::ui::composites::BreadcrumbTrail;
use crate::ui::composites::{DisclosureTextPanel, DisclosureTextPanelDisplay, TrackSurfaceElement};
use crate::ui::shells::library::track_detail_metadata::{
    pending_id3_edits_for_track_detail, render_library_track_detail_actions,
    render_library_track_detail_metadata,
};
use crate::ui::shells::track;
use crate::ui::style::spacing;
use crate::view_models::library::{DescriptionState, LibraryChromeDisplay, LibraryViewModel};
use crate::view_models::track_detail::{
    LibraryTrackActionState, TrackDetailSurfaceContext, TrackDetailVm,
};
use crate::view_models::workspace::BreadcrumbDisplay;
use crate::views::TrackView;

pub(crate) fn render_library_track_detail(
    frame: &InspectorFrame,
    breadcrumb: Option<BreadcrumbDisplay>,
    playlists: &[db::Playlist],
    chrome: &LibraryChromeDisplay,
    publisher_feed_guid: Option<&str>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let context = track_row_to_track_context(&frame.track);
    let context = frame.source_context.as_ref().unwrap_or(&context);
    let result = match &frame.tag_compare {
        LazyPanel::Loaded(result) => Some(result),
        LazyPanel::Loading | LazyPanel::Empty(_) | LazyPanel::Hidden => None,
    };
    div()
        .id(SharedString::from(chrome.track_detail_scroll_id))
        .flex_1()
        .min_h_0()
        .min_w_0()
        .overflow_y_scroll()
        .p(spacing::LG)
        .child(render_library_track_window(
            frame,
            breadcrumb,
            context,
            result,
            playlists,
            publisher_feed_guid,
            cx,
        ))
        .into_any_element()
}

fn render_library_track_window(
    frame: &InspectorFrame,
    breadcrumb: Option<BreadcrumbDisplay>,
    track_context: &TrackContext,
    result: Option<&TagCompareResult>,
    playlists: &[db::Playlist],
    publisher_feed_guid: Option<&str>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let pending_id3_edits = pending_id3_edits_for_track_detail(frame, track_context, result);
    let inspector_display = frame.inspector_display(track_context.track.description.as_deref());
    let track_view = TrackView::from_api(track_context.track.clone());
    let detail = TrackDetailVm::new(&track_view, TrackDetailSurfaceContext::Library)
        .with_feed_identity(track_context.feed.as_ref())
        .with_library_state(LibraryTrackActionState {
            downloaded: frame.local_subscription,
            subscription_busy: frame.subscription_busy,
            musicbrainz_available: inspector_display.musicbrainz_enabled,
        });
    let action_row = render_library_track_detail_actions(
        frame,
        &detail.page_actions(),
        detail.feed_url(),
        playlists,
        cx,
    );
    let track_core = render_library_track_detail_core(
        &track_context.track,
        track_context.feed.as_ref(),
        frame.track.feed_id,
        &frame.title,
        frame.image.clone(),
        inspector_display.description_state,
        action_row,
        breadcrumb,
        publisher_feed_guid,
        cx,
    );

    render_library_track_detail_metadata(
        frame,
        track_context,
        result,
        &pending_id3_edits,
        track_core,
        cx,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "publisher navigation (ADR 0077 packet 004), the feed identity section (ADR 0075 packet 022) and the album link (ADR 0083 task 004) each add one already-loaded value to an existing core renderer"
)]
pub(crate) fn render_library_track_detail_core(
    track: &Track,
    feed: Option<&Feed>,
    album_feed_id: i64,
    override_title: &str,
    hero_image: Option<Arc<Image>>,
    description_state: DescriptionState,
    primary_action_row: AnyElement,
    breadcrumb: Option<BreadcrumbDisplay>,
    publisher_feed_guid: Option<&str>,
    cx: &mut Context<LibraryApp>,
) -> AnyElement {
    let track_view = TrackView::from_api(track.clone());
    let detail_page = TrackDetailVm::new(&track_view, TrackDetailSurfaceContext::Library)
        .with_override_title(Some(override_title))
        .with_album_feed_id(album_feed_id)
        .with_publisher_feed_guid(publisher_feed_guid)
        .with_feed_identity(feed)
        .page();

    let entity = cx.entity();
    let name_links = track::render_track_name_links(&detail_page, move |target, _window, cx| {
        entity.update(cx, |this, cx| this.open_track_name_link(target, cx));
    });

    let mut slots = track::TrackDetailBehaviorSlots {
        hero_image,
        name_links,
        external_links: track::render_track_page_identity_actions(&detail_page),
        primary_actions: vec![TrackSurfaceElement::from_element(primary_action_row)],
        ..track::TrackDetailBehaviorSlots::default()
    };

    if let Some(feed_identity) = track::render_track_feed_identity_section(&detail_page, cx) {
        slots.section_elements.push(feed_identity);
    }

    let description_text = detail_page.detail().description();
    if let Some(description) =
        LibraryViewModel::display_description_text(description_text.as_deref())
    {
        let collapsed = !description_state.is_visible();
        slots.description_panel = Some(TrackSurfaceElement::from_element(
            DisclosureTextPanel::new(DisclosureTextPanelDisplay {
                id: "library-track-description".into(),
                label: SharedString::from(detail_page.detail().labels().description_label()),
                a11y_label: SharedString::from("Toggle track description"),
                body: SharedString::from(description.to_string()),
                collapsed,
            })
            .on_toggle(cx.listener(|this, _, _, cx| {
                this.toggle_track_description(cx);
            }))
            .into_any_element(),
        ));
    }

    let surface = track::build_track_detail_surface(&detail_page, slots).into_any_element();
    if let Some(breadcrumb) = breadcrumb {
        let entity = cx.entity();
        return div()
            .flex()
            .flex_col()
            .gap(spacing::MD)
            .child(
                BreadcrumbTrail::new(breadcrumb).on_select(move |entry, _window, cx| {
                    entity.update(cx, |this, cx| {
                        this.select_frame_breadcrumb(entry, cx);
                    });
                }),
            )
            .child(surface)
            .into_any_element();
    }

    surface
}
