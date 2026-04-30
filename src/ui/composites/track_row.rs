//! Track row composite — the standard list-row shape for tracks across all
//! screens.
//!
//! Sits above [`crate::ui::composites::ListRow`] and uses
//! [`crate::ui::composites::Thumbnail`] + [`crate::ui::primitives::Label`]
//! so the token → primitive → composite → screen hierarchy is maintained.
//!
//! Call sites supply display-ready strings (projected through
//! [`crate::view_models::track::TrackVm`]) and an optional set of trailing
//! action elements (download button, playlist popover, play button). The
//! composite owns no domain knowledge — it is purely presentational.
//!
//! # Examples
//!
//! ```ignore
//! use crate::ui::composites::{TrackRow, EntityKind, ThumbnailSize};
//! use crate::view_models::track::TrackVm;
//!
//! let vm = TrackVm::new(&track);
//! TrackRow::new("row:abc")
//!     .number(vm.track_number_label())
//!     .title(vm.title())
//!     .duration(vm.duration_display())
//!     .thumbnail(thumbnail, EntityKind::Track)
//!     .on_click(cx.listener(|this, _, _, cx| this.open_inspector(cx)))
//!     .trailing_child(download_btn)
//!     .trailing_child(playlist_popover)
//!     .trailing_child(play_btn)
//! ```

#![warn(clippy::pedantic)]

use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    div, prelude::*, App, ClickEvent, ElementId, Image, IntoElement, RenderOnce, SharedString,
    Styled, Window,
};

use crate::ui::composites::{EntityKind, ListRow, Thumbnail, ThumbnailSize};
use crate::ui::primitives::Label;
use crate::ui::tokens::{FontSize, SemanticColor, Spacing};

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// A single track in a list — number · thumbnail · title · duration · actions.
///
/// Constructed fresh each render via the builder API; no GPUI entity state.
#[derive(IntoElement)]
#[must_use]
pub struct TrackRow {
    id: ElementId,
    /// Track-number label (e.g. `"3"` or `"·"` when absent).
    number: String,
    title: String,
    duration: Option<String>,
    thumbnail: Option<Arc<Image>>,
    on_click: Option<ClickHandler>,
    trailing: Vec<gpui::AnyElement>,
}

impl TrackRow {
    /// Create a new row with the given element id.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            number: "\u{00B7}".into(), // middle dot placeholder
            title: String::new(),
            duration: None,
            thumbnail: None,
            on_click: None,
            trailing: Vec::new(),
        }
    }

    /// Track-number label shown in the leading column.
    pub fn number(mut self, n: impl Into<String>) -> Self {
        self.number = n.into();
        self
    }

    /// Display title.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }

    /// Optional `"M:SS"` duration string.
    pub fn duration(mut self, d: Option<String>) -> Self {
        self.duration = d;
        self
    }

    /// Album-art thumbnail. Pass `None` to render the entity-kind emoji fallback.
    pub fn thumbnail(mut self, img: Option<Arc<Image>>) -> Self {
        self.thumbnail = img;
        self
    }

    /// Called when the user clicks the title / main body area of the row.
    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Append a trailing action element (download button, playlist popover,
    /// play button, …). Rendered in the order they are added.
    pub fn trailing_child(mut self, child: impl IntoElement + 'static) -> Self {
        self.trailing.push(child.into_any_element());
        self
    }
}

impl RenderOnce for TrackRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let appearance = crate::ui::tokens::Appearance::current(cx);
        let gap = Spacing::SM.scaled(cx);

        let number = SharedString::from(self.number);
        let title = self.title;
        let duration = self.duration;
        let on_click = self.on_click;
        let thumbnail = self.thumbnail;

        // The main clickable body: number · thumb · title · duration.
        let body_id = {
            // Derive a stable child element id from the parent row id.
            let row_str = match &self.id {
                ElementId::Integer(n) => format!("track-body:{n}"),
                ElementId::Name(n) => format!("track-body:{n}"),
                other => format!("track-body:{other:?}"),
            };
            SharedString::from(row_str)
        };

        let mut body = div()
            .id(body_id)
            .flex_1()
            .min_w_0()
            .flex()
            .flex_row()
            .items_center()
            .gap(gap);

        if on_click.is_some() {
            body = body.cursor_pointer();
        }

        if let Some(handler) = on_click {
            body = body.on_click(move |event, window, cx| handler(event, window, cx));
        }

        body = body
            // Leading track-number column — fixed width, right-aligned.
            .child(
                div()
                    .w(crate::ui::theme::layout::TRACK_NUMBER_WIDTH)
                    .flex_shrink_0()
                    .text_right()
                    .text_color(SemanticColor::TertiaryLabel.resolve(appearance))
                    .text_size(FontSize::Micro.scaled(cx))
                    .child(number),
            )
            // Thumbnail.
            .child(Thumbnail::new(EntityKind::Track, ThumbnailSize::Sm).image(thumbnail))
            // Title — fills remaining space, truncates.
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(Label::new(title).size(FontSize::Micro).truncated()),
            );

        // Optional duration label at the right of the body.
        if let Some(dur) = duration {
            body = body.child(
                div()
                    .flex_shrink_0()
                    .text_color(SemanticColor::TertiaryLabel.resolve(appearance))
                    .text_size(FontSize::Micro.scaled(cx))
                    .child(SharedString::from(dur)),
            );
        }

        let mut row = ListRow::compact(self.id).child(body);
        for child in self.trailing {
            row = row.child(child);
        }
        row
    }
}

// ---------------------------------------------------------------------------
// Tests — pure builder logic, no GPUI runtime needed.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_number_is_middle_dot() {
        let row = TrackRow::new("test");
        assert_eq!(row.number, "\u{00B7}");
    }

    #[test]
    fn default_title_is_empty() {
        let row = TrackRow::new("test");
        assert!(row.title.is_empty());
    }

    #[test]
    fn default_duration_is_none() {
        let row = TrackRow::new("test");
        assert!(row.duration.is_none());
    }

    #[test]
    fn default_thumbnail_is_none() {
        let row = TrackRow::new("test");
        assert!(row.thumbnail.is_none());
    }

    #[test]
    fn builder_sets_number() {
        let row = TrackRow::new("test").number("7");
        assert_eq!(row.number, "7");
    }

    #[test]
    fn builder_sets_title() {
        let row = TrackRow::new("test").title("My Track");
        assert_eq!(row.title, "My Track");
    }

    #[test]
    fn builder_sets_duration() {
        let row = TrackRow::new("test").duration(Some("3:45".into()));
        assert_eq!(row.duration.as_deref(), Some("3:45"));
    }

    #[test]
    fn builder_clears_duration_with_none() {
        let row = TrackRow::new("test")
            .duration(Some("1:00".into()))
            .duration(None);
        assert!(row.duration.is_none());
    }

    #[test]
    fn trailing_children_accumulate() {
        // We can't inspect AnyElement contents but we can verify count.
        let row = TrackRow::new("test")
            .trailing_child(gpui::div())
            .trailing_child(gpui::div());
        assert_eq!(row.trailing.len(), 2);
    }

    #[test]
    fn chained_builder_fields_are_independent() {
        let row = TrackRow::new("test")
            .number("3")
            .title("Song")
            .duration(Some("2:30".into()));
        assert_eq!(row.number, "3");
        assert_eq!(row.title, "Song");
        assert_eq!(row.duration.as_deref(), Some("2:30"));
    }
}
