//! Show screen adapter.
//!
//! ADR 0060 moves queue and transport presentation into a screen mount. This
//! adapter binds existing playback projection and transport callbacks to the
//! Show shell without introducing a workspace frame.
//! ADR 0059 places event registration and retryable liveness checks inside Live Metadata.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use gpui::{Context, Entity};
use rusqlite::Connection;

use crate::application::{
    ApplicationCommand, ApplicationServices, CommandContext, CommandError, CommandOutcome,
};
use crate::broadcast::control::{self, UnitRef};
use crate::broadcast::encoder::{self, EncoderTarget};
use crate::broadcast::publisher_targets::{self, PublisherTargetCommandError, PublisherTargetList};
use crate::broadcast::registry::BroadcastRegistry;
use crate::broadcast::transport::Transport;
use crate::config::BroadcastHostConfig;
use crate::presentation::present_command;
use crate::presentation::{bridge_watch, RuntimeHost};
use crate::runtime::{
    BroadcastEncoderWatchTarget, BroadcastReadinessSnapshot, BroadcastReadinessWatchHandle,
    BroadcastServiceRole, BroadcastServiceWatchHandle, BroadcastServiceWatchSnapshot,
    BroadcastServiceWatchUnit,
};
use crate::ui::composites::{live_status_strip, LiveStatusStrip, LiveStatusStripSlots};
use crate::ui::shells::show::{render_show, ShowShell, ShowSlots};
use crate::view_models::live_status::LiveStatusDisplay;
use crate::view_models::queue_now_playing::QueueNowPlayingPageVm;
use crate::view_models::show::{
    EventCheckResponse, EventCommandFeedback, EventCommandState, EventControlIntent,
    EventRegistryAction, EventRegistryInput, EventRegistryStatus, EventReportOperation,
    EventSectionInput, EventSelectionInput, EventState, EventTargetInput, EventTargetListInput,
    PublisherServiceOperation, PublisherServiceRole, ShowCardKind, ShowCommandId, ShowLogRequestId,
    ShowPageVm, StreamEncoderOperation, PUBLISHER_LOG_LINE_COUNT,
};
use crate::view_models::workspace::FrameNavigationEntry;
use crate::{config, db};

use super::queue_now_playing::{queue_now_playing_vm, queue_transport_action};
use super::{AppTab, TopApp, WorkspaceScreenMount};

const PRODUCER_UNIT: &str = "mixxx-now-playing.service";

pub(super) fn build_show_screen(
    app: &TopApp,
    window_width: f32,
    cx: &mut Context<TopApp>,
) -> ShowShell {
    let entity = cx.entity();
    let service_entity = entity.clone();
    let stop_entity = entity.clone();
    let reset_entity = entity.clone();
    let select_card_entity = entity.clone();
    let open_panel_entity = entity.clone();
    let close_panel_entity = entity.clone();
    let show_cuelist_entity = entity.clone();
    let connect_stream_entity = entity.clone();
    let disconnect_stream_entity = entity.clone();
    let readiness_entity = entity.clone();
    let slots = with_log_slots(ShowSlots::new(), &entity);
    let slots = with_event_registry_slots(slots, &entity);
    render_show(
        app.show_page.clone().with_window_width(window_width),
        slots
            .on_skip_previous(queue_transport_action(
                entity.clone(),
                TopApp::skip_playback_previous,
            ))
            .on_play_pause(queue_transport_action(
                entity.clone(),
                TopApp::toggle_playback_paused,
            ))
            .on_skip_next(queue_transport_action(entity, TopApp::skip_playback_next))
            .on_open_broadcast_readiness(move |_, _, cx| {
                readiness_entity.update(cx, |this, cx| {
                    this.open_broadcast_readiness_in_music(cx);
                });
            })
            .on_select_card(move |kind, _, _, cx| {
                select_card_entity.update(cx, |this, cx| {
                    this.select_show_card_detail(kind, cx);
                });
            })
            .on_open_show_panel(move |_, _, cx| {
                open_panel_entity.update(cx, |this, cx| {
                    this.open_show_panel(cx);
                });
            })
            .on_close_show_panel(move |_, _, cx| {
                close_panel_entity.update(cx, |this, cx| {
                    this.close_show_panel(cx);
                });
            })
            .on_show_cuelist(move |_, _, cx| {
                show_cuelist_entity.update(cx, |this, cx| {
                    this.show_cuelist_panel(cx);
                });
            })
            .on_start_publisher_service(move |role, _, _, cx| {
                service_entity.update(cx, |this, cx| {
                    this.run_publisher_service_command(role, PublisherServiceOperation::Start, cx);
                });
            })
            .on_stop_publisher_service(move |role, _, _, cx| {
                stop_entity.update(cx, |this, cx| {
                    this.run_publisher_service_command(role, PublisherServiceOperation::Stop, cx);
                });
            })
            .on_reset_publisher_service(move |role, _, _, cx| {
                reset_entity.update(cx, |this, cx| {
                    this.run_publisher_service_command(role, PublisherServiceOperation::Reset, cx);
                });
            })
            .on_connect_stream(move |_, _, cx| {
                connect_stream_entity.update(cx, |this, cx| {
                    this.run_stream_encoder_command(StreamEncoderOperation::Connect, cx);
                });
            })
            .on_disconnect_stream(move |_, _, cx| {
                disconnect_stream_entity.update(cx, |this, cx| {
                    this.run_stream_encoder_command(StreamEncoderOperation::Disconnect, cx);
                });
            }),
    )
}

/// Binds the event row's explicit registry actions (ADR 0059).
fn with_event_registry_slots(slots: ShowSlots, entity: &Entity<TopApp>) -> ShowSlots {
    let select_event_entity = entity.clone();
    let event_logs_entity = entity.clone();
    let event_control_entity = entity.clone();
    slots
        .on_select_event(move |event_id, _, cx| {
            select_event_entity.update(cx, |this, cx| this.select_show_event(event_id, cx));
        })
        .on_open_event_logs(move |_, _, cx| {
            event_logs_entity.update(cx, |this, cx| {
                if this.show_page.log_pane.shows_event() {
                    this.close_publisher_logs(cx);
                } else if let Some(input) = &this.event_section_input {
                    this.show_page.log_pane.show_event(input);
                    this.reproject_show_page_from_current_queue();
                    cx.notify();
                }
            });
        })
        .on_event_control(move |intent, _, _, cx| {
            event_control_entity.update(cx, |this, cx| this.run_event_control(intent, cx));
        })
}

/// Binds independent log reads and shared split resizing to the Show owner (ADR 0063).
fn with_log_slots(slots: ShowSlots, entity: &Entity<TopApp>) -> ShowSlots {
    let logs_entity = entity.clone();
    let close_logs_entity = entity.clone();
    let log_layout_entity = entity.clone();
    let log_resize_start_entity = entity.clone();
    let log_resize_move_entity = entity.clone();
    let log_resize_end_entity = entity.clone();
    slots
        .on_open_publisher_logs(move |role, _, _, cx| {
            logs_entity.update(cx, |this, cx| {
                this.open_publisher_logs(role, cx);
            });
        })
        .on_close_publisher_logs(move |_, _, cx| {
            close_logs_entity.update(cx, |this, cx| {
                this.close_publisher_logs(cx);
            });
        })
        .on_log_layout(move |height, available, _, cx| {
            log_layout_entity.update(cx, |this, cx| {
                if this.show_page.log_pane.update_geometry(height, available) {
                    cx.notify();
                }
            });
        })
        .on_log_resize_start(move |event, _, cx| {
            let scale = crate::ui::tokens::ScaleFactor::current(cx).multiplier();
            log_resize_start_entity.update(cx, |this, _| {
                this.show_log_resize = Some((
                    f32::from(event.position.y) / scale,
                    this.show_page.log_pane.height,
                ));
            });
        })
        .on_log_resize_move(move |event, _, cx| {
            let scale = crate::ui::tokens::ScaleFactor::current(cx).multiplier();
            log_resize_move_entity.update(cx, |this, cx| {
                if !event.dragging() {
                    this.show_log_resize = None;
                } else if let Some((start_y, start_height)) = this.show_log_resize {
                    this.show_page
                        .log_pane
                        .resize(start_height + start_y - f32::from(event.position.y) / scale);
                    cx.notify();
                }
            });
        })
        .on_log_resize_end(move |_, _, cx| {
            log_resize_end_entity.update(cx, |this, _| this.show_log_resize = None);
        })
}

pub(super) fn build_live_status_strip(
    app: &TopApp,
    mount: WorkspaceScreenMount,
    cx: &mut Context<TopApp>,
) -> Option<LiveStatusStrip> {
    if !matches!(
        mount,
        WorkspaceScreenMount::Music | WorkspaceScreenMount::Settings
    ) {
        return None;
    }
    let display = LiveStatusDisplay::from_show_page(&app.show_page);
    if !display.active {
        return None;
    }

    let entity = cx.entity();
    Some(live_status_strip(
        display,
        LiveStatusStripSlots::new().on_open_show(move |_, _, cx| {
            entity.update(cx, |this, cx| {
                this.select_tab(AppTab::Show, cx);
            });
        }),
    ))
}

impl TopApp {
    pub(super) fn maybe_start_broadcast_readiness_watch(&mut self, cx: &mut Context<Self>) {
        if self.broadcast_readiness_watch.is_some() {
            return;
        }
        let Some(host) = self.runtime_host.clone() else {
            self.settings_status = "Broadcast readiness error: runtime unavailable".to_string();
            return;
        };

        let handle = start_broadcast_readiness_watch(&host, Arc::clone(&self.conn));
        self.broadcast_readiness_snapshot = Some(handle.latest());
        bridge_watch(
            handle.subscribe(),
            |this: &mut Self, snapshot, cx| {
                this.apply_broadcast_readiness_snapshot(snapshot, cx);
            },
            cx,
        );
        self.broadcast_readiness_watch = Some(handle);
    }

