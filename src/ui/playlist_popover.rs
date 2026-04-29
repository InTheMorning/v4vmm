//! Reusable "Add to Playlist" popover component.
//!
//! Wraps a trigger button that, when clicked, floats a HIG-compliant popover
//! listing all playlists with an inline "New Playlist" create flow at the bottom.
//! Exactly one popover is visible at a time; clicking outside or pressing Escape
//! dismisses it without committing any state.

use std::rc::Rc;

use gpui::{prelude::*, Corner, Div, div, px, App, Entity, SharedString, Window};
use gpui_component::{
    Sizable, Size,
    button::{Button, ButtonCustomVariant, ButtonVariants as _},
    divider::Divider,
    input::{Input, InputState},
    popover::Popover,
    v_flex,
};

use crate::db;
use crate::ui::theme::color;

// ---------------------------------------------------------------------------
// Callback type aliases (silence clippy::type_complexity)
// ---------------------------------------------------------------------------

type SelectHandler = Rc<dyn Fn(&i64, &mut Window, &mut App) + 'static>;
type CreateHandler = Rc<dyn Fn(&String, &mut Window, &mut App) + 'static>;

// ---------------------------------------------------------------------------
// Internal state (not pub — callers interact via builder methods)
// ---------------------------------------------------------------------------

struct AddToPlaylistState {
    open: bool,
    creating: bool,
    name_input: Entity<InputState>,
}

// ---------------------------------------------------------------------------
// Public component
// ---------------------------------------------------------------------------

/// A floating "Add to Playlist" popover anchored to a small trigger button.
///
/// The popover renders in two modes controlled by an internal `creating` flag:
///
/// * **List mode** — shows every playlist as a tappable row, followed by a
///   "New Playlist" entry that switches to create mode.
/// * **Create mode** — shows a text field and a "Create" button plus a back
///   arrow to return to list mode without committing.
///
/// # Examples
///
/// ```rust
/// AddToPlaylistPopover::new("add-track-42", playlists)
///     .on_select(cx.listener(|this, id: &i64, window, cx| {
///         this.add_track_to_playlist(track_id, *id, window, cx);
///     }))
///     .on_create(cx.listener(|this, name: &String, window, cx| {
///         this.create_playlist_and_add(name.clone(), track_id, window, cx);
///     }))
/// ```
#[derive(IntoElement)]
pub struct AddToPlaylistPopover {
    id: SharedString,
    playlists: Vec<db::Playlist>,
    on_select: Option<SelectHandler>,
    on_create: Option<CreateHandler>,
}

impl AddToPlaylistPopover {
    /// Creates the component with the playlist data to display.
    ///
    /// `id` must be unique across every simultaneously rendered instance (e.g.
    /// include the track or feed ID so two rows don't collide).
    pub fn new(id: impl Into<SharedString>, playlists: Vec<db::Playlist>) -> Self {
        Self {
            id: id.into(),
            playlists,
            on_select: None,
            on_create: None,
        }
    }

