//! Presentation adapter helpers.
//!
//! Presentation code may depend on UI runtimes such as GPUI. The application
//! layer must not depend on this module.

pub mod event_bridge;
pub mod gpui_command_runner;
pub mod gpui_event_bridge;
#[cfg(feature = "async-runtime")]
pub mod gpui_vm_bridge;
#[cfg(feature = "async-runtime")]
pub mod runtime_host;

pub use event_bridge::PresentationEventBridge;
pub use gpui_command_runner::GpuiCommandRunner;
pub use gpui_event_bridge::GpuiEventBridge;
#[cfg(feature = "async-runtime")]
pub use gpui_vm_bridge::bridge_watch;
#[cfg(feature = "async-runtime")]
pub use runtime_host::RuntimeHost;