    pub(super) fn maybe_start_broadcast_service_watch(&mut self, cx: &mut Context<Self>) {
        if self.publisher_service_watch.is_some() {
            return;
        }
        let Some(host) = self.runtime_host.clone() else {
            self.settings_status = "Publisher status error: runtime unavailable".to_string();
            return;
        };

        let selected_host = match selected_broadcast_host(&self.broadcast) {
            Ok(host) => host,
            Err(error) => {
                self.settings_status = format!("Publisher status error: {error:#}");
                return;
            }
        };
        let units = match broadcast_service_units(&selected_host) {
            Ok(units) => units,
            Err(error) => {
                self.settings_status = format!("Publisher status error: {error}");
                return;
            }
        };
        let encoder = match broadcast_encoder_watch_target(&self.broadcast) {
            Ok(encoder) => encoder,
            Err(error) => {
                self.settings_status = format!("Stream status error: {error:#}");
                return;
            }
        };
        self.show_commands.clear();
        let handle = start_broadcast_service_watch(&host, units, encoder);
        self.publisher_service_snapshot = Some(handle.latest());
        bridge_watch(
            handle.subscribe(),
            |this: &mut Self, snapshot, cx| {
                this.apply_publisher_service_snapshot(snapshot, cx);
            },
            cx,
        );
        self.publisher_service_watch = Some(handle);
    }

    pub(super) fn refresh_show_page(&self, cx: &mut Context<Self>) {
        let requested_event_input = self.event_section_input.clone();
        let command = RefreshShowPage::new(
            Arc::clone(&self.conn),
            Arc::clone(&self.application_services),
            self.queue_text_filter.clone(),
            self.broadcast.clone(),
        );
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, projection, cx| {
                let applied = apply_event_refresh(
                    &mut this.event_section_input,
                    requested_event_input.as_ref(),
                    projection.event_section_input,
                );
                this.reproject_show_page(projection.queue);
                let selection_changed = requested_event_input.as_ref().map(event_input_identity)
                    != this.event_section_input.as_ref().map(event_input_identity);
                if applied
                    && selection_changed
                    && this
                        .event_section_input
                        .as_ref()
                        .is_some_and(|input| input.selected_event.is_some())
                {
                    this.run_event_registry_command(EventRegistryAction::Check, cx);
                }
            },
            |this, error, _cx| {
                this.settings_status = format!("Show status error: {error:#}");
            },
        );
    }

    fn apply_publisher_service_snapshot(
        &mut self,
        snapshot: BroadcastServiceWatchSnapshot,
        cx: &mut Context<Self>,
    ) {
        self.show_commands.observe(&snapshot);
        self.publisher_service_snapshot = Some(snapshot);
        self.reproject_show_page_from_current_queue();
        cx.notify();
    }

    fn apply_broadcast_readiness_snapshot(
        &mut self,
        snapshot: BroadcastReadinessSnapshot,
        cx: &mut Context<Self>,
    ) {
        let mounted_report = self
            .music_readiness_list_is_current()
            .then(|| snapshot.report.clone())
            .flatten();
        self.broadcast_readiness_snapshot = Some(snapshot);
        if let Some(report) = mounted_report {
            self.library.update(cx, |library, cx| {
                library.show_broadcast_readiness_report(&report, cx);
            });
        }
        self.reproject_show_page_from_current_queue();
        cx.notify();
    }

    fn reproject_show_page(&mut self, queue: QueueNowPlayingPageVm) {
        if let Some(input) = &self.event_section_input {
            self.event_session.remember(input);
            if self.show_page.log_pane.shows_event() {
                self.show_page.log_pane.show_event(input);
            }
        }
        let panel_mode = self.show_page.panel_mode;
        let panel_open = self.show_page.panel_open;
        self.show_page = ShowPageVm::from_queue_publisher_readiness_and_event(
            queue,
            self.publisher_service_snapshot.as_ref(),
            self.show_page.log_pane.clone(),
            self.broadcast_readiness_snapshot.as_ref(),
            self.event_section_input.as_ref(),
        )
        .with_command_state(&self.show_commands)
        .with_status_message(&self.settings_status)
        .with_panel_state(panel_mode, panel_open);
    }

    fn reproject_show_page_from_current_queue(&mut self) {
        self.reproject_show_page(self.show_page.queue.clone());
    }

    fn select_show_card_detail(&mut self, kind: ShowCardKind, cx: &mut Context<Self>) {
        self.show_page = self.show_page.clone().select_card(kind);
        cx.notify();
    }

    fn open_show_panel(&mut self, cx: &mut Context<Self>) {
        self.show_page = self.show_page.clone().open_panel();
        cx.notify();
    }

    fn close_show_panel(&mut self, cx: &mut Context<Self>) {
        self.show_page = self.show_page.clone().close_panel();
        cx.notify();
    }

    fn show_cuelist_panel(&mut self, cx: &mut Context<Self>) {
        self.show_page = self.show_page.clone().show_cuelist_panel();
        cx.notify();
    }

    fn run_publisher_service_command(
        &mut self,
        role: PublisherServiceRole,
        operation: PublisherServiceOperation,
        cx: &mut Context<Self>,
    ) {
        let selected_host = match selected_broadcast_host(&self.broadcast) {
            Ok(host) => host,
            Err(error) => {
                self.settings_status = format!("Publisher command error: {error:#}");
                cx.notify();
                return;
            }
        };
        let command = match PublisherServiceCommand::new(&selected_host, role, operation) {
            Ok(command) => command,
            Err(error) => {
                self.settings_status = format!("Publisher command error: {error}");
                cx.notify();
                return;
            }
        };
        let Some(command_id) = self.show_commands.begin_service(role, operation) else {
            return;
        };
        self.settings_status.clear();
        self.reproject_show_page_from_current_queue();
        cx.notify();

        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, completion, cx| {
                this.finish_show_command(command_id, completion, cx);
            },
            move |this, error, cx| {
                this.finish_show_command(command_id, ShowCommandCompletion::new(Err(error)), cx);
            },
        );
    }

    fn finish_show_command(
        &mut self,
        command_id: ShowCommandId,
        completion: ShowCommandCompletion,
        cx: &mut Context<Self>,
    ) {
        if !self.show_commands.complete(
            command_id,
            completion.returned_at,
            completion.result.is_ok(),
        ) {
            return;
        }
        self.settings_status = completion
            .result
            .err()
            .map_or_else(String::new, |error| error.to_string());
        self.reproject_show_page_from_current_queue();
        self.invalidate_publisher_service_snapshot();
        cx.notify();
    }

    fn open_publisher_logs(&mut self, role: PublisherServiceRole, cx: &mut Context<Self>) {
        self.show_log_resize = None;
        let Some(request_id) = self.show_page.toggle_publisher_logs(role) else {
            cx.notify();
            return;
        };
        cx.notify();

        let selected_host = match selected_broadcast_host(&self.broadcast) {
            Ok(host) => host,
            Err(error) => {
                self.show_page
                    .log_pane
                    .apply_result(request_id, Err(format!("Publisher log error: {error:#}")));
                cx.notify();
                return;
            }
        };
        let command = match ReadPublisherLogs::new(&selected_host, role, request_id) {
            Ok(command) => command,
            Err(error) => {
                self.show_page
                    .log_pane
                    .apply_result(request_id, Err(format!("Publisher log error: {error}")));
                cx.notify();
                return;
            }
        };
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            |this, logs, cx| {
                if this
                    .show_page
                    .log_pane
                    .apply_result(logs.request_id, Ok(logs.text))
                {
                    cx.notify();
                }
            },
            move |this, error, cx| {
                if this
                    .show_page
                    .log_pane
                    .apply_result(request_id, Err(format!("Publisher log error: {error:#}")))
                {
                    cx.notify();
                }
            },
        );
    }

    fn close_publisher_logs(&mut self, cx: &mut Context<Self>) {
        self.show_page.close_publisher_logs();
        self.show_log_resize = None;
        cx.notify();
    }

    fn run_event_registry_command(&mut self, action: EventRegistryAction, cx: &mut Context<Self>) {
        let available = self
            .show_page
            .publisher
            .as_ref()
            .and_then(|section| section.event.as_ref())
            .is_some_and(|event| !event.actions.registry_action(action).disabled());
        if !available {
            return;
        }
        let Some(input) = self.event_section_input.as_mut() else {
            return;
        };
        self.event_session.next_request += 1;
        input.request = self.event_session.next_request;
        let request = input.request;
        let request_context = input.context.clone();
        let error_context = request_context.clone();
        let command = EventRegistryCommand {
            selection_revision: input.registry.revision,
            conn: Arc::clone(&self.conn),
            cfg_path: self.cfg_path.clone(),
            action,
            selected_event: input.selected_event.clone(),
        };
        begin_event_registry_action(input, action);
        if action == EventRegistryAction::Check {
            self.read_event_targets(request, cx);
        }
        self.reproject_show_page_from_current_queue();
        cx.notify();
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, event, cx| {
                if !this.event_request_is_current(request, &request_context) {
                    return;
                }
                let Some(input) = this.event_section_input.as_mut() else {
                    return;
                };
                let next = apply_event_registry_result(input, action, Ok(event));
                this.reproject_show_page_from_current_queue();
                cx.notify();
                if let Some(next) = next {
                    this.run_event_registry_command(next, cx);
                }
            },
            move |this, error, cx| {
                if !this.event_request_is_current(request, &error_context) {
                    return;
                }
                if let Some(input) = this.event_section_input.as_mut() {
                    apply_event_registry_result(input, action, Err(error));
                }
                this.reproject_show_page_from_current_queue();
                cx.notify();
            },
        );
    }

    fn run_event_target_command(
        &mut self,
        operation: EventTargetOperation,
        cx: &mut Context<Self>,
    ) {
        let available = self
            .show_page
            .publisher
            .as_ref()
            .and_then(|section| section.event.as_ref())
            .is_some_and(|event| match operation {
                EventTargetOperation::Attach => !event.actions.attach.disabled(),
                EventTargetOperation::Detach => !event.actions.detach.disabled(),
            });
        if !available {
            return;
        }
        let Some(input) = self.event_section_input.clone() else {
            "Event target command error: event state is not loaded"
                .clone_into(&mut self.settings_status);
            cx.notify();
            return;
        };
        let Some(event) = input.selected_event.clone() else {
            "Event target command error: no broadcast event selected"
                .clone_into(&mut self.settings_status);
            cx.notify();
            return;
        };
        let selected_host = match selected_broadcast_host(&self.broadcast) {
            Ok(host) => host,
            Err(error) => {
                self.settings_status = format!("Event target command error: {error:#}");
                cx.notify();
                return;
            }
        };
        let target_name = match event_target_name_for_operation(operation, &input) {
            Ok(name) => name,
            Err(error) => {
                self.settings_status = format!("Event target command error: {error}");
                cx.notify();
                return;
            }
        };
        let Some(command_id) = self.show_commands.begin_service(
            PublisherServiceRole::Publisher,
            PublisherServiceOperation::Start,
        ) else {
            return;
        };
        self.event_session.next_request += 1;
        let request = self.event_session.next_request;
        let context = input.context.clone();
        let error_context = context.clone();
        let command = EventTargetCommand {
            conn: Arc::clone(&self.conn),
            selection_revision: input.registry.revision,
            operation,
            transport: selected_host.transport,
            instance_name: selected_host.instance_name,
            target_name,
            event_id: event.event_id,
            token_path: event.token_path,
        };
        if let Some(input) = &mut self.event_section_input {
            input.request = request;
            input.feedback.target_mutation = EventCommandState::Working;
            input.feedback.active_action = Some(match operation {
                EventTargetOperation::Attach => EventControlIntent::Attach,
                EventTargetOperation::Detach => EventControlIntent::Detach,
            });
            input.record_event_report(EventReportOperation::TargetMutation, chrono::Utc::now());
            input.targets = EventTargetListInput::Unknown;
        }
        self.reproject_show_page_from_current_queue();
        cx.notify();
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, completion, cx| {
                this.finish_event_target(request, &context, command_id, completion, cx);
            },
            move |this, error, cx| {
                this.finish_event_target(
                    request,
                    &error_context,
                    command_id,
                    ShowCommandCompletion::new(Err(error)),
                    cx,
                );
            },
        );
    }

    fn run_stream_encoder_command(
        &mut self,
        operation: StreamEncoderOperation,
        cx: &mut Context<Self>,
    ) {
        let command = match StreamEncoderCommand::new(&self.broadcast, operation) {
            Ok(command) => command,
            Err(error) => {
                self.settings_status = format!("Stream command error: {error:#}");
                cx.notify();
                return;
            }
        };
        let Some(command_id) = self.show_commands.begin_stream(operation) else {
            return;
        };
        self.settings_status.clear();
        self.reproject_show_page_from_current_queue();
        cx.notify();

        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, completion, cx| {
                this.finish_show_command(command_id, completion, cx);
            },
            move |this, error, cx| {
                this.finish_show_command(command_id, ShowCommandCompletion::new(Err(error)), cx);
            },
        );
    }

    fn invalidate_publisher_service_snapshot(&self) {
        if let Some(handle) = self.publisher_service_watch.as_ref() {
            let _ = handle.refresh_now();
        }
    }

    pub(super) fn invalidate_broadcast_readiness_snapshot(&self) {
        if let Some(handle) = self.broadcast_readiness_watch.as_ref() {
            let _ = handle.refresh_now();
        }
    }

    fn open_broadcast_readiness_in_music(&mut self, cx: &mut Context<Self>) {
        let Some(report) = self
            .broadcast_readiness_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.report.clone())
        else {
            "Broadcast readiness is not available yet".clone_into(&mut self.settings_status);
            cx.notify();
            return;
        };

        self.tab = AppTab::Music;
        if let Some(content_list_id) = self.content_list_frame_id() {
            if let Err(error) = self
                .workspace_layout
                .reset_nav(content_list_id, FrameNavigationEntry::ReadinessIssues)
            {
                self.settings_status = format!("Error opening broadcast readiness: {error}");
            }
        }
        self.library.update(cx, |library, cx| {
            library.show_broadcast_readiness_report(&report, cx);
        });
        cx.notify();
    }

    fn music_readiness_list_is_current(&self) -> bool {
        matches!(self.tab, AppTab::Music)
            && self
                .content_list_frame_id()
                .and_then(|id| self.workspace_layout.frame_nav(id))
                .is_some_and(|nav| matches!(nav.current(), FrameNavigationEntry::ReadinessIssues))
    }
}

