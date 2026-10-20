//! Track row composite — the standard list-row shape for tracks across all
//! screens.
//!
//! ## Display contract: `TrackRowVm`
//!
//! Sits above [`crate::ui::composites::ListRow`] and uses
//! [`crate::ui::composites::Thumbnail`] + [`crate::ui::primitives::Label`]
//! so the token → primitive → composite → screen hierarchy is maintained.
//!
//! Call sites supply a display-ready row projection and the elements of the
//! typed trailing slots: the primary action, the state texts and the
//! playlist action. The composite owns no fallback policy — it is purely
//! presentational.
//!
//! Each trailing column has a fixed width from [`TrackRowColumn`] (operator
//! finding of 2026-10-20). An empty slot keeps its width, so each column
//! starts at the same position on each row of a list.
//!
//! Accessibility note (ADR 0038 task 005): the row label is projected by
//! `TrackRowVm` and retained on the row/ListRow contract.
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
//!     .primary_action(download_btn)
//!     .state_child(state_label)
//!     .playlist_action(playlist_popover)
//! ```

#![warn(clippy::pedantic)]

use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, App, ClickEvent, Div, ElementId, Image, IntoElement, RenderOnce,
    SharedString, Styled, Window,
};

use crate::ui::composites::{EntityKind, ListRow, ListRowA11yLabel, Thumbnail, ThumbnailSize};
use crate::ui::primitives::{Checkbox, CheckboxState, Label};
use crate::ui::tokens::{color, FontSize, SemanticColor, Spacing, TrackRowColumn};
use crate::view_models::list_selection::{RowCheckState, RowSelectionVm};
use crate::view_models::track_detail::TrackRowVm;

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(Clone, Debug, Eq, PartialEq)]
struct TrackRowDisplay {
    number: Option<String>,
    title: String,
    duration: Option<String>,
    /// The date text under the title. `None` shows no second line.
    date_text: Option<String>,
    a11y_label: String,
}

