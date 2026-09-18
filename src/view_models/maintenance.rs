//! Page navigation and layout contracts for repair and diagnostics (ADR 0074).

#![warn(clippy::pedantic)]

use super::startup::StartupAvailability;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum MaintenanceView {
    #[default]
    Instructions,
    Report,
}

impl MaintenanceView {
    pub(crate) fn toggle(self) -> PageChoice<Self> {
        let (value, label) = match self {
            Self::Instructions => (Self::Report, "Show report"),
            Self::Report => (Self::Instructions, "Show instructions"),
        };
        PageChoice {
            value,
            label,
            a11y_label: label.into(),
            selected: false,
            availability: StartupAvailability::Available,
        }
    }
}

#[derive(Clone)]
pub(crate) struct PageChoice<T> {
    pub(crate) value: T,
    pub(crate) label: &'static str,
    pub(crate) a11y_label: String,
    pub(crate) selected: bool,
    pub(crate) availability: StartupAvailability,
}

impl<T> PageChoice<T> {
    pub(crate) fn new(value: T, label: &'static str, selected: bool) -> Self {
        Self {
            value,
            label,
            a11y_label: format!("Open {label}{}", if selected { ", selected" } else { "" }),
            selected,
            availability: StartupAvailability::Available,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MaintenanceLayout {
    Columns,
    Stacked,
}

impl MaintenanceLayout {
    pub(crate) const fn for_width(width: f32) -> Self {
        if width < 760.0 {
            Self::Stacked
        } else {
            Self::Columns
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum RecoveryPage {
    #[default]
    Startup,
    Configuration,
    Database,
}

impl RecoveryPage {
    pub(crate) const MENU_LABEL: &'static str = "Choose recovery page";

    pub(crate) fn choices(self) -> Vec<PageChoice<Self>> {
        [
            (Self::Startup, "Startup"),
            (Self::Configuration, "Configuration"),
            (Self::Database, "Database"),
        ]
        .into_iter()
        .map(|(page, label)| PageChoice::new(page, label, self == page))
        .collect()
    }
}
