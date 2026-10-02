//! Content-pane fluid resize handlers and state accessors.

use gpui::Context;

use crate::config;
use crate::ui::layouts as layout;
use crate::view_models::workspace::ContentViewMode;

use super::TopApp;

impl TopApp {
    pub(super) fn initial_content_pane_width(
        workspace_layout_prefs: Option<&config::WorkspaceLayoutPrefs>,
    ) -> gpui::Pixels {
        let width = workspace_layout_prefs
            .and_then(|prefs| prefs.content_pane_width)
            .unwrap_or(f32::from(layout::CONTENT_PANE_DEFAULT_WIDTH));
        gpui::px(Self::clamped_content_pane_width(width))
    }

    pub(super) fn initial_content_list_view_mode(
        workspace_layout_prefs: Option<&config::WorkspaceLayoutPrefs>,
    ) -> ContentViewMode {
        workspace_layout_prefs
            .and_then(|prefs| prefs.content_list_view_mode)
            .unwrap_or_default()
    }

    fn persist_content_pane_width(
        &self,
        content_list_view_mode: ContentViewMode,
    ) -> anyhow::Result<()> {
        config::save_workspace_layout_prefs(
            &self.cfg_path,
            &config::WorkspaceLayoutPrefs {
                content_pane_width: Some(f32::from(self.content_pane_width)),
                content_list_view_mode: Some(content_list_view_mode),
            },
        )
    }

    pub(super) fn persist_content_list_view_mode(
        &self,
        content_list_view_mode: ContentViewMode,
    ) -> anyhow::Result<()> {
        config::save_workspace_layout_prefs(
            &self.cfg_path,
            &config::WorkspaceLayoutPrefs {
                content_pane_width: Some(f32::from(self.content_pane_width)),
                content_list_view_mode: Some(content_list_view_mode),
            },
        )
    }

    fn clamped_content_pane_width(width: f32) -> f32 {
        width
            .max(f32::from(layout::CONTENT_PANE_MIN_WIDTH))
            .min(f32::from(layout::CONTENT_PANE_MAX_WIDTH))
    }

    pub(super) fn begin_content_pane_resize(&mut self, cx: &mut Context<Self>) {
        self.is_content_pane_resizing = true;
        cx.notify();
    }

    pub(super) fn resize_content_pane(&mut self, x: f32, cx: &mut Context<Self>) {
        if self.is_content_pane_resizing {
            let clamped = x
                .max(f32::from(layout::CONTENT_PANE_MIN_WIDTH))
                .min(f32::from(layout::CONTENT_PANE_MAX_WIDTH));
            self.content_pane_width = gpui::px(clamped);
            cx.notify();
        }
    }

    pub(super) fn end_content_pane_resize(&mut self, cx: &mut Context<Self>) {
        self.is_content_pane_resizing = false;
        let content_list_view_mode = self.library.read(cx).content_view_mode();
        if let Err(error) = self.persist_content_pane_width(content_list_view_mode) {
            self.settings_status = format!("Error: {error:#}");
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamped_content_pane_width_uses_default_bounds() {
        assert_eq!(
            TopApp::clamped_content_pane_width(f32::from(layout::CONTENT_PANE_MIN_WIDTH) - 10.0),
            f32::from(layout::CONTENT_PANE_MIN_WIDTH),
            "pane width should clamp to the minimum bound"
        );
        assert_eq!(
            TopApp::clamped_content_pane_width(f32::from(layout::CONTENT_PANE_MAX_WIDTH) + 10.0),
            f32::from(layout::CONTENT_PANE_MAX_WIDTH),
            "pane width should clamp to the maximum bound"
        );
        assert_eq!(
            TopApp::clamped_content_pane_width(900.0),
            900.0,
            "in-range pane width should be preserved"
        );
    }
}
