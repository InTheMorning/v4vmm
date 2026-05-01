//! Command execution context.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

static NEXT_OPERATION_ID: AtomicU64 = AtomicU64::new(1);

/// Stable identifier for one application command operation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OperationId(u64);

impl OperationId {
    /// Creates an operation id from a caller-provided value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw operation id value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Correlates command work across logs and presentation bridges.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TraceId(u64);

impl TraceId {
    /// Creates a trace id from a caller-provided value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw trace id value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Shared cancellation state for long-running commands.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Creates a new non-cancelled token.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation for any command observing this token.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// Returns whether cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

/// Context passed to every command execution.
#[derive(Clone, Debug)]
pub struct CommandContext {
    operation_id: OperationId,
    cancellation: CancellationToken,
    trace_id: TraceId,
}

impl CommandContext {
    /// Creates a command context with generated operation and trace ids.
    #[must_use]
    pub fn next() -> Self {
        let id = NEXT_OPERATION_ID.fetch_add(1, Ordering::Relaxed);
        Self::new(
            OperationId::new(id),
            CancellationToken::new(),
            TraceId::new(id),
        )
    }

    /// Creates a command context with caller-provided identifiers.
    #[must_use]
    pub const fn new(
        operation_id: OperationId,
        cancellation: CancellationToken,
        trace_id: TraceId,
    ) -> Self {
        Self {
            operation_id,
            cancellation,
            trace_id,
        }
    }

    /// Returns the operation id for this command.
    #[must_use]
    pub const fn operation_id(&self) -> OperationId {
        self.operation_id
    }

    /// Returns the cancellation token for this command.
    #[must_use]
    pub const fn cancellation(&self) -> &CancellationToken {
        &self.cancellation
    }

    /// Returns the trace id for this command.
    #[must_use]
    pub const fn trace_id(&self) -> TraceId {
        self.trace_id
    }
}
