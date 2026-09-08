//! Frame chrome display contracts for workspace shells.

#![warn(clippy::pedantic)]

use serde::{Deserialize, Serialize};

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

/// Visual treatment projected for one library-membership filter state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LibraryFilterControlTreatment {
    /// Required library membership; render with selected emphasis.
    Highlighted,
    /// No library-membership constraint; render without selected emphasis.
    Off,
    /// Excluded library membership; render active text with strike-through.
    StruckThrough,
}

impl LibraryFilterControlTreatment {
    /// Returns whether this treatment needs strike-through text.
    #[must_use]
    pub(crate) const fn line_through(self) -> bool {
        matches!(self, Self::StruckThrough)
    }
}

/// Display data for one state in the library-membership tri-state control.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LibraryFilterControlStateDisplay {
    /// Filter value represented by this state.
    pub(crate) filter: ContentFilter,
    /// Visible axis label shown on the control face.
    pub(crate) text_label: &'static str,
    /// Text label naming the current state.
    pub(crate) state_label: &'static str,
    /// Accessibility label for assistive technologies and tooltips.
    pub(crate) a11y_label: &'static str,
    /// Non-color treatment for this state.
    pub(crate) treatment: LibraryFilterControlTreatment,
}

/// Display contract for one library-membership tri-state filter axis.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LibraryFilterControlDisplay {
    /// Stable element identifier for the control.
    pub(crate) id: String,
    /// Currently selected state.
    pub(crate) current: LibraryFilterControlStateDisplay,
    /// Filter selected by the next activation.
    pub(crate) next_filter: ContentFilter,
    /// Documented keyboard activation order for the three states.
    pub(crate) keyboard_cycle_order: [ContentFilter; 3],
}

impl LibraryFilterControlDisplay {
    const CONTENT_LIST_ID: &'static str = "workspace-content-list-library-filter";
    const KEYBOARD_CYCLE_ORDER: [ContentFilter; 3] = [
        ContentFilter::All,
        ContentFilter::Library,
        ContentFilter::Index,
    ];

    /// Creates the default content-list library-membership filter display.
    #[must_use]
    pub(crate) fn default_for_content_list(selected: ContentFilter) -> Self {
        Self {
            id: Self::CONTENT_LIST_ID.to_string(),
            current: Self::state_display(selected),
            next_filter: Self::next_filter(selected),
            keyboard_cycle_order: Self::KEYBOARD_CYCLE_ORDER,
        }
    }

    /// Returns every state in documented keyboard activation order.
    #[must_use]
    pub(crate) const fn state_displays() -> [LibraryFilterControlStateDisplay; 3] {
        [
            Self::state_display(ContentFilter::All),
            Self::state_display(ContentFilter::Library),
            Self::state_display(ContentFilter::Index),
        ]
    }

    const fn state_display(filter: ContentFilter) -> LibraryFilterControlStateDisplay {
        match filter {
            ContentFilter::All => LibraryFilterControlStateDisplay {
                filter,
                text_label: "Library",
                state_label: "Any",
                a11y_label: "Any library status",
                treatment: LibraryFilterControlTreatment::Off,
            },
            ContentFilter::Library => LibraryFilterControlStateDisplay {
                filter,
                text_label: "Library",
                state_label: "In library",
                a11y_label: "In library",
                treatment: LibraryFilterControlTreatment::Highlighted,
            },
            ContentFilter::Index => LibraryFilterControlStateDisplay {
                filter,
                text_label: "Library",
                state_label: "Not in library",
                a11y_label: "Not in library",
                treatment: LibraryFilterControlTreatment::StruckThrough,
            },
        }
    }

    const fn next_filter(filter: ContentFilter) -> ContentFilter {
        match filter {
            ContentFilter::All => ContentFilter::Library,
            ContentFilter::Library => ContentFilter::Index,
            ContentFilter::Index => ContentFilter::All,
        }
    }
}

/// Presentation mode for content rows.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ContentViewMode {
    /// Tiled artwork-first browser.
    #[default]
    Tiles,
    /// Compact row list.
    List,
}

impl ContentViewMode {
    /// Returns the visible segment label.
    #[must_use]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Tiles => "Tiles",
            Self::List => "List",
        }
    }

    /// Returns the stable segment id suffix.
    #[must_use]
    pub(crate) const fn id_suffix(self) -> &'static str {
        match self {
            Self::Tiles => "tiles",
            Self::List => "list",
        }
    }

    const fn content_list_a11y_label(self) -> &'static str {
        match self {
            Self::Tiles => "Show Music content as tiles",
            Self::List => "Show Music content as a list",
        }
    }
}

