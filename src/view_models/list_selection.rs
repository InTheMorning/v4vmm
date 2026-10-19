//! The shared selection model of a Releases or Tracks list (ADR 0084
//! Decision 10).
//!
//! `ListSelection<K, R>` is pure and carries no renderer type. It knows no
//! list type: each operation takes the visible rows, in their render
//! sequence, so the model can move the focus and extend a range without
//! reading any screen state.
//!
//! Operator Correction, 2026-10-18: a row is the unit of focus and of a
//! Shift+click range. A row has one or more members. A release row and a
//! Tracks row have one member. A singles group row has each single of
//! its group, and an open group also gives one row for each single, each
//! with its own focus and its own checkbox. `K` is the member key that
//! the selected set holds. `R` is the row's own identity, which the focus
//! and the anchor hold. A row of one member can set `R` to `K`, its own
//! key: [`SelectionRow::of_one`] builds this shape, and the Tracks and
//! the Needs attention lists use it for every row.

#![warn(clippy::pedantic)]

use std::collections::BTreeSet;

/// A keyboard focus step (ADR 0084 Decision 10): Up, Down, Page Up, Page
/// Down, Home and End.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FocusStep {
    /// Up. Moves the focus back by one row.
    Up,
    /// Down. Moves the focus forward by one row.
    Down,
    /// Page Up. Moves the focus back by [`SELECTION_PAGE_STEP`] rows.
    PageUp,
    /// Page Down. Moves the focus forward by [`SELECTION_PAGE_STEP`] rows.
    PageDown,
    /// Home. Moves the focus to the first row.
    Home,
    /// End. Moves the focus to the last row.
    End,
}

/// The number of rows a Page Up or a Page Down step moves.
pub(crate) const SELECTION_PAGE_STEP: usize = 10;

/// One visible row, given to a row-aware [`ListSelection`] operation
/// (Required correction item 1, 2026-10-18): the row's own identity, for
/// the focus and the anchor, and the member keys that the row's
/// checkbox, Space, and a range select act on.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectionRow<R: Clone + Eq, K: Clone + Eq + Ord> {
    /// The row's own identity: what the focus and the anchor hold.
    pub(crate) id: R,
    /// The member keys of the row. A release row and a Tracks row have
    /// one. A singles group row has each single of its group.
    pub(crate) members: Vec<K>,
}

impl<R: Clone + Eq, K: Clone + Eq + Ord> SelectionRow<R, K> {
    /// Builds a row of `id` and its `members`.
    #[must_use]
    pub(crate) fn new(id: R, members: Vec<K>) -> Self {
        Self { id, members }
    }
}

impl<T: Clone + Eq + Ord> SelectionRow<T, T> {
    /// A row of one member, whose own identity is that member's key
    /// (Required correction item 2: "The Tracks lists keep one member
    /// for each row").
    #[must_use]
    pub(crate) fn of_one(key: T) -> Self {
        Self {
            id: key.clone(),
            members: vec![key],
        }
    }
}

/// The checked state of one row's checkbox (Required correction item 5,
/// 2026-10-18). A row of two or more members, for example a singles
/// group row, can read mixed: some of its members are selected, but not
/// every one. A row of one member never reads mixed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RowCheckState {
    /// Every member of the row is selected.
    Checked,
    /// Some, but not every, member of the row is selected.
    Mixed,
    /// No member of the row is selected.
    Clear,
}

impl RowCheckState {
    /// Builds the state of a row of one member, from a plain `bool`. One
    /// member is either selected or not, so this never gives
    /// [`Self::Mixed`].
    #[must_use]
    pub(crate) const fn from_checked(checked: bool) -> Self {
        if checked {
            Self::Checked
        } else {
            Self::Clear
        }
    }

    /// Tells if the state is [`Self::Checked`].
    #[must_use]
    pub(crate) const fn is_checked(self) -> bool {
        matches!(self, Self::Checked)
    }
}

/// The selection of one list (ADR 0084 Decision 10).
///
/// `LibraryApp` holds one of these for the Releases list and one for the
/// track lists. Neither list type is named here: a caller gives the
/// visible rows, in their render sequence, to each operation that needs
/// them. `K` is the member key, held in the selected set. `R` is the
/// row's own identity, held by the focus and the anchor; it defaults to
/// `K` for a list whose every row has one member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ListSelection<K: Clone + Eq + Ord, R: Clone + Eq = K> {
    selected: BTreeSet<K>,
    anchor: Option<R>,
    focus: Option<R>,
}

