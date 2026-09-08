//! Show screen adapter.
//!
//! ADR 0060 moves queue and transport presentation into a screen mount. This
//! adapter binds existing playback projection and transport callbacks to the
//! Show shell without introducing a workspace frame.

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use gpui::Context;
use rusqlite::Connection;

use crate::application::{
    ApplicationCommand, ApplicationServices, CommandContext, CommandError, CommandOutcome,
};
use crate::broadcast::control::{self, UnitRef};
use crate::broadcast::encoder::{self, EncoderTarget};
use crate::broadcast::publisher_targets::{self, PublisherTargetCommandError, PublisherTargetList};
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
    EventSectionInput, EventSelectionInput, EventState, EventTargetInput, EventTargetListInput,
    PublisherLogPanelState, PublisherServiceRole, ShowPageVm, PUBLISHER_LOG_LINE_COUNT,
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
    let logs_entity = entity.clone();
    let close_logs_entity = entity.clone();
    let connect_stream_entity = entity.clone();
    let disconnect_stream_entity = entity.clone();
    let readiness_entity = entity.clone();
    let attach_event_entity = entity.clone();
    let detach_event_entity = entity.clone();
    render_show(
        app.show_page.clone().with_window_width(window_width),
        ShowSlots::new()
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
            .on_select_card(|_, _, _, _| {})
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
            |this, projection, _cx| {
                this.event_section_input = projection.event_section_input;
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
        self.show_page = ShowPageVm::from_queue_publisher_readiness_and_event(
            queue,
            self.publisher_service_snapshot.as_ref(),
            self.publisher_log_panel.clone(),
            self.broadcast_readiness_snapshot.as_ref(),
            self.event_section_input.as_ref(),
        );
    }

    fn reproject_show_page_from_current_queue(&mut self) {
        self.reproject_show_page(self.show_page.queue.clone());
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
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            |this, (), _cx| {
                this.settings_status.clear();
                this.invalidate_publisher_service_snapshot();
            },
            |this, error, _cx| {
                this.settings_status = format!("Publisher command error: {error:#}");
            },
        );
    }

    fn open_publisher_logs(&mut self, role: PublisherServiceRole, cx: &mut Context<Self>) {
        let selected_host = match selected_broadcast_host(&self.broadcast) {
            Ok(host) => host,
            Err(error) => {
                self.settings_status = format!("Publisher log error: {error:#}");
                cx.notify();
                return;
            }
        };
        let command = match ReadPublisherLogs::new(&selected_host, role) {
            Ok(command) => command,
            Err(error) => {
                self.settings_status = format!("Publisher log error: {error}");
                cx.notify();
                return;
            }
        };
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            |this, logs, _cx| {
                this.settings_status.clear();
                this.publisher_log_panel = PublisherLogPanelState::open(
                    logs.role,
                    logs.unit_name,
                    logs.line_count,
                    logs.text,
                );
                this.reproject_show_page_from_current_queue();
            },
            |this, error, _cx| {
                this.settings_status = format!("Publisher log error: {error:#}");
            },
        );
    }

    fn close_publisher_logs(&mut self, cx: &mut Context<Self>) {
        self.publisher_log_panel = PublisherLogPanelState::closed();
        self.reproject_show_page_from_current_queue();
        cx.notify();
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
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            |this, (), _cx| {
                this.settings_status.clear();
                this.invalidate_publisher_service_snapshot();
            },
            |this, error, _cx| {
                this.settings_status = format!("Stream command error: {error:#}");
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PublisherServiceOperation {
    Start,
    Stop,
    Reset,
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
    type Output = ();

    fn execute(self, context: &CommandContext) -> crate::application::CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        match self.operation {
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
        })?;

        Ok(CommandOutcome::without_events(()))
    }
}

struct ReadPublisherLogs {
    role: PublisherServiceRole,
    transport: crate::broadcast::transport::Transport,
    unit: UnitRef,
    line_count: usize,
}

impl ReadPublisherLogs {
    fn new(host: &BroadcastHostConfig, role: PublisherServiceRole) -> Result<Self, CommandError> {
        Ok(Self {
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
            role: self.role,
            unit_name: self.unit.unit().to_owned(),
            line_count: self.line_count,
            text,
        }))
    }
}

struct PublisherLogsResult {
    role: PublisherServiceRole,
    unit_name: String,
    line_count: usize,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StreamEncoderOperation {
    Connect,
    Disconnect,
}

struct StreamEncoderCommand {
    target: EncoderTarget,
    server_name: String,
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
            server_name: encoder.default_server_name.trim().to_owned(),
            operation,
        })
    }
}

impl ApplicationCommand for StreamEncoderCommand {
    type Output = ();

    fn execute(self, context: &CommandContext) -> crate::application::CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        match self.operation {
            StreamEncoderOperation::Connect => encoder::connect(&self.target, &self.server_name),
            StreamEncoderOperation::Disconnect => encoder::disconnect(&self.target),
        }
        .map_err(|error| {
            stream_command_error(format!(
                "{} stream encoder: {error:#}",
                stream_operation_label(self.operation)
            ))
        })?;

        Ok(CommandOutcome::without_events(()))
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
    Ok(Some(EventSelectionInput {
        label: event.label,
        event_id: event.event_id,
        endpoint: event.endpoint,
        token_file_missing: token_file_missing(&event.token_path),
        token_path: event.token_path,
        state: event_state(event.last_status),
    }))
}

fn event_section_input(
    selected_event: Option<EventSelectionInput>,
    broadcast: &crate::config::BroadcastConfig,
) -> anyhow::Result<Option<EventSectionInput>> {
    let attach_target_name = broadcast.drop_file_target.trim().to_owned();
    let Some(selected_event) = selected_event else {
        return Ok(Some(EventSectionInput {
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
        encoder.default_server_name.trim().to_owned(),
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
