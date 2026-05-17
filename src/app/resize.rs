//! Content-pane fluid resize handlers and state accessors.

use gpui::Context;

use crate::ui::layouts as layout;

use super::TopApp;

impl TopApp {
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
        cx.notify();
    }

    #[expect(dead_code, reason = "called via closure in render_workspace_content")]
    pub(super) fn is_content_pane_resizing(&self) -> bool {
        self.is_content_pane_resizing
    }
}