impl TrackRowDisplay {
    fn from_vm(vm: &TrackRowVm) -> Self {
        Self {
            number: vm.number.clone(),
            title: vm.title.clone(),
            duration: vm.duration.clone(),
            date_text: vm.date_text.clone(),
            a11y_label: vm.a11y_label(),
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
    /// Track-number label (e.g. `"3"` or `"·"` when absent). `None` shows
    /// no number column, and the cover leads the row.
    number: Option<String>,
    title: String,
    duration: Option<String>,
    /// The date text under the title (ADR 0084 task 015). `None` shows no
    /// second line.
    date_text: Option<String>,
    a11y_label: String,
    thumbnail: Option<Arc<Image>>,
    on_click: Option<ClickHandler>,
    /// The selection facts of the row (ADR 0084 Decision 10, packet 018).
    /// [`RowSelectionVm::none`] shows no checkbox, no selected fill and no
    /// focus outline.
    selection: RowSelectionVm,
    /// The click on the checkbox. The checkbox shows only when the view
    /// model makes it visible.
    on_toggle_selection: Option<ClickHandler>,
    scroll_probe: Option<AnyElement>,
    /// The element of the [`TrackRowColumn::PrimaryAction`] column.
    primary_action: Option<AnyElement>,
    /// The elements of the [`TrackRowColumn::State`] column, in order.
    state: Vec<AnyElement>,
    /// The element of the [`TrackRowColumn::PlaylistAction`] column.
    playlist_action: Option<AnyElement>,
}

impl TrackRow {
    fn new(id: impl Into<ElementId>, display: TrackRowDisplay) -> Self {
        Self {
            id: id.into(),
            number: display.number,
            title: display.title,
            duration: display.duration,
            date_text: display.date_text,
            a11y_label: display.a11y_label,
            thumbnail: None,
            on_click: None,
            selection: RowSelectionVm::none(),
            on_toggle_selection: None,
            scroll_probe: None,
            primary_action: None,
            state: Vec::new(),
            playlist_action: None,
        }
    }

    /// Create a row from the shared ADR 0035 row display contract.
    pub fn from_vm(id: impl Into<ElementId>, vm: &TrackRowVm) -> Self {
        Self::new(id, TrackRowDisplay::from_vm(vm))
    }

    /// Album-art thumbnail. Pass `None` to render the entity-kind monogram fallback.
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

    /// The selection facts of the row (ADR 0084 Decision 10, packet 018):
    /// the checkbox, the selected fill and the keyboard focus outline.
    pub(crate) fn selection(mut self, selection: RowSelectionVm) -> Self {
        self.selection = selection;
        self
    }

    /// Called when the user clicks the checkbox of the row.
    pub(crate) fn on_toggle_selection<F>(mut self, handler: F) -> Self
    where
        F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    {
        self.on_toggle_selection = Some(Rc::new(handler));
        self
    }

    /// The scroll probe of the focused row (ADR 0084 Decision 10, packet
    /// 018), from `ScrollReveal::probe`.
    pub(crate) fn scroll_probe(mut self, probe: Option<AnyElement>) -> Self {
        self.scroll_probe = probe;
        self
    }

    /// The element of the primary action column, for example the
    /// "Download" or the "Remove" button.
    pub(crate) fn primary_action(mut self, child: impl IntoElement + 'static) -> Self {
        self.primary_action = Some(child.into_any_element());
        self
    }

    /// Adds one element to the state column, for example the
    /// `MusicBrainz` status or the state label "New". The elements show in
    /// the order of the calls.
    pub(crate) fn state_child(mut self, child: impl IntoElement + 'static) -> Self {
        self.state.push(child.into_any_element());
        self
    }

    /// The element of the playlist action column: the "+ Playlist"
    /// trigger.
    pub(crate) fn playlist_action(mut self, child: impl IntoElement + 'static) -> Self {
        self.playlist_action = Some(child.into_any_element());
        self
    }
}

/// One fixed trailing column of the row. The width comes from the
/// `column` token, so an empty column keeps its width.
fn column_cell(column: TrackRowColumn, cx: &App) -> Div {
    div()
        .w(column.scaled(cx))
        .flex_shrink_0()
        .flex()
        .flex_row()
        .items_center()
        .gap(Spacing::XS.scaled(cx))
        .overflow_hidden()
        .whitespace_nowrap()
        .debug_selector(move || column_selector(column).to_owned())
}

/// The duration column at the right of the body. It keeps its width when
/// the row has no duration.
fn duration_cell(duration: Option<String>, cx: &App) -> Div {
    column_cell(TrackRowColumn::Duration, cx)
        .justify_end()
        .text_color(color(cx, SemanticColor::TertiaryLabel))
        .text_size(FontSize::Micro.scaled(cx))
        .when_some(duration, |cell, duration| {
            cell.child(SharedString::from(duration))
        })
}

/// The debug selector of each trailing column, for the layout tests.
const fn column_selector(column: TrackRowColumn) -> &'static str {
    match column {
        TrackRowColumn::Duration => "track-row-column-duration",
        TrackRowColumn::PrimaryAction => "track-row-column-primary-action",
        TrackRowColumn::State => "track-row-column-state",
        TrackRowColumn::PlaylistAction => "track-row-column-playlist-action",
    }
}

impl RenderOnce for TrackRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let gap = Spacing::SM.scaled(cx);
        let tertiary_label = color(cx, SemanticColor::TertiaryLabel);

