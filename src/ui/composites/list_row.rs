#![warn(clippy::pedantic)]
//! Generic list row layout — a horizontal stack with HIG-conformant padding,
//! gap, and corner radius. Callers fill it with arbitrary children
//! (thumbnail, title, trailing actions). All dimensions scale.
//!
//! Accessibility note (ADR 0038 task 005): clickable rows can carry a
//! VM-sourced `a11y_label`. GPUI 0.2.x does not expose a row-level label sink
//! for `div`; the field is retained as contract data until the framework can
//! consume it.
//!
//! ADR 0083 Decision 6, task 007: a row can have one menu cell. The cell
//! is the last fixed column. It holds the "⋯" button, which shows on a
//! hover of the row and on the keyboard focus of the row. A right-click on
//! the row opens a menu with the same items. [`RowMenu`] and
//! [`row_menu_parts`] give the same menu to the playlist row.

use std::rc::Rc;

use gpui::{
    div, prelude::*, App, ClickEvent, Div, ElementId, Entity, FocusHandle, InteractiveElement,
    IntoElement, MouseButton, ParentElement, RenderOnce, Styled, Window,
};

use crate::ui::composites::playlist_popover::PlaylistOption;
use crate::ui::composites::track_drag::{attach_track_drag, TrackDragPayload};
use crate::ui::composites::track_row_menu::{
    row_menu_state, track_row_menu_items, track_row_pointer_menu, TrackRowMenuCell,
    TrackRowMenuHandlers, TrackRowMenuState, TRACK_ROW_GROUP,
};
use crate::ui::primitives::{HStack, PointerContextMenu};
use crate::ui::tokens::{color, Radius, SemanticColor, Size, Spacing, TrackRowColumn};
use crate::view_models::library::TrackDragVm;
use crate::view_models::release_page::TrackRowMenuVm;

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListRowA11yLabel {
    pub label: gpui::SharedString,
}

/// Vertical density for a [`ListRow`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ListRowDensity {
    /// Tight rows for dense lists (e.g., track tables).
    Compact,
    /// Default — comfortable for most lists.
    #[default]
    Comfortable,
}

impl ListRowDensity {
    fn padding(self) -> Spacing {
        match self {
            Self::Compact => Spacing::XS,
            Self::Comfortable => Spacing::SM,
        }
    }

    fn gap(self) -> Spacing {
        match self {
            Self::Compact => Spacing::SM,
            Self::Comfortable => Spacing::MD,
        }
    }
}

/// One row in a list. Wraps an [`HStack`] with padding, gap, and rounding.
#[derive(IntoElement)]
#[must_use]
pub struct ListRow {
    id: ElementId,
    density: ListRowDensity,
    selected: bool,
    focused: bool,
    a11y_label: Option<gpui::SharedString>,
    on_click: Option<ClickHandler>,
    /// The leading selection checkbox (ADR 0084 Decision 10, amended
    /// 2026-10-19 for selection mode). `None` hides it and reserves no
    /// space: a `Normal`-mode row, and a row with no selection model,
    /// both give `None`.
    checkbox: Option<gpui::AnyElement>,
    /// The audition control position of a track row (ADR 0084 Decision
    /// 11). `None` shows no control and reserves no space, until
    /// [ADR 0068](../../../docs/adr/0068-show-cue-and-audition-isolation.md)
    /// fills it.
    audition: Option<gpui::AnyElement>,
    /// The scroll probe of the focused row of a detail page (ADR 0084
    /// Decision 10, packet 018). It paints nothing.
    scroll_probe: Option<gpui::AnyElement>,
    /// The row has a menu cell (ADR 0083 task 007). A cell with no
    /// `menu` is empty and keeps its width.
    has_menu_cell: bool,
    /// The menu of the menu cell.
    menu: Option<RowMenu>,
    /// The drag of one track to a playlist (ADR 0083 task 008). `None`
    /// starts no drag.
    drag: Option<TrackDragVm>,
    children: Vec<gpui::AnyElement>,
}

