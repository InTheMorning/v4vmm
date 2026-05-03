//! Track row composite — the standard list-row shape for tracks across all
//! screens.
//!
//! ## Display contract: `TrackRowVm` / `SharedTrackRowVm`
//!
//! Sits above [`crate::ui::composites::ListRow`] and uses
//! [`crate::ui::composites::Thumbnail`] + [`crate::ui::primitives::Label`]
//! so the token → primitive → composite → screen hierarchy is maintained.
//!
//! Call sites supply a display-ready row projection and an optional set of
//! trailing action elements (download button, playlist popover, play button).
//! The composite owns no fallback policy — it is purely presentational.
//!
//! # Examples
//!
//! ```ignore
//! use crate::ui::composites::{TrackRow, EntityKind, ThumbnailSize};
//! use crate::view_models::track_detail::TrackRowVm;
//!
//! TrackRow::from_vm("row:abc", &row_vm)
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
use crate::ui::tokens::{color, FontSize, SemanticColor, Spacing};
use crate::view_models::entity_detail::SharedTrackRowVm;
use crate::view_models::track_detail::TrackRowVm;

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(Clone, Debug, Eq, PartialEq)]
struct TrackRowDisplay {
    number: String,
    title: String,
    duration: Option<String>,
}

impl TrackRowDisplay {
    fn from_vm(vm: &TrackRowVm) -> Self {
        Self {
            number: vm.number.clone(),
            title: vm.title.clone(),
            duration: vm.duration.clone(),
        }
    }

    fn from_shared_track_row(vm: SharedTrackRowVm<'_>) -> Self {
        Self {
            number: vm.number_label(),
            title: vm.title(),
            duration: vm.duration_display(),
        }
    }
}

/// A single track in a list — number · thumbnail · title · duration · actions.
///
/// Constructed fresh each render from a display contract; no GPUI entity state.
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
    fn new(id: impl Into<ElementId>, display: TrackRowDisplay) -> Self {
        Self {
            id: id.into(),
            number: display.number,
            title: display.title,
            duration: display.duration,
            thumbnail: None,
            on_click: None,
            trailing: Vec::new(),
        }
    }

    /// Create a row from the shared ADR 0035 row display contract.
    pub fn from_vm(id: impl Into<ElementId>, vm: &TrackRowVm) -> Self {
        Self::new(id, TrackRowDisplay::from_vm(vm))
    }

    /// Create a row from the shared release/entity row display contract.
    pub fn from_shared_track_row(id: impl Into<ElementId>, vm: SharedTrackRowVm<'_>) -> Self {
        Self::new(id, TrackRowDisplay::from_shared_track_row(vm))
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
        let gap = Spacing::SM.scaled(cx);
        let tertiary_label = color(cx, SemanticColor::TertiaryLabel);

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
                    .w(crate::ui::layouts::TRACK_NUMBER_WIDTH)
                    .flex_shrink_0()
                    .text_right()
                    .text_color(tertiary_label)
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
                    .text_color(tertiary_label)
                    .text_size(FontSize::Micro.scaled(cx))
                    .child(SharedString::from(dur)),
            );
        }

        let mut row = ListRow::compact(self.id).child(body);
        for child in self.trailing {
            row = row.child(child);
        }
        div().min_h(crate::ui::layouts::ROW_HEIGHT).child(row)
    }
}

// ---------------------------------------------------------------------------
// Tests — pure builder logic, no GPUI runtime needed.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_vm_projects_row_contract() {
        let vm = TrackRowVm {
            element_key: "track:1".to_string(),
            number: "4".to_string(),
            title: "Song".to_string(),
            subtitle: Some("Artist".to_string()),
            duration: Some("3:00".to_string()),
        };
        let row = TrackRow::from_vm("test", &vm);

        assert_eq!(row.number, "4");
        assert_eq!(row.title, "Song");
        assert_eq!(row.duration.as_deref(), Some("3:00"));
    }

    #[test]
    fn display_contract_sets_core_fields() {
        let row = TrackRow::new(
            "test",
            TrackRowDisplay {
                number: "7".to_string(),
                title: "My Track".to_string(),
                duration: Some("3:45".to_string()),
            },
        );
        assert_eq!(row.number, "7");
        assert_eq!(row.title, "My Track");
        assert_eq!(row.duration.as_deref(), Some("3:45"));
    }

    #[test]
    fn default_thumbnail_is_none() {
        let row = TrackRow::new(
            "test",
            TrackRowDisplay {
                number: "\u{00B7}".to_string(),
                title: "Song".to_string(),
                duration: None,
            },
        );
        assert!(row.thumbnail.is_none());
    }

    #[test]
    fn trailing_children_accumulate() {
        // We can't inspect AnyElement contents but we can verify count.
        let row = TrackRow::new(
            "test",
            TrackRowDisplay {
                number: "3".to_string(),
                title: "Song".to_string(),
                duration: Some("2:30".to_string()),
            },
        )
        .trailing_child(gpui::div())
        .trailing_child(gpui::div());
        assert_eq!(row.trailing.len(), 2);
    }
}