        let number = self.number.map(SharedString::from);
        let title = self.title;
        let duration = self.duration;
        let date_text = self.date_text;
        let a11y_label = self.a11y_label;
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
        let checkbox_id = SharedString::from(format!("{body_id}-checkbox"));

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
            // Leading track-number column — fixed width, right-aligned. A
            // row with no number has no column, and the cover leads.
            .when_some(number, |body, number| {
                body.child(
                    div()
                        .w(crate::ui::layouts::TRACK_NUMBER_WIDTH)
                        .flex_shrink_0()
                        .text_right()
                        .text_color(tertiary_label)
                        .text_size(FontSize::Micro.scaled(cx))
                        .child(number),
                )
            })
            // Thumbnail.
            .child(Thumbnail::new(EntityKind::Track, ThumbnailSize::Sm).image(thumbnail))
            // The title fills the remaining space and truncates. The date
            // text, when the view model gives one, shows as a second line
            // under the title.
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(Label::new(title).size(FontSize::Micro).truncated())
                    .when_some(date_text, |column, date| {
                        column.child(
                            Label::new(date)
                                .size(FontSize::Micro)
                                .color(SemanticColor::TertiaryLabel)
                                .truncated(),
                        )
                    }),
            );

        body = body.child(duration_cell(duration, cx));

        let selection = self.selection;
        let mut row = ListRow::compact(self.id)
            .a11y_label(ListRowA11yLabel {
                label: a11y_label.into(),
            })
            .selected(selection.is_marked())
            .focused(selection.focused)
            .scroll_probe(self.scroll_probe);
        // ADR 0084 Decision 10: the checkbox leads the row, before the
        // track number. It shows only in selection mode. Decision 11: the
        // audition slot of `ListRow` stays empty.
        if selection.checkbox_visible {
            let on_toggle = self.on_toggle_selection;
            row = row.checkbox(
                Checkbox::new(checkbox_id, checkbox_state_for(selection.check_state))
                    .a11y_label(selection.checkbox_a11y_label)
                    .on_click(move |event, window, cx| {
                        cx.stop_propagation();
                        if let Some(handler) = &on_toggle {
                            handler(event, window, cx);
                        }
                    })
                    .into_any_element(),
            );
        }
        row = row
            .child(body)
            .child(column_cell(TrackRowColumn::PrimaryAction, cx).children(self.primary_action))
            .child(column_cell(TrackRowColumn::State, cx).children(self.state))
            .child(column_cell(TrackRowColumn::PlaylistAction, cx).children(self.playlist_action));
        div()
            .min_h(crate::ui::layouts::MIN_HIT_TARGET)
            .debug_selector(|| "track-row".to_owned())
            .child(row)
    }
}

/// Maps the typed check state of the view model to the state of the
/// checkbox primitive. A track row has one member, so it never reads mixed.
/// The playlist row uses it too (packet 025).
pub(crate) fn checkbox_state_for(check_state: RowCheckState) -> CheckboxState {
    match check_state {
        RowCheckState::Checked => CheckboxState::Checked,
        RowCheckState::Mixed => CheckboxState::Mixed,
        RowCheckState::Clear => CheckboxState::NotChecked,
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
            number: Some("4".to_string()),
            title: "Song".to_string(),
            subtitle: Some("Artist".to_string()),
            duration: Some("3:00".to_string()),
            date_text: Some("Apr 5, 2024".to_string()),
            a11y_label: "Song, 3 minutes".to_string(),
        };
        let row = TrackRow::from_vm("test", &vm);

        assert_eq!(row.number.as_deref(), Some("4"));
        assert_eq!(row.title, "Song");
        assert_eq!(row.duration.as_deref(), Some("3:00"));
        assert_eq!(row.date_text.as_deref(), Some("Apr 5, 2024"));
    }

    #[test]
    fn display_contract_sets_core_fields() {
        let row = TrackRow::new(
            "test",
            TrackRowDisplay {
                number: Some("7".to_string()),
                title: "My Track".to_string(),
                duration: Some("3:45".to_string()),
                date_text: None,
                a11y_label: "My Track, 3 minutes 45 seconds".to_string(),
            },
        );
        assert_eq!(row.number.as_deref(), Some("7"));
        assert_eq!(row.title, "My Track");
        assert_eq!(row.duration.as_deref(), Some("3:45"));
        assert_eq!(row.a11y_label, "My Track, 3 minutes 45 seconds");
    }

    #[test]
    fn default_thumbnail_is_none() {
        let row = TrackRow::new(
            "test",
            TrackRowDisplay {
                number: Some("\u{00B7}".to_string()),
                title: "Song".to_string(),
                duration: None,
                date_text: None,
                a11y_label: "Song".to_string(),
            },
        );
        assert!(row.thumbnail.is_none());
    }

    #[test]
    fn trailing_slots_hold_their_elements() {
        // We can't inspect AnyElement contents but we can verify count.
        let row = TrackRow::new(
            "test",
            TrackRowDisplay {
                number: Some("3".to_string()),
                title: "Song".to_string(),
                duration: Some("2:30".to_string()),
                date_text: None,
                a11y_label: "Song, 2 minutes 30 seconds".to_string(),
            },
        )
        .primary_action(gpui::div())
        .state_child(gpui::div())
        .state_child(gpui::div())
        .playlist_action(gpui::div());
        assert!(row.primary_action.is_some());
        assert_eq!(row.state.len(), 2);
        assert!(row.playlist_action.is_some());
    }
}

