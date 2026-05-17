//! Frame chrome display contracts for workspace shells.

#![warn(clippy::pedantic)]

use super::{
    breadcrumb::BreadcrumbDisplay,
    frame::{WorkspaceFrameId, WorkspaceFrameState},
    nav::FrameNavigationState,
};

/// Content source filter for a frame-local content surface.
///
/// Invalid filter states are unrepresentable. Renderers map these variants to
/// frame-local controls, while workspace view-models pass the selected value
/// through without importing UI framework types.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ContentFilter {
    /// Show content from every available source.
    #[default]
    All,
    /// Show content already present in the local library.
    Library,
    /// Show content available from the remote index.
    Index,
}

impl ContentFilter {
    /// Returns the visible label for this filter.
    #[must_use]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Library => "Library",
            Self::Index => "Index",
        }
    }
}

/// Display contract for one frame-local filter chip option.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FilterChipOption {
    /// Filter selected when this option is activated.
    pub(crate) value: ContentFilter,
    /// Visible chip label.
    pub(crate) label: &'static str,
    /// Accessibility label for assistive technologies and tooltips.
    pub(crate) a11y_label: &'static str,
    /// Whether this option should render unavailable.
    pub(crate) disabled: bool,
}

/// Display contract for a frame-local filter chip strip.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FilterChipStripDisplay {
    /// Stable element identifier for the filter strip.
    pub(crate) id: String,
    /// Ordered filter options.
    pub(crate) options: Vec<FilterChipOption>,
    /// Currently selected filter.
    pub(crate) selected: ContentFilter,
    /// Whether narrow frame chrome should collapse chips into a pull-down.
    pub(crate) narrow_collapse_to_pulldown: bool,
}

impl FilterChipStripDisplay {
    /// Creates the default content-list filter strip display.
    #[must_use]
    pub(crate) fn default_for_content_list(
        selected: ContentFilter,
        narrow_collapse_to_pulldown: bool,
    ) -> Self {
        Self::with_standard_options(
            "workspace-content-list-filter",
            selected,
            narrow_collapse_to_pulldown,
        )
    }

    /// Creates the default search-inspector filter strip display.
    #[must_use]
    pub(crate) fn default_for_search_inspector(
        selected: ContentFilter,
        narrow_collapse_to_pulldown: bool,
    ) -> Self {
        Self::with_standard_options(
            "workspace-search-inspector-filter",
            selected,
            narrow_collapse_to_pulldown,
        )
    }

    fn with_standard_options(
        id: impl Into<String>,
        selected: ContentFilter,
        narrow_collapse_to_pulldown: bool,
    ) -> Self {
        Self {
            id: id.into(),
            options: Self::standard_options(),
            selected,
            narrow_collapse_to_pulldown,
        }
    }

    fn standard_options() -> Vec<FilterChipOption> {
        vec![
            FilterChipOption {
                value: ContentFilter::All,
                label: "All",
                a11y_label: "Show library and index content",
                disabled: false,
            },
            FilterChipOption {
                value: ContentFilter::Library,
                label: "Library",
                a11y_label: "Show library content only",
                disabled: false,
            },
            FilterChipOption {
                value: ContentFilter::Index,
                label: "Index",
                a11y_label: "Show index content only",
                disabled: false,
            },
        ]
    }
}

/// Display contract for one frame-chrome button.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrameChromeButtonDisplay {
    /// Stable element or command identifier for the button.
    pub(crate) id: String,
    /// Accessibility label for assistive technologies and tooltips.
    pub(crate) a11y_label: &'static str,
    /// Whether the command should render unavailable.
    pub(crate) disabled: bool,
}

impl FrameChromeButtonDisplay {
    /// Creates a display contract for a frame-chrome button.
    #[must_use]
    pub(crate) fn new(id: impl Into<String>, a11y_label: &'static str, disabled: bool) -> Self {
        Self {
            id: id.into(),
            a11y_label,
            disabled,
        }
    }
}

/// Display contract for one frame-chrome menu item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrameChromeMenuItemDisplay {
    /// Stable element or command identifier for the menu item.
    pub(crate) id: String,
    /// Visible menu label.
    pub(crate) label: &'static str,
    /// Accessibility label for the menu item.
    pub(crate) a11y_label: &'static str,
    /// Whether the command should render unavailable.
    pub(crate) disabled: bool,
}

/// Display contract consumed by the shared workspace frame shell.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrameShellDisplay {
    /// Frame identifier represented by this shell.
    pub(crate) frame_id: WorkspaceFrameId,
    /// Primary frame title.
    pub(crate) title: String,
    /// Optional secondary frame context.
    pub(crate) subtitle: Option<String>,
    /// Optional trailing status text.
    pub(crate) status: Option<String>,
    /// Back navigation command display.
    pub(crate) back: FrameChromeButtonDisplay,
    /// Forward navigation command display.
    pub(crate) forward: FrameChromeButtonDisplay,
    /// Optional close command display.
    pub(crate) close: Option<FrameChromeButtonDisplay>,
    /// Additional frame action menu items.
    pub(crate) action_menu_items: Vec<FrameChromeMenuItemDisplay>,
    /// Optional frame-local content filter strip.
    pub(crate) filter_chip_strip: Option<FilterChipStripDisplay>,
    /// Optional frame-local breadcrumb path.
    pub(crate) breadcrumb: Option<BreadcrumbDisplay>,
    /// Stable content slot identifier for mounting frame body content.
    pub(crate) content_slot_id: String,
}

impl FrameShellDisplay {
    const BACK_A11Y_LABEL: &'static str = "Navigate back in frame";
    const FORWARD_A11Y_LABEL: &'static str = "Navigate forward in frame";
    const CLOSE_A11Y_LABEL: &'static str = "Close frame";

    /// Projects frame state and navigation into shell chrome display data.
    #[must_use]
    pub(crate) fn from_frame(
        frame: &WorkspaceFrameState,
        nav: &FrameNavigationState,
        allow_close: bool,
    ) -> Self {
        let frame_id = frame.id();
        Self {
            frame_id,
            title: frame.title().to_string(),
            subtitle: frame.subtitle().map(ToOwned::to_owned),
            status: frame.status().map(ToOwned::to_owned),
            back: FrameChromeButtonDisplay::new(
                format!("workspace-frame-{}-back", frame_id.value()),
                Self::BACK_A11Y_LABEL,
                !nav.can_go_back(),
            ),
            forward: FrameChromeButtonDisplay::new(
                format!("workspace-frame-{}-forward", frame_id.value()),
                Self::FORWARD_A11Y_LABEL,
                !nav.can_go_forward(),
            ),
            close: allow_close.then(|| {
                FrameChromeButtonDisplay::new(
                    format!("workspace-frame-{}-close", frame_id.value()),
                    Self::CLOSE_A11Y_LABEL,
                    false,
                )
            }),
            action_menu_items: Vec::new(),
            filter_chip_strip: None,
            breadcrumb: None,
            content_slot_id: format!("workspace-frame-{}-content", frame_id.value()),
        }
    }

    /// Returns this shell display with frame-local filter chips attached.
    #[must_use]
    pub(crate) fn with_filter_chip_strip(mut self, display: FilterChipStripDisplay) -> Self {
        self.filter_chip_strip = Some(display);
        self
    }

    /// Returns this shell display with frame-local breadcrumbs attached.
    #[must_use]
    pub(crate) fn with_breadcrumb(mut self, display: BreadcrumbDisplay) -> Self {
        self.breadcrumb = Some(display);
        self
    }
}