// `derive(Default)` would add a `K: Default` bound that this type never
// needs: an empty selection holds no key of any kind.
impl<K: Clone + Eq + Ord, R: Clone + Eq> Default for ListSelection<K, R> {
    fn default() -> Self {
        Self {
            selected: BTreeSet::new(),
            anchor: None,
            focus: None,
        }
    }
}

impl<K: Clone + Eq + Ord, R: Clone + Eq> ListSelection<K, R> {
    /// An empty selection, with no focus and no anchor.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// A plain click (Required correction item 1: a row is the unit of
    /// focus). It changes no selection. The caller opens the page of the
    /// row; this only moves the focus to the row's own identity.
    pub(crate) fn click(&mut self, row_id: R) {
        self.focus = Some(row_id);
    }

    /// Ctrl+click, the checkbox, and Space (Required correction item 3):
    /// when every member of `row` is selected, removes each member. Else
    /// adds each member. The anchor and the focus move to the row.
    pub(crate) fn toggle(&mut self, row: &SelectionRow<R, K>) {
        if Self::all_selected(&self.selected, &row.members) {
            for member in &row.members {
                self.selected.remove(member);
            }
        } else {
            for member in &row.members {
                self.selected.insert(member.clone());
            }
        }
        self.anchor = Some(row.id.clone());
        self.focus = Some(row.id.clone());
    }

    /// Shift+click (Required correction item 3). Selects every member of
    /// every row from the anchor row to `row`, in the `visible` sequence.
    /// With no anchor yet, `row` becomes the anchor, so a later
    /// Shift+click extends from this same row.
    pub(crate) fn extend_to(&mut self, row: &SelectionRow<R, K>, visible: &[SelectionRow<R, K>]) {
        let anchor = if let Some(anchor) = &self.anchor {
            anchor.clone()
        } else {
            self.anchor = Some(row.id.clone());
            row.id.clone()
        };
        let start = visible
            .iter()
            .position(|visible_row| visible_row.id == anchor);
        let end = visible
            .iter()
            .position(|visible_row| visible_row.id == row.id);
        match (start, end) {
            (Some(start), Some(end)) => {
                let (lo, hi) = if start <= end {
                    (start, end)
                } else {
                    (end, start)
                };
                for visible_row in &visible[lo..=hi] {
                    for member in &visible_row.members {
                        self.selected.insert(member.clone());
                    }
                }
            }
            _ => {
                for member in &row.members {
                    self.selected.insert(member.clone());
                }
            }
        }
        self.focus = Some(row.id.clone());
    }

    /// Ctrl+A. Selects every member of every row of `visible`.
    pub(crate) fn select_all(&mut self, visible: &[SelectionRow<R, K>]) {
        self.selected = visible
            .iter()
            .flat_map(|row| row.members.iter().cloned())
            .collect();
    }

    /// Space (Required correction item 3). Toggles the row with the
    /// keyboard focus. Does nothing with no focused row, or when
    /// `visible` no longer holds it.
    pub(crate) fn toggle_focused(&mut self, visible: &[SelectionRow<R, K>]) {
        let Some(focus) = self.focus.clone() else {
            return;
        };
        if let Some(row) = visible.iter().find(|row| row.id == focus) {
            self.toggle(row);
        }
    }

    /// Up, Down, Page Up, Page Down, Home and End. Moves the focus among
    /// the rows of `visible` and returns the newly focused row's own
    /// identity, so the caller can keep it in view. Stops at the first
    /// and the last row. Gives `None` when `visible` is empty.
    pub(crate) fn move_focus(
        &mut self,
        step: FocusStep,
        visible: &[SelectionRow<R, K>],
    ) -> Option<R> {
        if visible.is_empty() {
            self.focus = None;
            return None;
        }
        let last = visible.len() - 1;
        let current = self
            .focus
            .as_ref()
            .and_then(|focused| visible.iter().position(|row| row.id == *focused));
        let next_index = match step {
            FocusStep::Up => current.map_or(0, |index| index.saturating_sub(1)),
            FocusStep::Down => current.map_or(0, |index| (index + 1).min(last)),
            FocusStep::PageUp => {
                current.map_or(0, |index| index.saturating_sub(SELECTION_PAGE_STEP))
            }
            FocusStep::PageDown => {
                current.map_or(0, |index| (index + SELECTION_PAGE_STEP).min(last))
            }
            FocusStep::Home => 0,
            FocusStep::End => last,
        };
        let next = visible[next_index].id.clone();
        self.focus = Some(next.clone());
        Some(next)
    }