/// Registry commands have no publisher transport or configuration mutation capability (ADR 0059).
struct EventRegistryCommand {
    conn: Arc<Mutex<Connection>>,
    cfg_path: PathBuf,
    action: EventRegistryAction,
    selected_event: Option<EventSelectionInput>,
    selection_revision: i64,
}

impl ApplicationCommand for EventRegistryCommand {
    type Output = EventRegistryResult;

    fn execute(self, context: &CommandContext) -> crate::application::CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let conn = self
            .conn
            .lock()
            .map_err(|_| CommandError::Query("database lock poisoned".to_owned()))?;
        // Revalidate stored identity and liveness before any relay mutation.
        let selection = db::broadcast_event_selection(&conn).map_err(event_registry_error)?;
        let selected = selected_event_input(&conn).map_err(event_registry_error)?;
        if selection.revision != self.selection_revision
            || selection.event_id.as_deref()
                != self
                    .selected_event
                    .as_ref()
                    .map(|event| event.event_id.as_str())
        {
            return Err(CommandError::Other(
                "Selected event changed; refresh Show before trying again.".to_owned(),
            ));
        }
        let allowed = match self.action {
            EventRegistryAction::Create => {
                selection.event_id.is_none()
                    && db::broadcast_events(&conn)
                        .map_err(event_registry_error)?
                        .is_empty()
            }
            EventRegistryAction::Replace => selected
                .as_ref()
                .is_some_and(|event| event.state == EventState::Dead),
            EventRegistryAction::Check => selected.is_some(),
        };
        if !allowed {
            return Err(CommandError::Other(
                "Event action is unavailable in the current state.".to_owned(),
            ));
        }
        let endpoint = match &selected {
            Some(event) if self.action == EventRegistryAction::Check => event.endpoint.clone(),
            _ => config::load_musicindex_endpoint(&self.cfg_path).map_err(event_registry_error)?,
        };
        let directory = self
            .cfg_path
            .parent()
            .ok_or_else(|| CommandError::Other("Config path has no directory".to_owned()))?
            .join("broadcast")
            .join("tokens");
        let registry = BroadcastRegistry::with_token_directory(&conn, &endpoint, directory)
            .map_err(event_registry_error)?;
        let event = match self.action {
            EventRegistryAction::Create | EventRegistryAction::Replace => {
                registry
                    .create_event(None)
                    .map_err(event_registry_error)?
                    .event
            }
            EventRegistryAction::Check => {
                let event = selected
                    .as_ref()
                    .ok_or_else(|| CommandError::Other("No event selected".to_owned()))?;
                match registry.check_event(&event.event_id) {
                    Ok(checked) => checked.event,
                    Err(error) => {
                        return Ok(CommandOutcome::without_events(
                            EventRegistryResult::CheckFailed {
                                response: event_check_response(&error),
                                detail: format!("{error:#}"),
                            },
                        ));
                    }
                }
            }
        };
        let (selection, selection_error) = if self.action == EventRegistryAction::Check {
            (Some(selection), None)
        } else {
            match db::select_broadcast_event(&conn, &event.event_id) {
                Ok(selection) => (Some(selection), None),
                Err(error) => (
                    None,
                    Some(format!(
                        "Event registered, but saving the choice failed: {error:#}"
                    )),
                ),
            }
        };
        let events = db::broadcast_events(&conn)
            .map_err(event_registry_error)?
            .into_iter()
            .map(event_selection_input)
            .collect();
        Ok(CommandOutcome::without_events(
            EventRegistryResult::Complete(EventRegistrySuccess {
                event: event_selection_input(event),
                selection,
                selection_error,
                events,
            }),
        ))
    }
}

fn event_check_response(error: &anyhow::Error) -> EventCheckResponse {
    if let Some(error) = error.downcast_ref::<crate::api::LiveMetadataReadError>() {
        error
            .response_status
            .map_or(EventCheckResponse::NoResponse, EventCheckResponse::Http)
    } else if let Some(error) =
        error.downcast_ref::<crate::broadcast::registry::EventCheckSaveError>()
    {
        EventCheckResponse::SaveFailed(event_state(Some(error.observed_status)))
    } else {
        EventCheckResponse::Unclassified
    }
}

fn event_registry_error(error: impl std::fmt::Display) -> CommandError {
    CommandError::Other(format!("{error:#}"))
}

fn begin_event_registry_action(input: &mut EventSectionInput, action: EventRegistryAction) {
    input.feedback.active_action = Some(match action {
        EventRegistryAction::Create => EventControlIntent::Create,
        EventRegistryAction::Replace => EventControlIntent::Replace,
        EventRegistryAction::Check => EventControlIntent::Check,
    });
    match action {
        EventRegistryAction::Create | EventRegistryAction::Replace => {
            input.feedback.registration = EventCommandState::Working;
            input.feedback.check = EventCommandState::Idle;
            input.record_event_report(EventReportOperation::Check, chrono::Utc::now());
            input.record_event_report(EventReportOperation::Registration, chrono::Utc::now());
        }
        EventRegistryAction::Check => {
            input.feedback.check = EventCommandState::Working;
            input.record_event_report(EventReportOperation::Check, chrono::Utc::now());
        }
    }
}

enum EventRegistryResult {
    Complete(EventRegistrySuccess),
    CheckFailed {
        response: EventCheckResponse,
        detail: String,
    },
}

struct EventRegistrySuccess {
    event: EventSelectionInput,
    selection: Option<db::BroadcastEventSelection>,
    selection_error: Option<String>,
    events: Vec<EventSelectionInput>,
}

/// Registration and selection storage have independent results (ADR 0059).
fn apply_event_registry_result(
    input: &mut EventSectionInput,
    action: EventRegistryAction,
    result: Result<EventRegistryResult, CommandError>,
) -> Option<EventRegistryAction> {
    match result {
        Ok(EventRegistryResult::CheckFailed { response, detail }) => {
            input.feedback.check_response = response;
            input.feedback.verification_failure = Some(detail.clone());
            input.feedback.check = EventCommandState::Failed { detail };
            input.record_event_report(EventReportOperation::Check, chrono::Utc::now());
            None
        }
        Ok(EventRegistryResult::Complete(result)) => {
            input.registry.events = result.events;
            if action == EventRegistryAction::Check {
                input.feedback.check = EventCommandState::Succeeded;
            } else {
                input.feedback.registration = EventCommandState::Succeeded;
                input.record_event_report_for(
                    EventReportOperation::Registration,
                    chrono::Utc::now(),
                    Some(result.event.clone()),
                );
                if let Some(detail) = result.selection_error {
                    input.feedback.selection = EventCommandState::Failed { detail };
                    input.record_event_report_for(
                        EventReportOperation::Selection,
                        chrono::Utc::now(),
                        Some(result.event),
                    );
                    return None;
                }
                input.feedback.selection = EventCommandState::Succeeded;
                input.record_event_report_for(
                    EventReportOperation::Selection,
                    chrono::Utc::now(),
                    Some(result.event.clone()),
                );
                input.targets = EventTargetListInput::Unknown;
            }
            if let Some(selection) = result.selection {
                input.registry.selected_id = selection.event_id;
                input.registry.revision = selection.revision;
            }
            input.selected_event = Some(result.event);
            input.liveness_confirmed = action == EventRegistryAction::Check;
            input.feedback.verification_failure = None;
            if action == EventRegistryAction::Check {
                input.record_event_report(EventReportOperation::Check, chrono::Utc::now());
            }
            (action != EventRegistryAction::Check).then_some(EventRegistryAction::Check)
        }
        Err(error) => {
            let failure = EventCommandState::Failed {
                detail: error.to_string(),
            };
            match action {
                EventRegistryAction::Check => {
                    input.feedback.verification_failure = Some(error.to_string());
                    input.feedback.check = failure;
                    input.feedback.check_response = EventCheckResponse::Unclassified;
                    input.record_event_report(EventReportOperation::Check, chrono::Utc::now());
                }
                EventRegistryAction::Create | EventRegistryAction::Replace => {
                    input.feedback.registration = failure;
                    input.record_event_report(
                        EventReportOperation::Registration,
                        chrono::Utc::now(),
                    );
                }
            }
            None
        }
    }
}

