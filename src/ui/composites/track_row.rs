//! Track row composite — the standard list-row shape for tracks across all
//! screens.
//!
//! ## Display contract: `TrackRowVm`
//!
//! Sits above [`crate::ui::composites::ListRow`] and uses
//! [`crate::ui::composites::Thumbnail`] + [`crate::ui::primitives::Label`]
//! so the token → primitive → composite → screen hierarchy is maintained.
//!
//! Call sites supply a display-ready row projection and the typed view
//! models of the row state: the badges, the `MusicBrainz` status, the
//! download-state slot and the row menu (ADR 0083 task 006). The composite
//! owns no fallback policy — it is purely presentational.
//!
//! The row shows its information and its state at all times. The "⋯"
//! button shows on hover and on keyboard focus (ADR 0083 Decision 6). A
//! right-click on the row opens a menu with the same items.
//!
//! Each trailing column has a fixed width from [`TrackRowColumn`] (operator
//! finding of 2026-10-20). An empty or hidden slot keeps its width, so each
//! column starts at the same position on each row of a list.
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
//!     .badges(row_vm.badges.clone())
//!     .mb_status(row_vm.mb_status)
//!     .download_slot(row_vm.download.clone())
//!     .on_download(move |window, cx| download(window, cx))
//!     .menu(row_vm.menu.clone(), playlists)
//!     .on_menu_action(handlers, return_focus)
//! ```

#![warn(clippy::pedantic)]

use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    div, prelude::*, AnyElement, App, ClickEvent, Div, ElementId, FocusHandle, Image, IntoElement,
    MouseButton, RenderOnce, SharedString, Styled, Window,
};

use crate::ui::composites::track_row_menu::{
    row_menu_state, track_row_menu_items, track_row_pointer_menu, TrackRowMenuCell,
    TrackRowMenuHandlers, TRACK_ROW_GROUP,
};
use crate::ui::composites::{
    EntityKind, ListRow, ListRowA11yLabel, PlaylistOption, Thumbnail, ThumbnailSize,
};
use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::{Icon, IconName};
use crate::ui::primitives::status_badge::render_row_badge;
use crate::ui::primitives::{Button, Checkbox, CheckboxState, Label, Tooltip};
use crate::ui::tokens::{color, FontSize, SemanticColor, Size, Spacing, TrackRowColumn};
use crate::view_models::list_selection::{RowCheckState, RowSelectionVm};
use crate::view_models::release_page::{
    DownloadSlotState, DownloadSlotVm, MbStatusIcon, TrackRowBadgeVm, TrackRowMbStatusVm,
    TrackRowMenuVm,
};
use crate::view_models::track_detail::TrackRowVm;

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;
type ActionHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

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
    /// The badges after the title, in order (ADR 0083 task 006).
    badges: Vec<TrackRowBadgeVm>,
    /// The `MusicBrainz` status icon after the badges.
    mb_status: Option<TrackRowMbStatusVm>,
    /// The [`TrackRowColumn::DownloadState`] slot. `None` keeps the
    /// column empty.
    download: Option<DownloadSlotVm>,
    on_download: Option<ActionHandler>,
    /// The row menu. `None` shows no "⋯" button and attaches no
    /// right-click menu.
    menu: Option<TrackRowMenuVm>,
    menu_playlists: Rc<Vec<PlaylistOption>>,
    menu_handlers: Option<TrackRowMenuHandlers>,
    /// The focus that the right-click menu gives back on its dismissal.
    menu_return_focus: Option<FocusHandle>,
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
            badges: Vec::new(),
            mb_status: None,
            download: None,
            on_download: None,
            menu: None,
            menu_playlists: Rc::new(Vec::new()),
            menu_handlers: None,
            menu_return_focus: None,
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

    /// The badges after the title, for example "New" (ADR 0083 task 006).
    pub(crate) fn badges(mut self, badges: Vec<TrackRowBadgeVm>) -> Self {
        self.badges = badges;
        self
    }

    /// The `MusicBrainz` status icon after the badges.
    pub(crate) const fn mb_status(mut self, status: Option<TrackRowMbStatusVm>) -> Self {
        self.mb_status = status;
        self
    }

    /// The download-state slot before the duration.
    pub(crate) fn download_slot(mut self, slot: DownloadSlotVm) -> Self {
        self.download = Some(slot);
        self
    }

    /// Called when the user clicks the download button of the slot. The
    /// click does not open the row page.
    pub(crate) fn on_download(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_download = Some(Rc::new(handler));
        self
    }

    /// The row menu and the playlists of its "Add to playlist" level.
    /// `None` shows no "⋯" button and no right-click menu (selection
    /// mode).
    pub(crate) fn menu(
        mut self,
        menu: Option<TrackRowMenuVm>,
        playlists: Vec<PlaylistOption>,
    ) -> Self {
        self.menu = menu;
        self.menu_playlists = Rc::new(playlists);
        self
    }

    /// The handlers of the row menu, and the focus that the right-click
    /// menu gives back on its dismissal.
    pub(crate) fn on_menu_action(
        mut self,
        handlers: TrackRowMenuHandlers,
        return_focus: FocusHandle,
    ) -> Self {
        self.menu_handlers = Some(handlers);
        self.menu_return_focus = Some(return_focus);
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

/// One fixed column that holds an icon control. The control centers in
/// the column. Each such column has the height of the hit target of the
/// control, also when it is empty or hidden, so each row has the same
/// height in each state.
fn control_cell(column: TrackRowColumn, cx: &App) -> Div {
    column_cell(column, cx)
        .justify_center()
        .min_h(Size::MinHitTarget.scaled(cx))
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
        TrackRowColumn::DownloadState => "track-row-column-download-state",
        TrackRowColumn::Duration => "track-row-column-duration",
        TrackRowColumn::Menu => "track-row-column-menu",
    }
}

