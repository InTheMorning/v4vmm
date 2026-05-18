//! Lazy and deferred inspector panel state.

#![warn(clippy::pedantic)]

/// Deferred inspector panel state.
///
/// This remains generic and GPUI-free so screens can use the same state
/// contract for contributors, value routes, `MusicBrainz`, podroll, and tag
/// comparison panels while keeping fetch/render wiring outside the VM layer.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) enum LazyPanel<T> {
    #[default]
    Hidden,
    Loading,
    Empty(String),
    Loaded(T),
}

/// Result of toggling a deferred collapsible inspector panel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LazyPanelToggle {
    Fetch,
    Toggled,
}

impl LazyPanelToggle {
    #[must_use]
    pub(crate) fn should_fetch(self) -> bool {
        matches!(self, Self::Fetch)
    }

    #[must_use]
    pub(crate) fn should_notify(self) -> bool {
        matches!(self, Self::Fetch | Self::Toggled)
    }
}

impl<T> LazyPanel<T> {
    #[must_use]
    pub(crate) fn error(error: impl std::fmt::Display) -> Self {
        Self::Empty(format!("Error: {error}"))
    }

    pub(crate) fn begin_collapsible_toggle(
        &mut self,
        collapsed: &mut bool,
        force_toggle_only: bool,
    ) -> LazyPanelToggle {
        if force_toggle_only {
            *collapsed = !*collapsed;
            return LazyPanelToggle::Toggled;
        }

        match self {
            Self::Loaded(_) | Self::Empty(_) => {
                *collapsed = !*collapsed;
                LazyPanelToggle::Toggled
            }
            // Background prefetch may have moved the panel into `Loading`
            // before the user interacted with it. Treat the click as a
            // simple expand: the disclosure opens to reveal the loading
            // state and will swap to `Loaded`/`Empty` when the in-flight
            // fetch completes. We do not start a second fetch.
            Self::Loading => {
                *collapsed = false;
                LazyPanelToggle::Toggled
            }
            Self::Hidden => {
                *self = Self::Loading;
                *collapsed = false;
                LazyPanelToggle::Fetch
            }
        }
    }
}

impl<T> LazyPanel<Vec<T>> {
    pub(crate) fn from_items_result(
        result: Result<Vec<T>, impl std::fmt::Display>,
        empty_label: &str,
    ) -> Self {
        match result {
            Ok(items) if items.is_empty() => Self::Empty(empty_label.into()),
            Ok(items) => Self::Loaded(items),
            Err(error) => LazyPanel::error(error),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DeferredPanelKind {
    Contributors,
    ValueRoutes,
}

/// Static labels for a deferred inspector panel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DeferredPanelDisplay {
    pub(crate) section_id: &'static str,
    pub(crate) heading_label: &'static str,
    pub(crate) heading_a11y_label: &'static str,
    pub(crate) loading_label: &'static str,
    pub(crate) empty_label: &'static str,
}

impl DeferredPanelDisplay {
    #[must_use]
    pub(super) const fn for_kind(kind: DeferredPanelKind) -> Self {
        match kind {
            DeferredPanelKind::Contributors => Self {
                section_id: "section:contributors",
                heading_label: "Contributors",
                heading_a11y_label: "Toggle Contributors section",
                loading_label: "Loading contributors...",
                empty_label: "No contributors found",
            },
            DeferredPanelKind::ValueRoutes => Self {
                section_id: "section:value-routes",
                heading_label: "Value Routes",
                heading_a11y_label: "Toggle Value Routes section",
                loading_label: "Loading value routes...",
                empty_label: "No value routes found",
            },
        }
    }
}
