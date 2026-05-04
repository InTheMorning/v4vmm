//! Reusable "Add to Playlist" popover component.
//!
//! ## Display contract: `AddToPlaylistDisplay`
//!
//! Wraps a trigger button that, when clicked, floats a HIG-compliant popover
//! listing all playlists with an inline "New Playlist" create flow at the bottom.
//! Exactly one popover is visible at a time; clicking outside or pressing Escape
//! dismisses it without committing any state.
//!
//! Built on top of `crate::ui::primitives` — the trigger and inline buttons
//! are `primitives::Button` variants, the floating panel body is a
//! `primitives::Surface`, and section breaks use `primitives::Divider`.
//!
//! Accessibility note (ADR 0038 task 005): trigger, option, and inline create
//! controls all receive labels through display contracts. GPUI 0.2.x does not
//! yet expose a final accessibility sink; the primitive retains the labels as
//! contract data.

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
    trigger_a11y_label: SharedString,
    new_playlist_a11y_label: SharedString,
    back_a11y_label: SharedString,
    create_a11y_label: SharedString,
    disabled: bool,
    on_select: Option<SelectHandler>,
    on_create: Option<CreateHandler>,
}

/// Display-ready input for the shared playlist popover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddToPlaylistDisplay {
    pub id: SharedString,
    pub playlists: Vec<PlaylistOption>,
    pub trigger_label: SharedString,
    pub trigger_a11y_label: SharedString,
    pub new_playlist_a11y_label: SharedString,
    pub back_a11y_label: SharedString,
    pub create_a11y_label: SharedString,
}

/// Display-ready playlist option for the shared playlist popover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistOption {
    id: i64,
    name: SharedString,
    a11y_label: SharedString,
}

/// Display-ready playlist option fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistOptionDisplay {
    pub id: i64,
    pub name: SharedString,
    pub a11y_label: SharedString,
}

impl PlaylistOption {
    /// Create a display-ready playlist option.
    #[must_use]
    pub fn new(display: PlaylistOptionDisplay) -> Self {
        Self {
            id: display.id,
            name: display.name,
            a11y_label: display.a11y_label,
        }
    }
}

impl AddToPlaylistPopover {
    /// Create a new popover from display-ready popover facts.
    pub fn new(display: AddToPlaylistDisplay) -> Self {
        Self {
            id: display.id,
            playlists: display.playlists,
            trigger_label: display.trigger_label,
            trigger_a11y_label: display.trigger_a11y_label,
            new_playlist_a11y_label: display.new_playlist_a11y_label,
            back_a11y_label: display.back_a11y_label,
            create_a11y_label: display.create_a11y_label,
            disabled: false,
            on_select: None,
            on_create: None,
        }
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
        let trigger_a11y_label = self.trigger_a11y_label;
        let new_playlist_a11y_label = self.new_playlist_a11y_label;
        let back_a11y_label = self.back_a11y_label;
        let create_a11y_label = self.create_a11y_label;
        let on_select = self.on_select;
        let on_create = self.on_create;
        let can_create = on_create.is_some();
        let trigger_id = SharedString::from(format!("{}-btn", self.id));
        let disabled = self.disabled;

        Popover::new(self.id)
            .placement(PopoverPlacement::Below)
            .alignment(PopoverAlignment::Start)
            .surface_padding(Spacing::SM)
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
                    .a11y_label(trigger_a11y_label)
                    .disabled(disabled),
            )
            .content(move |_window, cx| {
                let (creating, name_input) = {
                    let s = state.read(cx);
                    (s.creating, s.name_input.clone())
                };

                if creating {
                    build_create_mode(
                        state.clone(),
                        name_input,
                        on_create.clone(),
                        back_a11y_label.clone(),
                        create_a11y_label.clone(),
                        cx,
                    )
                } else {
                    build_list_mode(
                        state.clone(),
                        playlists.clone(),
                        on_select.clone(),
                        can_create,
                        new_playlist_a11y_label.clone(),
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
    new_playlist_a11y_label: SharedString,
    cx: &App,
) -> Div {
    let playlist_buttons = playlists.iter().map(|p| {
        let playlist_id = p.id;
        let label = p.name.clone();
        let on_select = on_select.clone();
        let state = state.clone();
        Button::plain(SharedString::from(format!("pl-{playlist_id}")))
            .full_width()
            .align_leading()
            .label(label)
            .a11y_label(p.a11y_label.clone())
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
            .align_leading()
            .leading_icon(IconName::Add)
            .label("New Playlist")
            .a11y_label(new_playlist_a11y_label)
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
    back_a11y_label: SharedString,
    create_a11y_label: SharedString,
    cx: &App,
) -> Div {
    let back_btn = Button::plain("pl-back")
        .full_width()
        .align_leading()
        .leading_icon(IconName::Back)
        .label("Back")
        .a11y_label(back_a11y_label)
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
        .a11y_label(create_a11y_label)
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