/// The download-state cell (ADR 0083 task 006, decision D6): a download
/// button, the queued icon or the downloading icon, each with a tooltip.
/// An empty slot keeps its width.
fn download_cell(
    slot: Option<DownloadSlotVm>,
    on_download: Option<ActionHandler>,
    cx: &App,
) -> Div {
    let cell = control_cell(TrackRowColumn::DownloadState, cx);
    let Some(slot) = slot else {
        return cell;
    };
    let tooltip = slot.tooltip.unwrap_or_default();
    let a11y_label = slot.a11y_label.clone().unwrap_or_default();
    match slot.state {
        DownloadSlotState::Empty => cell,
        DownloadSlotState::Download => {
            let mut button = Button::styled(SharedString::from(slot.id), ControlStyle::RowAction)
                .leading_icon(IconName::CloudDownload)
                .label("")
                .tooltip(tooltip)
                .a11y_label(a11y_label)
                .disabled(!slot.available);
            if let (true, Some(handler)) = (slot.available, on_download) {
                button = button.on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    handler(window, cx);
                });
            }
            cell.child(button)
        }
        DownloadSlotState::Queued | DownloadSlotState::Downloading => {
            let icon = if slot.state == DownloadSlotState::Queued {
                IconName::Queued
            } else {
                IconName::InProgress
            };
            cell.child(status_icon(
                slot.id,
                icon,
                SemanticColor::SecondaryLabel,
                tooltip,
                cx,
            ))
        }
    }
}

/// One status icon with its word in a tooltip.
fn status_icon(
    id: String,
    icon: IconName,
    role: SemanticColor,
    tooltip: &'static str,
    cx: &App,
) -> impl IntoElement {
    let tooltip = Tooltip::new(tooltip);
    div()
        .id(SharedString::from(id))
        .flex_shrink_0()
        .flex()
        .items_center()
        .tooltip(move |window, cx| tooltip.build(window, cx))
        .child(Icon::new(icon).color(color(cx, role)))
}

