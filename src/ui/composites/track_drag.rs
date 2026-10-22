//! The drag of tracks to a playlist (ADR 0083 Decision 6, task 008).
//!
//! This composite holds the two drag payloads, the one drag preview, the
//! drag source of a track row and the drop target of a playlist row.
//!
//! - A `TrackRow` and a Tracks list row start a [`TrackDragPayload`]. In
//!   selection mode, a checked row carries each checked track (ADR 0083
//!   Decision 6, amended 2026-10-21; ADR 0084 task 031).
//! - The drag handle of a playlist entry starts a
//!   [`PlaylistTrackDragPayload`].
//! - Both drags show [`TrackDragPreview`].
//! - [`playlist_drop_target`] gives each drop to the handler of its host.
//!
//! The drag is for the mouse only (decision D11). The row menu and the
//! action bar stay the paths for the keyboard (ADR 0084 Decision 10).

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{
    div, prelude::*, AnyElement, App, Context, Div, Entity, FontWeight, MouseButton, Pixels, Point,
    SharedString, Stateful, Window,
};

use gpui_base::GlobalState;

use crate::ui::style::color;
use crate::ui::tokens::{color as token_color, FontSize, Radius, SemanticColor, Spacing};
use crate::view_models::library::{playlist_accepts_drop, TrackDragVm};

/// The drag of tracks from a `TrackRow` or from a Tracks list row
/// (decision D2). The payload keeps its own data, so the drop adds each
/// track after the source row unmounts.
#[derive(Clone, Debug)]
pub(crate) struct TrackDragPayload {
    /// The Library track ids, in list order.
    pub(crate) track_ids: Vec<i64>,
    /// The text of the preview: a title, or a count such as "3 tracks".
    pub(crate) label: SharedString,
}

impl TrackDragPayload {
    /// The payload of the drag fact of a row.
    #[must_use]
    pub(crate) fn from_vm(drag: &TrackDragVm) -> Self {
        Self {
            track_ids: drag.track_ids.clone(),
            label: SharedString::from(drag.label.clone()),
        }
    }
}

/// The drag of one playlist entry by its drag handle (ADR 0044). The
/// playlist rows of the same playlist use it to reorder. The playlist rows
/// of the Playlists list use `track_id` to add a copy of the track
/// (decision D2).
#[derive(Clone, Debug)]
pub(crate) struct PlaylistTrackDragPayload {
    pub(crate) playlist_id: i64,
    pub(crate) from_position: i64,
    pub(crate) track_id: i64,
    pub(crate) title: SharedString,
}

/// The one drag preview of a track (decision D9): the label in bold
/// `FontSize::Micro` text, with an accent border. The label is a title or
/// the count of a selection.
pub(crate) struct TrackDragPreview {
    title: SharedString,
}

impl TrackDragPreview {
    /// The preview entity that GPUI draws at the pointer during the drag.
    pub(crate) fn build(title: &SharedString, cx: &mut App) -> Entity<Self> {
        let title = title.clone();
        cx.new(|_| Self { title })
    }
}

impl Render for TrackDragPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .rounded(Radius::MD.scaled(cx))
            .border_1()
            .border_color(color::accent())
            .bg(color::bg_surface())
            .px(Spacing::MD.scaled(cx))
            .py(Spacing::SM.scaled(cx))
            .text_size(FontSize::Micro.scaled(cx))
            .font_weight(FontWeight::BOLD)
            .text_color(color::text_primary())
            .child(self.title.clone())
    }
}

/// Attaches the drag of one track to `el`, the clickable element of a row
/// (decision D12).
///
/// - The drag starts only after the pointer moves. GPUI then clears the
///   pending click of `el`, so a drag never opens the row page.
/// - The left mouse-down of `el` suppresses the text selection of the
///   window root, so the drag selects no text. The controls in `el` get
///   the mouse-down before `el`.
/// - The mouse-down continues to the ancestors of `el`. Thus a click on
///   the row gives the focus to its list column or detail column, also in
///   selection mode (ADR 0084 task 031, review correction C of
///   2026-10-21). Do not stop the propagation and do not prevent the
///   default here: GPUI then does not move the focus.
pub(crate) fn attach_track_drag(el: Stateful<Div>, payload: TrackDragPayload) -> Stateful<Div> {
    el.on_mouse_down(MouseButton::Left, |_, _, cx| {
        if cx.has_global::<GlobalState>() {
            GlobalState::suppress_text_selection(cx);
        }
    })
    .on_drag(
        payload,
        |payload: &TrackDragPayload, _: Point<Pixels>, _: &mut Window, cx: &mut App| {
            TrackDragPreview::build(&payload.label, cx)
        },
    )
}

