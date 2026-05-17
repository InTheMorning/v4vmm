//! Discover track inspector core: header, hero, actions, contributors, and value routes.
//!
//! Metadata editing remains in `src/discover.rs` until the metadata slice moves it.

#![warn(clippy::pedantic)]

use std::collections::BTreeMap;

use gpui::{
    div, prelude::*, AnyElement, ClipboardItem, Context, FontWeight, InteractiveElement,
    SharedString, Styled,
};

use crate::api::PaymentRoute;
use crate::discover::{InspectorFrame, SearchApp};
use crate::metadata::TrackContext;
use crate::ui::composites::{
    identity_action_button, DisclosureGroup, DisclosureGroupDisplay, IdentityActionButtonDisplay,
    IdentityActionKind, ReleaseSurfaceElement, TrackDetailSurface, TrackSurfaceElement,
};
use crate::ui::primitives::{LoadingMessage, Tooltip};
use crate::ui::shells::discover::actions::{
    discover_inspector_action_row, render_play_icon_button_with_id,
};
use crate::ui::shells::discover::track_inspector_metadata::render_discover_track_inspector_metadata;
use crate::ui::shells::entity::{render_contributor_rows, ContributorRowSlot};
use crate::ui::shells::track;
use crate::ui::style::{color, spacing, typography};
use crate::ui::tokens::Spacing;
use crate::view_models::entity_detail::{
    ContributorIdentityActionDisplay, ContributorIdentityActionKind, ContributorListVm,
    ContributorRowVm, EntitySurfaceContext,
};
use crate::view_models::search::{
    DeferredPanelKind, LazyPanel, PaymentRouteVm, SearchViewModel, TrackFeedLinkDisplay,
    TrackInspectorHeaderVm,
};
use crate::view_models::track::TrackVm;
use crate::view_models::track_detail::{TrackDetailSurfaceContext, TrackDetailVm};
use crate::views::{ContributorView, TrackView};

pub(crate) fn render_discover_track_inspector_core(
    frame: &InspectorFrame,
    track_context: &TrackContext,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let track = &track_context.track;
    let vm = TrackVm::new(track);
    let track_view = TrackView::from_api(track.clone());
    let detail_page = TrackDetailVm::new(&track_view, TrackDetailSurfaceContext::Discover).page();
    let header_vm = TrackInspectorHeaderVm::new(track);
    let feed_link = header_vm.feed_link_display();
    let audio_display = vm.play_audio_display();
    let mut external_links = vec![TrackSurfaceElement::from_element(
        render_track_header_subtitle(feed_link, audio_display, cx),
    )];
    external_links.extend(track::render_track_page_identity_actions(&detail_page));

    let surface = track::build_track_detail_surface(
        &detail_page,
        track::TrackDetailBehaviorSlots {
            hero_image: frame.image.clone(),
            external_links,
            primary_actions: vec![TrackSurfaceElement::from_element(
                discover_inspector_action_row(frame, app, cx),
            )],
            section_elements: vec![
                TrackSurfaceElement::from_element(render_discover_track_inspector_lazy_sections(
                    frame, app, cx,
                )),
                TrackSurfaceElement::from_element(render_discover_track_inspector_metadata(
                    frame,
                    track_context,
                    cx,
                )),
            ],
            ..track::TrackDetailBehaviorSlots::default()
        },
    );

    TrackInspectorPane::new(surface).into_any_element()
}

#[derive(IntoElement)]
#[must_use]
struct TrackInspectorPane {
    surface: TrackDetailSurface,
}

impl TrackInspectorPane {
    const fn new(surface: TrackDetailSurface) -> Self {
        Self { surface }
    }
}

impl RenderOnce for TrackInspectorPane {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(Spacing::LG.scaled(cx))
            .child(self.surface)
    }
}

pub(crate) fn render_discover_track_inspector_lazy_sections(
    frame: &InspectorFrame,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(spacing::LG)
        .child(render_lazy_contributors(frame, app, cx))
        .child(render_lazy_value_routes(frame, cx))
        .into_any_element()
}

