//! Show screen adapter.
//!
//! ADR 0060 moves queue and transport presentation into a screen mount. This
//! adapter binds existing playback projection and transport callbacks to the
//! Show shell without introducing a workspace frame.

use std::sync::{Arc, Mutex};

use gpui::Context;
use rusqlite::Connection;

use crate::application::{
    ApplicationCommand, ApplicationServices, CommandContext, CommandError, CommandOutcome,
};
use crate::broadcast::control::{self, UnitRef};
use crate::broadcast::encoder::{self, EncoderTarget};
use crate::config::BroadcastHostConfig;
use crate::presentation::present_command;
use crate::presentation::{bridge_watch, RuntimeHost};
use crate::runtime::{
    BroadcastEncoderWatchTarget, BroadcastServiceRole, BroadcastServiceWatchHandle,
    BroadcastServiceWatchSnapshot, BroadcastServiceWatchUnit,
};
use crate::ui::composites::{live_status_strip, LiveStatusStrip, LiveStatusStripSlots};
use crate::ui::shells::show::{render_show, ShowShell, ShowSlots};
use crate::view_models::live_status::LiveStatusDisplay;
use crate::view_models::queue_now_playing::QueueNowPlayingPageVm;
use crate::view_models::show::{
    PublisherLogPanelState, PublisherServiceRole, ShowPageVm, PUBLISHER_LOG_LINE_COUNT,
};

use super::queue_now_playing::{queue_now_playing_vm, queue_transport_action};
use super::{AppTab, TopApp, WorkspaceScreenMount};

const PRODUCER_UNIT: &str = "mixxx-now-playing.service";

pub(super) fn build_show_screen(app: &TopApp, cx: &mut Context<TopApp>) -> ShowShell {
    let entity = cx.entity();
    let service_entity = entity.clone();
    let stop_entity = entity.clone();
    let reset_entity = entity.clone();
    let logs_entity = entity.clone();
    let close_logs_entity = entity.clone();
    let connect_stream_entity = entity.clone();
    let disconnect_stream_entity = entity.clone();
    render_show(
        app.show_page.clone(),
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
        );
        present_command(
            &self.command_runner,
            command,
            CommandContext::next(),
            cx,
            |this, queue, _cx| {
                this.reproject_show_page(queue);
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

    fn reproject_show_page(&mut self, queue: QueueNowPlayingPageVm) {
        self.show_page = ShowPageVm::from_queue_and_publisher(
            queue,
            self.publisher_service_snapshot.as_ref(),
            self.publisher_log_panel.clone(),
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
}

struct RefreshShowPage {
    conn: Arc<Mutex<Connection>>,
    application_services: Arc<ApplicationServices>,
    queue_text_filter: Option<String>,
}

impl RefreshShowPage {
    fn new(
        conn: Arc<Mutex<Connection>>,
        application_services: Arc<ApplicationServices>,
        queue_text_filter: Option<String>,
    ) -> Self {
        Self {
            conn,
            application_services,
            queue_text_filter,
        }
    }
}

impl ApplicationCommand for RefreshShowPage {
    type Output = QueueNowPlayingPageVm;

    fn execute(self, context: &CommandContext) -> crate::application::CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }

        let conn = self
            .conn
            .lock()
            .map_err(|_| CommandError::Query("database lock poisoned".to_string()))?;
        Ok(CommandOutcome::without_events(queue_now_playing_vm(
            &self.application_services,
            &conn,
            self.queue_text_filter,
        )))
    }
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

fn start_broadcast_service_watch(
    host: &RuntimeHost,
    units: Vec<BroadcastServiceWatchUnit>,
    encoder: BroadcastEncoderWatchTarget,
) -> BroadcastServiceWatchHandle {
    let _enter = host.handle().enter();
    crate::runtime::broadcast_service_watch::start(units, encoder)
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
