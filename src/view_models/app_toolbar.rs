//! App-toolbar display contracts.
//!
//! The app shell owns toolbar rendering and command wiring. This module owns
//! the stable ids, labels, and accessibility text so the shell does not grow
//! screen-local display policy.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AppToolbarTabKey {
    Library,
    Discover,
    Settings,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AppToolbarTabDisplay {
    pub(crate) key: AppToolbarTabKey,
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) a11y_label: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NowPlayingFrameDisplay {
    pub(crate) id: &'static str,
    pub(crate) a11y_label: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AppToolbarDisplay {
    pub(crate) id: &'static str,
    pub(crate) leading_id: &'static str,
    pub(crate) center_id: &'static str,
    pub(crate) mark_id: &'static str,
    pub(crate) mark_a11y_label: &'static str,
    pub(crate) tabs: [AppToolbarTabDisplay; 3],
    pub(crate) now_playing: NowPlayingFrameDisplay,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct AppToolbarVm;

impl AppToolbarVm {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self
    }

    #[must_use]
    pub(crate) const fn display(self) -> AppToolbarDisplay {
        let _ = self;
        AppToolbarDisplay {
            id: "app-toolbar",
            leading_id: "app-toolbar-leading",
            center_id: "app-toolbar-center",
            mark_id: "app-toolbar-mark",
            mark_a11y_label: "Application mark",
            tabs: [
                AppToolbarTabDisplay {
                    key: AppToolbarTabKey::Library,
                    id: "app-tab-library",
                    label: "Library",
                    a11y_label: "Show Library",
                },
                AppToolbarTabDisplay {
                    key: AppToolbarTabKey::Discover,
                    id: "app-tab-discover",
                    label: "Discover",
                    a11y_label: "Show Discover",
                },
                AppToolbarTabDisplay {
                    key: AppToolbarTabKey::Settings,
                    id: "app-tab-settings",
                    label: "Settings",
                    a11y_label: "Show Settings",
                },
            ],
            now_playing: NowPlayingFrameDisplay {
                id: "app-toolbar-now-playing",
                a11y_label: "Now Playing controls",
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbar_display_projects_stable_regions() {
        let display = AppToolbarVm::new().display();

        assert_eq!(display.id, "app-toolbar");
        assert_eq!(display.leading_id, "app-toolbar-leading");
        assert_eq!(display.center_id, "app-toolbar-center");
        assert_eq!(display.now_playing.id, "app-toolbar-now-playing");
    }

    #[test]
    fn toolbar_tabs_project_labels_and_a11y() {
        let display = AppToolbarVm::new().display();
        let labels: Vec<_> = display.tabs.iter().map(|tab| tab.label).collect();
        let a11y: Vec<_> = display.tabs.iter().map(|tab| tab.a11y_label).collect();

        assert_eq!(labels, ["Library", "Discover", "Settings"]);
        assert_eq!(a11y, ["Show Library", "Show Discover", "Show Settings"]);
    }

    #[test]
    fn toolbar_display_avoids_product_naming() {
        let display = AppToolbarVm::new().display();
        let strings = [
            display.mark_a11y_label,
            display.now_playing.a11y_label,
            display.tabs[0].label,
            display.tabs[1].label,
            display.tabs[2].label,
        ];

        assert!(
            strings
                .iter()
                .all(|value| !value.contains("MusicIndex") && !value.contains("V4V")),
            "top-level toolbar strings should remain neutral until naming is decided"
        );
    }
}