    /// "Clear" and Escape. Clears the selection and the anchor. Keeps the
    /// keyboard focus, so Up and Down keep working right after.
    pub(crate) fn clear(&mut self) {
        self.selected.clear();
        self.anchor = None;
    }

    /// Drops each selected key that `visible_members` no longer names,
    /// for example after a removal or a filter.
    pub(crate) fn retain_visible(&mut self, visible_members: &[K]) {
        let visible: BTreeSet<&K> = visible_members.iter().collect();
        self.selected.retain(|key| visible.contains(key));
    }

    /// The number of selected rows.
    #[must_use]
    pub(crate) fn selected_count(&self) -> usize {
        self.selected.len()
    }

    /// Tells if `key` is selected.
    #[must_use]
    pub(crate) fn is_selected(&self, key: &K) -> bool {
        self.selected.contains(key)
    }

    /// The row with the keyboard focus, when one exists.
    #[must_use]
    pub(crate) fn focused(&self) -> Option<&R> {
        self.focus.as_ref()
    }

    /// Moves the keyboard focus to `row`, with no change to the selection
    /// (ADR 0084 Decision 10: Left on a single moves to its group row).
    pub(crate) fn focus_row(&mut self, row: R) {
        self.focus = Some(row);
    }

    /// Tells if a checkbox shows on each row (ADR 0084 Decision 10): true
    /// once a selection exists.
    #[must_use]
    pub(crate) fn shows_checkboxes(&self) -> bool {
        !self.selected.is_empty()
    }

    /// Every selected key, for building the action bar and the removal
    /// confirmation.
    pub(crate) fn selected_keys(&self) -> impl Iterator<Item = &K> {
        self.selected.iter()
    }

    /// The checked state of a row's `members` (Required correction item
    /// 5): checked when the row has one or more members and every one is
    /// selected, mixed when some but not every member is selected, and
    /// clear when none is.
    #[must_use]
    pub(crate) fn row_check_state(&self, members: &[K]) -> RowCheckState {
        if members.is_empty() {
            return RowCheckState::Clear;
        }
        let selected_count = members.iter().filter(|key| self.is_selected(key)).count();
        if selected_count == members.len() {
            RowCheckState::Checked
        } else if selected_count == 0 {
            RowCheckState::Clear
        } else {
            RowCheckState::Mixed
        }
    }

    fn all_selected(selected: &BTreeSet<K>, members: &[K]) -> bool {
        !members.is_empty() && members.iter().all(|key| selected.contains(key))
    }
}

/// The selection facts of one list row (ADR 0084 Decision 10): whether its
/// checkbox shows, its checked state, whether it carries the keyboard
/// focus, and the checkbox's accessibility label.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RowSelectionVm {
    pub(crate) checkbox_visible: bool,
    /// The checkbox's typed checked state (Required correction item 5):
    /// checked, mixed, or clear, never a second boolean beside a plain
    /// checked flag.
    pub(crate) check_state: RowCheckState,
    pub(crate) focused: bool,
    pub(crate) checkbox_a11y_label: String,
}

impl RowSelectionVm {
    /// No selection exists for this row's list, and the row is not the
    /// keyboard focus. The checkbox stays hidden until the pointer is on
    /// the row, which is the screen's own hover state (ADR 0084 Decision
    /// 10 Design: "The row").
    #[must_use]
    pub(crate) fn none() -> Self {
        Self {
            checkbox_visible: false,
            check_state: RowCheckState::Clear,
            focused: false,
            checkbox_a11y_label: String::new(),
        }
    }

