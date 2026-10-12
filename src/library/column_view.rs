//! One view for each Music column (ADR 0084 Decision 1).
//!
//! `LibraryApp` stays one entity with one state. `LibraryColumnView` is a
//! thin render-only view that holds the Library entity and names one
//! column. It observes the Library entity, so a Library notify renders the
//! column again. Its own `render` always goes through `Entity::update`, so
//! each Library listener keeps using `cx.listener` on `Context<LibraryApp>`
//! as it does today.

#![warn(clippy::pedantic)]

use gpui::{Context, Entity, IntoElement, Render, Subscription, Window};

use super::LibraryApp;

/// One of the three Music columns (ADR 0084 Decision 1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LibraryColumn {
    /// The destinations and the library tree (unchanged by this packet).
    Sidebar,
    /// The content list. It always shows, whatever page is open.
    List,
    /// The open page, or the empty state when none is open.
    Detail,
}

/// Renders one Music column from the shared Library entity.
pub(crate) struct LibraryColumnView {
    library: Entity<LibraryApp>,
    column: LibraryColumn,
    _library_sub: Subscription,
}

impl LibraryColumnView {
    /// Creates a column view over `library`. It observes `library`, so a
    /// Library notify renders this column again.
    pub(crate) fn new(
        library: Entity<LibraryApp>,
        column: LibraryColumn,
        cx: &mut Context<Self>,
    ) -> Self {
        let library_sub = cx.observe(&library, |_, _, cx| cx.notify());
        Self {
            library,
            column,
            _library_sub: library_sub,
        }
    }
}

impl Render for LibraryColumnView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.library.update(cx, |library, cx| {
            library.render_column(self.column, window, cx)
        })
    }
}
