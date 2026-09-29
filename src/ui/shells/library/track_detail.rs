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
use crate::ui::composites::{
    action_button, ActionButtonDisplay, DisclosureTextPanel, DisclosureTextPanelDisplay,
    TrackSurfaceElement,
};
use crate::ui::shells::library::track_detail_metadata::{
    pending_id3_edits_for_track_detail, render_library_track_detail_actions,
    render_library_track_detail_metadata,
};
use crate::ui::shells::track;
use crate::ui::style::spacing;
use crate::view_models::entity_detail::EntityActionTarget;
use crate::view_models::library::{DescriptionState, LibraryChromeDisplay, LibraryViewModel};
use crate::view_models::track_detail::{
    TrackDetailPageVm, TrackDetailSurfaceContext, TrackDetailVm,
};
use crate::view_models::workspace::BreadcrumbDisplay;
use crate::views::{ArtistRef, TrackView};

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
    let track_core = render_library_track_detail_core(
        &track_context.track,
        track_context.feed.as_ref(),
        &frame.title,
        frame.image.clone(),
        inspector_display.description_state,
        render_library_track_detail_actions(frame, &pending_id3_edits, playlists, cx),
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
    reason = "publisher navigation (ADR 0077 packet 004) and the feed identity section (ADR 0075 packet 022) each add one already-loaded value to an existing core renderer"
)]
pub(crate) fn render_library_track_detail_core(
    track: &Track,
    feed: Option<&Feed>,
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
        .with_publisher_feed_guid(publisher_feed_guid)
        .with_feed_identity(feed)
        .page();

    let mut primary_actions = vec![TrackSurfaceElement::from_element(primary_action_row)];
    if let Some(button) = render_track_publisher_action_button(&detail_page, cx) {
        primary_actions.push(TrackSurfaceElement::from_element(button));
    }

    let mut slots = track::TrackDetailBehaviorSlots {
        hero_image,
        external_links: track::render_track_page_identity_actions(&detail_page),
        primary_actions,
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

/// ADR 0077 packet 004, R4-03: the "open publisher" action of a track's
/// album feed. `None` when the album names no publisher, so the track shows
/// no action.
fn render_track_publisher_action_button(
    page: &TrackDetailPageVm<'_>,
    cx: &mut Context<LibraryApp>,
) -> Option<AnyElement> {
    let publisher_action = page.detail().publisher_action()?;
    let a11y_label = publisher_action.a11y_label();
    let EntityActionTarget::Artist(ArtistRef::PublisherFeed(publisher_feed_guid)) =
        publisher_action.target
    else {
        unreachable!("TrackDetailVm::publisher_action always targets a publisher feed GUID")
    };
    Some(
        action_button(
            ActionButtonDisplay {
                label: SharedString::from(publisher_action.label),
                a11y_label: SharedString::from(a11y_label),
            },
            cx,
        )
        .disabled(!publisher_action.enabled)
        .on_click(cx.listener(move |this, _, _, cx| {
            this.open_publisher_page(&publisher_feed_guid, cx);
        }))
        .into_any_element(),
    )
}