    /// Builds the selection facts of a row of one member, named `title`
    /// (a release row, a Tracks row, or a Needs attention row).
    /// `checkbox_visible` is true when a selection exists or the row has
    /// the keyboard focus (ADR 0084 Decision 10 Design: "The row"). The
    /// pointer-hover case is the screen's own state and is applied there,
    /// not here.
    #[must_use]
    pub(crate) fn for_row(
        title: &str,
        checked: bool,
        focused: bool,
        shows_checkboxes: bool,
    ) -> Self {
        Self {
            checkbox_visible: shows_checkboxes || focused,
            check_state: RowCheckState::from_checked(checked),
            focused,
            checkbox_a11y_label: format!("Select {title}"),
        }
    }

    /// Builds the selection facts of a singles group row (Required
    /// correction item 5, 2026-10-18). `check_state` can read mixed: some,
    /// but not every, single of the group is selected. The accessibility
    /// label states the mixed fact in words, so the state never reads
    /// through color alone.
    #[must_use]
    pub(crate) fn for_group_row(
        title: &str,
        check_state: RowCheckState,
        focused: bool,
        shows_checkboxes: bool,
    ) -> Self {
        let checkbox_a11y_label = match check_state {
            RowCheckState::Mixed => format!("{title}, some singles selected"),
            RowCheckState::Checked | RowCheckState::Clear => format!("Select {title}"),
        };
        Self {
            checkbox_visible: shows_checkboxes || focused,
            check_state,
            focused,
            checkbox_a11y_label,
        }
    }

