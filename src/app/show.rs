//! Show screen adapter.
//!
//! ADR 0060 moves queue and transport presentation into a screen mount. This
//! adapter binds existing playback projection and transport callbacks to the
//! Show shell without introducing a workspace frame.
//! ADR 0059 places event registration and retryable liveness checks inside Live Metadata.

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
    EventCommandFeedback, EventCommandState, EventRegistryAction, EventSectionInput,
    EventSelectionInput, EventState, EventTargetInput, EventTargetListInput,
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
    let attach_event_entity = entity.clone();
    let detach_event_entity = entity.clone();
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
            .on_attach_event_target(move |_, _, cx| {
                attach_event_entity.update(cx, |this, cx| {
                    this.run_event_target_command(EventTargetOperation::Attach, cx);
                });
            })
            .on_detach_event_target(move |_, _, cx| {
                detach_event_entity.update(cx, |this, cx| {
                    this.run_event_target_command(EventTargetOperation::Detach, cx);
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
    let create_event_entity = entity.clone();
    let replace_event_entity = entity.clone();
    let check_event_entity = entity.clone();
    slots
        .on_create_event(move |_, _, cx| {
            create_event_entity.update(cx, |this, cx| {
                this.run_event_registry_command(EventRegistryAction::Create, cx);
            });
        })
        .on_replace_event(move |_, _, cx| {
            replace_event_entity.update(cx, |this, cx| {
                this.run_event_registry_command(EventRegistryAction::Replace, cx);
            });
        })
        .on_check_event(move |_, _, cx| {
            check_event_entity.update(cx, |this, cx| {
                this.run_event_registry_command(EventRegistryAction::Check, cx);
            });
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
            move |this, projection, _cx| {
                apply_event_refresh(
                    &mut this.event_section_input,
                    requested_event_input.as_ref(),
                    projection.event_section_input,
                );
                this.reproject_show_page(projection.queue);
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
        let command = EventRegistryCommand {
            conn: Arc::clone(&self.conn),
            cfg_path: self.cfg_path.clone(),
            action,
            selected_event: input.selected_event.clone(),
        };
        begin_event_registry_action(input, action);
        self.reproject_show_page_from_current_queue();
        cx.notify();
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            move |this, event, cx| {
                let Some(input) = this.event_section_input.as_mut() else {
                    return;
                };
                let next = apply_event_registry_result(input, action, Ok(event));
                this.reproject_show_page_from_current_queue();
                cx.notify();
                if let Some(next) = next {
                    this.run_event_registry_command(next, cx);
                } else {
                    this.refresh_show_page(cx);
                }
            },
            move |this, error, cx| {
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
        let command = EventTargetCommand {
            operation,
            transport: selected_host.transport,
            instance_name: selected_host.instance_name,
            target_name,
            event_id: event.event_id,
            token_path: event.token_path,
        };

        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            |this, (), cx| {
                this.settings_status.clear();
                this.invalidate_publisher_service_snapshot();
                this.refresh_show_page(cx);
            },
            |this, error, _cx| {
                this.settings_status = format!("Event target command error: {error:#}");
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
}

impl ApplicationCommand for EventRegistryCommand {
    type Output = EventSelectionInput;

    fn execute(self, context: &CommandContext) -> crate::application::CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let conn = self
            .conn
            .lock()
            .map_err(|_| CommandError::Query("database lock poisoned".to_owned()))?;
        // Revalidate stored identity and liveness before any relay mutation.
        let selected = selected_event_input(&conn).map_err(event_registry_error)?;
        if selected != self.selected_event {
            return Err(CommandError::Other(
                "Selected event changed; refresh Show before trying again.".to_owned(),
            ));
        }
        let allowed = match self.action {
            EventRegistryAction::Create => selected.is_none(),
            EventRegistryAction::Replace => selected
                .as_ref()
                .is_some_and(|event| event.state == EventState::Dead),
            EventRegistryAction::Check => selected
                .as_ref()
                .is_some_and(|event| event.state == EventState::Unknown),
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
                registry
                    .check_event(&event.event_id)
                    .map_err(event_registry_error)?
                    .event
            }
        };
        Ok(CommandOutcome::without_events(event_selection_input(event)))
    }
}

fn event_registry_error(error: impl std::fmt::Display) -> CommandError {
    CommandError::Other(format!("{error:#}"))
}

fn begin_event_registry_action(input: &mut EventSectionInput, action: EventRegistryAction) {
    match action {
        EventRegistryAction::Create | EventRegistryAction::Replace => {
            input.feedback = EventCommandFeedback {
                registration: EventCommandState::Working,
                check: EventCommandState::Idle,
            };
        }
        EventRegistryAction::Check => input.feedback.check = EventCommandState::Working,
    }
}

/// Applies the same mounted-row transition used by the presenter and command regression tests.
fn apply_event_registry_result(
    input: &mut EventSectionInput,
    action: EventRegistryAction,
    result: Result<EventSelectionInput, CommandError>,
) -> Option<EventRegistryAction> {
    match (action, result) {
        (EventRegistryAction::Create | EventRegistryAction::Replace, Ok(event)) => {
            input.selected_event = Some(event);
            // Targets are queried separately after the check; old identity's attachment is not reused.
            input.targets = EventTargetListInput::Unknown;
            input.feedback.registration = EventCommandState::Succeeded;
            Some(EventRegistryAction::Check)
        }
        (EventRegistryAction::Check, Ok(event)) => {
            input.selected_event = Some(event);
            input.feedback.check = EventCommandState::Succeeded;
            None
        }
        (action, Err(error)) => {
            let failure = EventCommandState::Failed {
                detail: error.to_string(),
            };
            match action {
                EventRegistryAction::Check => input.feedback.check = failure,
                EventRegistryAction::Create | EventRegistryAction::Replace => {
                    input.feedback.registration = failure;
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
) {
    if current.as_ref() != requested
        || current
            .as_ref()
            .is_some_and(|input| input.feedback.working())
    {
        return;
    }
    if let (Some(old), Some(new)) = (current.as_ref(), refreshed.as_mut()) {
        if old.selected_event.as_ref().map(|event| &event.event_id)
            == new.selected_event.as_ref().map(|event| &event.event_id)
        {
            new.feedback = old.feedback.clone();
        }
    }
    *current = refreshed;
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

        let (queue, selected_event) = {
            let conn = self
                .conn
                .lock()
                .map_err(|_| CommandError::Query("database lock poisoned".to_string()))?;
            let queue =
                queue_now_playing_vm(&self.application_services, &conn, self.queue_text_filter);
            let selected_event = selected_event_input(&conn)
                .map_err(|error| CommandError::Query(format!("{error:#}")))?;
            (queue, selected_event)
        };
        let event_section_input = event_section_input(selected_event, &self.broadcast)
            .map_err(|error| CommandError::Query(format!("{error:#}")))?;
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
    operation: EventTargetOperation,
    transport: Transport,
    instance_name: String,
    target_name: String,
    event_id: String,
    token_path: String,
}

impl ApplicationCommand for EventTargetCommand {
    type Output = ();

    fn execute(self, context: &CommandContext) -> crate::application::CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        match self.operation {
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
            event_target_command_error(format!(
                "{} event target: {error}",
                event_target_operation_label(self.operation)
            ))
        })?;

        Ok(CommandOutcome::without_events(()))
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
        .find(|target| target.event_id == event_id)
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
    let Some(event) = db::broadcast_events(conn)?.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(event_selection_input(event)))
}

fn event_selection_input(event: db::BroadcastEventRow) -> EventSelectionInput {
    EventSelectionInput {
        label: event.label,
        event_id: event.event_id,
        endpoint: event.endpoint,
        token_file_missing: token_file_missing(&event.token_path),
        token_path: event.token_path,
        state: event_state(event.last_status),
    }
}

fn event_section_input(
    selected_event: Option<EventSelectionInput>,
    broadcast: &crate::config::BroadcastConfig,
) -> anyhow::Result<Option<EventSectionInput>> {
    let attach_target_name = broadcast.drop_file_target.trim().to_owned();
    let Some(selected_event) = selected_event else {
        return Ok(Some(EventSectionInput {
            feedback: EventCommandFeedback::default(),
            selected_event: None,
            targets: EventTargetListInput::Unknown,
            attach_target_name,
            remote_host: false,
        }));
    };

    let host = selected_broadcast_host(broadcast)?;
    let targets = match publisher_targets::list_targets(&host.transport, &host.instance_name) {
        Ok(targets) => event_targets_loaded(targets),
        Err(PublisherTargetCommandError::CommandsUnavailable) => {
            EventTargetListInput::CommandsUnavailable
        }
        Err(PublisherTargetCommandError::NotReachable) => EventTargetListInput::NotReachable,
        Err(error) => EventTargetListInput::Failed {
            detail: error.to_string(),
        },
    };
    Ok(Some(EventSectionInput {
        feedback: EventCommandFeedback::default(),
        selected_event: Some(selected_event),
        targets,
        attach_target_name,
        remote_host: matches!(host.transport, Transport::Ssh { .. }),
    }))
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
            let token_path = temp.path().join("old.token");
            crate::broadcast::tokens::write_token_file(&token_path, "old-secret").unwrap();
            let locked = conn.lock().unwrap();
            db::insert_broadcast_event(
                &locked,
                &db::BroadcastEventInput {
                    event_id: "event-old".to_owned(),
                    label: None,
                    endpoint: endpoint.clone(),
                    token_path: token_path.to_string_lossy().into_owned(),
                    created_at: 1,
                    last_checked_at: Some(2),
                    last_status: Some(db::BroadcastEventStatus::Dead),
                },
            )
            .unwrap();
            db::broadcast_event_by_event_id(&locked, "event-old").unwrap()
        } else {
            None
        };
        let mut input = EventSectionInput {
            feedback: EventCommandFeedback::default(),
            selected_event: selected_event_input(&conn.lock().unwrap()).unwrap(),
            targets: EventTargetListInput::Loaded { targets: vec![] },
            attach_target_name: "default".to_owned(),
            remote_host: false,
        };
        let command = |action, input: &EventSectionInput| EventRegistryCommand {
            conn: Arc::clone(&conn),
            cfg_path: cfg_path.clone(),
            action,
            selected_event: input.selected_event.clone(),
        };
        let before_registration = Some(input.clone());
        let next = show_event_execute(command(action, &input), &mut input);
        assert_eq!(next, Some(EventRegistryAction::Check));
        let registered = input.selected_event.clone().unwrap();
        assert_eq!(registered.event_id, "event-new");
        assert_eq!(registered.state, EventState::Unknown);
        let token_before = fs::read(&registered.token_path).unwrap();
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
        let row = show_event_project(&input);
        assert_eq!(row.event.event_id.as_deref(), Some("event-new"));
        assert_eq!(row.event.token_path.as_ref(), Some(&registered.token_path));
        assert!(row.registration_message.unwrap().contains("registered"));
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
        let failed = show_event_project(&input);
        assert!(failed.registration_message.unwrap().contains("registered"));
        assert!(failed.check_message.unwrap().contains("failed"));
        assert_eq!(failed.actions.check.label, "Retry check");
        assert!(!failed.actions.check.disabled());
        assert!(
            failed.actions.create.disabled()
                && failed.actions.replace.disabled()
                && failed.actions.attach.disabled()
        );
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
        // Production uses this same refresh merger after an independent target-list read.
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
        assert_eq!(
            !row.actions.check.disabled(),
            expected == EventState::Unknown
        );
        assert_eq!(row.event.event_id.as_deref(), Some("event-new"));
        assert_eq!(row.event.token_path.as_ref(), Some(&registered.token_path));
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
        assert!(!format!("{row:?}").contains("fixture-secret"));
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
            feedback: EventCommandFeedback::default(),
            selected_event: None,
            targets: EventTargetListInput::Unknown,
            attach_target_name: "default".to_owned(),
            remote_host: false,
        };
        let command = |action, input: &EventSectionInput| EventRegistryCommand {
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
}
