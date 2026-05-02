//! Reusable "Add to Playlist" popover component.
//!
//! Wraps a trigger button that, when clicked, floats a HIG-compliant popover
//! listing all playlists with an inline "New Playlist" create flow at the bottom.
//! Exactly one popover is visible at a time; clicking outside or pressing Escape
//! dismisses it without committing any state.
//!
//! Built on top of `crate::ui::primitives` — the trigger and inline buttons
//! are `primitives::Button` variants, the floating panel body is a
//! `primitives::Surface`, and section breaks use `primitives::Divider`.

#![warn(clippy::pedantic)]

use std::rc::Rc;

use gpui::{div, prelude::*, App, Div, Entity, IntoElement, RenderOnce, SharedString, Window};
use gpui_component::{
    input::{Input, InputState},
    v_flex,
};

use crate::ui::icons::IconName;
use crate::ui::primitives::{
    Button, ButtonSize, Divider, Popover, PopoverAlignment, PopoverPlacement,
};
use crate::ui::tokens::{FontSize, Size, Spacing};

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
#[derive(IntoElement)]
#[must_use]
pub struct AddToPlaylistPopover {
    id: SharedString,
    playlists: Vec<PlaylistOption>,
    trigger_label: SharedString,
    disabled: bool,
    on_select: Option<SelectHandler>,
    on_create: Option<CreateHandler>,
}

/// Display-ready playlist option for the shared playlist popover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistOption {
    id: i64,
    name: SharedString,
}

impl PlaylistOption {
    /// Create a display-ready playlist option.
    pub fn new(id: i64, name: impl Into<SharedString>) -> Self {
        Self {
            id,
            name: name.into(),
        }
    }
}

impl AddToPlaylistPopover {
    /// Create a new popover with the given `id` and playlist list.
    pub fn new(id: impl Into<SharedString>, playlists: Vec<PlaylistOption>) -> Self {
        Self {
            id: id.into(),
            playlists,
            trigger_label: "+ Playlist".into(),
            disabled: false,
            on_select: None,
            on_create: None,
        }
    }

    /// Override the trigger label for release-level actions.
    pub fn trigger_label(mut self, label: impl Into<SharedString>) -> Self {
        self.trigger_label = label.into();
        self
    }

    /// Disable the trigger when the surrounding screen action is unavailable.
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called when the user selects an existing playlist (receives its id).
    pub fn on_select(mut self, handler: impl Fn(&i64, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Called when the user creates a new playlist (receives the name string).
    pub fn on_create(mut self, handler: impl Fn(&String, &mut Window, &mut App) + 'static) -> Self {
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
        let trigger_label = self.trigger_label;
        let on_select = self.on_select;
        let on_create = self.on_create;
        let can_create = on_create.is_some();
        let trigger_id = SharedString::from(format!("{}-btn", self.id));
        let disabled = self.disabled;

        Popover::new(self.id)
            .placement(PopoverPlacement::Below)
            .alignment(PopoverAlignment::Start)
            .surface_padding(Spacing::XS)
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
                // HIG: secondary inline action — a tinted button reads cleanly
                // on every row background our tokens emit, and is far more
                // discoverable than a plain ghost label.
                Button::tinted(trigger_id)
                    .size(ButtonSize::Sm)
                    .label(trigger_label)
                    .disabled(disabled),
            )
            .content(move |_window, cx| {
                let (creating, name_input) = {
                    let s = state.read(cx);
                    (s.creating, s.name_input.clone())
                };

                if creating {
                    build_create_mode(state.clone(), name_input, on_create.clone(), cx)
                } else {
                    build_list_mode(
                        state.clone(),
                        playlists.clone(),
                        on_select.clone(),
                        can_create,
                        cx,
                    )
                }
            })
    }
}

// ---------------------------------------------------------------------------
// List mode
// ---------------------------------------------------------------------------

#[expect(
    clippy::needless_pass_by_value,
    reason = "Entity<T> and Rc<T> are cheap handles cloned into multiple closures; value semantics are idiomatic"
)]
fn build_list_mode(
    state: Entity<AddToPlaylistState>,
    playlists: Rc<Vec<PlaylistOption>>,
    on_select: Option<SelectHandler>,
    can_create: bool,
    cx: &App,
) -> Div {
    let playlist_buttons = playlists.iter().map(|p| {
        let playlist_id = p.id;
        let label = p.name.clone();
        let on_select = on_select.clone();
        let state = state.clone();
        Button::plain(SharedString::from(format!("pl-{playlist_id}")))
            .full_width()
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

    let mut content = v_flex()
        .w(Size::MenuRegular.scaled(cx))
        .max_h(Size::ColumnRegular.scaled(cx))
        .gap(Spacing::XXS.scaled(cx))
        .when(playlists.is_empty(), |el: Div| {
            el.child(
                div()
                    .px(Spacing::MD.scaled(cx))
                    .py(Spacing::SM.scaled(cx))
                    .text_size(FontSize::Caption.scaled(cx))
                    .child("No playlists yet"),
            )
        })
        .children(playlist_buttons);

    if can_create {
        let new_btn = Button::plain("pl-new")
            .full_width()
            .leading_icon(IconName::Add)
            .label("New Playlist")
            .on_click({
                let state = state.clone();
                move |_, _window, cx| {
                    state.update(cx, |s, cx| {
                        s.creating = true;
                        cx.notify();
                    });
                }
            });
        content = content
            .child(
                div()
                    .my(Spacing::XS.scaled(cx))
                    .child(Divider::horizontal()),
            )
            .child(new_btn);
    }

    content
}

// ---------------------------------------------------------------------------
// Create mode
// ---------------------------------------------------------------------------

#[expect(
    clippy::needless_pass_by_value,
    reason = "Entity<T> handles are cloned into closures; value semantics are idiomatic GPUI style"
)]
fn build_create_mode(
    state: Entity<AddToPlaylistState>,
    name_input: Entity<InputState>,
    on_create: Option<CreateHandler>,
    cx: &App,
) -> Div {
    let back_btn = Button::plain("pl-back")
        .full_width()
        .leading_icon(IconName::Back)
        .label("Back")
        .on_click({
            let state = state.clone();
            move |_, _window, cx| {
                state.update(cx, |s, cx| {
                    s.creating = false;
                    cx.notify();
                });
            }
        });

    let create_btn = Button::filled("pl-create-confirm")
        .full_width()
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
        .w(Size::MenuRegular.scaled(cx))
        .gap(Spacing::XS.scaled(cx))
        .child(back_btn)
        .child(
            div()
                .my(Spacing::XS.scaled(cx))
                .child(Divider::horizontal()),
        )
        .child(
            div()
                .px(Spacing::XS.scaled(cx))
                .child(Input::new(&name_input)),
        )
        .child(div().px(Spacing::XS.scaled(cx)).child(create_btn))
}
