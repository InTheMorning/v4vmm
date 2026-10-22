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

use crate::ui::control_styles::ControlStyle;
use crate::ui::icons::IconName;
use crate::ui::primitives::primary_selection::PrimarySelectionExt as _;
use crate::ui::primitives::{
    Button, ButtonSize, Divider, Popover, PopoverAlignment, PopoverPlacement,
};
use crate::ui::tokens::{FontSize, Size, Spacing};

// ---------------------------------------------------------------------------
// Callback type aliases (silence clippy::type_complexity)
// ---------------------------------------------------------------------------

pub(crate) type SelectHandler = Rc<dyn Fn(&i64, &mut Window, &mut App) + 'static>;
pub(crate) type CreateHandler = Rc<dyn Fn(&String, &mut Window, &mut App) + 'static>;
type StateHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

/// The state changes that the list mode and the create mode ask of their
/// owner. The popover owns its state. The track row menu (ADR 0083 task
/// 006) owns its own state and uses the same two modes.
#[derive(Clone)]
pub(crate) struct PlaylistChoiceControls {
    /// Closes the surface after a choice or a creation.
    pub(crate) close: StateHandler,
    /// Shows the create mode.
    pub(crate) show_create: StateHandler,
    /// Shows the list mode again.
    pub(crate) show_list: StateHandler,
}

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
    trigger_style: Option<ControlStyle>,
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
            trigger_style: None,
            on_select: None,
            on_create: None,
        }
    }

    /// Give the trigger a named control style, for example
    /// [`ControlStyle::Primary`] when "Add to playlist" is the main action
    /// of a page (ADR 0083 Decision 5). Without a style the trigger is a
    /// small tinted button.
    pub const fn trigger_style(mut self, style: ControlStyle) -> Self {
        self.trigger_style = Some(style);
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
        let trigger_a11y_label = self.trigger_a11y_label;
        let new_playlist_a11y_label = self.new_playlist_a11y_label;
        let back_a11y_label = self.back_a11y_label;
        let create_a11y_label = self.create_a11y_label;
        let on_select = self.on_select;
        let on_create = self.on_create;
        let can_create = on_create.is_some();
        let trigger_id = SharedString::from(format!("{}-btn", self.id));
        let disabled = self.disabled;
        let trigger_style = self.trigger_style;

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
                trigger_style
                    .map_or_else(
                        || Button::tinted(trigger_id.clone()).size(ButtonSize::Sm),
                        |style| Button::styled(trigger_id.clone(), style),
                    )
                    .label(trigger_label)
                    .a11y_label(trigger_a11y_label)
                    .disabled(disabled),
            )
            .content(move |_window, cx| {
                let (creating, name_input) = {
                    let s = state.read(cx);
                    (s.creating, s.name_input.clone())
                };
                let controls = popover_controls(&state);

                if creating {
                    build_create_mode(
                        &controls,
                        name_input,
                        on_create.clone(),
                        back_a11y_label.clone(),
                        create_a11y_label.clone(),
                        cx,
                    )
                } else {
                    build_list_mode(
                        &controls,
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

/// The controls of the popover: each one changes the keyed popover state.
fn popover_controls(state: &Entity<AddToPlaylistState>) -> PlaylistChoiceControls {
    let close_state = state.clone();
    let create_state = state.clone();
    let list_state = state.clone();
    PlaylistChoiceControls {
        close: Rc::new(move |_, cx| {
            close_state.update(cx, |s, cx| {
                s.open = false;
                s.creating = false;
                cx.notify();
            });
        }),
        show_create: Rc::new(move |_, cx| {
            create_state.update(cx, |s, cx| {
                s.creating = true;
                cx.notify();
            });
        }),
        show_list: Rc::new(move |_, cx| {
            list_state.update(cx, |s, cx| {
                s.creating = false;
                cx.notify();
            });
        }),
    }
}

// ---------------------------------------------------------------------------
// List mode
// ---------------------------------------------------------------------------

/// The playlist choices and "New Playlist". A choice closes the surface
/// through `controls` and then calls `on_select`.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Rc<T> handles are cloned into multiple closures; value semantics are idiomatic"
)]
pub(crate) fn build_list_mode(
    controls: &PlaylistChoiceControls,
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
        let close = controls.close.clone();
        Button::plain(SharedString::from(format!("pl-{playlist_id}")))
            .full_width()
            .align_leading()
            .label(label)
            .a11y_label(p.a11y_label.clone())
            .on_click(move |_, window, cx| {
                close(window, cx);
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
        let show_create = controls.show_create.clone();
        let new_btn = Button::plain("pl-new")
            .full_width()
            .align_leading()
            .leading_icon(IconName::Add)
            .label("New Playlist")
            .a11y_label(new_playlist_a11y_label)
            .on_click(move |_, window, cx| show_create(window, cx));
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

/// The name field, "Back" and "Create & Add". A creation closes the
/// surface through `controls` and then calls `on_create`.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Entity<T> handles are cloned into closures; value semantics are idiomatic GPUI style"
)]
pub(crate) fn build_create_mode(
    controls: &PlaylistChoiceControls,
    name_input: Entity<InputState>,
    on_create: Option<CreateHandler>,
    back_a11y_label: SharedString,
    create_a11y_label: SharedString,
    cx: &App,
) -> Div {
    let show_list = controls.show_list.clone();
    let back_btn = Button::plain("pl-back")
        .full_width()
        .align_leading()
        .leading_icon(IconName::Back)
        .label("Back")
        .a11y_label(back_a11y_label)
        .on_click(move |_, window, cx| show_list(window, cx));

    let create_btn = Button::filled("pl-create-confirm")
        .full_width()
        .label("Create & Add")
        .a11y_label(create_a11y_label)
        .on_click({
            let close = controls.close.clone();
            let name_input = name_input.clone();
            move |_, window, cx| {
                let name = name_input.read(cx).value().to_string();
                if name.trim().is_empty() {
                    return;
                }
                close(window, cx);
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
        .child({
            let field = div()
                .px(Spacing::XS.scaled(cx))
                .child(Input::new(&name_input).with_primary_selection(&name_input));
            #[cfg(test)]
            let field = field.debug_selector(|| "pl-name-field".to_owned());
            field
        })
        .child(div().px(Spacing::XS.scaled(cx)).child(create_btn))
}

// ---------------------------------------------------------------------------
// ADR 0084 task 030 P1: the popover of the release page header.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use gpui::{div, prelude::*, px, Modifiers, TestAppContext, VisualTestContext};

    use super::{AddToPlaylistDisplay, AddToPlaylistPopover};

    struct PopoverTest;

    impl gpui::Render for PopoverTest {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            let popover = AddToPlaylistPopover::new(AddToPlaylistDisplay {
                id: "hdr".into(),
                playlists: Vec::new(),
                trigger_label: "Add to playlist".into(),
                trigger_a11y_label: "Add the release to a playlist".into(),
                new_playlist_a11y_label: "New playlist".into(),
                back_a11y_label: "Back".into(),
                create_a11y_label: "Create and add".into(),
            })
            .on_create(|_, _, _| {});
            div().w(px(900.0)).h(px(800.0)).p(px(100.0)).child(popover)
        }
    }

    fn click(cx: &mut VisualTestContext, selector: &'static str) {
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
        let bounds = cx
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("{selector} must have bounds"));
        cx.simulate_click(bounds.center(), Modifiers::default());
        cx.run_until_parked();
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
    }

    /// R30-04 (ADR 0083 Decision 6): the popover of the release page
    /// header shows its create mode after a click on "New Playlist".
    #[gpui::test]
    fn adr_0084_music_open_defects_r30_04_header_popover_shows_create_mode(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (_view, cx) = cx.add_window_view(|_, _| PopoverTest);
        click(cx, "hdr-btn");
        click(cx, "pl-new");
        assert!(cx.debug_bounds("pl-name-field").is_some());
        assert!(cx.debug_bounds("pl-create-confirm").is_some());
    }
}