/// A queued refresh cannot overwrite a registration or check completed since it was requested.
fn apply_event_refresh(
    current: &mut Option<EventSectionInput>,
    requested: Option<&EventSectionInput>,
    mut refreshed: Option<EventSectionInput>,
) -> bool {
    if current.as_ref() != requested
        || current.as_ref().is_some_and(|input| {
            input.feedback.working()
                && refreshed
                    .as_ref()
                    .is_none_or(|new| new.context == input.context)
        })
    {
        return false;
    }
    if let (Some(old), Some(new)) = (current.as_ref(), refreshed.as_mut()) {
        if old.selected_event.as_ref().map(|event| &event.event_id)
            == new.selected_event.as_ref().map(|event| &event.event_id)
            && new.context == old.context
            && new.registry.revision == old.registry.revision
        {
            new.liveness_confirmed = old.liveness_confirmed;
            new.feedback = old.feedback.clone();
            new.targets = old.targets.clone();
            new.request = old.request;
        }
    }
    *current = refreshed;
    true
}

struct RefreshShowPage {
    conn: Arc<Mutex<Connection>>,
    application_services: Arc<ApplicationServices>,
    queue_text_filter: Option<String>,
    broadcast: config::BroadcastConfig,
}

impl RefreshShowPage {
    fn new(
        conn: Arc<Mutex<Connection>>,
        application_services: Arc<ApplicationServices>,
        queue_text_filter: Option<String>,
        broadcast: config::BroadcastConfig,
    ) -> Self {
        Self {
            conn,
            application_services,
            queue_text_filter,
            broadcast,
        }
    }
}

impl ApplicationCommand for RefreshShowPage {
    type Output = ShowPageProjection;

    fn execute(self, context: &CommandContext) -> crate::application::CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }

        let (queue, event_section_input) = {
            let conn = self
                .conn
                .lock()
                .map_err(|_| CommandError::Query("database lock poisoned".to_string()))?;
            let queue =
                queue_now_playing_vm(&self.application_services, &conn, self.queue_text_filter);
            let input = load_event_section(&conn, &self.broadcast);
            (queue, Some(input))
        };
        Ok(CommandOutcome::without_events(ShowPageProjection {
            queue,
            event_section_input,
        }))
    }
}

struct ShowPageProjection {
    queue: QueueNowPlayingPageVm,
    event_section_input: Option<EventSectionInput>,
}

/// Capture completion in the worker, before presentation callback scheduling (ADR 0059).
struct ShowCommandCompletion {
    returned_at: Instant,
    result: Result<(), CommandError>,
}

impl ShowCommandCompletion {
    fn new(result: Result<(), CommandError>) -> Self {
        Self {
            returned_at: Instant::now(),
            result,
        }
    }
}

struct PublisherServiceCommand {
    role: PublisherServiceRole,
    transport: crate::broadcast::transport::Transport,
    unit: UnitRef,
    operation: PublisherServiceOperation,
}

impl PublisherServiceCommand {
    fn new(
        host: &BroadcastHostConfig,
        role: PublisherServiceRole,
        operation: PublisherServiceOperation,
    ) -> Result<Self, CommandError> {
        Ok(Self {
            role,
            transport: host.transport.clone(),
            unit: broadcast_service_unit(host, role).map_err(publisher_command_error)?,
            operation,
        })
    }
}

impl ApplicationCommand for PublisherServiceCommand {
    type Output = ShowCommandCompletion;

    fn execute(self, context: &CommandContext) -> crate::application::CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let result = match self.operation {
            PublisherServiceOperation::Start => control::start(&self.transport, &self.unit),
            PublisherServiceOperation::Stop => control::stop(&self.transport, &self.unit),
            PublisherServiceOperation::Reset => control::reset(&self.transport, &self.unit),
        }
        .map_err(|error| {
            publisher_command_error(format!(
                "{} {}: {error:#}",
                operation_label(self.operation),
                role_label(self.role)
            ))
        });

        Ok(CommandOutcome::without_events(ShowCommandCompletion::new(
            result,
        )))
    }
}

struct ReadPublisherLogs {
    request_id: ShowLogRequestId,
    role: PublisherServiceRole,
    transport: crate::broadcast::transport::Transport,
    unit: UnitRef,
    line_count: usize,
}

impl ReadPublisherLogs {
    fn new(
        host: &BroadcastHostConfig,
        role: PublisherServiceRole,
        request_id: ShowLogRequestId,
    ) -> Result<Self, CommandError> {
        Ok(Self {
            request_id,
            role,
            transport: host.transport.clone(),
            unit: broadcast_service_unit(host, role).map_err(publisher_command_error)?,
            line_count: PUBLISHER_LOG_LINE_COUNT,
        })
    }
}

impl ApplicationCommand for ReadPublisherLogs {
    type Output = PublisherLogsResult;

    fn execute(self, context: &CommandContext) -> crate::application::CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let text =
            control::logs(&self.transport, &self.unit, self.line_count).map_err(|error| {
                publisher_command_error(format!("read {} logs: {error:#}", role_label(self.role)))
            })?;
        Ok(CommandOutcome::without_events(PublisherLogsResult {
            request_id: self.request_id,
            text,
        }))
    }
}

struct PublisherLogsResult {
    request_id: ShowLogRequestId,
    text: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EventTargetOperation {
    Attach,
    Detach,
}

struct EventTargetCommand {
    conn: Arc<Mutex<Connection>>,
    selection_revision: i64,
    operation: EventTargetOperation,
    transport: Transport,
    instance_name: String,
    target_name: String,
    event_id: String,
    token_path: String,
}

impl ApplicationCommand for EventTargetCommand {
    type Output = ShowCommandCompletion;

    fn execute(self, context: &CommandContext) -> crate::application::CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let conn = self
            .conn
            .lock()
            .map_err(|_| event_registry_error("database lock poisoned"))?;
        let selection = db::broadcast_event_selection(&conn).map_err(event_registry_error)?;
        if selection.revision != self.selection_revision
            || selection.event_id.as_deref() != Some(&self.event_id)
        {
            return Err(event_registry_error(
                "Selected event changed before target mutation",
            ));
        }
        let selected =
            db::broadcast_event_by_event_id(&conn, &self.event_id).map_err(event_registry_error)?;
        if self.operation == EventTargetOperation::Attach
            && selected
                .as_ref()
                .is_none_or(|event| event.last_status != Some(db::BroadcastEventStatus::Live))
        {
            return Err(event_registry_error("Event is no longer confirmed Live"));
        }
        if self.operation == EventTargetOperation::Detach {
            let targets = publisher_targets::list_targets(&self.transport, &self.instance_name)
                .map_err(event_registry_error)?;
            if !targets
                .targets
                .iter()
                .any(|target| target.name == self.target_name && target.event_id == self.event_id)
            {
                return Err(event_registry_error(
                    "Configured target changed before Detach; check it again",
                ));
            }
        }
        let result = match self.operation {
            EventTargetOperation::Attach => publisher_targets::attach_event(
                &self.transport,
                &self.instance_name,
                &self.target_name,
                &self.event_id,
                Path::new(&self.token_path),
            ),
            EventTargetOperation::Detach => publisher_targets::detach_target(
                &self.transport,
                &self.instance_name,
                &self.target_name,
            ),
        }
        .map_err(|error| {
            if let PublisherTargetCommandError::RestartFailed { detail } = error {
                return event_target_command_error(format!(
                    "Target configuration saved; Publisher restart failed: {detail}"
                ));
            }
            event_target_command_error(format!(
                "{} event target: {error}",
                event_target_operation_label(self.operation)
            ))
        });
        Ok(CommandOutcome::without_events(ShowCommandCompletion::new(
            result,
        )))
    }
}

fn event_target_name_for_operation(
    operation: EventTargetOperation,
    input: &EventSectionInput,
) -> Result<String, CommandError> {
    match operation {
        EventTargetOperation::Attach => {
            let target_name = input.attach_target_name.trim();
            if target_name.is_empty() {
                return Err(CommandError::Other(
                    "broadcast target name is not configured".to_owned(),
                ));
            }
            Ok(target_name.to_owned())
        }
        EventTargetOperation::Detach => attached_target_name(input).ok_or_else(|| {
            CommandError::Other("selected broadcast event is not attached".to_owned())
        }),
    }
}

fn attached_target_name(input: &EventSectionInput) -> Option<String> {
    let event_id = input.selected_event.as_ref()?.event_id.as_str();
    let EventTargetListInput::Loaded { targets } = &input.targets else {
        return None;
    };
    targets
        .iter()
        .find(|target| {
            target.name == input.attach_target_name.trim() && target.event_id == event_id
        })
        .map(|target| target.name.trim().to_owned())
        .filter(|target_name| !target_name.is_empty())
}

fn start_broadcast_service_watch(
    host: &RuntimeHost,
    units: Vec<BroadcastServiceWatchUnit>,
    encoder: BroadcastEncoderWatchTarget,
) -> BroadcastServiceWatchHandle {
    let _enter = host.handle().enter();
    crate::runtime::broadcast_service_watch::start(units, encoder)
}

fn start_broadcast_readiness_watch(
    host: &RuntimeHost,
    conn: Arc<Mutex<Connection>>,
) -> BroadcastReadinessWatchHandle {
    let _enter = host.handle().enter();
    crate::runtime::broadcast_readiness::start(conn)
}