/// Display data for one content view-mode segment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ContentViewModeOptionDisplay {
    /// Presentation mode selected by the segment.
    pub(crate) mode: ContentViewMode,
    /// Stable segment identifier.
    pub(crate) id: &'static str,
    /// Visible segment label.
    pub(crate) label: &'static str,
    /// Accessibility label for assistive technologies and tooltips.
    pub(crate) a11y_label: &'static str,
}

/// Display contract for selecting a content view mode.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ContentViewModeControlDisplay {
    /// Stable control identifier.
    pub(crate) id: &'static str,
    /// Currently selected mode.
    pub(crate) selected: ContentViewMode,
    /// Ordered mode options.
    pub(crate) options: [ContentViewModeOptionDisplay; 2],
}

impl ContentViewModeControlDisplay {
    const CONTENT_LIST_ID: &'static str = "workspace-content-list-view-mode";
    const CONTENT_LIST_TILES_ID: &'static str = "workspace-content-list-view-mode-tiles";
    const CONTENT_LIST_LIST_ID: &'static str = "workspace-content-list-view-mode-list";

    /// Creates the default content-list view-mode display.
    #[must_use]
    pub(crate) const fn default_for_content_list(selected: ContentViewMode) -> Self {
        Self {
            id: Self::CONTENT_LIST_ID,
            selected,
            options: [
                Self::content_list_option(ContentViewMode::Tiles),
                Self::content_list_option(ContentViewMode::List),
            ],
        }
    }

    const fn content_list_option(mode: ContentViewMode) -> ContentViewModeOptionDisplay {
        ContentViewModeOptionDisplay {
            mode,
            id: match mode {
                ContentViewMode::Tiles => Self::CONTENT_LIST_TILES_ID,
                ContentViewMode::List => Self::CONTENT_LIST_LIST_ID,
            },
            label: mode.label(),
            a11y_label: mode.content_list_a11y_label(),
        }
    }
}

/// Width class for projecting frame-local filter chrome.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum FilterChipStripWidthClass {
    /// Normal frame width; render every chip directly.
    #[default]
    Normal,
    /// Narrow frame width; collapse chips into the pull-down control.
    Narrow,
}

impl FilterChipStripWidthClass {
    const fn narrow_collapse_to_pulldown(self) -> bool {
        matches!(self, Self::Narrow)
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

    /// Creates the default search-inspector filter display for a width class.
    #[must_use]
    pub(crate) fn default_for_search_inspector_width_class(
        selected: ContentFilter,
        width_class: FilterChipStripWidthClass,
    ) -> Self {
        Self::default_for_search_inspector(selected, width_class.narrow_collapse_to_pulldown())
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
    /// Optional frame-local library-membership filter control.
    pub(crate) library_filter_control: Option<LibraryFilterControlDisplay>,
    /// Optional frame-local content view-mode control.
    pub(crate) view_mode_control: Option<ContentViewModeControlDisplay>,
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
            library_filter_control: None,
            view_mode_control: None,
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

    /// Returns this shell display with frame-local library filter control attached.
    #[must_use]
    pub(crate) fn with_library_filter_control(
        mut self,
        display: LibraryFilterControlDisplay,
    ) -> Self {
        self.library_filter_control = Some(display);
        self
    }

    /// Returns this shell display with frame-local view-mode control attached.
    #[must_use]
    pub(crate) fn with_view_mode_control(mut self, display: ContentViewModeControlDisplay) -> Self {
        self.view_mode_control = Some(display);
        self
    }

    /// Returns this shell display with frame-local breadcrumbs attached.
    #[must_use]
    pub(crate) fn with_breadcrumb(mut self, display: BreadcrumbDisplay) -> Self {
        self.breadcrumb = Some(display);
        self
    }

    /// Returns whether the frame header row carries visible information.
    #[must_use]
    pub(crate) fn header_visible(&self) -> bool {
        !self.title.is_empty()
            || self.subtitle.is_some()
            || self.status.is_some()
            || !self.back.disabled
            || !self.forward.disabled
            || self.close.is_some()
            || !self.action_menu_items.is_empty()
    }
}