/// One drop of tracks on a playlist row (ADR 0083 task 008, ADR 0084
/// task 031). The host adds each track of `track_ids`, in this order,
/// through one append.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlaylistDrop {
    pub(crate) track_ids: Vec<i64>,
    pub(crate) playlist_id: i64,
}

/// The handler of a drop on a playlist row.
pub(crate) type PlaylistDropHandler = Rc<dyn Fn(&PlaylistDrop, &mut Window, &mut App)>;

/// The shared handler of the drop targets of one list.
pub(crate) fn playlist_drop_handler(
    handler: impl Fn(&PlaylistDrop, &mut Window, &mut App) + 'static,
) -> PlaylistDropHandler {
    Rc::new(handler)
}

/// Makes `row` the drop target of a dragged track for the playlist
/// `playlist_id` (ADR 0083 Decision 6, task 008).
///
/// - The row accepts a track from a track row, and an entry of another
///   playlist (decision D7). It does not accept an entry of its own
///   playlist.
/// - An accepting row shows a 2 px accent outline while the pointer is on
///   it (decision D8). The outline is on a layer over the row, so the row
///   does not move.
/// - A drop gives one [`PlaylistDrop`] to `on_drop`, with each dragged
///   track.
pub(crate) fn playlist_drop_target(
    row: AnyElement,
    playlist_id: i64,
    on_drop: &PlaylistDropHandler,
    cx: &App,
) -> AnyElement {
    let accent = token_color(cx, SemanticColor::Accent);
    let on_track_drop = Rc::clone(on_drop);
    let on_drop = Rc::clone(on_drop);
    let target = div()
        .absolute()
        .inset_0()
        .rounded(Radius::SM.scaled(cx))
        .can_drop(move |drag, _window, _cx| {
            drag.downcast_ref::<TrackDragPayload>().is_some()
                || drag
                    .downcast_ref::<PlaylistTrackDragPayload>()
                    .is_some_and(|entry| {
                        playlist_accepts_drop(Some(entry.playlist_id), playlist_id)
                    })
        })
        .drag_over::<TrackDragPayload>(move |el, _, _, _| el.border_2().border_color(accent))
        .drag_over::<PlaylistTrackDragPayload>(move |el, _, _, _| {
            el.border_2().border_color(accent)
        })
        .on_drop(
            move |payload: &TrackDragPayload, window: &mut Window, cx: &mut App| {
                let drop = PlaylistDrop {
                    track_ids: payload.track_ids.clone(),
                    playlist_id,
                };
                on_track_drop(&drop, window, cx);
            },
        )
        .on_drop(
            move |payload: &PlaylistTrackDragPayload, window: &mut Window, cx: &mut App| {
                let drop = PlaylistDrop {
                    track_ids: vec![payload.track_id],
                    playlist_id,
                };
                on_drop(&drop, window, cx);
            },
        );
    div()
        .relative()
        .flex_shrink_0()
        .child(row)
        .child(target)
        .into_any_element()
}