struct StreamEncoderCommand {
    target: EncoderTarget,
    server_name: Option<String>,
    operation: StreamEncoderOperation,
}

impl StreamEncoderCommand {
    fn new(
        broadcast: &crate::config::BroadcastConfig,
        operation: StreamEncoderOperation,
    ) -> Result<Self, CommandError> {
        let encoder = broadcast
            .encoder
            .as_ref()
            .ok_or_else(|| CommandError::Other("broadcast.encoder is not configured".to_owned()))?;
        Ok(Self {
            target: encoder.target().map_err(stream_command_error)?,
            server_name: encoder
                .default_server_name
                .as_deref()
                .map(|name| name.trim().to_owned()),
            operation,
        })
    }
}

impl ApplicationCommand for StreamEncoderCommand {
    type Output = ShowCommandCompletion;

    fn execute(self, context: &CommandContext) -> crate::application::CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let result = match self.operation {
            StreamEncoderOperation::Connect => {
                encoder::connect(&self.target, self.server_name.as_deref())
            }
            StreamEncoderOperation::Disconnect => encoder::disconnect(&self.target),
        }
        .map_err(|error| {
            stream_command_error(format!(
                "{} stream encoder: {error:#}",
                stream_operation_label(self.operation)
            ))
        });

        Ok(CommandOutcome::without_events(ShowCommandCompletion::new(
            result,
        )))
    }
}

fn selected_broadcast_host(
    broadcast: &crate::config::BroadcastConfig,
) -> anyhow::Result<BroadcastHostConfig> {
    Ok(broadcast.selected_host()?.clone())
}

fn selected_event_input(conn: &Connection) -> anyhow::Result<Option<EventSelectionInput>> {
    let selection = db::broadcast_event_selection(conn)?;
    selection
        .event_id
        .as_deref()
        .map(|id| db::broadcast_event_by_event_id(conn, id))
        .transpose()
        .map(|event| event.flatten().map(event_selection_input))
}

fn event_selection_input(event: db::BroadcastEventRow) -> EventSelectionInput {
    EventSelectionInput {
        created_at: event.created_at,
        last_checked_at: event.last_checked_at,
        label: event.label,
        event_id: event.event_id,
        endpoint: event.endpoint,
        token_file_missing: token_file_missing(&event.token_path),
        token_path: event.token_path,
        state: event_state(event.last_status),
    }
}

fn load_event_section(conn: &Connection, broadcast: &config::BroadcastConfig) -> EventSectionInput {
    let mut input = empty_event_section(broadcast);
    let result = (|| -> anyhow::Result<()> {
        let selection = db::broadcast_event_selection(conn)?;
        input.registry.events = db::broadcast_events(conn)?
            .into_iter()
            .map(event_selection_input)
            .collect();
        input.selected_event = selected_event_input(conn)?;
        input.registry.selected_id = selection.event_id;
        input.registry.revision = selection.revision;
        Ok(())
    })();
    input.registry.status = match result {
        Ok(()) => EventRegistryStatus::Loaded,
        Err(error) => EventRegistryStatus::Failed(format!("{error:#}")),
    };
    input
}

fn empty_event_section(broadcast: &config::BroadcastConfig) -> EventSectionInput {
    EventSectionInput {
        liveness_confirmed: false,
        registry: EventRegistryInput {
            status: EventRegistryStatus::Loading,
            ..EventRegistryInput::default()
        },
        feedback: EventCommandFeedback::default(),
        selected_event: None,
        targets: EventTargetListInput::Unknown,
        attach_target_name: broadcast.drop_file_target.trim().to_owned(),
        remote_host: broadcast
            .selected_host()
            .is_ok_and(|host| matches!(host.transport, Transport::Ssh { .. })),
        context: event_context(broadcast),
        request: 0,
    }
}

fn event_context(broadcast: &config::BroadcastConfig) -> String {
    broadcast.selected_host().map_or_else(
        |error| error.to_string(),
        |host| {
            format!(
                "{} / {} / {:?} / {}",
                host.name,
                host.instance_name,
                host.transport,
                broadcast.drop_file_target.trim()
            )
        },
    )
}

fn read_targets(broadcast: &config::BroadcastConfig) -> EventTargetListInput {
    let host = match selected_broadcast_host(broadcast) {
        Ok(host) => host,
        Err(error) => {
            return EventTargetListInput::Failed {
                detail: format!("{error:#}"),
            }
        }
    };
    match publisher_targets::list_targets(&host.transport, &host.instance_name) {
        Ok(targets) => event_targets_loaded(targets),
        Err(PublisherTargetCommandError::CommandsUnavailable) => {
            EventTargetListInput::CommandsUnavailable
        }
        Err(PublisherTargetCommandError::NotReachable) => EventTargetListInput::NotReachable,
        Err(error) => EventTargetListInput::Failed {
            detail: error.to_string(),
        },
    }
}

fn event_targets_loaded(targets: PublisherTargetList) -> EventTargetListInput {
    EventTargetListInput::Loaded {
        targets: targets
            .targets
            .into_iter()
            .map(|target| EventTargetInput {
                name: target.name,
                event_id: target.event_id,
            })
            .collect(),
    }
}

const fn event_state(status: Option<db::BroadcastEventStatus>) -> EventState {
    match status {
        Some(db::BroadcastEventStatus::Live) => EventState::Live,
        Some(db::BroadcastEventStatus::Dead) => EventState::Dead,
        Some(db::BroadcastEventStatus::Unknown) | None => EventState::Unknown,
    }
}

fn token_file_missing(path: &str) -> bool {
    fs::metadata(path).is_err()
}

fn broadcast_encoder_watch_target(
    broadcast: &crate::config::BroadcastConfig,
) -> anyhow::Result<BroadcastEncoderWatchTarget> {
    let Some(encoder) = &broadcast.encoder else {
        return Ok(BroadcastEncoderWatchTarget::not_configured());
    };
    Ok(BroadcastEncoderWatchTarget::configured(
        encoder
            .default_server_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map_or_else(|| "Encoder default".to_owned(), ToOwned::to_owned),
        encoder.target()?,
    ))
}

fn broadcast_service_units(
    host: &BroadcastHostConfig,
) -> anyhow::Result<Vec<BroadcastServiceWatchUnit>> {
    let host_name = host.name.trim().to_owned();
    Ok(vec![
        BroadcastServiceWatchUnit::new(
            BroadcastServiceRole::Publisher,
            host_name.clone(),
            host.transport.clone(),
            broadcast_service_unit(host, BroadcastServiceRole::Publisher)?,
        ),
        BroadcastServiceWatchUnit::new(
            BroadcastServiceRole::Producer,
            host_name,
            host.transport.clone(),
            broadcast_service_unit(host, BroadcastServiceRole::Producer)?,
        ),
    ])
}

fn broadcast_service_unit(
    host: &BroadcastHostConfig,
    role: PublisherServiceRole,
) -> anyhow::Result<UnitRef> {
    match role {
        PublisherServiceRole::Publisher => UnitRef::publisher(&host.instance_name),
        PublisherServiceRole::Producer => UnitRef::new(PRODUCER_UNIT),
    }
}

fn publisher_command_error(error: impl std::fmt::Display) -> CommandError {
    CommandError::Other(error.to_string())
}

fn stream_command_error(error: impl std::fmt::Display) -> CommandError {
    CommandError::Other(error.to_string())
}

fn event_target_command_error(error: impl std::fmt::Display) -> CommandError {
    CommandError::Other(error.to_string())
}

const fn role_label(role: PublisherServiceRole) -> &'static str {
    match role {
        PublisherServiceRole::Publisher => "publisher",
        PublisherServiceRole::Producer => "producer",
    }
}

const fn operation_label(operation: PublisherServiceOperation) -> &'static str {
    match operation {
        PublisherServiceOperation::Start => "start",
        PublisherServiceOperation::Stop => "stop",
        PublisherServiceOperation::Reset => "reset",
    }
}

const fn stream_operation_label(operation: StreamEncoderOperation) -> &'static str {
    match operation {
        StreamEncoderOperation::Connect => "connect",
        StreamEncoderOperation::Disconnect => "disconnect",
    }
}

