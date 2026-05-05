//! Discover inspector and row action controls.

#![warn(clippy::pedantic)]

use gpui::{div, prelude::*, AnyElement, ClickEvent, Context, SharedString, Styled};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::spinner::Spinner;
use gpui_component::tooltip::Tooltip;
use gpui_component::{Disableable, Size};

use crate::api::{Feed, Track};
use crate::db;
use crate::library_service;
use crate::search::{InspectorDetail, InspectorFrame, SearchApp};
use crate::ui::composites::{
    action_button, ActionButtonDisplay, ActionRow, ActionRowDisplay, ActionRowMessage,
    AddToPlaylistDisplay, AddToPlaylistPopover, PlaylistOption, PlaylistOptionDisplay,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::layouts as layout;
use crate::ui::primitives::Button as UiButton;
use crate::ui::sizable_bridge::SizableScaled;
use crate::ui::style::{color, radius, spacing};
use crate::view_models::entity_detail::{
    EntityActionKind, EntityActionTarget, EntityActionTone, EntityActionVm,
};
use crate::view_models::playlist_option_displays;
use crate::view_models::search::{ActionRowVm, TrackRowActionVm};
use crate::view_models::track::TrackPlayAudioDisplay;
use crate::views::FeedRef;

#[expect(
    clippy::too_many_lines,
    reason = "Discover inspector action row wires several command targets explicitly"
)]
pub(crate) fn discover_inspector_action_row(
    frame: &InspectorFrame,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let vm = ActionRowVm::new(
        &frame.entity_type,
        frame.subscription_busy,
        frame.local_subscription,
        frame.subscription_message.as_deref(),
    );

    if !vm.is_visible() {
        return div().into_any_element();
    }

    let is_feed = frame.entity_type == "feed";
    let release_target = EntityActionTarget::Feed(FeedRef::Musicindex(frame.entity_id.clone()));
    let release_subscription_action =
        is_feed.then(|| vm.release_primary_action(release_target.clone()));
    let (subscription_label, subscription_disabled) =
        if let Some(action) = release_subscription_action {
            (action.label, !action.enabled)
        } else {
            (vm.subscription_button_label(), frame.subscription_busy)
        };
    let release_playlist_action = if is_feed {
        vm.release_playlist_action(release_target)
    } else {
        None
    };
    let playlist_label = vm.playlist_trigger_label(release_playlist_action.as_ref());
    let playlist_disabled = if is_feed {
        frame.subscription_busy
            || release_playlist_action
                .as_ref()
                .is_some_and(|action| !action.enabled)
    } else {
        frame.subscription_busy
    };
    let playlist_target = inspector_playlist_target(frame, app);
    let create_playlist_target = playlist_target.clone();
    let playlists = app.vm.playlists_snapshot();
    let playlist_display = vm.inspector_playlist_display(&frame.entity_id, playlist_label);

    let controls = vec![
        action_button(
            ActionButtonDisplay {
                label: SharedString::from(subscription_label.clone()),
                a11y_label: SharedString::from(subscription_label),
            },
            cx,
        )
        .disabled(subscription_disabled)
        .on_click(cx.listener(|this, _, _, cx| {
            this.toggle_local_subscription(cx);
        }))
        .into_any_element(),
        AddToPlaylistPopover::new(AddToPlaylistDisplay {
            id: SharedString::from(playlist_display.popover_id),
            playlists: playlist_options(&playlists),
            trigger_label: SharedString::from(playlist_display.trigger_label),
            trigger_a11y_label: SharedString::from("Add to playlist"),
            new_playlist_a11y_label: SharedString::from("Create a new playlist"),
            back_a11y_label: SharedString::from("Back to playlist choices"),
            create_a11y_label: SharedString::from("Create playlist and add item"),
        })
        .disabled(playlist_disabled || playlist_target.is_none())
        .on_select(cx.listener(move |this, playlist_id: &i64, _window, cx| {
            if let Some(target) = &playlist_target {
                match target {
                    InspectorPlaylistTarget::Track(track_id) => {
                        this.add_track_to_playlist(*track_id, *playlist_id, cx);
                    }
                    InspectorPlaylistTarget::TrackPending {
                        feed_url,
                        feed_guid,
                        track_guid,
                    } => {
                        this.add_search_track_to_playlist(
                            feed_guid,
                            feed_url.as_deref(),
                            track_guid,
                            *playlist_id,
                            cx,
                        );
                    }
                    InspectorPlaylistTarget::Feed {
                        feed_url,
                        feed_guid,
                    } => {
                        this.add_feed_to_playlist(feed_guid, feed_url.as_deref(), *playlist_id, cx);
                    }
                }
            }
        }))
        .on_create(cx.listener(move |this, name: &String, _window, cx| {
            if let Some(target) = &create_playlist_target {
                match target {
                    InspectorPlaylistTarget::Track(track_id) => {
                        this.create_playlist_and_add_track(name, *track_id, cx);
                    }
                    InspectorPlaylistTarget::TrackPending {
                        feed_url,
                        feed_guid,
                        track_guid,
                    } => {
                        this.create_playlist_and_add_discover_track(
                            name,
                            feed_guid,
                            feed_url.as_deref(),
                            track_guid,
                            cx,
                        );
                    }
                    InspectorPlaylistTarget::Feed {
                        feed_url,
                        feed_guid,
                    } => {
                        this.create_playlist_and_add_feed(name, feed_guid, feed_url.as_deref(), cx);
                    }
                }
            }
        }))
        .into_any_element(),
    ];

    let mut row = ActionRow::new(ActionRowDisplay {
        a11y_label: SharedString::from(vm.action_row_a11y_label()),
    })
    .control_group(controls);

    if let Some(message) = vm.subscription_message_display() {
        row = row.message(ActionRowMessage::from_status_display(message));
    }

    row.into_any_element()
}