fn render_track_header_subtitle(
    feed_link: Option<TrackFeedLinkDisplay>,
    audio_display: crate::view_models::track::TrackPlayAudioDisplay,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(spacing::SM)
        .min_w_0()
        .when_some(feed_link, |el, link| {
            el.child(render_feed_link_value(link, cx))
        })
        .child(render_play_icon_button_with_id(
            SharedString::from(audio_display.button_id.clone()),
            audio_display,
            cx,
        ))
        .into_any_element()
}

fn render_feed_link_value(link: TrackFeedLinkDisplay, cx: &mut Context<SearchApp>) -> AnyElement {
    let TrackFeedLinkDisplay {
        element_id,
        guid,
        label,
        tooltip,
        ..
    } = link;
    let title = label;
    let click_title = title.clone();
    let tooltip = Tooltip::new(tooltip);
    div()
        .id(SharedString::from(element_id))
        .cursor_pointer()
        .text_color(color::accent())
        .text_size(typography::SIZE_MICRO)
        .line_height(typography::LINE_DETAIL)
        .tooltip(move |window, cx| tooltip.build(window, cx))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.push_inspector("feed".into(), guid.clone(), click_title.clone(), cx);
        }))
        .child(SharedString::from(title))
        .into_any_element()
}

fn render_lazy_contributors(
    frame: &InspectorFrame,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let collapsed = frame.contributors_collapsed || matches!(frame.contributors, LazyPanel::Hidden);

    div()
        .flex()
        .flex_col()
        .gap(spacing::XS)
        .child(render_contributors_heading(collapsed, cx))
        .when(!collapsed, |el| match &frame.contributors {
            LazyPanel::Loaded(items) => el.children(contributor_elements(items, app, cx)),
            LazyPanel::Loading => el.child(LoadingMessage::new(
                SearchViewModel::deferred_panel_display(DeferredPanelKind::Contributors)
                    .loading_label,
            )),
            LazyPanel::Empty(label) => el.child(muted_line(
                SearchViewModel::deferred_panel_empty_line(label),
            )),
            LazyPanel::Hidden => el,
        })
        .into_any_element()
}

fn render_lazy_value_routes(frame: &InspectorFrame, cx: &mut Context<SearchApp>) -> AnyElement {
    let collapsed = frame.value_routes_collapsed || matches!(frame.value_routes, LazyPanel::Hidden);

    div()
        .flex()
        .flex_col()
        .gap(spacing::XS)
        .child(render_value_routes_heading(collapsed, cx))
        .when(!collapsed, |el| match &frame.value_routes {
            LazyPanel::Loaded(items) => el.children(value_route_elements(items)),
            LazyPanel::Loading => el.child(LoadingMessage::new(
                SearchViewModel::deferred_panel_display(DeferredPanelKind::ValueRoutes)
                    .loading_label,
            )),
            LazyPanel::Empty(label) => el.child(muted_line(
                SearchViewModel::deferred_panel_empty_line(label),
            )),
            LazyPanel::Hidden => el,
        })
        .into_any_element()
}

