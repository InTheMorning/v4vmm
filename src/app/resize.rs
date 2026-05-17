//! Content-pane fluid resize handlers and state accessors.

use gpui::Context;

use crate::config;
use crate::ui::layouts as layout;

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

    fn persist_content_pane_width(&self) -> anyhow::Result<()> {
        config::save_workspace_layout_prefs(
            &self.cfg_path,
            &config::WorkspaceLayoutPrefs {
                content_pane_width: Some(f32::from(self.content_pane_width)),
            },
        )
    }

    fn clamped_content_pane_width(width: f32) -> f32 {
        width
            .max(f32::from(layout::CONTENT_PANE_MIN_WIDTH))
            .min(f32::from(layout::CONTENT_PANE_MAX_WIDTH))
    }

    #[expect(dead_code, reason = "called via closure in render_workspace_content")]
    pub(super) fn set_content_pane_width(&mut self, width: gpui::Pixels, cx: &mut Context<Self>) {
        self.content_pane_width = width;
        cx.notify();
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
        if let Err(error) = self.persist_content_pane_width() {
            self.settings_status = format!("Error: {error:#}");
        }
        cx.notify();
    }

    #[expect(dead_code, reason = "called via closure in render_workspace_content")]
    pub(super) fn is_content_pane_resizing(&self) -> bool {
        self.is_content_pane_resizing
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