/// The icon and the color role of each `MusicBrainz` status (decision
/// D12). The tooltip holds the word, so the state never relies on color
/// alone.
const fn mb_status_icon(icon: MbStatusIcon) -> (IconName, SemanticColor) {
    match icon {
        MbStatusIcon::Pending => (IconName::MbPending, SemanticColor::TertiaryLabel),
        MbStatusIcon::Processing => (IconName::MbLookingUp, SemanticColor::WarningLabel),
        MbStatusIcon::NoMissingFields => {
            (IconName::MbNoMissingFields, SemanticColor::TertiaryLabel)
        }
        MbStatusIcon::Done => (IconName::MbDone, SemanticColor::SuccessLabel),
        MbStatusIcon::Skipped => (IconName::MbSkipped, SemanticColor::DangerLabel),
    }
}

impl RenderOnce for TrackRow {
    #[expect(
        clippy::too_many_lines,
        reason = "the render composes each fixed part of the one track row in order"
    )]
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
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
        let mb_id = format!("{body_id}-mb");

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
            // The title fills the remaining space and truncates. The badges
            // and the `MusicBrainz` icon follow the title. The date text,
            // when the view model gives one, shows as a second line under
            // the title.
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(Spacing::XS.scaled(cx))
                            .child(
                                div()
                                    .min_w_0()
                                    .child(Label::new(title).size(FontSize::Micro).truncated()),
                            )
                            .children(
                                self.badges
                                    .iter()
                                    .map(|badge| render_row_badge(badge.label, cx)),
                            )
                            .when_some(self.mb_status, |line, status| {
                                let (icon, role) = mb_status_icon(status.icon);
                                line.child(status_icon(mb_id, icon, role, status.label, cx))
                            }),
                    )
                    .when_some(date_text, |column, date| {
                        column.child(
                            Label::new(date)
                                .size(FontSize::Micro)
                                .color(SemanticColor::TertiaryLabel)
                                .truncated(),
                        )
                    }),
            );

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
        // ADR 0083 Decision 6: the menu and the right-click menu exist
        // only when the view model gives a menu, and the host gives the
        // handlers and the return focus.
        let menu = match (self.menu, self.menu_handlers, self.menu_return_focus) {
            (Some(menu), Some(handlers), Some(focus)) => {
                let state = row_menu_state(&menu.id, window, cx);
                let items = track_row_menu_items(&menu, &self.menu_playlists, &handlers, &state);
                Some((menu, items, state, focus))
            }
            _ => None,
        };
        let mut menu_cell =
            TrackRowMenuCell::new(control_cell(TrackRowColumn::Menu, cx), selection.focused);
        let mut pointer_menu = None;
        let mut right_click_state = None;
        if let Some((menu, items, state, focus)) = menu {
            if let Some(position) = state.read(cx).pointer() {
                pointer_menu = Some(track_row_pointer_menu(
                    &menu,
                    position,
                    focus,
                    items.clone(),
                    &state,
                ));
            }
            right_click_state = Some(state.clone());
            menu_cell = menu_cell.menu(menu, items, state);
        }
        row = row
            .child(body)
            .child(download_cell(self.download, self.on_download, cx))
            .child(duration_cell(duration, cx))
            .child(menu_cell);
        div()
            .min_h(crate::ui::layouts::MIN_HIT_TARGET)
            .group(TRACK_ROW_GROUP)
            .debug_selector(|| "track-row".to_owned())
            .when_some(right_click_state, |root, state| {
                root.on_mouse_down(MouseButton::Right, move |event, _, cx| {
                    state.update(cx, |state, cx| {
                        state.open_at(event.position);
                        cx.notify();
                    });
                    cx.stop_propagation();
                })
            })
            .child(row)
            .children(pointer_menu)
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
    fn row_state_slots_hold_their_view_models() {
        use crate::view_models::library::TrackRowWork;
        use crate::view_models::release_page::{ReleaseTrackRowVm, TrackRowBadge, TrackRowBadgeVm};
        use crate::views::{FeedView, TrackView};

        let feed = FeedView {
            feed_guid: Some("feed-guid".to_owned()),
            tracks: vec![TrackView {
                title: Some("Song".to_owned()),
                track_guid: Some("guid-3".to_owned()),
                ..TrackView::default()
            }],
            ..FeedView::default()
        };
        let vm = ReleaseTrackRowVm::index(
            &feed,
            0,
            &feed.tracks[0],
            RowSelectionVm::none(),
            TrackRowWork::Idle,
        );
        let badge = TrackRowBadgeVm {
            badge: TrackRowBadge::New,
            label: "New",
        };
        let row = TrackRow::from_vm("test", &vm.row)
            .badges(vec![badge])
            .download_slot(vm.download.clone())
            .menu(vm.menu.clone(), Vec::new());
        assert_eq!(row.badges, vec![badge]);
        assert_eq!(row.download, Some(vm.download));
        assert_eq!(row.menu, vm.menu);
        assert!(
            row.menu_handlers.is_none(),
            "no handler, so no menu renders"
        );
    }
}