impl ListRow {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            density: ListRowDensity::default(),
            selected: false,
            focused: false,
            a11y_label: None,
            on_click: None,
            checkbox: None,
            audition: None,
            scroll_probe: None,
            has_menu_cell: false,
            menu: None,
            drag: None,
            children: Vec::new(),
        }
    }

    pub fn compact(id: impl Into<ElementId>) -> Self {
        Self::new(id).density(ListRowDensity::Compact)
    }

    pub fn density(mut self, density: ListRowDensity) -> Self {
        self.density = density;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn a11y_label(mut self, label: ListRowA11yLabel) -> Self {
        self.a11y_label = Some(label.label);
        self
    }

    /// The leading selection checkbox (ADR 0084 Decision 10, amended
    /// 2026-10-19). A caller shows this only in selection mode: a caller
    /// with no selection model, or a `Normal`-mode row, never calls
    /// this, so the row reserves no checkbox space.
    pub fn checkbox(mut self, checkbox: gpui::AnyElement) -> Self {
        self.checkbox = Some(checkbox);
        self
    }

    /// The audition control position of a track row (ADR 0084 Decision
    /// 11). No packet fills this yet, so no caller passes one.
    pub fn audition(mut self, audition: gpui::AnyElement) -> Self {
        self.audition = Some(audition);
        self
    }

    /// The scroll probe of the focused row of a detail page (ADR 0084
    /// Decision 10, packet 018), from `ScrollReveal::probe`. The row gives
    /// it its own bounds, so the column can keep the row in view.
    pub(crate) fn scroll_probe(mut self, probe: Option<gpui::AnyElement>) -> Self {
        self.scroll_probe = probe;
        self
    }

    /// The menu cell of the row (ADR 0083 task 007). The cell is the last
    /// fixed column. It keeps its width when `menu` is `None`, so each cell
    /// of a list starts at the same position. A row with a menu is the
    /// hover group of the cell and opens the same menu on a right-click.
    pub(crate) fn menu_cell(mut self, menu: Option<RowMenu>) -> Self {
        self.has_menu_cell = true;
        self.menu = menu;
        self
    }

    /// The drag fact of a track row (ADR 0083 task 008). With `Some`, a
    /// drag of the row starts a drag of the track to a playlist.
    pub(crate) fn drag(mut self, drag: Option<TrackDragVm>) -> Self {
        self.drag = drag;
        self
    }

    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for ListRow {
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ListRow {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let pad_x = self.density.padding().scaled(cx);
        let pad_y = self.density.padding().scaled(cx);
        let radius = Radius::SM.scaled(cx);
        let stack = HStack::new().spacing(self.density.gap()).center();
        let selected = self.selected;
        let focused = self.focused;
        let on_click = self.on_click.clone();
        let selected_bg = color(cx, SemanticColor::SelectedContent);
        let accent = color(cx, SemanticColor::Accent);
        let focus = color(cx, SemanticColor::Focus);
        let hover_bg = color(cx, SemanticColor::SecondarySystemBackground);
        let has_click_action = on_click.is_some();

        // ADR 0084 Decision 10 Amendment, 2026-10-19: the checkbox shows
        // only in selection mode. The caller gives one only then, so it
        // always renders, with no hover reveal.
        let checkbox = self.checkbox;

        // ADR 0083 task 007: the menu cell is the last fixed column.
        let has_menu_cell = self.has_menu_cell;
        let (menu_cell, menu_root) = if has_menu_cell {
            let (cell, root) = row_menu_parts(self.menu, menu_cell(cx), focused, window, cx);
            (Some(cell.into_any_element()), root)
        } else {
            (None, RowMenuRoot::empty())
        };

        // ADR 0084 Decision 1: a row in a scrolling list keeps its content
        // height. A list column has a fixed height, and a flex child shrinks
        // by default, so a row without this overlaps the next row.
        let mut row = div()
            .id(self.id)
            .flex_shrink_0()
            .px(pad_x)
            .py(pad_y)
            .when(has_click_action, |el| {
                el.min_h(Size::MinHitTarget.scaled(cx))
            })
            .rounded(radius)
            .border_1()
            .border_color(gpui::transparent_black())
            .when(selected, |el| el.bg(selected_bg))
            .map(|el| match row_outline(selected, focused) {
                RowOutline::None => el,
                RowOutline::Selected => el.border_color(accent),
                RowOutline::Focus => el.border_2().border_color(focus),
            })
            .child(
                stack.children(
                    // ADR 0084 Decision 10: the checkbox leads the row.
                    // Decision 11: the audition slot trails it. Each is
                    // `None` until its owning decision fills it, so neither
                    // reserves space here.
                    checkbox
                        .into_iter()
                        .chain(self.children)
                        .chain(self.audition)
                        .chain(menu_cell),
                ),
            )
            .when(has_menu_cell, |el| el.group(TRACK_ROW_GROUP))
            .when_some(self.scroll_probe, |el, probe| el.relative().child(probe));
        row = menu_root.attach(row);

        if let Some(handler) = on_click {
            row = row
                .cursor_pointer()
                .hover(move |el| el.bg(hover_bg))
                .on_click(move |event, window, cx| {
                    handler(event, window, cx);
                });
        }

        // ADR 0083 task 008, decision D12: the drag starts on the same
        // element as the click, so a drag clears the pending click.
        if let Some(drag) = &self.drag {
            row = attach_track_drag(row, TrackDragPayload::from_vm(drag));
        }

        row
    }
}

/// The menu of a list row: the view model of the menu, the playlists of
/// its "Add to playlist" level, its handlers and the focus that the
/// right-click menu gives back (ADR 0083 task 007).
#[derive(Clone)]
pub(crate) struct RowMenu {
    pub(crate) menu: TrackRowMenuVm,
    pub(crate) playlists: Rc<Vec<PlaylistOption>>,
    pub(crate) handlers: TrackRowMenuHandlers,
    pub(crate) return_focus: FocusHandle,
}

/// The part of a row menu that the row root holds: the right-click
/// handler and the open right-click menu.
pub(crate) struct RowMenuRoot {
    state: Option<Entity<TrackRowMenuState>>,
    pointer_menu: Option<PointerContextMenu>,
}

impl RowMenuRoot {
    /// A row with no menu: no right-click handler and no menu.
    pub(crate) const fn empty() -> Self {
        Self {
            state: None,
            pointer_menu: None,
        }
    }

    /// Attaches the right-click handler and the open right-click menu to
    /// `root`. A row with no menu does not change.
    pub(crate) fn attach<E: InteractiveElement + ParentElement>(self, root: E) -> E {
        let mut root = root;
        if let Some(state) = self.state {
            root = root.on_mouse_down(MouseButton::Right, move |event, _, cx| {
                state.update(cx, |state, cx| {
                    state.open_at(event.position);
                    cx.notify();
                });
                cx.stop_propagation();
            });
        }
        root.children(self.pointer_menu)
    }
}

/// The menu cell and the root part of a row menu for one render (ADR 0083
/// Decision 6). `cell` is the fixed column of the "⋯" button. With no
/// menu, the cell is empty and keeps its width, and the root part is
/// empty.
pub(crate) fn row_menu_parts(
    menu: Option<RowMenu>,
    cell: Div,
    focused: bool,
    window: &mut Window,
    cx: &mut App,
) -> (TrackRowMenuCell, RowMenuRoot) {
    let menu_cell = TrackRowMenuCell::new(cell, focused);
    let Some(RowMenu {
        menu,
        playlists,
        handlers,
        return_focus,
    }) = menu
    else {
        return (menu_cell, RowMenuRoot::empty());
    };
    let state = row_menu_state(&menu.id, window, cx);
    let items = track_row_menu_items(&menu, &playlists, &handlers, &state);
    let pointer_menu = state.read(cx).pointer().map(|position| {
        track_row_pointer_menu(&menu, position, return_focus, items.clone(), &state)
    });
    (
        menu_cell.menu(menu, items, state.clone()),
        RowMenuRoot {
            state: Some(state),
            pointer_menu,
        },
    )
}

/// The fixed menu column of a list row. Its width is the token width of
/// the menu column of a track row, so the hover fill of the button is not
/// cut. It has the height of a hit target, so each row has the same
/// height with or without a menu.
pub(crate) fn menu_cell(cx: &App) -> Div {
    div()
        .w(TrackRowColumn::Menu.scaled(cx))
        .flex_shrink_0()
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .min_h(Size::MinHitTarget.scaled(cx))
        .debug_selector(|| "list-row-menu-cell".to_owned())
}

/// The outline of a list row and of a selectable tile (ADR 0084 Decision
/// 10).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RowOutline {
    None,
    Selected,
    Focus,
}

