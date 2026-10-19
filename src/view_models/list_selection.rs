//! The shared selection model of a Releases or Tracks list (ADR 0084
//! Decision 10).
//!
//! `ListSelection<K>` is pure and carries no renderer type. It knows no
//! list type: each operation takes the visible row keys in their render
//! sequence, so the model can move the focus and extend a range without
//! reading any screen state.

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

/// The selection of one list (ADR 0084 Decision 10).
///
/// `LibraryApp` holds one of these for the Releases list and one for the
/// track lists. Neither list type is named here: a caller gives the
/// visible row keys, in their render sequence, to each operation that
/// needs them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ListSelection<K: Clone + Eq + Ord> {
    selected: BTreeSet<K>,
    anchor: Option<K>,
    focus: Option<K>,
}

// `derive(Default)` would add a `K: Default` bound that this type never
// needs: an empty selection holds no key of any kind.
impl<K: Clone + Eq + Ord> Default for ListSelection<K> {
    fn default() -> Self {
        Self {
            selected: BTreeSet::new(),
            anchor: None,
            focus: None,
        }
    }
}

impl<K: Clone + Eq + Ord> ListSelection<K> {
    /// An empty selection, with no focus and no anchor.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// A plain click. It changes no selection. The caller opens the page
    /// of the row; this only moves the focus to it.
    pub(crate) fn click(&mut self, key: K) {
        self.focus = Some(key);
    }

    /// Ctrl+click and the checkbox. Adds or removes `key`. The anchor and
    /// the focus move to it.
    pub(crate) fn toggle(&mut self, key: K) {
        if !self.selected.remove(&key) {
            self.selected.insert(key.clone());
        }
        self.anchor = Some(key.clone());
        self.focus = Some(key);
    }

    /// The checkbox of a singles group row (ADR 0084 Decision 6). `keys`
    /// is every single of the group. The row is checked when each single
    /// is selected; this adds every single when one or more is not
    /// selected, and removes every single when each one already is. The
    /// anchor and the focus move to the first key.
    pub(crate) fn toggle_group(&mut self, keys: &[K]) {
        let Some(first) = keys.first().cloned() else {
            return;
        };
        if Self::all_selected(&self.selected, keys) {
            for key in keys {
                self.selected.remove(key);
            }
        } else {
            for key in keys {
                self.selected.insert(key.clone());
            }
        }
        self.anchor = Some(first.clone());
        self.focus = Some(first);
    }

    /// Shift+click. Selects each row from the anchor to `key`, in the
    /// `visible` sequence. With no anchor yet, `key` becomes the anchor,
    /// so a later Shift+click extends from this same row.
    pub(crate) fn extend_to(&mut self, key: K, visible: &[K]) {
        let anchor = if let Some(anchor) = &self.anchor {
            anchor.clone()
        } else {
            self.anchor = Some(key.clone());
            key.clone()
        };
        let start = visible
            .iter()
            .position(|visible_key| *visible_key == anchor);
        let end = visible.iter().position(|visible_key| *visible_key == key);
        match (start, end) {
            (Some(start), Some(end)) => {
                let (lo, hi) = if start <= end {
                    (start, end)
                } else {
                    (end, start)
                };
                for visible_key in &visible[lo..=hi] {
                    self.selected.insert(visible_key.clone());
                }
            }
            _ => {
                self.selected.insert(key.clone());
            }
        }
        self.focus = Some(key);
    }

    /// Ctrl+A. Selects every row of `visible`.
    pub(crate) fn select_all(&mut self, visible: &[K]) {
        self.selected = visible.iter().cloned().collect();
    }

    /// Space. Toggles the row with the keyboard focus. Does nothing with
    /// no focused row.
    pub(crate) fn toggle_focused(&mut self) {
        if let Some(focus) = self.focus.clone() {
            self.toggle(focus);
        }
    }

    /// Up, Down, Page Up, Page Down, Home and End. Moves the focus inside
    /// `visible` and returns the newly focused key, so the caller can
    /// keep it in view. Stops at the first and the last row. Gives `None`
    /// when `visible` is empty.
    pub(crate) fn move_focus(&mut self, step: FocusStep, visible: &[K]) -> Option<K> {
        if visible.is_empty() {
            self.focus = None;
            return None;
        }
        let last = visible.len() - 1;
        let current = self
            .focus
            .as_ref()
            .and_then(|focused| visible.iter().position(|key| key == focused));
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
        let next = visible[next_index].clone();
        self.focus = Some(next.clone());
        Some(next)
    }

    /// "Clear" and Escape. Clears the selection and the anchor. Keeps the
    /// keyboard focus, so Up and Down keep working right after.
    pub(crate) fn clear(&mut self) {
        self.selected.clear();
        self.anchor = None;
    }