    /// Tells if the row's background shows the selected mark: the
    /// checkbox reads checked or mixed.
    #[must_use]
    pub(crate) const fn is_marked(&self) -> bool {
        !matches!(self.check_state, RowCheckState::Clear)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(values: &[i32]) -> Vec<SelectionRow<i32, i32>> {
        values.iter().copied().map(SelectionRow::of_one).collect()
    }

    /// R9-01: `toggle`, `extend_to`, `select_all`, `toggle_focused` and
    /// `clear` give the selection of Decision 10 on a recorded list of 6
    /// rows. `extend_to` follows the visible sequence.
    #[test]
    fn adr_0084_selection_r9_01_operations_give_the_decision_10_selection() {
        let visible = rows(&[1, 2, 3, 4, 5, 6]);
        let mut selection: ListSelection<i32, i32> = ListSelection::new();

        // toggle: Ctrl+click adds, then a second toggle removes.
        selection.toggle(&SelectionRow::of_one(2));
        assert!(selection.is_selected(&2));
        assert_eq!(selection.selected_count(), 1);
        selection.toggle(&SelectionRow::of_one(2));
        assert!(!selection.is_selected(&2));
        assert_eq!(selection.selected_count(), 0);

        // extend_to: Shift+click selects the anchor-to-row range, in the
        // visible sequence, not in key order.
        selection.toggle(&SelectionRow::of_one(5));
        selection.extend_to(&SelectionRow::of_one(3), &visible);
        assert_eq!(selection.selected_count(), 3);
        for key in [3, 4, 5] {
            assert!(selection.is_selected(&key), "{key} is in the range");
        }
        assert!(!selection.is_selected(&1));
        assert!(!selection.is_selected(&6));

        // select_all: Ctrl+A selects every visible row.
        selection.select_all(&visible);
        assert_eq!(selection.selected_count(), 6);

        // clear: "Clear" and Escape empty the selection.
        selection.clear();
        assert_eq!(selection.selected_count(), 0);
        assert!(!selection.shows_checkboxes());

        // toggle_focused: Space toggles the row with the keyboard focus.
        // `clear()` keeps the keyboard focus, so Home gives a known
        // starting row here.
        selection.move_focus(FocusStep::Home, &visible);
        selection.toggle_focused(&visible);
        assert_eq!(selection.selected_count(), 1);
        assert!(selection.is_selected(&1));
    }

    /// R9-01: `extend_to` follows the visible sequence, not ascending key
    /// order, when the anchor is after the target row in that sequence.
    #[test]
    fn adr_0084_selection_r9_01_extend_to_follows_the_visible_sequence() {
        let visible = rows(&[6, 5, 4, 3, 2, 1]);
        let mut selection: ListSelection<i32, i32> = ListSelection::new();

        selection.toggle(&SelectionRow::of_one(5));
        selection.extend_to(&SelectionRow::of_one(2), &visible);

        for key in [5, 4, 3, 2] {
            assert!(selection.is_selected(&key));
        }
        assert!(!selection.is_selected(&6));
        assert!(!selection.is_selected(&1));
    }

    /// R9-02: `move_focus` gives the next row for ±1, ±10, Home and End,
    /// and stops at the first and the last row.
    #[test]
    fn adr_0084_selection_r9_02_move_focus_steps_and_stops_at_the_ends() {
        let visible = rows(&(0..20).collect::<Vec<_>>());
        let mut selection: ListSelection<i32, i32> = ListSelection::new();

        assert_eq!(selection.move_focus(FocusStep::Down, &visible), Some(0));
        assert_eq!(selection.move_focus(FocusStep::Down, &visible), Some(1));
        assert_eq!(selection.move_focus(FocusStep::Up, &visible), Some(0));
        // Up at the first row stops there.
        assert_eq!(selection.move_focus(FocusStep::Up, &visible), Some(0));

        assert_eq!(
            selection.move_focus(FocusStep::PageDown, &visible),
            Some(10)
        );
        assert_eq!(
            selection.move_focus(FocusStep::PageDown, &visible),
            Some(19)
        );
        // Page Down at the last row stops there.
        assert_eq!(
            selection.move_focus(FocusStep::PageDown, &visible),
            Some(19)
        );
        assert_eq!(selection.move_focus(FocusStep::PageUp, &visible), Some(9));

        assert_eq!(selection.move_focus(FocusStep::End, &visible), Some(19));
        assert_eq!(selection.move_focus(FocusStep::Home, &visible), Some(0));

        let empty: Vec<SelectionRow<i32, i32>> = Vec::new();
        assert_eq!(selection.move_focus(FocusStep::Down, &empty), None);
        assert_eq!(selection.focused(), None);
    }

    /// R9-03: `retain_visible` drops each key that the list no longer
    /// shows.
    #[test]
    fn adr_0084_selection_r9_03_retain_visible_drops_hidden_keys() {
        let mut selection: ListSelection<i32> = ListSelection::new();
        selection.select_all(&rows(&[1, 2, 3, 4]));
        assert_eq!(selection.selected_count(), 4);

        selection.retain_visible(&[2, 4]);

        assert_eq!(selection.selected_count(), 2);
        assert!(selection.is_selected(&2));
        assert!(selection.is_selected(&4));
        assert!(!selection.is_selected(&1));
        assert!(!selection.is_selected(&3));
    }

    /// R9-04: a group row's `toggle` adds every member when one or more
    /// is not selected, and removes every member when each one already
    /// is.
    #[test]
    fn adr_0084_selection_r9_04_toggle_group_adds_or_removes_every_member() {
        let members = [10, 11, 12];
        let group: SelectionRow<i32, i32> = SelectionRow::new(100, members.to_vec());
        let mut selection: ListSelection<i32, i32> = ListSelection::new();

        selection.toggle(&group);
        assert_eq!(selection.selected_count(), 3);
        for key in &members {
            assert!(selection.is_selected(key));
        }

        selection.toggle(&group);
        assert_eq!(selection.selected_count(), 0);
    }

    /// R9-05: `RowSelectionVm` shows the checkbox for a row with the
    /// focus, and for each row while a selection exists. It carries the
    /// "Select <title>" label.
    #[test]
    fn adr_0084_selection_r9_05_row_selection_vm_shows_the_checkbox() {
        let none = RowSelectionVm::for_row("Face Pale", false, false, false);
        assert!(!none.checkbox_visible);

        let focused_only = RowSelectionVm::for_row("Face Pale", false, true, false);
        assert!(focused_only.checkbox_visible);
        assert_eq!(focused_only.check_state, RowCheckState::Clear);

        let during_selection = RowSelectionVm::for_row("Face Pale", true, false, true);
        assert!(during_selection.checkbox_visible);
        assert_eq!(during_selection.check_state, RowCheckState::Checked);
        assert_eq!(during_selection.checkbox_a11y_label, "Select Face Pale");
    }

    /// R9-11: Space on a focused group row adds each single, and Space
    /// again removes each single.
    #[test]
    fn adr_0084_selection_r9_11_space_on_group_row_adds_then_removes_every_single() {
        let group: SelectionRow<i32, i32> = SelectionRow::new(100, vec![10, 11, 12]);
        let visible = vec![group.clone()];
        let mut selection: ListSelection<i32, i32> = ListSelection::new();

        selection.click(100);
        selection.toggle_focused(&visible);
        assert_eq!(selection.selected_count(), 3);
        for key in [10, 11, 12] {
            assert!(selection.is_selected(&key));
        }

        selection.toggle_focused(&visible);
        assert_eq!(selection.selected_count(), 0);
    }

    /// R9-12: an open group gives one focus row for the group and one
    /// for each single. Down from the group row focuses its first
    /// single.
    #[test]
    fn adr_0084_selection_r9_12_open_group_gives_a_focus_row_for_each_single() {
        let visible: Vec<SelectionRow<i32, i32>> = vec![
            SelectionRow::new(100, vec![10, 11]),
            SelectionRow::of_one(10),
            SelectionRow::of_one(11),
        ];
        let mut selection: ListSelection<i32, i32> = ListSelection::new();

        assert_eq!(selection.move_focus(FocusStep::Down, &visible), Some(100));
        assert_eq!(selection.move_focus(FocusStep::Down, &visible), Some(10));
        assert_eq!(selection.move_focus(FocusStep::Down, &visible), Some(11));
    }

    /// R9-13: the checkbox of a single row toggles only that single. The
    /// group checkbox then reads mixed.
    #[test]
    fn adr_0084_selection_r9_13_single_row_checkbox_toggles_only_that_single() {
        let mut selection: ListSelection<i32, i32> = ListSelection::new();

        selection.toggle(&SelectionRow::of_one(10));

        assert!(selection.is_selected(&10));
        assert!(!selection.is_selected(&11));
        assert_eq!(selection.row_check_state(&[10, 11]), RowCheckState::Mixed);
    }

    /// R9-14: the group checkbox reads checked when each single is
    /// selected, mixed when some are, and clear when none is. A mixed
    /// row's own toggle adds every member; it never removes the member
    /// that is already selected.
    #[test]
    fn adr_0084_selection_r9_14_group_check_state_tracks_its_members() {
        let members = [10, 11, 12];
        let group: SelectionRow<i32, i32> = SelectionRow::new(100, members.to_vec());
        let mut selection: ListSelection<i32, i32> = ListSelection::new();
        assert_eq!(selection.row_check_state(&members), RowCheckState::Clear);

        selection.toggle(&SelectionRow::of_one(10));
        assert_eq!(selection.row_check_state(&members), RowCheckState::Mixed);

        selection.toggle(&group);
        assert_eq!(selection.row_check_state(&members), RowCheckState::Checked);
        for key in members {
            assert!(selection.is_selected(&key));
        }

        selection.toggle(&group);
        assert_eq!(selection.row_check_state(&members), RowCheckState::Clear);
    }

    /// R9-14: `RowSelectionVm::for_group_row` states the mixed fact in
    /// words, so the checkbox never reads through color alone.
    #[test]
    fn adr_0084_selection_r9_14_group_row_vm_states_mixed_in_words() {
        let mixed = RowSelectionVm::for_group_row("Haleen", RowCheckState::Mixed, false, true);
        assert_eq!(mixed.check_state, RowCheckState::Mixed);
        assert_eq!(mixed.checkbox_a11y_label, "Haleen, some singles selected");

        let checked = RowSelectionVm::for_group_row("Haleen", RowCheckState::Checked, false, true);
        assert_eq!(checked.checkbox_a11y_label, "Select Haleen");
    }

    /// R9-15: Shift+click from a release row to a group row selects each
    /// member of each row between them.
    #[test]
    fn adr_0084_selection_r9_15_shift_click_spans_a_release_row_to_a_group_row() {
        let release_row: SelectionRow<i32, i32> = SelectionRow::of_one(1);
        let visible: Vec<SelectionRow<i32, i32>> = vec![
            release_row.clone(),
            SelectionRow::of_one(2),
            SelectionRow::new(100, vec![20, 21]),
        ];
        let mut selection: ListSelection<i32, i32> = ListSelection::new();

        selection.toggle(&release_row);
        selection.extend_to(&visible[2], &visible);

        for key in [1, 2, 20, 21] {
            assert!(selection.is_selected(&key), "{key} is in the range");
        }
        assert_eq!(selection.selected_count(), 4);
    }
}