#[cfg(test)]
mod track_drag_tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use gpui::{
        div, prelude::*, AppContext, Modifiers, MouseButton, Pixels, Point, TestAppContext,
        VisualTestContext,
    };

    use super::{playlist_drop_target, PlaylistDrop, PlaylistDropHandler};
    use crate::ui::composites::{ListRow, SelectableText, TrackRow};
    use crate::view_models::library::TrackDragVm;
    use crate::view_models::list_selection::RowSelectionVm;
    use crate::view_models::track_detail::TrackRowVm;

    /// The recorded results of one test window.
    #[derive(Default)]
    struct Record {
        drops: Vec<PlaylistDrop>,
        clicks: usize,
    }

    struct DropTest {
        record: Rc<RefCell<Record>>,
        /// The focus of the column that holds the source row.
        column_focus: gpui::FocusHandle,
        drag: Option<TrackDragVm>,
        /// The selection facts of the source row. A checkbox shows in
        /// `Selecting`.
        selection: RowSelectionVm,
        show_source: bool,
    }

    fn row_vm() -> TrackRowVm {
        TrackRowVm {
            element_key: "track:42".to_owned(),
            number: Some("1".to_owned()),
            title: "Face Pale".to_owned(),
            subtitle: None,
            duration: Some("3:00".to_owned()),
            date_text: None,
            a11y_label: "Face Pale".to_owned(),
        }
    }

    impl gpui::Render for DropTest {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            cx: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            let drops = Rc::clone(&self.record);
            let on_drop: PlaylistDropHandler = Rc::new(move |drop, _, _| {
                drops.borrow_mut().drops.push(drop.clone());
            });
            let clicks = Rc::clone(&self.record);
            let source = self.show_source.then(|| {
                TrackRow::from_vm("drag-source", &row_vm())
                    .selection(self.selection.clone())
                    .drag(self.drag.clone())
                    .on_click(move |_, _, _| clicks.borrow_mut().clicks += 1)
            });
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(SelectableText::new(
                    "drop-text-before",
                    "Text before the track row",
                ))
                .child(
                    div()
                        .w(gpui::px(400.))
                        .track_focus(&self.column_focus)
                        .children(source),
                )
                .child(div().debug_selector(|| "drop-target-7".to_owned()).child(
                    playlist_drop_target(
                        ListRow::new("playlist-7").child("Mix").into_any_element(),
                        7,
                        &on_drop,
                        cx,
                    ),
                ))
                .child(
                    div()
                        .debug_selector(|| "drop-text".to_owned())
                        .child(SelectableText::new(
                            "drop-text",
                            "Text after the playlist rows",
                        )),
                )
        }
    }

    struct DropRoot(gpui::Entity<gpui_component::Root>);

    impl gpui::Render for DropRoot {
        fn render(
            &mut self,
            _: &mut gpui::Window,
            _: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            div().size_full().child(self.0.clone())
        }
    }

    fn open(
        cx: &mut TestAppContext,
        drag: Option<TrackDragVm>,
    ) -> (
        Rc<RefCell<Record>>,
        gpui::Entity<DropTest>,
        &mut VisualTestContext,
    ) {
        open_with(cx, drag, RowSelectionVm::none())
    }

    fn open_with(
        cx: &mut TestAppContext,
        drag: Option<TrackDragVm>,
        selection: RowSelectionVm,
    ) -> (
        Rc<RefCell<Record>>,
        gpui::Entity<DropTest>,
        &mut VisualTestContext,
    ) {
        cx.update(gpui_component::init);
        let record = Rc::new(RefCell::new(Record::default()));
        let state_record = Rc::clone(&record);
        let state = Rc::new(RefCell::new(None));
        let state_out = Rc::clone(&state);
        let (_root, cx) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| DropTest {
                record: state_record,
                column_focus: cx.focus_handle(),
                drag,
                selection,
                show_source: true,
            });
            *state_out.borrow_mut() = Some(view.clone());
            DropRoot(cx.new(|cx| gpui_component::Root::new(view, window, cx)))
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        let view = state.borrow_mut().take().expect("the test view");
        (record, view, cx)
    }

    fn drag_fact() -> TrackDragVm {
        TrackDragVm::one(42, "Face Pale")
    }

    /// A point on the title of the source row, in its clickable body.
    fn source_point(cx: &mut VisualTestContext) -> Point<Pixels> {
        let row = cx.debug_bounds("track-row").expect("the source row");
        row.origin + gpui::point(gpui::px(120.), row.size.height / 2.)
    }

    fn target_point(cx: &mut VisualTestContext) -> Point<Pixels> {
        cx.debug_bounds("drop-target-7")
            .expect("the playlist row")
            .center()
    }

    /// R83-8-03 (ADR 0083 Decision 6, task 008): a drag from a `TrackRow`
    /// onto a playlist row calls the drop handler one time, with the track
    /// id and the playlist id. The drag does not open the row page.
    #[gpui::test]
    fn adr_0083_track_drag_r83_8_03_track_row_drops_on_a_playlist_row(cx: &mut TestAppContext) {
        let (record, _view, cx) = open(cx, Some(drag_fact()));
        let source = source_point(cx);
        let target = target_point(cx);
        cx.simulate_mouse_down(source, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(target, MouseButton::Left, Modifiers::default());
        cx.update(|_, cx| assert!(cx.has_active_drag()));
        cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::default());
        cx.update(|_, cx| assert!(!cx.has_active_drag()));
        assert_eq!(
            record.borrow().drops,
            vec![PlaylistDrop {
                track_ids: vec![42],
                playlist_id: 7,
            }]
        );
        assert_eq!(record.borrow().clicks, 0, "a drag opens no page");
    }

    /// R83-8-03, decision D1: the drag continues after its source row
    /// unmounts, as when the spring-load clears the detail column or
    /// replaces the Tracks list. The drop still reaches the playlist row.
    #[gpui::test]
    fn adr_0083_track_drag_r83_8_03_drag_continues_after_the_source_unmounts(
        cx: &mut TestAppContext,
    ) {
        let (record, view, cx) = open(cx, Some(drag_fact()));
        let source = source_point(cx);
        cx.simulate_mouse_down(source, MouseButton::Left, Modifiers::default());
        let moved = source + gpui::point(gpui::px(40.), gpui::px(0.));
        cx.simulate_mouse_move(moved, MouseButton::Left, Modifiers::default());
        cx.update(|_, cx| assert!(cx.has_active_drag()));
        view.update(cx, |this, cx| {
            this.show_source = false;
            cx.notify();
        });
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        assert!(cx.debug_bounds("track-row").is_none(), "the source is gone");
        let target = target_point(cx);
        cx.simulate_mouse_move(target, MouseButton::Left, Modifiers::default());
        cx.update(|_, cx| assert!(cx.has_active_drag(), "the drag continues"));
        cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::default());
        assert_eq!(
            record.borrow().drops,
            vec![PlaylistDrop {
                track_ids: vec![42],
                playlist_id: 7,
            }]
        );
    }

    /// R83-8-05 (decision D12): a drag from a `TrackRow` does not start a
    /// text selection. A click with no move still opens the page.
    #[gpui::test]
    fn adr_0083_track_drag_r83_8_05_drag_starts_no_text_selection(cx: &mut TestAppContext) {
        let (record, _view, cx) = open(cx, Some(drag_fact()));
        let source = source_point(cx);
        let target = target_point(cx);
        cx.simulate_mouse_down(source, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(target, MouseButton::Left, Modifiers::default());
        let text = cx.debug_bounds("drop-text").expect("the text").center();
        cx.simulate_mouse_move(text, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| {
            assert!(
                !gpui_base::TextSelection::has_selection(window, cx),
                "a track drag must not select text"
            );
        });
        cx.simulate_mouse_up(text, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| {
            assert!(!gpui_base::TextSelection::has_selection(window, cx));
        });
        // ADR 0044: after a drop, a plain move must not extend a selection.
        cx.simulate_mouse_down(source, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(target, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(text, None, Modifiers::default());
        cx.update(|window, cx| {
            assert!(
                !gpui_base::TextSelection::has_selection(window, cx),
                "a track drag left the root selecting text after the drop"
            );
        });
        record.borrow_mut().drops.clear();

        cx.simulate_mouse_down(source, MouseButton::Left, Modifiers::default());
        cx.update(|_, cx| assert!(!cx.has_active_drag()));
        cx.simulate_mouse_up(source, MouseButton::Left, Modifiers::default());
        assert_eq!(record.borrow().clicks, 1, "a click with no move opens");
        assert!(record.borrow().drops.is_empty());
    }

    /// R83-8-06 (decisions D3 and D5): a `TrackRow` with no drag fact
    /// starts no drag.
    #[gpui::test]
    fn adr_0083_track_drag_r83_8_06_row_with_no_drag_fact_starts_no_drag(cx: &mut TestAppContext) {
        let (record, _view, cx) = open(cx, None);
        let source = source_point(cx);
        let target = target_point(cx);
        cx.simulate_mouse_down(source, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(target, MouseButton::Left, Modifiers::default());
        cx.update(|_, cx| assert!(!cx.has_active_drag()));
        cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::default());
        assert!(record.borrow().drops.is_empty());
        assert_eq!(record.borrow().clicks, 0);
    }

    /// R31-10 (ADR 0083 Decision 6, amended 2026-10-21): a drag from a
    /// checked `TrackRow` in `Selecting` onto a playlist row calls the
    /// drop handler one time, with each checked track id in list order.
    #[gpui::test]
    fn adr_0083_selection_drag_r31_10_checked_row_drops_the_selection(cx: &mut TestAppContext) {
        let drag = TrackDragVm::selection(true, &[42, 7, 19]);
        let checked = RowSelectionVm::for_row("Face Pale", true, false, true);
        let (record, _view, cx) = open_with(cx, drag, checked);
        let source = source_point(cx);
        let target = target_point(cx);
        cx.simulate_mouse_down(source, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(target, MouseButton::Left, Modifiers::default());
        cx.update(|_, cx| assert!(cx.has_active_drag()));
        cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::default());
        assert_eq!(
            record.borrow().drops,
            vec![PlaylistDrop {
                track_ids: vec![42, 7, 19],
                playlist_id: 7,
            }]
        );
        assert_eq!(record.borrow().clicks, 0, "a drag changes no selection");
    }

    /// Review correction C of 2026-10-21 (ADR 0084 task 031): a click on
    /// a checked `TrackRow` in `Selecting` gives the focus to its column.
    /// The drag of the same row still starts and selects no text.
    #[gpui::test]
    fn adr_0083_selection_drag_click_on_a_checked_row_focuses_its_column(cx: &mut TestAppContext) {
        let drag = TrackDragVm::selection(true, &[42, 7]);
        let checked = RowSelectionVm::for_row("Face Pale", true, false, true);
        let (record, view, cx) = open_with(cx, drag, checked);
        let focus = cx.update(|_, cx| view.read(cx).column_focus.clone());
        let source = source_point(cx);
        cx.update(|window, _| assert!(!focus.is_focused(window)));
        cx.simulate_mouse_down(source, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(source, MouseButton::Left, Modifiers::default());
        cx.update(|window, _| {
            assert!(
                focus.is_focused(window),
                "a click on a checked row must focus its column"
            );
        });
        assert_eq!(record.borrow().clicks, 1);

        let target = target_point(cx);
        cx.simulate_mouse_down(source, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(target, MouseButton::Left, Modifiers::default());
        cx.update(|window, cx| {
            assert!(cx.has_active_drag(), "the drag still starts");
            assert!(
                !gpui_base::TextSelection::has_selection(window, cx),
                "the drag must not select text"
            );
        });
        cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::default());
        assert_eq!(
            record.borrow().drops,
            vec![PlaylistDrop {
                track_ids: vec![42, 7],
                playlist_id: 7,
            }]
        );
    }

    /// R31-11 (ADR 0083 Decision 6, amended 2026-10-21): a `TrackRow` that
    /// is not checked in `Selecting` starts no drag. Its drag fact is
    /// `None`.
    #[gpui::test]
    fn adr_0083_selection_drag_r31_11_unchecked_row_starts_no_drag(cx: &mut TestAppContext) {
        let drag = TrackDragVm::selection(false, &[42, 7]);
        assert_eq!(drag, None);
        let unchecked = RowSelectionVm::for_row("Face Pale", false, false, true);
        let (record, _view, cx) = open_with(cx, drag, unchecked);
        let source = source_point(cx);
        let target = target_point(cx);
        cx.simulate_mouse_down(source, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(target, MouseButton::Left, Modifiers::default());
        cx.update(|_, cx| assert!(!cx.has_active_drag()));
        cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::default());
        assert!(record.borrow().drops.is_empty());
    }
}