// ---------------------------------------------------------------------------
// ADR 0039 task 002 M1: fixed geometry does not depend on string length.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod reservation_tests {
    use gpui::{div, prelude::*, px, TestAppContext};

    use super::{TrackRow, TrackRowDisplay};
    use crate::view_models::library::TrackRowWork;
    use crate::view_models::release_page::DownloadSlotVm;

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
                row = row.download_slot(DownloadSlotVm::project(
                    "download".to_owned(),
                    "Song",
                    false,
                    TrackRowWork::Idle,
                ));
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
// Operator finding of 2026-10-20 and ADR 0083 task 006: the trailing
// columns of a track row align.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod column_tests {
    use std::rc::Rc;

    use gpui::{
        div, prelude::*, px, Bounds, FocusHandle, Pixels, TestAppContext, VisualTestContext,
    };

    use super::{column_selector, TrackRow, TrackRowDisplay};
    use crate::ui::composites::track_row_menu::TrackRowMenuHandlers;
    use crate::ui::tokens::{ScaleFactor, TrackRowColumn};
    use crate::view_models::library::TrackRowWork;
    use crate::view_models::list_selection::RowSelectionVm;
    use crate::view_models::release_page::ReleaseTrackRowVm;
    use crate::views::{FeedView, TrackView};

    /// The state of the row under test.
    #[derive(Clone, Copy, Debug)]
    enum RowCase {
        /// No slot, no duration and no menu.
        Empty,
        /// A download button, a duration and a hidden "⋯" cell.
        Filled,
        /// The same row with the keyboard focus: the "⋯" cell is visible.
        Focused,
        /// Selection mode: a checkbox, a queued icon and no menu.
        Selecting,
    }

    struct ColumnTest {
        case: RowCase,
        focus: FocusHandle,
    }

    fn handlers() -> TrackRowMenuHandlers {
        TrackRowMenuHandlers {
            action: Rc::new(|_, _, _| {}),
            select_playlist: Rc::new(|_, _, _| {}),
            create_playlist: Rc::new(|_, _, _| {}),
        }
    }

    fn row_vm(case: RowCase) -> ReleaseTrackRowVm {
        let feed = FeedView {
            feed_guid: Some("feed-guid".to_owned()),
            tracks: vec![TrackView {
                title: Some("Song".to_owned()),
                track_guid: Some("guid-1".to_owned()),
                track_number: Some(1),
                duration_secs: Some(3723),
                ..TrackView::default()
            }],
            ..FeedView::default()
        };
        let selection = match case {
            RowCase::Empty | RowCase::Filled => RowSelectionVm::none(),
            RowCase::Focused => RowSelectionVm {
                focused: true,
                ..RowSelectionVm::none()
            },
            RowCase::Selecting => RowSelectionVm {
                checkbox_visible: true,
                ..RowSelectionVm::none()
            },
        };
        let work = match case {
            RowCase::Selecting => TrackRowWork::Queued,
            RowCase::Empty | RowCase::Filled | RowCase::Focused => TrackRowWork::Idle,
        };
        ReleaseTrackRowVm::index(&feed, 0, &feed.tracks[0], selection, work)
    }

    impl gpui::Render for ColumnTest {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            let vm = row_vm(self.case);
            let row = if matches!(self.case, RowCase::Empty) {
                TrackRow::new(
                    "row",
                    TrackRowDisplay {
                        number: Some("1".to_string()),
                        title: "Song".to_string(),
                        duration: None,
                        date_text: None,
                        a11y_label: "Song".to_string(),
                    },
                )
            } else {
                TrackRow::from_vm("row", &vm.row)
                    .selection(vm.selection.clone())
                    .badges(vm.badges.clone())
                    .download_slot(vm.download.clone())
                    .menu(vm.menu.clone(), Vec::new())
                    .on_menu_action(handlers(), self.focus.clone())
            };
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

    /// R83-6-10 (ADR 0083 Decision 6): each trailing column has the width
    /// of its `TrackRowColumn` token when it is empty, filled or hidden,
    /// and in `Selecting`. Each column starts at the same position in each
    /// case. The duration is before the menu.
    #[gpui::test]
    fn adr_0083_row_actions_r83_6_10_columns_keep_their_width_in_each_state(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (view, cx) = cx.add_window_view(|_, cx| ColumnTest {
            case: RowCase::Empty,
            focus: cx.focus_handle(),
        });

        let mut cases = Vec::new();
        for case in [
            RowCase::Empty,
            RowCase::Filled,
            RowCase::Focused,
            RowCase::Selecting,
        ] {
            view.update(cx, |this, cx| {
                this.case = case;
                cx.notify();
            });
            cases.push((case, column_bounds(cx)));
        }

        let (_, first) = &cases[0];
        for (case, bounds) in &cases {
            // The focus outline of `ListRow` is 1 px wider than the plain
            // border, so a focused row moves its content by 1 px. The
            // focused case compares the positions of the cells to each
            // other.
            let shift = if matches!(case, RowCase::Focused) {
                bounds[0].origin.x - first[0].origin.x
            } else {
                px(0.0)
            };
            assert!(shift <= px(1.0), "{case:?}: the row moved by {shift:?}");
            for (index, column) in TrackRowColumn::ROW_ORDER.iter().enumerate() {
                assert_eq!(
                    bounds[index].size.width,
                    column.scaled_px(ScaleFactor::Medium),
                    "{case:?} {column:?}: the cell must have the token width"
                );
                assert_eq!(
                    bounds[index].origin.x - shift,
                    first[index].origin.x,
                    "{case:?} {column:?}: the cell must start at the same position"
                );
            }
            let duration = bounds[1];
            let menu = bounds[2];
            assert!(
                duration.origin.x + duration.size.width <= menu.origin.x,
                "{case:?}: the duration must be before the menu"
            );
        }
    }

    /// R83-6-11: the row order is the download slot, the duration and the
    /// menu. A larger scale does not make a column narrower.
    #[test]
    fn adr_0083_row_actions_r83_6_11_row_order_and_scale() {
        assert_eq!(
            TrackRowColumn::ROW_ORDER,
            [
                TrackRowColumn::DownloadState,
                TrackRowColumn::Duration,
                TrackRowColumn::Menu,
            ]
        );
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
        }
        // `Size::MinHitTarget` is 44 px at the medium scale. It is the
        // minimum width of the hit area of a `Button`, so an icon column of
        // this width cuts no hover fill (operator walk of 2026-10-20).
        assert_eq!(
            TrackRowColumn::DownloadState.scaled_px(ScaleFactor::Medium),
            px(44.0)
        );
        assert_eq!(
            TrackRowColumn::Menu.scaled_px(ScaleFactor::Medium),
            px(44.0)
        );
        assert_eq!(
            TrackRowColumn::Duration.scaled_px(ScaleFactor::Medium),
            px(44.0)
        );
    }
}