#[derive(Clone, Debug)]
enum InspectorPlaylistTarget {
    Track(i64),
    TrackPending {
        feed_url: Option<String>,
        feed_guid: String,
        track_guid: String,
    },
    Feed {
        feed_url: Option<String>,
        feed_guid: String,
    },
}

fn inspector_playlist_target(
    frame: &InspectorFrame,
    app: &SearchApp,
) -> Option<InspectorPlaylistTarget> {
    match (&frame.detail, frame.entity_type.as_str()) {
        (InspectorDetail::Track(track_context), _) => {
            let track = &track_context.track;
            let local_id = if let Ok(conn) = app.conn.lock() {
                library_service::find_track_id(
                    &conn,
                    track.feed_url.as_deref(),
                    track.track_guid.as_deref(),
                    track.enclosure_url.as_deref(),
                )
                .ok()
                .flatten()
            } else {
                None
            };
            match local_id {
                Some(id) => Some(InspectorPlaylistTarget::Track(id)),
                None => match (track.feed_guid.clone(), track.track_guid.clone()) {
                    (Some(fg), Some(tg)) => Some(InspectorPlaylistTarget::TrackPending {
                        feed_url: track.feed_url.clone(),
                        feed_guid: fg,
                        track_guid: tg,
                    }),
                    _ => None,
                },
            }
        }
        (InspectorDetail::Feed(feed), "feed") => Some(InspectorPlaylistTarget::Feed {
            feed_url: feed.feed_url.clone(),
            feed_guid: frame.entity_id.clone(),
        }),
        _ => None,
    }
}

fn playlist_options(playlists: &[db::Playlist]) -> Vec<PlaylistOption> {
    playlist_option_displays(playlists)
        .into_iter()
        .map(|option| {
            PlaylistOption::new(PlaylistOptionDisplay {
                id: option.id,
                name: SharedString::from(option.name),
                a11y_label: SharedString::from(option.a11y_label),
            })
        })
        .collect()
}

pub(crate) fn render_play_icon_button_with_id(
    id: SharedString,
    display: TrackPlayAudioDisplay,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    render_play_icon_button_parts(
        id,
        display.button_label,
        display.url,
        display.tooltip,
        display.disabled,
        cx,
    )
}

fn render_play_icon_button_parts(
    id: SharedString,
    button_label: &'static str,
    click_url: Option<String>,
    tooltip: String,
    disabled: bool,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    // CONTROL-COMPAT(reason): native Button is needed for compact icon chrome.
    Button::new(id)
        .label(button_label)
        .scaled(Size::XSmall, cx)
        .compact()
        .ghost()
        .w(layout::ACTION_ICON_SIZE)
        .h(layout::ACTION_ICON_SIZE)
        .px(spacing::NONE)
        .py(spacing::NONE)
        .text_color(color::text_on_accent())
        .rounded(radius::SM)
        .border_1()
        .border_color(color::accent())
        .tooltip(tooltip)
        .disabled(disabled)
        .on_click(cx.listener(move |_this, _: &ClickEvent, _window, _cx| {
            if let Some(url) = &click_url {
                let _ = open::that(url);
            }
        }))
        .into_any_element()
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "track and feed are cloned into a deferred row-action listener"
)]
pub(crate) fn render_track_download_button(
    track: Track,
    feed: Option<Feed>,
    is_downloaded: bool,
    is_in_flight: bool,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let action_vm = TrackRowActionVm::new(&track, is_downloaded, is_in_flight);
    let display = action_vm.download_display();

    if action_vm.is_in_flight() {
        let tip = SharedString::from(display.busy_tooltip);
        return div()
            .id(SharedString::from(display.busy_indicator_id))
            .w(layout::ACTION_ICON_SIZE)
            .h(layout::ACTION_ICON_SIZE)
            .flex()
            .items_center()
            .justify_center()
            .rounded(radius::SM)
            .border_1()
            .border_color(color::accent())
            .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
            .child(
                Spinner::new()
                    .scaled(Size::XSmall, cx)
                    .color(color::accent().into()),
            )
            .into_any_element();
    }

    let EntityActionVm {
        kind,
        label,
        enabled,
        tone,
        ..
    } = action_vm.primary_action();
    let style = match tone {
        EntityActionTone::DestructiveQuiet => ControlStyle::DestructiveRowAction,
        _ => ControlStyle::RowAction,
    };
    let track_for_click = track.clone();
    let feed_for_click = feed.clone();

    UiButton::styled(SharedString::from(display.button_id), style)
        .label(label)
        .disabled(!enabled)
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            if kind == EntityActionKind::Remove {
                this.remove_track_row(track_for_click.clone(), feed_for_click.clone(), cx);
            } else {
                this.download_track_row(track_for_click.clone(), feed_for_click.clone(), cx);
            }
        }))
        .into_any_element()
}
