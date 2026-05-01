//! Local read-model query boundary.

/// Reads local application snapshots for presentation and CLI adapters.
#[derive(Clone, Debug, Default)]
pub struct ApplicationQueryService;

impl ApplicationQueryService {
    /// Creates a local query service.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}
