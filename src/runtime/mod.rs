//! Async view-model runtime (ADR 0040).
//!
//! This module owns the tokio-actor + `tokio::sync::watch` plumbing used by
//! screens to dispatch commands and observe view-model snapshots without
//! blocking the GPUI foreground executor (which is the same thread as the
//! renderer; see ADR 0040 § "GPUI scheduling reality").
//!
//! Layer rules (enforced by `tests/architecture_tests.rs` once the bridge
//! lands in Phase D step `gpui-vm-bridge`):
//!
//! * Modules under `src/runtime/` may import `tokio` / `tokio_util` but
//!   MUST NOT import `gpui`, `gpui_component`, or any module under
//!   `src/library`, `src/search`, `src/app`, `src/ui`. The runtime is
//!   GPUI-free so it stays testable on a plain tokio runtime.
//! * Only `src/presentation/` glue may bridge a runtime VM to a GPUI
//!   `Entity<T>`.
//!
//! Bring-up status: skeleton only. Phase D adds the GPUI bridge,
//! `AsyncCommandRunner`, and the first migrated VM.

#![warn(clippy::pedantic)]

pub mod actor;
pub mod paged_list_vm;
pub mod vm_bus;

pub use actor::{Actor, ActorHandle, Snapshot};
pub use paged_list_vm::{PageRequest, PagedListVm, Placeholder, RowSlot};
pub use vm_bus::{VmBus, VmEvent};