// ---------------------------------------------------------------------------
// ADR 0039 task 002 M1: fixed geometry does not depend on string length.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod reservation_tests {
    use gpui::{div, prelude::*, px, TestAppContext};

    use super::{TrackRow, TrackRowDisplay};

    struct RowHeightTest {
        display: TrackRowDisplay,
        with_duration: bool,
        with_trailing: bool,
    }

    impl gpui::Render for RowHeightTest {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            let mut row = TrackRow::new("row", self.display.clone());
            if !self.with_duration {
                row.duration = None;
            }
            if self.with_trailing {
                row = row.primary_action(gpui::div().w(px(20.0)).h(px(20.0)));
            }
            div().w(px(400.0)).child(row)
        }
    }

    /// ADR 0039 task 002 M1: `TrackRow`'s height is a floor
    /// (`.min_h(MIN_HIT_TARGET)`), not a cap, but it must still stay
    /// independent of the track title's string length and unaffected by
    /// which optional slots (duration, trailing actions) are present versus
    /// their content length — the title truncates via `Label::truncated()`
    /// (a single-line contract) rather than growing the row.
    #[gpui::test]
    fn adr_0039_track_row_height_is_independent_of_title_length(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let short = TrackRowDisplay {
            number: Some("1".to_string()),
            title: "A".to_string(),
            duration: Some("0:01".to_string()),
            date_text: None,
            a11y_label: "A".to_string(),
        };
        let long = TrackRowDisplay {
            number: Some("1".to_string()),
            title: "A very long track title that would otherwise wrap this row".repeat(3),
            duration: Some("0:01".to_string()),
            date_text: None,
            a11y_label: "long".to_string(),
        };

        let (view, cx) = cx.add_window_view(|_, _| RowHeightTest {
            display: short.clone(),
            with_duration: true,
            with_trailing: false,
        });

        let mut heights = Vec::new();
        for (display, with_duration, with_trailing) in [
            (short.clone(), true, false),
            (long.clone(), true, false),
            (short.clone(), false, false),
            (long.clone(), false, false),
            (short, true, true),
            (long, true, true),
        ] {
            view.update(cx, |this, cx| {
                this.display = display;
                this.with_duration = with_duration;
                this.with_trailing = with_trailing;
                cx.notify();
            });
            cx.update(|window, cx| {
                let _ = window.draw(cx);
            });
            let bounds = cx
                .debug_bounds("track-row")
                .expect("track row bounds recorded");
            heights.push(bounds.size.height);
        }

        let first = heights[0];
        for height in &heights[1..] {
            assert_eq!(
                *height, first,
                "TrackRow height changed with string length or optional slots: {heights:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Operator finding of 2026-10-20: the trailing columns of a track row align.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod column_tests {
    use gpui::{div, prelude::*, px, Bounds, Pixels, TestAppContext, VisualTestContext};

    use super::{column_selector, TrackRow, TrackRowDisplay};
    use crate::ui::tokens::{ScaleFactor, TrackRowColumn};

    struct ColumnTest {
        filled: bool,
    }

    impl gpui::Render for ColumnTest {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            let display = TrackRowDisplay {
                number: Some("1".to_string()),
                title: "Song".to_string(),
                duration: self.filled.then(|| "1:02:03".to_string()),
                date_text: None,
                a11y_label: "Song".to_string(),
            };
            let mut row = TrackRow::new("row", display);
            if self.filled {
                row = row
                    .primary_action(div().w(px(30.0)).h(px(20.0)))
                    .state_child(div().w(px(10.0)).h(px(10.0)))
                    .state_child(div().w(px(10.0)).h(px(10.0)))
                    .playlist_action(div().w(px(40.0)).h(px(20.0)));
            }
            div().w(px(900.0)).child(row)
        }
    }

    fn column_bounds(cx: &mut VisualTestContext) -> Vec<Bounds<Pixels>> {
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        TrackRowColumn::ROW_ORDER
            .iter()
            .map(|column| {
                cx.debug_bounds(column_selector(*column))
                    .expect("track row column bounds recorded")
            })
            .collect()
    }

    /// Operator finding of 2026-10-20: each trailing column of a track row
    /// has the width of its `TrackRowColumn` token. An empty slot keeps
    /// that width, so each column starts at the same position on a row
    /// with content and on a row with no content.
    #[gpui::test]
    fn adr_0084_row_fix_empty_columns_keep_their_token_width(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (view, cx) = cx.add_window_view(|_, _| ColumnTest { filled: false });

        let empty = column_bounds(cx);
        view.update(cx, |this, cx| {
            this.filled = true;
            cx.notify();
        });
        let filled = column_bounds(cx);

        for (index, column) in TrackRowColumn::ROW_ORDER.iter().enumerate() {
            let token = column.scaled_px(ScaleFactor::Medium);
            assert_eq!(
                empty[index].size.width, token,
                "{column:?}: an empty slot must keep the token width"
            );
            assert_eq!(
                filled[index].size.width, token,
                "{column:?}: a filled slot must have the token width"
            );
            assert_eq!(
                empty[index].origin.x, filled[index].origin.x,
                "{column:?}: the column must start at the same position on each row"
            );
        }
    }

    /// The text part of each column follows the TYPE scale of its font
    /// role, and the inset part follows the CHROME scale (ADR 0039). A
    /// larger scale does not make a column narrower.
    #[test]
    fn adr_0084_row_fix_column_widths_grow_with_the_scale() {
        let scales = [
            ScaleFactor::XSmall,
            ScaleFactor::Small,
            ScaleFactor::Medium,
            ScaleFactor::Large,
            ScaleFactor::XLarge,
        ];
        for column in TrackRowColumn::ROW_ORDER {
            let widths: Vec<Pixels> = scales.iter().map(|s| column.scaled_px(*s)).collect();
            assert!(
                widths.windows(2).all(|pair| pair[0] <= pair[1]),
                "{column:?}: {widths:?}"
            );
            let medium = f32::from(column.text_px())
                + column
                    .inset()
                    .map_or(0.0, |spacing| f32::from(spacing.px()) * 2.0);
            assert_eq!(column.scaled_px(ScaleFactor::Medium), px(medium));
            let xlarge_text = f32::from(column.text_px())
                * column.font_size().type_multiplier(ScaleFactor::XLarge);
            assert!(f32::from(column.scaled_px(ScaleFactor::XLarge)) >= xlarge_text);
        }
    }
}