const fn event_target_operation_label(operation: EventTargetOperation) -> &'static str {
    match operation {
        EventTargetOperation::Attach => "attach",
        EventTargetOperation::Detach => "detach",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    use crate::broadcast::control::ServiceState;
    use crate::runtime::broadcast_service_watch::{
        BroadcastEncoderSnapshot, BroadcastServiceUnitSnapshot,
    };
    use crate::view_models::show::{EventSectionDisplay, ShowLogPaneDisplay};

    fn show_event_db(temp: &tempfile::TempDir) -> Connection {
        let cfg = config::Config {
            music_dir: temp.path().join("music"),
            db_path: temp.path().join("app.sqlite"),
            flac_path: None,
            playback: config::PlaybackConfig::default(),
            broadcast: config::BroadcastConfig::default(),
            ui_scale: config::UiScale::default(),
            theme_profile: crate::theme_profile::ThemeProfile::default(),
            workspace_layout: None,
            workspace: None,
        };
        db::open_db(&cfg).unwrap()
    }

    fn show_event_relay(statuses: Vec<u16>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let thread = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for status in statuses {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                let mut byte = [0];
                while !request.ends_with(b"\r\n\r\n") {
                    stream.read_exact(&mut byte).unwrap();
                    request.push(byte[0]);
                }
                let request = String::from_utf8(request).unwrap();
                let body_length = request
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|length| length.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                stream.read_exact(&mut vec![0; body_length]).unwrap();
                let line = request.lines().next().unwrap().to_owned();
                let body = if line.starts_with("POST ") {
                    serde_json::json!({
                        "event_id": "event-new", "broadcaster_token": "fixture-secret",
                        "metadata_url": "/v1/liveitems/event-new/metadata", "events_url": "/events",
                    })
                    .to_string()
                } else {
                    serde_json::json!({
                        "event_id": "event-new", "seq": 1, "updated_at": "2026-09-09T00:00:00Z", "metadata": {},
                    }).to_string()
                };
                requests.push(line);
                write!(stream, "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
            requests
        });
        (endpoint, thread)
    }

    fn show_event_project(input: &EventSectionInput) -> EventSectionDisplay {
        let snapshot = BroadcastServiceWatchSnapshot {
            read_started_at: Instant::now(),
            at: Instant::now(),
            units: [
                PublisherServiceRole::Producer,
                PublisherServiceRole::Publisher,
            ]
            .into_iter()
            .map(|role| BroadcastServiceUnitSnapshot {
                role,
                host_name: "Local".to_owned(),
                unit_name: "fixture.service".to_owned(),
                state: ServiceState::Active,
            })
            .collect(),
            encoder: BroadcastEncoderSnapshot {
                server_name: "Not configured".to_owned(),
                configured: false,
                status: encoder::EncoderStatus::not_installed(),
            },
        };
        ShowPageVm::from_queue_publisher_readiness_and_event(
            QueueNowPlayingPageVm::builder().build(),
            Some(&snapshot),
            ShowLogPaneDisplay::closed(),
            None,
            Some(input),
        )
        .publisher
        .unwrap()
        .event
        .unwrap()
    }

    fn show_event_execute(
        command: EventRegistryCommand,
        input: &mut EventSectionInput,
    ) -> Option<EventRegistryAction> {
        let action = command.action;
        assert!(!show_event_project(input)
            .actions
            .registry_action(action)
            .disabled());
        begin_event_registry_action(input, action);
        let working = show_event_project(input);
        assert!(
            working.actions.create.disabled()
                && working.actions.replace.disabled()
                && working.actions.check.disabled()
        );
        let result = command
            .execute(&CommandContext::next())
            .map(|outcome| outcome.into_parts().0);
        apply_event_registry_result(input, action, result)
    }

    /// Situational ADR 0059: Replace preserves the dead registry entry and its token.
    fn seed_dead_recovery_event(
        conn: &Connection,
        directory: &Path,
        endpoint: &str,
    ) -> db::BroadcastEventRow {
        let token_path = directory.join("old.token");
        crate::broadcast::tokens::write_token_file(&token_path, "old-secret").unwrap();
        db::insert_broadcast_event(
            conn,
            &db::BroadcastEventInput {
                event_id: "event-old".to_owned(),
                label: None,
                endpoint: endpoint.to_owned(),
                token_path: token_path.to_string_lossy().into_owned(),
                created_at: 1,
                last_checked_at: Some(2),
                last_status: Some(db::BroadcastEventStatus::Dead),
            },
        )
        .unwrap();
        db::broadcast_event_by_event_id(conn, "event-old")
            .unwrap()
            .unwrap()
    }

    /// Situational ADR 0059: registration saves a private token and reports the new identity.
    fn assert_event_registration(input: &EventSectionInput) -> EventSelectionInput {
        let registered = input.selected_event.clone().unwrap();
        assert_eq!(registered.event_id, "event-new");
        assert_eq!(registered.state, EventState::Unknown);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                fs::metadata(&registered.token_path)
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        let row = show_event_project(input);
        assert_eq!(row.event.event_id.as_deref(), Some("event-new"));
        assert_eq!(row.event.token_path.as_ref(), Some(&registered.token_path));
        assert!(row.registration_message.unwrap().contains("registered"));
        registered
    }

    /// Situational ADR 0059: queue refresh preserves recovery state and never exposes token contents.
    fn assert_recovered_event_refresh(
        input: &mut EventSectionInput,
        expected: EventState,
        registered: &EventSelectionInput,
    ) {
        // The independent target read supplies current configuration; queue refresh preserves it.
        input.targets = EventTargetListInput::Loaded { targets: vec![] };
        let mut refreshed = input.clone();
        refreshed.feedback = EventCommandFeedback::default();
        refreshed.targets = EventTargetListInput::Loaded { targets: vec![] };
        let requested = Some(input.clone());
        let mut current = requested.clone();
        apply_event_refresh(&mut current, requested.as_ref(), Some(refreshed));
        let row = show_event_project(current.as_ref().unwrap());
        assert_eq!(row.event.state.state, expected);
        assert_eq!(!row.actions.attach.disabled(), expected == EventState::Live);
        assert_eq!(
            !row.actions.replace.disabled(),
            expected == EventState::Dead
        );
        assert!(!row.actions.check.disabled());
        assert_eq!(row.event.event_id.as_deref(), Some("event-new"));
        assert_eq!(row.event.token_path.as_ref(), Some(&registered.token_path));
        assert!(!format!("{row:?}").contains("fixture-secret"));
    }

    /// Situational ADR 0059: a failed check preserves registration success and offers a retry.
    fn assert_event_check_retry(input: &EventSectionInput) {
        let failed = show_event_project(input);
        assert_eq!(input.feedback.check_response, EventCheckResponse::Http(503));
        assert!(failed
            .diagnostics
            .contains("answered HTTP 503 for event event-new"));
        assert!(failed.diagnostics.contains("App created event event-new"));
        assert!(failed
            .diagnostics
            .contains("App kept the saved status of event event-new as Unknown"));
        assert!(failed.registration_message.unwrap().contains("registered"));
        assert!(failed.check_message.unwrap().contains("failed"));
        assert_eq!(failed.actions.check.label, "Retry check");
        assert!(!failed.actions.check.disabled());
        assert!(
            failed.actions.create.disabled()
                && failed.actions.replace.disabled()
                && failed.actions.attach.disabled()
        );
    }

    /// Situational ADR 0059: run real registry commands and the mounted-row presenter transitions.
    fn show_event_recovery_sequence(action: EventRegistryAction, retry_status: u16) {
        let temp = tempfile::tempdir().unwrap();
        let conn = Arc::new(Mutex::new(show_event_db(&temp)));
        let (endpoint, relay) = show_event_relay(vec![200, 503, retry_status]);
        let cfg_path = temp.path().join("config.toml");
        let config_before = format!(
            "musicindex_endpoint = {endpoint:?}\n[broadcast]\ndrop_file_target = \"default\"\n"
        );
        fs::write(&cfg_path, &config_before).unwrap();
        // A sentinel publisher configuration must survive registration/checking byte for byte.
        let publisher_path = temp.path().join("publisher-config.toml");
        fs::write(&publisher_path, "targets = []\n").unwrap();
        let old = if action == EventRegistryAction::Replace {
            Some(seed_dead_recovery_event(
                &conn.lock().unwrap(),
                temp.path(),
                &endpoint,
            ))
        } else {
            None
        };
        let mut input = EventSectionInput {
            liveness_confirmed: true,
            registry: EventRegistryInput::default(),
            context: String::new(),
            request: 0,
            feedback: EventCommandFeedback::default(),
            selected_event: selected_event_input(&conn.lock().unwrap()).unwrap(),
            targets: EventTargetListInput::Loaded { targets: vec![] },
            attach_target_name: "default".to_owned(),
            remote_host: false,
        };
        let selection = db::broadcast_event_selection(&conn.lock().unwrap()).unwrap();
        input.registry.selected_id = selection.event_id;
        input.registry.revision = selection.revision;
        let command = |action, input: &EventSectionInput| EventRegistryCommand {
            selection_revision: input.registry.revision,
            conn: Arc::clone(&conn),
            cfg_path: cfg_path.clone(),
            action,
            selected_event: input.selected_event.clone(),
        };
        let before_registration = Some(input.clone());
        let next = show_event_execute(command(action, &input), &mut input);
        assert_eq!(next, Some(EventRegistryAction::Check));
        let registered = assert_event_registration(&input);
        let token_before = fs::read(&registered.token_path).unwrap();
        // A refresh begun before registration must not erase the new identity or feedback.
        let mut current = Some(input.clone());
        apply_event_refresh(
            &mut current,
            before_registration.as_ref(),
            before_registration.clone(),
        );
        assert_eq!(current, Some(input.clone()));
        show_event_execute(command(next.unwrap(), &input), &mut input);
        assert_eq!(input.selected_event.as_ref(), Some(&registered));
        assert_event_check_retry(&input);
        show_event_execute(command(EventRegistryAction::Check, &input), &mut input);
        let stored = selected_event_input(&conn.lock().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(stored.event_id, registered.event_id);
        assert_eq!(stored.token_path, registered.token_path);
        assert_eq!(fs::read(&registered.token_path).unwrap(), token_before);
        let expected = match retry_status {
            200 => EventState::Live,
            404 => EventState::Dead,
            _ => EventState::Unknown,
        };
        assert_eq!(stored.state, expected);
        assert_eq!(input.selected_event.as_ref(), Some(&stored));
        assert_recovered_event_refresh(&mut input, expected, &registered);
        assert_eq!(fs::read_to_string(&cfg_path).unwrap(), config_before);
        assert_eq!(
            fs::read_to_string(&publisher_path).unwrap(),
            "targets = []\n"
        );
        let locked = conn.lock().unwrap();
        assert_eq!(
            db::broadcast_events(&locked).unwrap().len(),
            if old.is_some() { 2 } else { 1 }
        );
        assert_eq!(
            db::broadcast_event_by_event_id(&locked, "event-old").unwrap(),
            old
        );
        if let Some(old) = old {
            assert_eq!(fs::read_to_string(old.token_path).unwrap(), "old-secret");
        }
        assert_eq!(
            relay.join().unwrap(),
            [
                "POST /v1/liveitems HTTP/1.1",
                "GET /v1/liveitems/event-new/metadata HTTP/1.1",
                "GET /v1/liveitems/event-new/metadata HTTP/1.1",
            ]
        );
    }

    #[test]
    fn show_event_create_initial_failure_then_retry_live() {
        show_event_recovery_sequence(EventRegistryAction::Create, 200);
    }
    #[test]
    fn show_event_replace_initial_failure_then_retry_live() {
        show_event_recovery_sequence(EventRegistryAction::Replace, 200);
    }
    #[test]
    fn show_event_create_failed_retry_remains_retryable() {
        show_event_recovery_sequence(EventRegistryAction::Create, 503);
    }
    #[test]
    fn show_event_replace_failed_retry_remains_retryable() {
        show_event_recovery_sequence(EventRegistryAction::Replace, 503);
    }
    #[test]
    fn show_event_create_retry_404_stores_dead_before_offering_replace() {
        show_event_recovery_sequence(EventRegistryAction::Create, 404);
    }
    #[test]
    fn show_event_replace_retry_404_stores_dead_before_offering_replace() {
        show_event_recovery_sequence(EventRegistryAction::Replace, 404);
    }
    /// Situational ADR 0059: an observed Live response is not success until its status is stored.
    #[test]
    fn show_event_status_write_failure_retains_unknown_and_retry() {
        let temp = tempfile::tempdir().unwrap();
        let conn = Arc::new(Mutex::new(show_event_db(&temp)));
        let (endpoint, relay) = show_event_relay(vec![200, 200, 200]);
        let cfg_path = temp.path().join("config.toml");
        fs::write(&cfg_path, format!("musicindex_endpoint = {endpoint:?}\n")).unwrap();
        let mut input = EventSectionInput {
            liveness_confirmed: true,
            registry: EventRegistryInput::default(),
            context: String::new(),
            request: 0,
            feedback: EventCommandFeedback::default(),
            selected_event: None,
            targets: EventTargetListInput::Unknown,
            attach_target_name: "default".to_owned(),
            remote_host: false,
        };
        let selection = db::broadcast_event_selection(&conn.lock().unwrap()).unwrap();
        input.registry.selected_id = selection.event_id;
        input.registry.revision = selection.revision;
        let command = |action, input: &EventSectionInput| EventRegistryCommand {
            selection_revision: input.registry.revision,
            conn: Arc::clone(&conn),
            cfg_path: cfg_path.clone(),
            action,
            selected_event: input.selected_event.clone(),
        };
        assert_eq!(
            show_event_execute(command(EventRegistryAction::Create, &input), &mut input),
            Some(EventRegistryAction::Check)
        );
        let registered = input.selected_event.clone();
        conn.lock().unwrap().execute_batch("CREATE TRIGGER reject_event_check BEFORE UPDATE ON broadcast_events BEGIN SELECT RAISE(FAIL, 'status write failed'); END;").unwrap();
        show_event_execute(command(EventRegistryAction::Check, &input), &mut input);
        assert_eq!(input.selected_event, registered);
        assert_eq!(
            selected_event_input(&conn.lock().unwrap()).unwrap(),
            registered
        );
        let row = show_event_project(&input);
        assert!(row.check_message.unwrap().contains("status write failed"));
        assert_eq!(
            input.feedback.check_response,
            EventCheckResponse::SaveFailed(EventState::Live)
        );
        assert!(row
            .diagnostics
            .contains("returned metadata for event event-new"));
        assert!(row
            .diagnostics
            .contains("could not finish saving that answer"));
        assert!(!row.diagnostics.contains("received no HTTP response"));
        assert_eq!(row.actions.check.label, "Retry check");
        assert!(!row.actions.check.disabled());
        assert!(row.actions.replace.disabled() && row.actions.attach.disabled());
        conn.lock()
            .unwrap()
            .execute_batch("DROP TRIGGER reject_event_check;")
            .unwrap();
        show_event_execute(command(EventRegistryAction::Check, &input), &mut input);
        assert_eq!(
            show_event_project(&input).event.state.state,
            EventState::Live
        );
        assert_eq!(relay.join().unwrap().len(), 3);
    }

    /// Situational ADR 0059: an in-flight registry command owns its row until completion.
    #[test]
    fn show_event_refresh_cannot_clear_progress_or_failure_feedback() {
        let mut input = EventSectionInput {
            liveness_confirmed: true,
            registry: EventRegistryInput::default(),
            context: String::new(),
            request: 0,
            feedback: EventCommandFeedback::default(),
            selected_event: None,
            targets: EventTargetListInput::Unknown,
            attach_target_name: "default".to_owned(),
            remote_host: false,
        };
        begin_event_registry_action(&mut input, EventRegistryAction::Create);
        let requested = Some(input.clone());
        let mut current = requested.clone();
        let mut refreshed = input.clone();
        refreshed.feedback = EventCommandFeedback::default();
        apply_event_refresh(&mut current, requested.as_ref(), Some(refreshed));
        assert_eq!(current, requested);
        apply_event_registry_result(
            &mut input,
            EventRegistryAction::Create,
            Err(CommandError::Other("registration rejected".to_owned())),
        );
        let row = show_event_project(&input);
        assert!(row
            .registration_message
            .unwrap()
            .contains("registration rejected"));
        assert!(!row.actions.create.disabled());
        assert!(row.actions.replace.disabled() && row.actions.check.disabled());
        assert_eq!(row.event.state.state, EventState::None);
    }
    fn stored_test_event(
        conn: &Connection,
        directory: &Path,
        id: &str,
        endpoint: &str,
        created: i64,
    ) {
        let token = directory.join(format!("{id}.token"));
        crate::broadcast::tokens::write_token_file(&token, "preserve-secret").unwrap();
        db::insert_broadcast_event(
            conn,
            &db::BroadcastEventInput {
                event_id: id.to_owned(),
                label: None,
                endpoint: endpoint.to_owned(),
                token_path: token.to_string_lossy().into_owned(),
                created_at: created,
                last_checked_at: Some(created),
                last_status: Some(db::BroadcastEventStatus::Dead),
            },
        )
        .unwrap();
    }

    /// Situational ADR 0059: chosen older rows remain operable; changing revision rejects stale commands.
    #[test]
    fn compact_event_saved_choice_drives_commands_and_refresh() {
        let temp = tempfile::tempdir().unwrap();
        let conn = Arc::new(Mutex::new(show_event_db(&temp)));
        let (endpoint, relay) = show_event_relay(vec![404, 200]);
        let cfg_path = temp.path().join("config.toml");
        fs::write(&cfg_path, format!("musicindex_endpoint = {endpoint:?}\n")).unwrap();
        {
            let conn = conn.lock().unwrap();
            stored_test_event(&conn, temp.path(), "older", &endpoint, 1);
            db::select_broadcast_event(&conn, "older").unwrap();
            stored_test_event(&conn, temp.path(), "newer", &endpoint, 2);
        }
        let mut input =
            load_event_section(&conn.lock().unwrap(), &config::BroadcastConfig::default());
        assert_eq!(input.registry.events[0].event_id, "newer");
        assert_eq!(input.selected_event.as_ref().unwrap().event_id, "older");
        for action in [EventRegistryAction::Check, EventRegistryAction::Replace] {
            let command = EventRegistryCommand {
                conn: Arc::clone(&conn),
                cfg_path: cfg_path.clone(),
                action,
                selected_event: input.selected_event.clone(),
                selection_revision: input.registry.revision,
            };
            let result = command
                .execute(&CommandContext::next())
                .unwrap()
                .into_parts()
                .0;
            apply_event_registry_result(&mut input, action, Ok(result));
        }
        assert_eq!(input.selected_event.as_ref().unwrap().event_id, "event-new");
        assert_eq!(
            fs::read_to_string(temp.path().join("older.token")).unwrap(),
            "preserve-secret"
        );
        assert_eq!(
            fs::read_to_string(temp.path().join("newer.token")).unwrap(),
            "preserve-secret"
        );
        let stale = EventRegistryCommand {
            conn: Arc::clone(&conn),
            cfg_path,
            action: EventRegistryAction::Check,
            selected_event: input.selected_event.clone(),
            selection_revision: input.registry.revision,
        };
        db::select_broadcast_event(&conn.lock().unwrap(), "older").unwrap();
        assert!(stale.execute(&CommandContext::next()).is_err());
        assert_eq!(
            relay.join().unwrap(),
            [
                "GET /v1/liveitems/older/metadata HTTP/1.1",
                "POST /v1/liveitems HTTP/1.1"
            ]
        );
        assert_eq!(
            selected_event_input(&conn.lock().unwrap())
                .unwrap()
                .unwrap()
                .event_id,
            "older"
        );
    }

    /// Situational ADR 0059: failed preference storage cannot erase successful registration or repeat POST.
    #[test]
    fn compact_event_registration_survives_selection_save_failure() {
        let temp = tempfile::tempdir().unwrap();
        let conn = Arc::new(Mutex::new(show_event_db(&temp)));
        let (endpoint, relay) = show_event_relay(vec![200]);
        let cfg_path = temp.path().join("config.toml");
        fs::write(&cfg_path, format!("musicindex_endpoint = {endpoint:?}\n")).unwrap();
        conn.lock().unwrap().execute_batch("CREATE TRIGGER reject_choice BEFORE INSERT ON broadcast_event_selection BEGIN SELECT RAISE(FAIL, 'choice disk failure'); END;").unwrap();
        let mut input =
            load_event_section(&conn.lock().unwrap(), &config::BroadcastConfig::default());
        let command = EventRegistryCommand {
            conn: Arc::clone(&conn),
            cfg_path,
            action: EventRegistryAction::Create,
            selected_event: None,
            selection_revision: 0,
        };
        let result = command
            .execute(&CommandContext::next())
            .unwrap()
            .into_parts()
            .0;
        let EventRegistryResult::Complete(ref success) = result else {
            panic!("registration must return its stored event");
        };
        let token_path = success.event.token_path.clone();
        assert_eq!(
            apply_event_registry_result(&mut input, EventRegistryAction::Create, Ok(result)),
            None
        );
        assert_eq!(input.feedback.registration, EventCommandState::Succeeded);
        assert!(matches!(
            input.feedback.selection,
            EventCommandState::Failed { .. }
        ));
        assert_eq!(input.registry.events.len(), 1);
        assert!(show_event_project(&input).actions.create.disabled());
        assert_eq!(fs::read_to_string(token_path).unwrap(), "fixture-secret");
        assert_eq!(relay.join().unwrap().len(), 1);
    }

    /// Situational ADR 0059: a missing saved entry and a failed registry read never offer Create.
    #[test]
    fn compact_event_missing_choice_and_registry_errors_are_explicit() {
        let temp = tempfile::tempdir().unwrap();
        let conn = show_event_db(&temp);
        stored_test_event(&conn, temp.path(), "old", "http://127.0.0.1:1", 1);
        db::select_broadcast_event(&conn, "old").unwrap();
        stored_test_event(&conn, temp.path(), "new", "http://127.0.0.1:1", 2);
        let old = db::broadcast_event_by_event_id(&conn, "old")
            .unwrap()
            .unwrap();
        db::delete_broadcast_event(&conn, old.id).unwrap();
        let input = load_event_section(&conn, &config::BroadcastConfig::default());
        assert!(input.selected_event.is_none());
        assert_eq!(input.registry.selected_id.as_deref(), Some("old"));
        assert_eq!(show_event_project(&input).badge.label, "Event unavailable");
        assert!(show_event_project(&input).actions.create.disabled());
        conn.execute_batch("DROP TABLE broadcast_events").unwrap();
        let input = load_event_section(&conn, &config::BroadcastConfig::default());
        assert!(matches!(
            input.registry.status,
            EventRegistryStatus::Failed(_)
        ));
        let event = show_event_project(&input);
        assert_eq!(event.badge.label, "Events unavailable");
        assert!(event.actions.create.disabled());
        assert_eq!(event.primary.unwrap().intent, EventControlIntent::Refresh);
    }

    /// Situational ADR 0059: refreshes cannot resurrect old confirmation or another context's results.
    #[test]
    fn compact_event_context_revision_and_configured_detach_are_scoped() {
        let mut input = empty_event_section(&config::BroadcastConfig::default());
        input.context = "host / instance / default".to_owned();
        input.request = 7;
        input.registry.status = EventRegistryStatus::Loaded;
        input.registry.selected_id = Some("a".to_owned());
        input.registry.revision = 3;
        input.selected_event = Some(EventSelectionInput {
            created_at: 1,
            last_checked_at: Some(2),
            label: None,
            event_id: "a".to_owned(),
            endpoint: "http://localhost".to_owned(),
            token_path: "/fixture/token".to_owned(),
            state: EventState::Dead,
            token_file_missing: false,
        });
        input.attach_target_name = "default".to_owned();
        input.targets = EventTargetListInput::Loaded {
            targets: vec![EventTargetInput {
                name: "unused".to_owned(),
                event_id: "a".to_owned(),
            }],
        };
        assert!(attached_target_name(&input).is_none());
        let EventTargetListInput::Loaded { targets } = &mut input.targets else {
            unreachable!()
        };
        targets.push(EventTargetInput {
            name: "default".to_owned(),
            event_id: "a".to_owned(),
        });
        assert_eq!(
            event_target_name_for_operation(EventTargetOperation::Detach, &input).unwrap(),
            "default"
        );
        let requested = Some(input.clone());
        input.request += 1;
        let mut current = Some(input.clone());
        assert!(!apply_event_refresh(
            &mut current,
            requested.as_ref(),
            requested.clone()
        ));
        assert_eq!(current.as_ref().unwrap().request, 8);
        let mut new_context = input.clone();
        new_context.context = "another host".to_owned();
        new_context.feedback = EventCommandFeedback::default();
        new_context.targets = EventTargetListInput::Unknown;
        input.feedback.check = EventCommandState::Working;
        current = Some(input.clone());
        apply_event_refresh(&mut current, Some(&input), Some(new_context.clone()));
        assert_eq!(current, Some(new_context));
    }
}

/// Session-only latest results; no token contents or persistent audit trail (ADR 0059).
#[derive(Default)]
pub(super) struct EventSession {
    next_request: u64,
    feedback: HashMap<(String, String), EventCommandFeedback>,
}

impl EventSession {
    fn remember(&mut self, input: &EventSectionInput) {
        if let Some(event) = &input.selected_event {
            self.feedback.insert(
                (input.context.clone(), event.event_id.clone()),
                input.feedback.clone(),
            );
        }
    }
}

impl TopApp {
    fn event_request_is_current(&self, request: u64, context: &str) -> bool {
        context == event_context(&self.broadcast)
            && self
                .event_section_input
                .as_ref()
                .is_some_and(|input| input.request == request && input.context == context)
    }

    fn run_event_control(&mut self, intent: EventControlIntent, cx: &mut Context<Self>) {
        match intent {
            EventControlIntent::Create => {
                self.run_event_registry_command(EventRegistryAction::Create, cx);
            }
            EventControlIntent::Replace => {
                self.run_event_registry_command(EventRegistryAction::Replace, cx);
            }
            EventControlIntent::Check => {
                self.run_event_registry_command(EventRegistryAction::Check, cx);
            }
            EventControlIntent::Attach => {
                self.run_event_target_command(EventTargetOperation::Attach, cx);
            }
            EventControlIntent::Detach => {
                self.run_event_target_command(EventTargetOperation::Detach, cx);
            }
            EventControlIntent::Refresh => self.refresh_show_page(cx),
            EventControlIntent::ReadTargets => {
                if self
                    .event_section_input
                    .as_ref()
                    .is_none_or(|input| input.feedback.working())
                {
                    return;
                }
                self.event_session.next_request += 1;
                if let Some(input) = self.event_section_input.as_mut() {
                    input.request = self.event_session.next_request;
                    input.feedback.active_action = Some(intent);
                }
                self.read_event_targets(self.event_session.next_request, cx);
            }
            EventControlIntent::CopyFeedTag => {}
        }
    }

    fn select_show_event(&mut self, event_id: String, cx: &mut Context<Self>) {
        let Some(input) = &mut self.event_section_input else {
            return;
        };
        if input.feedback.working()
            || !input
                .registry
                .events
                .iter()
                .any(|event| event.event_id == event_id)
        {
            return;
        }
        self.event_session.remember(input);
        self.event_session.next_request += 1;
        input.request = self.event_session.next_request;
        input.feedback.selection = EventCommandState::Working;
        let requested_event = input
            .registry
            .events
            .iter()
            .find(|event| event.event_id == event_id)
            .cloned();
        input.record_event_report_for(
            EventReportOperation::Selection,
            chrono::Utc::now(),
            requested_event.clone(),
        );
        let request = input.request;
        let context = input.context.clone();
        let error_context = context.clone();
        let command = SelectShowEvent {
            conn: Arc::clone(&self.conn),
            broadcast: self.broadcast.clone(),
            event_id,
            expected_revision: input.registry.revision,
        };
        self.reproject_show_page_from_current_queue();
        cx.notify();
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, mut input, cx| {
                if !this.event_request_is_current(request, &context) {
                    return;
                }
                if let Some(event) = &input.selected_event {
                    input.feedback = this
                        .event_session
                        .feedback
                        .get(&(context.clone(), event.event_id.clone()))
                        .cloned()
                        .unwrap_or_default();
                }
                input.feedback.selection = EventCommandState::Succeeded;
                input.record_event_report(EventReportOperation::Selection, chrono::Utc::now());
                input.request = request;
                this.event_section_input = Some(input);
                this.reproject_show_page_from_current_queue();
                this.run_event_registry_command(EventRegistryAction::Check, cx);
            },
            move |this, error, cx| {
                if !this.event_request_is_current(request, &error_context) {
                    return;
                }
                if let Some(input) = &mut this.event_section_input {
                    input.feedback.selection = EventCommandState::Failed {
                        detail: error.to_string(),
                    };
                    input.record_event_report_for(
                        EventReportOperation::Selection,
                        chrono::Utc::now(),
                        requested_event.clone(),
                    );
                }
                this.reproject_show_page_from_current_queue();
                cx.notify();
            },
        );
    }

    fn read_event_targets(&mut self, request: u64, cx: &mut Context<Self>) {
        let Some(input) = &mut self.event_section_input else {
            return;
        };
        input.feedback.target_read = EventCommandState::Working;
        input.record_event_report(EventReportOperation::TargetRead, chrono::Utc::now());
        let context = input.context.clone();
        let error_context = context.clone();
        let command = ReadEventTargets {
            broadcast: self.broadcast.clone(),
        };
        self.reproject_show_page_from_current_queue();
        cx.notify();
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, targets, cx| {
                if !this.event_request_is_current(request, &context) {
                    return;
                }
                this.apply_target_read(targets, cx);
            },
            move |this, error, cx| {
                if !this.event_request_is_current(request, &error_context) {
                    return;
                }
                this.apply_target_read(
                    EventTargetListInput::Failed {
                        detail: error.to_string(),
                    },
                    cx,
                );
            },
        );
    }

    fn apply_target_read(&mut self, targets: EventTargetListInput, cx: &mut Context<Self>) {
        if let Some(input) = &mut self.event_section_input {
            input.feedback.target_read = match &targets {
                EventTargetListInput::Loaded { .. } => EventCommandState::Succeeded,
                EventTargetListInput::Failed { detail } => EventCommandState::Failed {
                    detail: detail.clone(),
                },
                EventTargetListInput::CommandsUnavailable => EventCommandState::Failed {
                    detail: "Publisher target commands unavailable".to_owned(),
                },
                EventTargetListInput::NotReachable => EventCommandState::Failed {
                    detail: "Publisher not reachable".to_owned(),
                },
                EventTargetListInput::Unknown => EventCommandState::Failed {
                    detail: "No target observation".to_owned(),
                },
            };
            input.targets = targets;
            input.record_event_report(EventReportOperation::TargetRead, chrono::Utc::now());
            if input.feedback.target_mutation == EventCommandState::Working {
                input.feedback.target_mutation = EventCommandState::Succeeded;
                input.record_event_report(EventReportOperation::TargetMutation, chrono::Utc::now());
            }
        }
        self.reproject_show_page_from_current_queue();
        cx.notify();
    }

    fn finish_event_target(
        &mut self,
        request: u64,
        context: &str,
        command_id: ShowCommandId,
        completion: ShowCommandCompletion,
        cx: &mut Context<Self>,
    ) {
        // Even a failed restart requires fresh service observation: the target write may have succeeded.
        if !self
            .show_commands
            .complete(command_id, completion.returned_at, true)
        {
            return;
        }
        self.invalidate_publisher_service_snapshot();
        if !self.event_request_is_current(request, context) {
            return;
        }
        if let Some(input) = &mut self.event_section_input {
            if let Err(error) = completion.result {
                input.feedback.target_mutation = EventCommandState::Failed {
                    detail: error.to_string(),
                };
                input.record_event_report(EventReportOperation::TargetMutation, chrono::Utc::now());
            }
        }
        self.read_event_targets(request, cx);
    }
}

