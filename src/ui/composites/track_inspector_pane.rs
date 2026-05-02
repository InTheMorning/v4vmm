//! Shared track inspector pane shell.
//!
//! `src/ui_track.rs` currently owns legacy top-level inspector glue during the
//! ADR 0035 migration. This composite owns the reusable track-surface frame;
//! Task 005 drains the legacy shell once Library and Discover both route
//! through this pane.

#![warn(clippy::pedantic)]

use gpui::{div, App, IntoElement, ParentElement, RenderOnce, Styled, Window};

use crate::ui::tokens::Spacing;

use super::TrackDetailSurface;

#[derive(IntoElement)]
#[must_use]
pub struct TrackInspectorPane {
    surface: TrackDetailSurface,
}

impl TrackInspectorPane {
    pub const fn new(surface: TrackDetailSurface) -> Self {
        Self { surface }
    }
}

impl RenderOnce for TrackInspectorPane {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(Spacing::LG.scaled(cx))
            .child(self.surface)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_models::track_detail::{TrackDetailSurfaceContext, TrackDetailVm};
    use crate::views::TrackView;

    #[test]
    fn pane_wraps_surface() {
        let track = TrackView::default();
        let vm = TrackDetailVm::new(&track, TrackDetailSurfaceContext::Library);
        let surface = TrackDetailSurface::new(&vm);
        let _pane = TrackInspectorPane::new(surface);
    }
}
