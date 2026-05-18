//! Presentation adapter helpers.
//!
//! Presentation code may depend on UI runtimes such as GPUI. The application
//! layer must not depend on this module.

pub mod async_command_presenter;
pub mod event_bridge;
pub mod gpui_event_bridge;
pub mod gpui_vm_bridge;
pub mod runtime_host;

pub use async_command_presenter::present_command;
pub use event_bridge::PresentationEventBridge;
pub use gpui_event_bridge::GpuiEventBridge;
pub use gpui_vm_bridge::bridge_watch;
pub use runtime_host::RuntimeHost;