struct SelectShowEvent {
    conn: Arc<Mutex<Connection>>,
    broadcast: config::BroadcastConfig,
    event_id: String,
    expected_revision: i64,
}

impl ApplicationCommand for SelectShowEvent {
    type Output = EventSectionInput;
    fn execute(self, context: &CommandContext) -> crate::application::CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let conn = self
            .conn
            .lock()
            .map_err(|_| event_registry_error("database lock poisoned"))?;
        let current = db::broadcast_event_selection(&conn).map_err(event_registry_error)?;
        if current.revision != self.expected_revision {
            return Err(event_registry_error(
                "Event choice changed; refresh the events list",
            ));
        }
        db::select_broadcast_event(&conn, &self.event_id).map_err(event_registry_error)?;
        Ok(CommandOutcome::without_events(load_event_section(
            &conn,
            &self.broadcast,
        )))
    }
}

struct ReadEventTargets {
    broadcast: config::BroadcastConfig,
}
impl ApplicationCommand for ReadEventTargets {
    type Output = EventTargetListInput;
    fn execute(self, context: &CommandContext) -> crate::application::CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        Ok(CommandOutcome::without_events(read_targets(
            &self.broadcast,
        )))
    }
}

fn event_input_identity(input: &EventSectionInput) -> (&str, Option<&str>, i64) {
    (
        &input.context,
        input.registry.selected_id.as_deref(),
        input.registry.revision,
    )
}
