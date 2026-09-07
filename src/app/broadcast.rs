//! Broadcast frame adapter.
//!
//! This module bridges top-level app state into the ADR 0059 Broadcast frame
//! display contract. Service and event mutations are wired by later ADR 0059
//! tasks; this packet mounts the frame and binds inert command slots.

use gpui::{App, ClickEvent, ClipboardItem, Context, Entity, Window};

use crate::runtime::BroadcastObservationOutcome;
use crate::ui::shells::broadcast::{render_broadcast, BroadcastShell, BroadcastSlots};
use crate::view_models::broadcast::{
    BroadcastPageVm, PublisherSectionInput, ServiceState, SourceSectionInput,
};

use super::TopApp;

pub(super) fn build_broadcast_frame(app: &TopApp, cx: &mut Context<TopApp>) -> BroadcastShell {
    let entity = cx.entity();
    render_broadcast(
        broadcast_vm(app),
        BroadcastSlots::new()
            .on_create_event(broadcast_status_action(
                entity.clone(),
                "Broadcast event creation is not wired yet",
            ))
            .on_resume_event(broadcast_status_action(
                entity.clone(),
                "Broadcast event resume is not wired yet",
            ))
            .on_forget_event(broadcast_status_action(
                entity.clone(),
                "Broadcast event removal is not wired yet",
            ))
            .on_copy_feed_tag(copy_feed_tag_action(entity.clone()))
            .on_start_service(broadcast_status_action(
                entity.clone(),
                "Broadcast service start is not wired yet",
            ))
            .on_stop_service(broadcast_status_action(
                entity.clone(),
                "Broadcast service stop is not wired yet",
            ))
            .on_reset_service(broadcast_status_action(
                entity.clone(),
                "Broadcast service reset is not wired yet",
            ))
            .on_open_logs(broadcast_status_action(
                entity.clone(),
                "Broadcast logs are not wired yet",
            ))
            .on_select_source(broadcast_status_action(
                entity,
                "Broadcast source selection is not wired yet",
            )),
    )
}

fn broadcast_vm(_app: &TopApp) -> BroadcastPageVm {
    BroadcastPageVm::builder()
        .source(SourceSectionInput::default())
        .publisher(PublisherSectionInput {
            publisher_state: ServiceState::NotInstalled,
            producer_state: ServiceState::NotInstalled,
            ..PublisherSectionInput::default()
        })
        .observation(BroadcastObservationOutcome::NoEvent)
        .build()
}

fn broadcast_status_action(
    entity: Entity<TopApp>,
    message: &'static str,
) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    move |_, _, cx| {
        entity.update(cx, |this, cx| {
            this.settings_status = message.to_string();
            cx.notify();
        });
    }
}

fn copy_feed_tag_action(
    entity: Entity<TopApp>,
) -> impl Fn(&str, &ClickEvent, &mut Window, &mut App) + 'static {
    move |feed_tag, _, _, cx| {
        cx.write_to_clipboard(ClipboardItem::new_string(feed_tag.to_string()));
        entity.update(cx, |this, cx| {
            this.settings_status = "Broadcast feed tag copied".to_string();
            cx.notify();
        });
    }
}