    /// Called with the chosen playlist's `id` when the user selects a row.
    pub fn on_select(
        mut self,
        handler: impl Fn(&i64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Called with the entered name when the user confirms a new playlist.
    pub fn on_create(
        mut self,
        handler: impl Fn(&String, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_create = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for AddToPlaylistPopover {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state_key = SharedString::from(format!("{}-state", self.id));
        let state: Entity<AddToPlaylistState> =
            window.use_keyed_state(state_key, cx, |window, cx| AddToPlaylistState {
                open: false,
                creating: false,
                name_input: cx.new(|cx| InputState::new(window, cx).placeholder("Playlist name")),
            });

        let open = state.read(cx).open;
        let playlists = Rc::new(self.playlists);
        let on_select = self.on_select;
        let on_create = self.on_create;
        let trigger_id = SharedString::from(format!("{}-btn", self.id));

        let trigger_style = ButtonCustomVariant::new(cx)
            .foreground(color::accent().into())
            .border(color::border_strong().into())
            .hover(color::bg_surface_hi().into())
            .active(color::bg_selected().into());

        Popover::new(self.id)
            .anchor(Corner::TopLeft)
            .overlay_closable(true)
            .open(open)
            .on_open_change({
                let state = state.clone();
                move |is_open: &bool, _window, cx| {
                    state.update(cx, |s, cx| {
                        s.open = *is_open;
                        if !*is_open {
                            s.creating = false;
                        }
                        cx.notify();
                    });
                }
            })
            .trigger(
                Button::new(trigger_id)
                    .custom(trigger_style)
                    .outline()
                    .with_size(Size::Small)
                    .label("+ Playlist"),
            )
            .content(move |_ps, _window, cx| {
                let (creating, name_input) = {
                    let s = state.read(cx);
                    (s.creating, s.name_input.clone())
                };

                if creating {
                    build_create_mode(state.clone(), name_input, on_create.clone())
                } else {
                    build_list_mode(state.clone(), playlists.clone(), on_select.clone())
                }
            })
    }
}

// ---------------------------------------------------------------------------
// List mode
// ---------------------------------------------------------------------------

fn build_list_mode(
    state: Entity<AddToPlaylistState>,
    playlists: Rc<Vec<db::Playlist>>,
    on_select: Option<SelectHandler>,
) -> Div {
    let playlist_buttons = playlists.iter().map(|p| {
        let playlist_id = p.id;
        let label = SharedString::from(p.name.clone());
        let on_select = on_select.clone();
        let state = state.clone();
        Button::new(SharedString::from(format!("pl-{playlist_id}")))
            .ghost()
            .w_full()
            .label(label)
            .on_click(move |_, window, cx| {
                state.update(cx, |s, cx| {
                    s.open = false;
                    cx.notify();
                });
                if let Some(cb) = &on_select {
                    cb(&playlist_id, window, cx);
                }
            })
    });

    let new_btn = Button::new("pl-new")
        .ghost()
        .w_full()
        .label("＋ New Playlist")
        .on_click({
            let state = state.clone();
            move |_, _window, cx| {
                state.update(cx, |s, cx| {
                    s.creating = true;
                    cx.notify();
                });
            }
        });

    v_flex()
        .w(px(220.))
        .max_h(px(320.))
        .when(playlists.is_empty(), |el: Div| {
            el.child(
                div()
                    .px_3()
                    .py_2()
                    .opacity(0.5)
                    .child("No playlists yet"),
            )
        })
        .children(playlist_buttons)
        .child(Divider::horizontal())
        .child(new_btn)
}

// ---------------------------------------------------------------------------
// Create mode
// ---------------------------------------------------------------------------

fn build_create_mode(
    state: Entity<AddToPlaylistState>,
    name_input: Entity<InputState>,
    on_create: Option<CreateHandler>,
) -> Div {
    let back_btn = Button::new("pl-back")
        .ghost()
        .w_full()
        .label("← Back")
        .on_click({
            let state = state.clone();
            move |_, _window, cx| {
                state.update(cx, |s, cx| {
                    s.creating = false;
                    cx.notify();
                });
            }
        });

    let create_btn = Button::new("pl-create-confirm")
        .w_full()
        .label("Create & Add")
        .on_click({
            let state = state.clone();
            let name_input = name_input.clone();
            move |_, window, cx| {
                let name = name_input.read(cx).value().to_string();
                if name.trim().is_empty() {
                    return;
                }
                state.update(cx, |s, cx| {
                    s.open = false;
                    s.creating = false;
                    cx.notify();
                });
                name_input.update(cx, |s, cx| {
                    s.set_value("", window, cx);
                });
                if let Some(cb) = &on_create {
                    cb(&name, window, cx);
                }
            }
        });

    v_flex()
        .w(px(220.))
        .gap_2()
        .p_1()
        .child(back_btn)
        .child(Divider::horizontal())
        .child(Input::new(&name_input))
        .child(create_btn)
}