fn contributor_elements(
    contributors: &[ContributorView],
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> Vec<AnyElement> {
    render_contributor_rows(
        ContributorListVm::new(contributors, EntitySurfaceContext::Discover),
        |contributor| {
            let thumbnail = app.thumbnail_for_url(contributor.image_url(), cx);
            ContributorRowSlot {
                thumbnail,
                actions: contributor_identity_actions(contributor),
            }
        },
    )
}

fn contributor_identity_actions(contributor: &ContributorRowVm<'_>) -> Vec<ReleaseSurfaceElement> {
    contributor
        .identity_actions()
        .into_iter()
        .map(|action| {
            let ContributorIdentityActionDisplay {
                id,
                kind,
                target,
                a11y_label,
            } = action;
            let target_for_click = target;
            let a11y_label = SharedString::from(a11y_label);
            match kind {
                ContributorIdentityActionKind::Website => {
                    identity_action_button(IdentityActionButtonDisplay {
                        id: SharedString::from(id),
                        kind: IdentityActionKind::Website,
                        a11y_label,
                    })
                    .on_click(move |_, _, _| {
                        let _ = open::that(&target_for_click);
                    })
                    .into_any_element()
                }
                ContributorIdentityActionKind::Nostr => {
                    identity_action_button(IdentityActionButtonDisplay {
                        id: SharedString::from(id),
                        kind: IdentityActionKind::Nostr,
                        a11y_label,
                    })
                    .on_click(move |_, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(target_for_click.clone()));
                    })
                    .into_any_element()
                }
            }
        })
        .map(ReleaseSurfaceElement::from_element)
        .collect()
}

fn value_route_elements(routes: &[PaymentRoute]) -> Vec<AnyElement> {
    let mut groups = BTreeMap::<&'static str, Vec<&PaymentRoute>>::new();
    for route in routes {
        let group = PaymentRouteVm::new(route).group();
        groups.entry(group).or_default().push(route);
    }

    groups
        .into_iter()
        .flat_map(|(group, routes)| {
            let group_display = PaymentRouteVm::group_display(group);
            let mut elements = vec![group_heading(group_display.heading)];
            elements.extend(routes.into_iter().map(|route| {
                let vm = PaymentRouteVm::new(route);
                let summary = vm.summary();
                let address = vm.address();
                let custom_fields = vm.custom_fields();
                div()
                    .flex()
                    .flex_col()
                    .gap(spacing::XXS)
                    .text_size(typography::SIZE_MICRO)
                    .child(SharedString::from(summary))
                    .when_some(address, |el, address| {
                        el.child(
                            div()
                                .text_color(color::text_muted())
                                .text_size(typography::SIZE_MICRO)
                                .line_clamp(2)
                                .child(SharedString::from(address)),
                        )
                    })
                    .when_some(custom_fields, |el, custom_fields| {
                        el.child(
                            div()
                                .text_color(color::text_muted())
                                .text_size(typography::SIZE_MICRO)
                                .child(SharedString::from(custom_fields)),
                        )
                    })
                    .into_any_element()
            }));
            elements
        })
        .collect()
}

fn render_contributors_heading(collapsed: bool, cx: &mut Context<SearchApp>) -> AnyElement {
    let display = SearchViewModel::deferred_panel_display(DeferredPanelKind::Contributors);
    DisclosureGroup::new(DisclosureGroupDisplay {
        id: display.section_id.into(),
        label: display.heading_label.into(),
        a11y_label: display.heading_a11y_label.into(),
    })
    .collapsed(collapsed)
    .on_toggle(cx.listener(|this, _, _, cx| {
        this.toggle_contributors(cx);
    }))
    .into_any_element()
}

fn render_value_routes_heading(collapsed: bool, cx: &mut Context<SearchApp>) -> AnyElement {
    let display = SearchViewModel::deferred_panel_display(DeferredPanelKind::ValueRoutes);
    DisclosureGroup::new(DisclosureGroupDisplay {
        id: display.section_id.into(),
        label: display.heading_label.into(),
        a11y_label: display.heading_a11y_label.into(),
    })
    .collapsed(collapsed)
    .on_toggle(cx.listener(|this, _, _, cx| {
        this.toggle_value_routes(cx);
    }))
    .into_any_element()
}

fn muted_line(display_text: String) -> AnyElement {
    div()
        .text_color(color::text_muted())
        .text_size(typography::SIZE_MICRO)
        .child(SharedString::from(display_text))
        .into_any_element()
}

fn group_heading(label: &'static str) -> AnyElement {
    div()
        .text_size(typography::SIZE_MICRO)
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(color::text_muted())
        .mt(spacing::SM)
        .child(label)
        .into_any_element()
}