    /// Drops each selected key that `visible` no longer shows, for
    /// example after a removal or a filter.
    pub(crate) fn retain_visible(&mut self, visible: &[K]) {
        let visible: BTreeSet<&K> = visible.iter().collect();
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
    pub(crate) fn focused(&self) -> Option<&K> {
        self.focus.as_ref()
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

    fn all_selected(selected: &BTreeSet<K>, keys: &[K]) -> bool {
        !keys.is_empty() && keys.iter().all(|key| selected.contains(key))
    }
}

/// The selection facts of one list row (ADR 0084 Decision 10): whether its
/// checkbox shows, whether it is checked, whether it carries the keyboard
/// focus, and the checkbox's accessibility label.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RowSelectionVm {
    pub(crate) checkbox_visible: bool,
    pub(crate) checked: bool,
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
            checked: false,
            focused: false,
            checkbox_a11y_label: String::new(),
        }
    }

    /// Builds the selection facts of a row named `title`. `checkbox_visible`
    /// is true when a selection exists or the row has the keyboard focus
    /// (ADR 0084 Decision 10 Design: "The row"). The pointer-hover case is
    /// the screen's own state and is applied there, not here.
    #[must_use]
    pub(crate) fn for_row(
        title: &str,
        checked: bool,
        focused: bool,
        shows_checkboxes: bool,
    ) -> Self {
        Self {
            checkbox_visible: shows_checkboxes || focused,
            checked,
            focused,
            checkbox_a11y_label: format!("Select {title}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(values: &[i32]) -> Vec<i32> {
        values.to_vec()
    }

    /// R9-01: `toggle`, `extend_to`, `select_all`, `toggle_focused` and
    /// `clear` give the selection of Decision 10 on a recorded list of 6
    /// keys. `extend_to` follows the visible sequence.
    #[test]
    fn adr_0084_selection_r9_01_operations_give_the_decision_10_selection() {
        let visible = keys(&[1, 2, 3, 4, 5, 6]);
        let mut selection: ListSelection<i32> = ListSelection::new();

        // toggle: Ctrl+click adds, then a second toggle removes.
        selection.toggle(2);
        assert!(selection.is_selected(&2));
        assert_eq!(selection.selected_count(), 1);
        selection.toggle(2);
        assert!(!selection.is_selected(&2));
        assert_eq!(selection.selected_count(), 0);

        // extend_to: Shift+click selects the anchor-to-key range, in the
        // visible sequence, not in key order.
        selection.toggle(5);
        selection.extend_to(3, &visible);
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
        selection.toggle_focused();
        assert_eq!(selection.selected_count(), 1);
        assert!(selection.is_selected(&1));
    }

    /// R9-01: `extend_to` follows the visible sequence, not ascending key
    /// order, when the anchor is after the target key in that sequence.
    #[test]
    fn adr_0084_selection_r9_01_extend_to_follows_the_visible_sequence() {
        let visible = keys(&[6, 5, 4, 3, 2, 1]);
        let mut selection: ListSelection<i32> = ListSelection::new();

        selection.toggle(5);
        selection.extend_to(2, &visible);

        for key in [5, 4, 3, 2] {
            assert!(selection.is_selected(&key));
        }
        assert!(!selection.is_selected(&6));
        assert!(!selection.is_selected(&1));
    }

    /// R9-02: `move_focus` gives the next key for ±1, ±10, Home and End,
    /// and stops at the first and the last row.
    #[test]
    fn adr_0084_selection_r9_02_move_focus_steps_and_stops_at_the_ends() {
        let visible: Vec<i32> = (0..20).collect();
        let mut selection: ListSelection<i32> = ListSelection::new();

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

        let empty: Vec<i32> = Vec::new();
        assert_eq!(selection.move_focus(FocusStep::Down, &empty), None);
        assert_eq!(selection.focused(), None);
    }

    /// R9-03: `retain_visible` drops each key that the list no longer
    /// shows.
    #[test]
    fn adr_0084_selection_r9_03_retain_visible_drops_hidden_keys() {
        let mut selection: ListSelection<i32> = ListSelection::new();
        selection.select_all(&keys(&[1, 2, 3, 4]));
        assert_eq!(selection.selected_count(), 4);

        selection.retain_visible(&keys(&[2, 4]));

        assert_eq!(selection.selected_count(), 2);
        assert!(selection.is_selected(&2));
        assert!(selection.is_selected(&4));
        assert!(!selection.is_selected(&1));
        assert!(!selection.is_selected(&3));
    }

    /// Decision 6: `toggle_group` adds every member when one or more is
    /// not selected, and removes every member when each one already is.
    #[test]
    fn adr_0084_selection_r9_04_toggle_group_adds_or_removes_every_member() {
        let members = keys(&[10, 11, 12]);
        let mut selection: ListSelection<i32> = ListSelection::new();

        selection.toggle_group(&members);
        assert_eq!(selection.selected_count(), 3);
        for key in &members {
            assert!(selection.is_selected(key));
        }

        selection.toggle_group(&members);
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
        assert!(!focused_only.checked);

        let during_selection = RowSelectionVm::for_row("Face Pale", true, false, true);
        assert!(during_selection.checkbox_visible);
        assert!(during_selection.checked);
        assert_eq!(during_selection.checkbox_a11y_label, "Select Face Pale");
    }
}