/// The keyboard focus outline wins, so each focused row shows it, selected
/// or not. The operator found on 2026-10-19 that the focus showed only on a
/// selected row.
pub(crate) const fn row_outline(selected: bool, focused: bool) -> RowOutline {
    match (selected, focused) {
        (_, true) => RowOutline::Focus,
        (true, false) => RowOutline::Selected,
        (false, false) => RowOutline::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{px, Bounds, Pixels, TestAppContext, VisualTestContext};

    use crate::ui::tokens::ScaleFactor;
    use crate::view_models::release_page::TrackRowMenuFacts;

    /// The state of the menu cell under test.
    #[derive(Clone, Copy, Debug)]
    enum CellCase {
        /// The row has a menu cell with no menu.
        Empty,
        /// The row has a menu, and the cell is hidden until a hover.
        Hidden,
        /// The row has the keyboard focus, so the cell is visible.
        Visible,
    }

    struct MenuCellTest {
        case: CellCase,
        focus: FocusHandle,
    }

    impl gpui::Render for MenuCellTest {
        fn render(
            &mut self,
            _window: &mut Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl IntoElement {
            let menu = TrackRowMenuVm::project(&TrackRowMenuFacts {
                inspect_sources: true,
                ..TrackRowMenuFacts::new("row", "Song", false)
            })
            .map(|menu| RowMenu {
                menu,
                playlists: Rc::new(Vec::new()),
                handlers: TrackRowMenuHandlers {
                    action: Rc::new(|_, _, _| {}),
                    select_playlist: Rc::new(|_, _, _| {}),
                    create_playlist: Rc::new(|_, _, _| {}),
                },
                return_focus: self.focus.clone(),
            });
            let (menu, focused) = match self.case {
                CellCase::Empty => (None, false),
                CellCase::Hidden => (menu, false),
                CellCase::Visible => (menu, true),
            };
            div().w(px(600.0)).child(
                ListRow::new("row")
                    .focused(focused)
                    .child(div().flex_1().min_w_0().child("Song"))
                    .menu_cell(menu),
            )
        }
    }

    fn cell_bounds(cx: &mut VisualTestContext) -> Bounds<Pixels> {
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        cx.debug_bounds("list-row-menu-cell")
            .expect("the menu cell bounds are recorded")
    }

    /// R83-7-12 (ADR 0083 Decision 6, task 007): the menu cell of
    /// `ListRow` has its token width when it is empty, hidden or visible.
    /// The cell starts at the same position in each case.
    #[gpui::test]
    fn adr_0083_list_row_actions_r83_7_12_menu_cell_keeps_its_width(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (view, cx) = cx.add_window_view(|_, cx| MenuCellTest {
            case: CellCase::Empty,
            focus: cx.focus_handle(),
        });
        let mut cases = Vec::new();
        for case in [CellCase::Empty, CellCase::Hidden, CellCase::Visible] {
            view.update(cx, |this, cx| {
                this.case = case;
                cx.notify();
            });
            cases.push((case, cell_bounds(cx)));
        }
        let first = cases[0].1;
        for (case, bounds) in cases {
            assert_eq!(
                bounds.size.width,
                TrackRowColumn::Menu.scaled_px(ScaleFactor::Medium),
                "{case:?}: the cell must have the token width"
            );
            // The focus outline is 1 px wider than the plain border, so a
            // focused row moves its content by at most 1 px.
            let shift = bounds.origin.x - first.origin.x;
            assert!(
                shift >= px(-1.0) && shift <= px(1.0),
                "{case:?}: the cell must start at the same position, moved by {shift:?}"
            );
        }
    }

    #[test]
    fn adr_0084_selection_focus_outline_shows_on_each_focused_row() {
        assert_eq!(row_outline(false, true), RowOutline::Focus);
        assert_eq!(row_outline(true, true), RowOutline::Focus);
        assert_eq!(row_outline(true, false), RowOutline::Selected);
        assert_eq!(row_outline(false, false), RowOutline::None);
    }

    #[test]
    fn clickable_row_carries_accessibility_label() {
        let row = ListRow::new("result:1").a11y_label(ListRowA11yLabel {
            label: "Open search result".into(),
        });

        assert_eq!(
            row.a11y_label,
            Some(gpui::SharedString::from("Open search result"))
        );
    }
}
